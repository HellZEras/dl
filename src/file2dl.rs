use crate::errors::{FileDownloadError, UrlError};
use crate::tmp::init_tmp_if_supported;
use crate::url::Url;
use crate::utils::{gen_name, init_req};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::mpsc::channel;
use std::thread::{sleep, JoinHandle};
use std::time::Duration;

pub trait Download {
    fn single_thread_dl(self, dir: &str) -> Result<(), FileDownloadError>;
}
impl Download for File2Dl {
    fn single_thread_dl(mut self, dir: &str) -> Result<(), FileDownloadError> {
        if !Path::new(dir).exists() {
            create_dir(dir)?
        }
        let filename = gen_name(&self, dir)?;
        let full_path = format!("{dir}/{}", &filename);
        let mut file = File::create(full_path.clone())?;
        let mut res = init_req(&self)?;
        let (tx, rx) = channel::<(Vec<u8>, usize)>();
        let write: JoinHandle<Result<(), FileDownloadError>> = std::thread::spawn(move || {
            while let Ok((buffer, offset)) = rx.recv() {
                file.seek(SeekFrom::Start(offset as u64)).unwrap();
                file.write(&buffer).unwrap();
            }
            Ok(())
        });
        let mut threads = Vec::new();
        let dir = dir.to_string();
        let send: JoinHandle<Result<(), FileDownloadError>> = std::thread::spawn(move || {
            loop {
                if self.status {
                    let mut buffer = vec![0; 8192];
                    let bytes_read = res.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    buffer.truncate(bytes_read);
                    tx.send((buffer.clone(), self.size_on_disk))?;
                    self.size_on_disk += bytes_read;
                    println!("{}%", self.size_on_disk / self.url.total_size * 100);
                    init_tmp_if_supported(&self, &filename, &dir)?;
                } else {
                    sleep(Duration::from_millis(100))
                }
            }
            Ok(())
        });
        threads.push(write);
        threads.push(send);
        for thread in threads {
            let packed_thread = thread.join().expect("Failed to join thread");
            packed_thread?
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct File2Dl {
    pub url: Url,
    pub size_on_disk: usize,
    pub status: bool,
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
        }
    }
    pub fn new(link: &str) -> Result<Self, UrlError> {
        let url = Url::from(link)?;
        Ok(Self {
            url,
            size_on_disk: 0,
            status: false,
        })
    }
}
