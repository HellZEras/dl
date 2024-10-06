use crate::errors::{FileDownloadError, UrlError};
use crate::tmp::{init_tmp, MetaData};
use crate::url::Url;
use crate::utils::{convert_mbs, gen, get_file_size, init_req};
use reqwest::blocking::Client;
use reqwest::header::RANGE;
use std::fs::{create_dir, read_dir, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};
pub trait Download {
    fn single_thread_dl(&self) -> Result<(), FileDownloadError>;
}
impl Download for File2Dl {
    fn single_thread_dl(&self) -> Result<(), FileDownloadError> {
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
        let client = reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(15))
            .build()?;
        if self.bandwidth.load(Ordering::Relaxed) == 0 {
            download_without_bw(self, &mut file)?;
        } else {
            match self.url.range_support {
                true => download_with_range_supp(self, &mut file, &client)?,
                false => download_withou_range_supp(self, &mut file, &client)?,
            }
        }
        Ok(())
    }
}

fn download_without_bw(f2dl: &File2Dl, file: &mut File) -> Result<(), FileDownloadError> {
    let mut res = init_req(f2dl)?;

    loop {
        if f2dl.status.load(Ordering::Relaxed) {
            let mut buffer = vec![0; 8192];
            let bytes_read = res.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            buffer.truncate(bytes_read);
            file.seek(SeekFrom::Start(
                f2dl.size_on_disk.load(Ordering::Relaxed) as u64
            ))?;
            file.write_all(&buffer)?;
            f2dl.size_on_disk.fetch_add(bytes_read, Ordering::Relaxed);
        } else {
            sleep(Duration::from_millis(100));
        }
    }
    Ok(())
}

fn download_withou_range_supp(
    f2dl: &File2Dl,
    file: &mut File,
    client: &Client,
) -> Result<(), FileDownloadError> {
    let mut res = client.get(&f2dl.url.link).send()?;
    let mut buffer = vec![0; 8192];
    let mut downloaded_in_interval = 0;
    let mut interval_start = Instant::now();

    loop {
        if !f2dl.status.load(Ordering::Relaxed) {
            sleep(Duration::from_millis(100));
            continue;
        }

        let bytes_read = res.read(&mut buffer)?;
        if bytes_read == 0 {
            return Ok(());
        }

        buffer.truncate(bytes_read);

        file.seek(SeekFrom::Start(
            f2dl.size_on_disk.load(Ordering::Relaxed) as u64
        ))?;
        file.write_all(&buffer)?;

        f2dl.size_on_disk.fetch_add(bytes_read, Ordering::Relaxed);

        downloaded_in_interval += bytes_read;

        let elapsed = interval_start.elapsed();

        if downloaded_in_interval >= f2dl.bandwidth.load(Ordering::Relaxed) {
            if elapsed < Duration::from_secs(1) {
                sleep(Duration::from_secs(1) - elapsed);
            }
            downloaded_in_interval = 0;
            interval_start = Instant::now();
        }
    }
}

fn download_with_range_supp(
    f2dl: &File2Dl,
    file: &mut File,
    client: &Client,
) -> Result<(), FileDownloadError> {
    loop {
        if f2dl.status.load(Ordering::Relaxed) {
            let now = Instant::now();
            let size_on_disk = f2dl.size_on_disk.load(Ordering::Relaxed);
            let bandwidth = f2dl.bandwidth.load(Ordering::Relaxed);
            let max_range = if size_on_disk + bandwidth > f2dl.url.total_size {
                f2dl.url.total_size
            } else {
                size_on_disk + bandwidth
            };
            let mut res = client
                .get(&f2dl.url.link)
                .header(RANGE, format!("bytes={}-{}", size_on_disk, max_range))
                .send()?;
            let mut buffer = vec![0; 8192];
            let bytes_read = res.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            buffer.truncate(bytes_read);
            file.seek(SeekFrom::Start(size_on_disk as u64))?;
            file.write_all(&buffer)?;
            f2dl.size_on_disk.fetch_add(bytes_read, Ordering::Relaxed);
            if Instant::now() - now < Duration::from_secs(1) {
                sleep(Duration::from_secs(1) - (Instant::now() - now));
            }
        } else {
            sleep(Duration::from_millis(100));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct File2Dl {
    pub url: Url,
    pub size_on_disk: Arc<AtomicUsize>,
    pub status: Arc<AtomicBool>,
    pub name_on_disk: String,
    pub dir: String,
    pub bandwidth: Arc<AtomicUsize>,
}

impl File2Dl {
    pub fn switch_status(&self) {
        let current_status = self.status.load(Ordering::Relaxed);
        self.status.store(!current_status, Ordering::Relaxed);
    }

    pub fn default() -> Self {
        Self {
            url: Url::default(),
            size_on_disk: Arc::new(AtomicUsize::new(0)),
            status: Arc::new(AtomicBool::new(false)),
            name_on_disk: String::default(),
            dir: String::from("Downloads"),
            bandwidth: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn new(link: &str, dir: &str, bandwidth: f64) -> Result<Self, UrlError> {
        let url = Url::from(link)?;
        let (name_on_disk, size_on_disk) = gen(url.clone(), dir)?;
        let bandwidth_as_bytes = convert_mbs(bandwidth) as usize;
        Ok(Self {
            url,
            size_on_disk: Arc::new(AtomicUsize::new(size_on_disk)),
            status: Arc::new(AtomicBool::new(false)),
            name_on_disk,
            dir: dir.to_owned(),
            bandwidth: Arc::new(AtomicUsize::new(bandwidth_as_bytes)),
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
                        size_on_disk,
                        status: Arc::new(AtomicBool::new(false)),
                        name_on_disk: file_name,
                        dir: dir.to_string(),
                        bandwidth: Arc::new(AtomicUsize::new(meta_data.bandwidth)),
                    };
                    collection.push(file2dl);
                }
            }
        }
        Ok(collection)
    }
}
