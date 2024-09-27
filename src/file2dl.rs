use std::fs::{create_dir, read_dir, File};
use std::path::Path;
use random_string::generate;
use reqwest::Error;
use tokio::sync::watch;
use crate::errors::{DirectoryError, FileDownloadError, UrlError};
use crate::errors::DirectoryError::InvalidDir;
use crate::url::Url;

const CHARSET: &str = "abcdefghijklmnopqrstuvwxyz0123456789";

type Status = (watch::Sender<bool>, watch::Receiver<bool>);
pub trait Download {
    async fn single_thread_dl(self, dir: &str) -> Result<(), FileDownloadError>;
}
impl Download for File2Dl {
    async fn single_thread_dl(self, dir: &str) -> Result<(), FileDownloadError> {
        if !Path::new(dir).exists() {
            create_dir(dir)?
        }
        let filename = {
            if self.url.filename.is_some() {
                Self::gen_if_some(dir, self.url.filename.as_ref().unwrap())
            } else {
                Self::gen_if_none(dir)
            }
        };
        let full_path = format!("{dir}/{filename}");
        File::create(full_path)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct File2Dl {
    pub url: Url,
    pub size_on_disk: usize,
    status: Status,
}

impl File2Dl {
    fn gen_if_some(dl_location: &str, filename: &str) -> String {
        let path = Path::new(dl_location);
        let mut index = 1;
        let mut name = filename.to_string();
        while path.join(name.clone()).exists() {
            let final_name = format!("{}_{index}", filename);
            name = final_name;
            index += 1;
        }
        name
    }
    fn gen_if_none(dl_location: &str) -> String {
        let mut filename = generate(8, CHARSET);
        let mut full_path = Path::new(dl_location);
        while full_path.join(filename.clone()).exists() {
            filename = generate(8, CHARSET);
        }
        format!("{}.unknown", filename)
    }
    fn index_dl_directory(dir: &str) -> Result<Vec<String>, DirectoryError> {
        if !Path::new(dir).is_dir() {
            return Err(InvalidDir);
        }

        let vec = read_dir(dir)?
            .filter_map(|entry| {
                if let Ok(path) = entry {
                    path.file_name().to_str().map(|filename| filename.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();

        Ok(vec)
    }
    pub fn default() -> Self {
        Self {
            url: Url::default(),
            size_on_disk: 0,
            status: watch::channel(false),
        }
    }
    pub async fn from(link: &str) -> Result<Self, UrlError> {
        let url = Url::from(link).await?;
        Ok(
            Self {
                url,
                size_on_disk: 0,
                status: watch::channel(false),
            }
        )
    }
}