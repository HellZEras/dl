use crate::errors::{FileDownloadError, UrlError};
use crate::tmp::init_tmp_if_supported;
use crate::url::Url;
use crate::utils::{gen_name, init_req};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

pub trait Download {
    fn single_thread_dl(&mut self) -> Result<(), FileDownloadError>;
}
impl Download for File2Dl {
    fn single_thread_dl(&mut self) -> Result<(), FileDownloadError> {
        if !Path::new(&self.dir).exists() {
            create_dir(&self.dir)?
        }
        let full_path = format!("{}/{}", &self.dir, &self.name_on_disk);
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .truncate(false)
            .open(full_path.clone())?;
        let mut res = init_req(self)?;

        loop {
            if self.status.load(Ordering::Relaxed) {
                let mut buffer = vec![0; 8192];
                let bytes_read = res.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                buffer.truncate(bytes_read);
                file.seek(SeekFrom::Start(
                    self.size_on_disk.load(Ordering::Relaxed) as u64
                ))?;
                file.write_all(&buffer)?;
                self.size_on_disk.fetch_add(bytes_read, Ordering::Relaxed);
                init_tmp_if_supported(self, &self.name_on_disk)?;
            } else {
                sleep(Duration::from_millis(100));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct File2Dl {
    pub url: Url,
    pub size_on_disk: Arc<AtomicUsize>,
    pub status: Arc<AtomicBool>,
    pub name_on_disk: String,
    pub dir: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum State {
    Complete,
    #[default]
    Incomplete,
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
        }
    }

    pub fn new(link: &str, dir: &str) -> Result<Self, UrlError> {
        let url = Url::from(link)?;
        let (name_on_disk, size_on_disk) = gen_name(url.clone(), dir)?;
        Ok(Self {
            url,
            size_on_disk: Arc::new(AtomicUsize::new(size_on_disk)),
            status: Arc::new(AtomicBool::new(false)),
            name_on_disk,
            dir: dir.to_owned(),
        })
    }
}
