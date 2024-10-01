use std::{thread::sleep, time::Duration};

use crossterm::{
    event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
    terminal::enable_raw_mode,
};
use file2dl::Download;

use crate::file2dl::File2Dl;
mod errors;
mod file2dl;
mod tmp;
mod url;
mod utils;

fn main() {
    let file = File2Dl::new(
        "https://filesampleshub.com/download/document/txt/sample2.txt",
        "Downloads",
    )
    .unwrap();
    file.single_thread_dl().unwrap();
    // let collection = File2Dl::from("Downloads").unwrap();
    // for file in collection {
    //     dbg!(file);
    // }
}
