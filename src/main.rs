use errors::FileDownloadError;

use crate::file2dl::{Download, File2Dl};

mod errors;
mod file2dl;
mod url;

#[tokio::main]
async fn main() -> Result<(), FileDownloadError> {
    let file = File2Dl::new("https://filesampleshub.com/download/document/txt/sample1.txt")
        .await
        .unwrap();
    dbg!(&file);
    file.switch_status().unwrap();
    file.single_thread_dl("Downloads").await?;
    Ok(())
}
