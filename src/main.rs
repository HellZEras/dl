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

    enable_raw_mode().unwrap();
    let copy = file.clone();
    std::thread::spawn(move || loop {
        if let Ok(Event::Key(KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })) = read()
        {
            let current_status = copy.status.load(std::sync::atomic::Ordering::Relaxed);
            copy.status
                .store(!current_status, std::sync::atomic::Ordering::Relaxed);
            println!("toggled");
        }
        sleep(Duration::from_millis(100));
    });

    file.clone().single_thread_dl().unwrap();
}
