use crate::file2dl::{Download, File2Dl};
use crossterm::event::{read, Event, KeyEvent};
use crossterm::terminal::enable_raw_mode;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod errors;
mod file2dl;
mod tmp;
mod url;
mod utils;

fn main() {
    let file =
        File2Dl::new("https://filesampleshub.com/download/document/txt/sample3.txt").unwrap();
    let mfile = Arc::new(Mutex::new(file));
    let mfile_clone = mfile.clone();
    enable_raw_mode().unwrap();
    // Spawn a new thread to listen for key events.
    thread::spawn(move || loop {
        match read().unwrap() {
            Event::Key(KeyEvent {
                code: crossterm::event::KeyCode::Char('p'),
                modifiers: crossterm::event::KeyModifiers::NONE,
                kind: crossterm::event::KeyEventKind::Press,
                state: crossterm::event::KeyEventState::NONE,
            }) => {
                let mut lock = mfile_clone.lock().unwrap();
                lock.switch_status();
                println!("{}", lock.status);
            }
            _ => {}
        }
    });
    let lock = mfile.lock().unwrap();
    lock.clone().single_thread_dl("Downloads").unwrap()
}
