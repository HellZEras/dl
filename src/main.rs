use crate::file2dl::{Download, File2Dl};

mod errors;
mod url;
mod file2dl;

#[tokio::main]
async fn main() {
    let file = File2Dl::from("https://examplefile.com/file-download/22").await.unwrap();
    dbg!(&file);
    file.single_thread_dl("Downloads").await.unwrap()
}
