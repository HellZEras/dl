use futures_util::StreamExt;
use reqwest::header::RANGE;
use reqwest::Client;
use tokio::time::{self, sleep, Instant};

use crate::errors::{FileDownloadError, UrlError};
use crate::tmp::{init_tmp, MetaData};
use crate::url::Url;
use crate::utils::{convert_mbs, gen, get_file_size};
use std::fs::{create_dir, read_dir, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
pub trait Download {
    async fn single_thread_dl(&mut self) -> Result<(), FileDownloadError>;
}
impl Download for File2Dl {
    async fn single_thread_dl(&mut self) -> Result<(), FileDownloadError> {
        if !Path::new(&self.dir).exists() {
            create_dir(&self.dir)?
        }
        let full_path = format!("{}/{}", &self.dir, &self.name_on_disk);
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .truncate(false)
            .open(full_path.clone())?;
        if self.url.range_support {
            init_tmp(self, &self.name_on_disk)?;
        }
        let client = reqwest::ClientBuilder::new().build()?;
        download(self, &mut file, &client).await?;
        self.complete.store(true, Ordering::Relaxed);
        Ok(())
    }
}

async fn download(
    f2dl: &mut File2Dl,
    file: &mut File,
    client: &Client,
) -> Result<(), FileDownloadError> {
    let res = if f2dl.url.range_support {
        client
            .get(&f2dl.url.link)
            .header(
                RANGE,
                format!(
                    "bytes={}-{}",
                    f2dl.size_on_disk.load(Ordering::Relaxed),
                    f2dl.url.total_size
                ),
            )
            .send()
            .await?
    } else {
        client.get(&f2dl.url.link).send().await?
    };

    let mut stream = res.bytes_stream();
    let mut bytes_downloaded_in_sec: usize = 0;
    let mut last_time = Instant::now();

    while let Some(packed_chunk) = stream.next().await {
        f2dl.status.1.wait_for(|x| *x).await.unwrap();
        let chunk = packed_chunk?;
        let bandwidth = f2dl.bandwidth.load(Ordering::Relaxed);
        bytes_downloaded_in_sec += chunk.len();
        if last_time.elapsed() >= Duration::from_secs(1) {
            f2dl.transfer_rate
                .store(bytes_downloaded_in_sec, Ordering::Relaxed);
            bytes_downloaded_in_sec = 0;
            last_time = Instant::now();
        }
        file.write_all(&chunk)?;
        f2dl.size_on_disk.fetch_add(chunk.len(), Ordering::Relaxed);
        if chunk.len() as usize > bandwidth && bandwidth != 0 {
            if last_time.elapsed() < Duration::from_secs(1) {
                sleep(time::Duration::from_secs(1) - last_time.elapsed()).await;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct File2Dl {
    pub url: Url,
    pub size_on_disk: Arc<AtomicUsize>,
    pub status: (
        tokio::sync::watch::Sender<bool>,
        tokio::sync::watch::Receiver<bool>,
    ),
    pub name_on_disk: String,
    pub dir: String,
    pub bandwidth: Arc<AtomicUsize>,
    pub complete: Arc<AtomicBool>,
    pub transfer_rate: Arc<AtomicUsize>,
}

impl File2Dl {
    pub fn switch_status(&mut self) -> Result<(), tokio::sync::watch::error::SendError<bool>> {
        let rx = !*self.status.1.borrow();
        let tx = &self.status.0;
        tx.send(rx)
    }

    pub fn default() -> Self {
        Self {
            url: Url::default(),
            size_on_disk: Arc::new(AtomicUsize::new(0)),
            status: tokio::sync::watch::channel(false),
            name_on_disk: String::default(),
            dir: String::from("Downloads"),
            bandwidth: Arc::new(AtomicUsize::new(0)),
            complete: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            transfer_rate: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn new(link: &str, dir: &str, bandwidth: f64) -> Result<Self, UrlError> {
        let url = Url::from(link).await?;
        let (name_on_disk, size_on_disk) = gen(url.clone(), dir)?;
        let bandwidth_as_bytes = convert_mbs(bandwidth) as usize;
        Ok(Self {
            url,
            size_on_disk: Arc::new(AtomicUsize::new(size_on_disk)),
            status: tokio::sync::watch::channel(false),
            name_on_disk,
            dir: dir.to_owned(),
            bandwidth: Arc::new(AtomicUsize::new(bandwidth_as_bytes)),
            complete: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            transfer_rate: Arc::new(AtomicUsize::new(0)),
        })
    }
    pub fn from(dir: &str) -> Result<Vec<File2Dl>, std::io::Error> {
        let mut collection: Vec<File2Dl> = Vec::new();
        let path = Path::new(dir);
        if path.is_dir() {
            for entry in read_dir(path)? {
                let file = entry?;
                let file_name = {
                    let os_str = file.file_name();
                    os_str
                        .to_str()
                        .expect("Failed to parse filename")
                        .to_string()
                };

                if file_name.contains(".metadata") {
                    continue;
                }
                let tmp_file_name = format!(".{}.metadata", &file_name);
                let tmp_path = Path::new(dir).join(tmp_file_name);
                if tmp_path.exists() {
                    let mut buffer = String::new();
                    let mut tmp_file = File::open(tmp_path)?;
                    tmp_file.read_to_string(&mut buffer)?;
                    let meta_data: MetaData = serde_json::from_str(&buffer)?;
                    let file2dl_path = Path::new(dir).join(&file_name);
                    let size_on_disk = Arc::new(AtomicUsize::new(get_file_size(&file2dl_path)?));
                    let url = Url {
                        link: meta_data.link,
                        filename: Some(meta_data.file_name),
                        total_size: meta_data.total_size,
                        range_support: true,
                    };
                    let file2dl = File2Dl {
                        url,
                        size_on_disk: size_on_disk.clone(),
                        status: tokio::sync::watch::channel(false),
                        name_on_disk: file_name,
                        dir: dir.to_string(),
                        bandwidth: Arc::new(AtomicUsize::new(meta_data.bandwidth)),
                        complete: Arc::new(std::sync::atomic::AtomicBool::new(
                            meta_data.total_size == size_on_disk.load(Ordering::Relaxed),
                        )),
                        transfer_rate: Arc::new(AtomicUsize::new(0)),
                    };
                    collection.push(file2dl);
                }
            }
        }
        Ok(collection)
    }
}
