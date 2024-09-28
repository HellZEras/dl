use crate::errors::{FileDownloadError, UrlError};
use crate::url::Url;
use futures_util::StreamExt;
use random_string::generate;
use reqwest::header::RANGE;
use reqwest::ClientBuilder;
use std::fs::{create_dir, File};
use std::ops::Deref;
use std::os::windows::fs::FileExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::watch;
use tokio::sync::watch::error::SendError;

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
        let filename = {
            if self.url.filename.is_some() {
                Self::gen_if_some(
                    dir,
                    self.url.filename.as_ref().unwrap(),
                    self.url.total_size,
                )?
            } else {
                Self::gen_if_none(dir)
            }
        };
        let full_path = format!("{dir}/{filename}");
        let file = File::create(full_path)?;
        let res = {
            let client = ClientBuilder::new()
                .timeout(Duration::from_secs(7))
                .build()?;
            if self.url.range_support {
                let range_value = format!("bytes={}-{}", self.size_on_disk, self.url.total_size);
                client
                    .get(self.url.link)
                    .header(RANGE, range_value)
                    .send()
                    .await?
            } else {
                client.get(self.url.link).send().await?
            }
        };
        let mut stream = res.bytes_stream();
        while let Some(packed_bytes) = stream.next().await {
            self.status.1.wait_for(|cond| *cond).await?;
            let bytes = packed_bytes?;
            file.seek_write(&bytes, self.size_on_disk as u64)?;
            self.size_on_disk += bytes.len()
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct File2Dl {
    pub url: Url,
    pub size_on_disk: usize,
    status: Status,
}

impl File2Dl {
    pub fn switch_status(&self) -> Result<(), SendError<bool>> {
        let rx = !*self.status.1.borrow();
        let tx = &self.status.0;
        tx.send(rx)
    }

    fn get_file_size(path: &PathBuf) -> Result<usize, std::io::Error> {
        let file = File::open(path)?;
        Ok(file.metadata()?.len() as usize)
    }

    fn gen_if_some(
        dl_location: &str,
        filename: &str,
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
        }
    }
    pub async fn new(link: &str) -> Result<Self, UrlError> {
        let url = Url::from(link).await?;
        Ok(Self {
            url,
            size_on_disk: 0,
            status: watch::channel(false),
        })
    }
}
