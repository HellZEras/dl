use thiserror::Error;

#[derive(Error, Debug)]
pub enum UrlError {
    #[error("Not a valid url")]
    InvalidUrl,
    #[error("Couldn't reach url")]
    FailedRequest(#[from] reqwest::Error),
}

#[derive(Error, Debug)]
pub enum DirectoryError {
    #[error("Directory provided is invalid")]
    InvalidDir,
    #[error("Failed to parse directory provided")]
    DirParse(#[from] std::io::Error),
}
#[derive(Error, Debug)]
pub enum FileDownloadError {
    #[error("Indexing downloads directory failed")]
    DirIndex(#[from] DirectoryError),
    #[error("Creating file failed")]
    FileCreation(#[from] std::io::Error),
}
