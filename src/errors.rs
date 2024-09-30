use std::sync::mpsc::SendError;
use thiserror::Error;
use tokio::task::JoinHandle;

#[derive(Error, Debug)]
pub enum UrlError {
    #[error("Not a valid url")]
    InvalidUrl,
    #[error("Couldn't reach url")]
    FailedRequest(#[from] reqwest::Error),
}

#[derive(Error, Debug)]
pub enum DirectoryError {
    #[error("Failed to parse directory provided")]
    DirParse(#[from] std::io::Error),
}
#[derive(Error, Debug)]
pub enum FileDownloadError {
    #[error("Indexing downloads directory failed")]
    DirIndex(#[from] DirectoryError),
    #[error("Creating file failed")]
    FileCreation(#[from] std::io::Error),
    #[error("Request failed")]
    RequestFailure(#[from] reqwest::Error),
    #[error("Sending the data to be written failed")]
    WatchChannel(#[from] SendError<(Vec<u8>, usize)>),
}
