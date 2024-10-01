use crate::file2dl::State::Incomplete;
use crate::file2dl::{File2Dl, State};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::Ordering;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MetaData {
    pub filename: String,
    pub link: String,
    pub total_size: usize,
    pub state: State,
}
pub fn init_meta_data(path: &Path, file: &str) -> Result<Option<MetaData>, std::io::Error> {
    let tmp_file = format!(".{file}.metadata");
    let tmp_path = path.join(tmp_file);
    if tmp_path.exists() {
        let mut file = File::open(tmp_path)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        let data: MetaData = serde_json::from_str(&buf)?;
        Ok(Some(data))
    } else {
        Ok(None)
    }
}
pub fn init_tmp_if_supported(f2dl: &File2Dl, filename: &str) -> Result<(), std::io::Error> {
    if f2dl.url.range_support {
        let json_str = {
            let size_on_disk = f2dl.size_on_disk.load(Ordering::Relaxed);
            let state = {
                if size_on_disk == f2dl.url.total_size {
                    State::Complete
                } else {
                    Incomplete
                }
            };
            let tmp_str = MetaData {
                filename: filename.to_owned(),
                link: f2dl.url.link.clone(),
                total_size: f2dl.url.total_size,
                state,
            };
            json!(tmp_str).to_string()
        };
        let tmp_name = format!("{}/.{}.metadata", f2dl.dir, filename);
        let mut tmp_file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(tmp_name)?;
        tmp_file.write_all(json_str.as_bytes())?;
    }

    Ok(())
}
