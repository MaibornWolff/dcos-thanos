use std::collections::HashMap;
use std::{thread, time};
use serde_derive::{Serialize, Deserialize};
use serde_json::Value;
use reqwest::header;


#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Dashboard {
    pub dashboard: Value,
    pub folderId: u32,
    pub overwrite: bool
}

#[derive(Serialize, Deserialize)]
struct Folder {
    pub title: String,
    pub uid: String,
    pub id: u32,
}

#[derive(Serialize, Deserialize)]
struct FolderCreateInfo {
    pub title: String,
    pub uid: Option<String>,
}


#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Datasource {
    pub id: Option<u32>,
    pub name: String,
    pub r#type: String,
    pub url: String,
    pub access: String,
    pub basicAuth: bool,
    pub isDefault: bool,
}


pub struct GrafanaAPI {
    base_url: String,
    username: String,
    password: String,
    client: reqwest::blocking::Client,
    folders: HashMap<String, Folder>,
    folders_loaded: bool
}


impl GrafanaAPI {

    pub fn new(base_url: String, username: String, password: String) -> GrafanaAPI {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/json"));
        let client = reqwest::blocking::ClientBuilder::new()
            .danger_accept_invalid_certs(true).default_headers(headers)
            .connect_timeout(time::Duration::from_secs(2))
            .build().unwrap();
        GrafanaAPI{base_url, username, password, client, folders: HashMap::new(), folders_loaded: false}
    }

    pub fn wait_for_healthy(&mut self) {
        loop {
            match self.client.get(&String::from(format!("{}/api/health", self.base_url))).send() {
                Ok(response) => {
                    if response.status().is_success() {
                        break;
                    }
                },
                Err(e) => {
                    println!("{}", e);
                }
            }
            thread::sleep(time::Duration::from_secs(5));
        };
    }

    fn load_folders(&mut self) {
        let response = self.get("/api/folders").send().unwrap();
        if !response.status().is_success() {
            println!("{}", response.text().unwrap());
            panic!("Could not load existing folders");
        }
        let result: Vec<Folder> = response.json().unwrap();
        for folder in result {
            self.folders.insert(folder.title.clone(), folder);
        }
        self.folders_loaded = true;
    }

    #[allow(unused_must_use)]
    pub fn clear_dashboards(&mut self) {
        let response = self.get("/api/folders").send().unwrap();
        if response.status().is_success() {
            let result: Vec<Folder> = response.json().unwrap();
            for folder in result {
                println!("Deleting folder {} with id {} and uid {}", folder.title, folder.id, folder.uid);
                // Ignore errors and continue
                self.delete(&format!("/api/folders/{}", folder.uid)).send();
            }
        }
    }

    pub fn get_or_create_folder(&mut self, path: String) -> u32 {
        if !self.folders_loaded {
            self.load_folders();
        }
        if self.folders.contains_key(&path) {
            self.folders.get(&path).unwrap().id
        } else {
            self.create_folder(path, None)
        }
    }

    fn create_folder(&mut self, title: String, uid: Option<String>) -> u32 {
        let folder = FolderCreateInfo{title, uid};
        let folder_json = serde_json::to_string(&folder).unwrap();
        let response = self.post("/api/folders").body(folder_json).send().unwrap();
        if !response.status().is_success() {
            println!("{}", response.text().unwrap());
            panic!("Could not create folder");
        }
        let folder: Folder = response.json().unwrap();
        let id = folder.id;
        self.folders.insert(folder.title.clone(), folder);
        id
    }

    pub fn upload_dashboard(&mut self, dashboard: String) {
        let response = self.post("/api/dashboards/db").body(dashboard).send().unwrap();
        if !response.status().is_success() {
            println!("{}", response.text().unwrap());
            panic!("Could not upload dashboard");
        }
    }

    pub fn create_or_update_datasource(&mut self, datasource: Datasource) {
        let datasource_json = serde_json::to_string(&datasource).unwrap();
        if let Ok(response) = self.get(&format!("/api/datasources/name/{}", datasource.name)).send() {
            if response.status().is_success() {
                let existing_datasource: Datasource = response.json().unwrap();
                // update
                let response = self.put(&format!("/api/datasources/{}", existing_datasource.id.unwrap())).body(datasource_json).send().unwrap();
                if !response.status().is_success() {
                    println!("{}", response.text().unwrap());
                    panic!("Could not update datasource");
                }
                return;
            }
        }
        // create
        let response = self.post("/api/datasources").body(datasource_json).send().unwrap();
        if !response.status().is_success() {
            println!("{}", response.text().unwrap());
            panic!("Could not create datasource");
        }
    }

    fn get(&mut self, endpoint: &str) -> reqwest::blocking::RequestBuilder {
        self.client.get(&String::from(format!("{}{}", self.base_url, endpoint))).basic_auth(&self.username, Some(&self.password))
    }

    fn post(&mut self, endpoint: &str) -> reqwest::blocking::RequestBuilder {
        self.client.post(&String::from(format!("{}{}", self.base_url, endpoint))).basic_auth(&self.username, Some(&self.password))
    }

    fn put(&mut self, endpoint: &str) -> reqwest::blocking::RequestBuilder {
        self.client.put(&String::from(format!("{}{}", self.base_url, endpoint))).basic_auth(&self.username, Some(&self.password))
    }

    fn delete(&mut self, endpoint: &str) -> reqwest::blocking::RequestBuilder {
        self.client.delete(&String::from(format!("{}{}", self.base_url, endpoint))).basic_auth(&self.username, Some(&self.password))
    }

}