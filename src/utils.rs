use crate::file2dl::File2Dl;
use crate::file2dl::State::Incomplete;
use crate::tmp::{init_meta_data, MetaData};
use crate::url::Url;
use random_string::generate;
use std::fs::{read_dir, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use reqwest::blocking::{ClientBuilder, Response};
use reqwest::header::RANGE;
use tokio::sync::watch::channel;

const CHARSET: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

fn get_file_size(path: &PathBuf) -> Result<usize, std::io::Error> {
    let file = File::open(path)?;
    Ok(file.metadata()?.len() as usize)
}
//if name is Some in the struct it will check if the size on disk of the file is smaller than content length
//and in that case it returns that same file name instead of the generated name to resume download
//else it would generate a name with a counter to avoid duplication
pub fn gen_if_some(dl_location: &str, filename: &str, url: &str) -> Result<String, std::io::Error> {
    let path = Path::new(dl_location);
    if path.join(filename).exists() {
        let tmp_file = format!("{filename}.metadata");
        let tmp_path = path.join(tmp_file);
        if tmp_path.exists() {
            let mut file = File::open(&tmp_path)?;
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            let meta_data = serde_json::from_str::<MetaData>(&buf)?;
            if meta_data.state == Incomplete
                && meta_data.link == url
                && get_file_size(&path.join(filename))? == meta_data.size_on_disk
            {
                return Ok(filename.to_owned());
            }
        }
    }

    let mut counter = 2;
    let mut name = filename.to_string();

    while path.join(name.clone()).exists() {
        name = filename.to_string();
        let point_index = filename.find('.').unwrap();
        let insert_str = format!("_{counter}");
        name.insert_str(point_index, &insert_str);

        counter += 1;
    }
    Ok(name)
}
pub fn gen_if_none(dl_location: &str) -> String {
    let mut filename = generate(8, CHARSET);
    let full_path = Path::new(dl_location);
    while full_path.join(filename.clone()).exists() {
        filename = generate(8, CHARSET);
    }
    format!("{}.unknown", filename)
}

pub fn gen_name(f2dl: &File2Dl, dl_location: &str) -> Result<String, std::io::Error> {
    if f2dl.url.filename.is_some() {
        Ok(gen_if_some(
            dl_location,
            f2dl.url.filename.as_ref().unwrap(),
            &f2dl.url.link,
        )?)
    } else {
        Ok(gen_if_none(dl_location))
    }
}
fn build_file2dl(collection: Vec<Option<MetaData>>) -> Vec<File2Dl> {
    collection
        .into_iter()
        .filter_map(|packed_metadata| {
            packed_metadata.map(|metadata| {
                let url = Url {
                    link: metadata.link,
                    filename: Some(metadata.filename),
                    total_size: metadata.total_size,
                    range_support: true,
                };
                File2Dl {
                    url,
                    size_on_disk: metadata.size_on_disk,
                    status: false,
                }
            })
        })
        .collect()
}
pub fn from_dir(dir: &str) -> Result<Vec<File2Dl>, std::io::Error> {
    let path: &Path = Path::new(dir);
    let mut collection = Vec::new();
    if path.is_dir() && path.exists() {
        for packed_file in read_dir(path)? {
            if let Some(file) = packed_file?.file_name().to_str() {
                if !file.contains(".metadata") {
                    let tmp_data = init_meta_data(path, file)?;
                    collection.push(tmp_data);
                }
            }
        }
    }
    let processed_collection = build_file2dl(collection);
    Ok(processed_collection)
}

pub fn init_req(file2dl: &File2Dl) -> Result<Response, reqwest::Error> {
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(7))
        .build()?;
    if file2dl.url.range_support {
        let range_value = format!("bytes={}-{}", file2dl.size_on_disk, file2dl.url.total_size);
        Ok(client
            .get(file2dl.url.link.clone())
            .header(RANGE, range_value)
            .send()?
        )
    } else {
        Ok(client.get(file2dl.url.link.clone()).send()?)
    }
}