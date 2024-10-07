use crate::file2dl::File2Dl;
use crate::tmp::MetaData;
use crate::url::Url;
use random_string::generate;
use reqwest::blocking::{ClientBuilder, Response};
use reqwest::header::RANGE;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

const CHARSET: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

pub fn get_file_size(path: &PathBuf) -> Result<usize, std::io::Error> {
    let file = File::open(path)?;
    Ok(file.metadata()?.len() as usize)
}
//if name is Some in the struct it will check if the size on disk of the file is smaller than content length
//and in that case it returns that same file name instead of the generated name to resume download
//else it would generate a name with a counter to avoid duplication
pub fn gen_if_name_some(
    link: &str,
    dir: &str,
    filename: &str,
    total_size: usize,
) -> Result<(String, usize), std::io::Error> {
    let path = Path::new(dir);
    if path.join(filename).exists() {
        let tmp_file = format!(".{}.metadata", filename);
        let tmp_path = path.join(tmp_file);
        if tmp_path.exists() {
            let mut file = File::open(&tmp_path)?;
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            let meta_data = serde_json::from_str::<MetaData>(&buf)?;
            let file_size = get_file_size(&path.join(filename))?;
            if file_size < total_size
                && meta_data.link == link
                && total_size == meta_data.total_size
            {
                return Ok((filename.to_owned(), file_size));
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
    Ok((name, 0))
}
pub fn gen_if_name_none(dl_location: &str) -> (String, usize) {
    let mut filename = generate(8, CHARSET);
    let full_path = Path::new(dl_location);
    while full_path.join(&filename).exists() {
        filename = generate(8, CHARSET);
    }
    (format!("{}.unknown", filename), 0)
}

pub fn gen(url: Url, dir: &str) -> Result<(String, usize), std::io::Error> {
    let filename = url.filename;
    if filename.clone().is_some() {
        Ok(gen_if_name_some(
            &url.link,
            dir,
            &filename.unwrap_or_default(),
            url.total_size,
        )?)
    } else {
        Ok(gen_if_name_none(dir))
    }
}

pub fn convert_mbs(mbs: f64) -> f64 {
    mbs * 1024.0 * 1024.0
}
pub fn init_req(file2dl: &File2Dl) -> Result<Response, reqwest::Error> {
    let client = ClientBuilder::new().build()?;
    if file2dl.url.range_support {
        let range_value = format!(
            "bytes={}-{}",
            file2dl.size_on_disk.load(Ordering::Relaxed),
            file2dl.url.total_size
        );
        Ok(client
            .get(file2dl.url.link.clone())
            .header(RANGE, range_value)
            .send()?)
    } else {
        Ok(client.get(file2dl.url.link.clone()).send()?)
    }
}
