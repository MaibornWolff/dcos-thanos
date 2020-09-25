use std::io;
use std::path::Path;
use serde_json::Value;

mod grafana;

use grafana::{GrafanaAPI, Dashboard, Datasource};


fn main() {
    let grafana_url = std::env::var("GRAFANA_URL").unwrap();
    let grafana_username = std::env::var("GRAFANA_USERNAME").unwrap();
    let grafana_password = std::env::var("GRAFANA_PASSWORD").unwrap();
    let mut api = GrafanaAPI::new(grafana_url, grafana_username, grafana_password);

    api.wait_for_healthy();

    match std::env::args().nth(1).unwrap().as_str() {
        "dashboards" => load_dashboards(&mut api),
        "datasources" => load_datasources(&mut api),
        "all" => {
            load_datasources(&mut api);
            load_dashboards(&mut api);
        },
        _ => panic!("Unknown command")
    }
}

fn load_datasources(api: &mut GrafanaAPI) {
    println!("Loading datasources");
    let datasource_url = std::env::var("GRAFANA_DATASOURCE_URL").unwrap();
    let datasource = Datasource{id: None, name: String::from("prometheus"), r#type: String::from("prometheus"), url: datasource_url, access: String::from("proxy"), basicAuth: false, isDefault: true};
    api.create_or_update_datasource(datasource);
    println!("Finished creating/updating grafana datasources");
}

fn load_dashboards(api: &mut GrafanaAPI) {
    let fetcher_url = std::env::var("FETCHER_URL").unwrap();
    let base_folder = std::env::var("FETCHER_BASE_FOLDER").unwrap_or(String::from(""));
    let grafana_clear_dashboards = std::env::var("GRAFANA_CLEAR_DASHBOARDS").unwrap_or(String::from("")).to_lowercase() == "true";

    let client = reqwest::blocking::ClientBuilder::new().danger_accept_invalid_certs(true).build().unwrap();
    let response = client.get(&fetcher_url).send();
    if response.is_err() {
        stop();
    }
    let mut response = response.unwrap();
    if !response.status().is_success() {
        stop();
    }
    let mut buf: Vec<u8> = vec![];
    response.copy_to(&mut buf).unwrap();
    let cursor = io::Cursor::new(buf);

    if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
        if grafana_clear_dashboards {
            api.clear_dashboards();
        }
        for i in 0..archive.len() {
            let file = archive.by_index(i).unwrap();
            #[allow(deprecated)]
            let outpath = file.sanitized_name();
            // process only json files
            if outpath.starts_with(&base_folder) && (&*file.name()).ends_with(".json") {
                let folder = outpath.parent().unwrap_or(Path::new("")).strip_prefix(&base_folder).unwrap();
                let filename = outpath.file_name().unwrap();

                if let Ok(val) = serde_json::from_reader(file) {
                    process_dashboard(api, String::from(folder.to_str().unwrap()), String::from(filename.to_str().unwrap()), val);
                } else {
                    println!("Could not parse {}/{}, ignoring it", folder.to_str().unwrap(), filename.to_str().unwrap());
                }
            }
        }
    } else {
        stop();
    }
}


fn process_dashboard(api: &mut GrafanaAPI, folder: String, filename: String, mut data: Value) {
    println!("Uploading {}/{}", folder, filename);
    let obj = data.as_object_mut().unwrap();
    if !obj.contains_key("title") || obj.get("title").unwrap().as_str().unwrap() == "" {
        obj["title"] = Value::String(filename.replace(".json", ""));
    }
    // null id field to avoid collisions with possibly existing dashboards/folders with the same id
    if obj.contains_key("id") {
        obj.insert(String::from("id"), Value::Null);
    }
    let mut vars = Vec::new();
    if obj.contains_key("__inputs") {
        let inputs = obj.get("__inputs").unwrap().as_array().unwrap();
        for input in inputs {
            let input_obj = input.as_object().unwrap();
            let name = String::from(input_obj.get("name").unwrap().as_str().unwrap());
            let input_type = input_obj.get("type").unwrap().as_str().unwrap();
            let value = match input_type {
                "datasource" => input_obj.get("label").unwrap().as_str().unwrap(),
                "constant" => input_obj.get("value").unwrap().as_str().unwrap(),
                _ => ""
            };
            vars.push((name, String::from(value)));
        }
    }

    let dashboard = Dashboard{dashboard: data, folderId: determine_folder_id(api, folder), overwrite: true};
    let mut dashboard_json = serde_json::to_string(&dashboard).unwrap();
    for (key, value) in vars.iter() {
        dashboard_json = dashboard_json.replace(&format!("${{{}}}", key), value);
    }
    api.upload_dashboard(dashboard_json);
}


fn determine_folder_id(api: &mut GrafanaAPI, folder: String) -> u32 {
    if folder == "" {
        0 // "General"
    } else {
        api.get_or_create_folder(folder)
    }
}


fn stop() {
    println!("Could not download dashboards. Stopping here");
    std::process::exit(0);
}