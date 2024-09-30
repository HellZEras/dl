use crate::errors::{FileDownloadError, UrlError};
use crate::tmp::init_tmp_if_supported;
use crate::url::Url;
use crate::utils::{gen_name, init_req};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::{sleep, JoinHandle};
use std::time::Duration;

pub trait Download {
    fn single_thread_dl(self) -> Result<(), FileDownloadError>;
}
impl Download for Arc<Mutex<File2Dl>> {
    fn single_thread_dl(self) -> Result<(), FileDownloadError> {
        let mut cpy = self.lock().unwrap().clone();
        if !Path::new(&cpy.dir).exists() {
            create_dir(&cpy.dir)?
        }
        let filename = gen_name(&cpy)?;
        let full_path = format!("{}/{}", &cpy.dir, &filename);
        let mut file = File::create(full_path.clone())?;
        let mut res = init_req(&cpy)?;

        let send: JoinHandle<Result<(), FileDownloadError>> = std::thread::spawn(move || {
            loop {
                let status = self.try_lock().unwrap().status;
                if status {
                    let mut buffer = vec![0; 8192];
                    let bytes_read = res.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    buffer.truncate(bytes_read);
                    file.seek(SeekFrom::Start(cpy.size_on_disk as u64))?;
                    file.write_all(&buffer)?;
                    cpy.size_on_disk += bytes_read;
                    println!(
                        "{}%",
                        ((cpy.size_on_disk as f64 / cpy.url.total_size as f64) * 100.0) as i64
                    );
                    init_tmp_if_supported(&cpy, &filename)?;
                    let mut lock = self.try_lock().unwrap();
                    lock.size_on_disk = cpy.size_on_disk;
                } else {
                    sleep(Duration::from_millis(100))
                }
            }
            Ok(())
        });
        send.join().expect("Failed to join thread")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct File2Dl {
    pub url: Url,
    pub size_on_disk: usize,
    pub status: bool,
    pub dir: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum State {
    Complete,
    #[default]
    Incomplete,
}

impl File2Dl {
    pub fn switch_status(&mut self) {
        self.status = !self.status;
    }

    pub fn default() -> Self {
        Self {
            url: Url::default(),
            size_on_disk: 0,
            status: false,
            dir: String::from("Downloads"),
        }
    }
    pub fn new(link: &str, dir: &str) -> Result<Self, UrlError> {
        let url = Url::from(link)?;
        Ok(Self {
            url,
            size_on_disk: 0,
            status: false,
            dir: dir.to_owned(),
        })
    }
}
