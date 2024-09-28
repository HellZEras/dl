use crate::errors::{FileDownloadError, UrlError};
use crate::tmp::init_tmp_if_supported;
use crate::url::Url;
use crate::utils::{gen_name, init_req};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tokio::sync::watch::error::SendError;
use tokio::sync::watch::{self, channel};

type Status = (watch::Sender<bool>, watch::Receiver<bool>);
pub trait Download {
    async fn single_thread_dl(self, dir: &str) -> Result<(), FileDownloadError>;
}
impl Download for File2Dl {
    async fn single_thread_dl(mut self, dir: &str) -> Result<(), FileDownloadError> {
        if !Path::new(dir).exists() {
            create_dir(dir)?
        }
        let filename = gen_name(&self, dir)?;
        let full_path = format!("{dir}/{filename}");
        let mut file = File::create(full_path.clone())?;
        let mut stream = {
            let res = init_req(&self).await?;
            res.bytes_stream()
        };
        while let Some(packed_bytes) = stream.next().await {
            self.status.1.wait_for(|cond| *cond).await?;
            let bytes = packed_bytes?;
            file.seek(SeekFrom::Start(self.size_on_disk as u64))?;
            file.write_all(bytes.as_ref())?;
            self.size_on_disk += bytes.len();
            init_tmp_if_supported(&self, &filename, dir)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct File2Dl {
    pub url: Url,
    pub size_on_disk: usize,
    pub status: Status,
    pub state: State,
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

    pub fn default() -> Self {
        Self {
            url: Url::default(),
            size_on_disk: 0,
            status: channel(false),
            state: State::default(),
        }
    }
    pub async fn new(link: &str) -> Result<Self, UrlError> {
        let url = Url::from(link).await?;
        Ok(Self {
            url,
            size_on_disk: 0,
            status: channel(false),
            state: State::default(),
        })
    }
}
