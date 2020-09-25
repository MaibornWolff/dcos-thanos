use std::io;
use std::io::prelude::*;
use std::fs;
use std::path::Path;


fn main() {
    let target_dir_var = std::env::var("FETCHER_TARGET_DIR").unwrap_or(String::from("."));
    let target_dir = Path::new(&target_dir_var);
    let url = match std::env::args().nth(1) {
        Some(s) => s,
        None => std::env::var("FETCHER_URL").unwrap()
    };
    let client = reqwest::blocking::ClientBuilder::new().danger_accept_invalid_certs(true).build().unwrap();
    let mut response = client.get(&url).send().unwrap();
    let mut buf: Vec<u8> = vec![];
    response.copy_to(&mut buf).unwrap();
    if url.ends_with(".zip") {
        let cursor = io::Cursor::new(buf);
        extract_zip(&target_dir, cursor);
    } else {
        let filename = url.rsplit("/").next().unwrap();
        let path = target_dir.join(filename);
        write_file(&path, buf.as_slice());
    }
}


fn write_file(filename: &Path, buf: &[u8]) {
    let mut file = fs::File::create(filename).unwrap();
    file.write(buf).unwrap();
}


fn extract_zip<R: io::Read + io::Seek>(target_dir: &Path, reader: R) {
    let mut archive = zip::ZipArchive::new(reader).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        #[allow(deprecated)]
        let outpath = target_dir.join(file.sanitized_name());

        if (&*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }
}
