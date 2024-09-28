use errors::FileDownloadError;
use file2dl::{Download, State};

use crate::file2dl::File2Dl;

mod errors;
mod file2dl;
mod tmp;
mod url;
mod utils;

#[tokio::main]
async fn main() -> Result<(), FileDownloadError> {
    let files = File2Dl::new("https://filesampleshub.com/download/document/txt/sample3.txt")
        .await
        .unwrap();
    files.switch_status().unwrap();
    files.single_thread_dl("Downloads").await.unwrap();
    Ok(())
}
