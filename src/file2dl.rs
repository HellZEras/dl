use crate::errors::{FileDownloadError, UrlError};
use crate::url::Url;
use futures_util::StreamExt;
use random_string::generate;
use reqwest::header::RANGE;
use reqwest::{ClientBuilder, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{create_dir, read_dir, File, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::fs::FileExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::watch::error::SendError;
use tokio::sync::watch::{self, channel};

const CHARSET: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

type Status = (watch::Sender<bool>, watch::Receiver<bool>);
pub trait Download {
    async fn single_thread_dl(self, dir: &str) -> Result<(), FileDownloadError>;
}
impl Download for File2Dl {
    async fn single_thread_dl(mut self, dir: &str) -> Result<(), FileDownloadError> {
        if !Path::new(dir).exists() {
            create_dir(dir)?
        }
        let filename = self.gen_name(dir)?;
        dbg!(filename.clone());
        let full_path = format!("{dir}/{filename}");
        let file = File::create(full_path.clone())?;
        let mut stream = {
            let res = self.init_req().await?;
            res.bytes_stream()
        };
        while let Some(packed_bytes) = stream.next().await {
            self.status.1.wait_for(|cond| *cond).await?;
            let bytes = packed_bytes?;
            file.seek_write(&bytes, self.size_on_disk as u64)?;
            self.size_on_disk += bytes.len();
            self.init_tmp_if_supported(&filename, dir)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct File2Dl {
    pub url: Url,
    pub size_on_disk: usize,
    status: Status,
    pub state: State,
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MetaData {
    filename: String,
    link: String,
    size_on_disk: usize,
    total_size: usize,
    state: State,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum State {
    Complete,
    #[default]
    Incomplete,
}

impl File2Dl {
    pub fn switch_status(&self) -> Result<(), SendError<bool>> {
        let rx = !*self.status.1.borrow();
        let tx = &self.status.0;
        tx.send(rx)
    }
    fn init_tmp_if_supported(&self, filename: &str, dir: &str) -> Result<(), std::io::Error> {
        if self.url.range_support {
            let json_str = {
                let state = {
                    if self.size_on_disk == self.url.total_size {
                        State::Complete
                    } else {
                        State::Incomplete
                    }
                };
                let tmp_str = MetaData {
                    filename: filename.to_owned(),
                    link: self.url.link.clone(),
                    size_on_disk: self.size_on_disk,
                    total_size: self.url.total_size,
                    state,
                };
                json!(tmp_str).to_string()
            };
            let full_path = format!("{dir}/{filename}");
            let tmp_name = format!("{}.metadata", full_path);
            let mut tmp_file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(tmp_name)?;
            tmp_file.write_all(json_str.as_bytes())?;
        }

        Ok(())
    }
    async fn init_req(&self) -> Result<Response, reqwest::Error> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(7))
            .build()?;
        if self.url.range_support {
            let range_value = format!("bytes={}-{}", self.size_on_disk, self.url.total_size);
            Ok(client
                .get(self.url.link.clone())
                .header(RANGE, range_value)
                .send()
                .await?)
        } else {
            Ok(client.get(self.url.link.clone()).send().await?)
        }
    }
    fn gen_name(&self, dl_location: &str) -> Result<String, std::io::Error> {
        if self.url.filename.is_some() {
            Ok(Self::gen_if_some(
                dl_location,
                self.url.filename.as_ref().unwrap(),
                &self.url.link,
                self.url.total_size,
            )?)
        } else {
            Ok(Self::gen_if_none(dl_location))
        }
    }

    fn get_file_size(path: &PathBuf) -> Result<usize, std::io::Error> {
        let file = File::open(path)?;
        Ok(file.metadata()?.len() as usize)
    }
    //if name is Some in the struct it will check if the size on disk of the file is smaller than content length
    //and in that case it returns that same file name instead of the generated name to resume download
    //else it would generate a name with a counter to avoid duplication
    fn gen_if_some(
        dl_location: &str,
        filename: &str,
        url: &str,
        total_size: usize,
    ) -> Result<String, std::io::Error> {
        let path = Path::new(dl_location);
        if path.join(filename).exists() && Self::get_file_size(&path.join(filename))? < total_size {
            return Ok(filename.to_owned());
        }

        let mut counter = 2;
        let mut name = filename.to_string();

        while path.join(name.clone()).exists() {
            name = filename.to_string();
            let point_index = filename.find('.').unwrap();
            let insert_str = format!("_{counter}");
            name.insert_str(point_index, &insert_str);
            let tmp_file_name = format!("{name}.metadata");
            if path.join(tmp_file_name.clone()).exists() {
                let mut buf = String::new();
                let mut file = File::open(path.join(tmp_file_name.clone()))?;
                let current_size = file.metadata()?.len() as usize;
                file.read_to_string(&mut buf)?;
                let temp_data: MetaData = serde_json::from_str(&buf)?;
                if current_size == temp_data.size_on_disk
                    && temp_data.link == url
                    && temp_data.state == State::Incomplete
                {
                    return Ok(tmp_file_name);
                }
            }
            counter += 1;
        }
        Ok(name)
    }
    fn gen_if_none(dl_location: &str) -> String {
        let mut filename = generate(8, CHARSET);
        let full_path = Path::new(dl_location);
        while full_path.join(filename.clone()).exists() {
            filename = generate(8, CHARSET);
        }
        format!("{}.unknown", filename)
    }

    pub fn default() -> Self {
        Self {
            url: Url::default(),
            size_on_disk: 0,
            status: watch::channel(false),
            state: State::default(),
        }
    }
    pub async fn new(link: &str) -> Result<Self, UrlError> {
        let url = Url::from(link).await?;
        Ok(Self {
            url,
            size_on_disk: 0,
            status: watch::channel(false),
            state: State::default(),
        })
    }
    fn init_meta_data(path: &Path, file: &str) -> Result<Option<MetaData>, std::io::Error> {
        let size_read = File::open(path.join(file))?.metadata()?.len() as usize;
        let tmp_file = format!("{file}.metadata");
        let tmp_path = path.join(tmp_file);
        if tmp_path.exists() {
            let mut file = File::open(tmp_path)?;
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            let data: MetaData = serde_json::from_str(&buf)?;
            if data.size_on_disk != size_read {
                Ok(Some(MetaData {
                    filename: data.filename,
                    size_on_disk: 0,
                    total_size: data.total_size,
                    link: data.link,
                    state: State::Incomplete,
                }))
            } else {
                Ok(Some(data))
            }
        } else {
            Ok(None)
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
                        status: channel(false),
                        state: metadata.state,
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
                        let tmp_data = Self::init_meta_data(path, file)?;
                        collection.push(tmp_data);
                    }
                }
            }
        }
        let processed_collection = Self::build_file2dl(collection);
        Ok(processed_collection)
    }
}
