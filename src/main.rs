use std::{
    sync::{Arc, Mutex},
    thread::sleep,
    time::Duration,
};

use arrayvec::ArrayString;
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
        "https://dl.google.com/tag/s/appguid%3D%7B8A69D345-D564-463C-AFF1-A69D9E530F96%7D%26iid%3D%7B1174C0E1-F11C-F91A-BDB4-818C8DAE6F5E%7D%26lang%3Den%26browser%3D3%26usagestats%3D0%26appname%3DGoogle%2520Chrome%26needsadmin%3Dprefers%26ap%3Dx64-statsdef_1%26installdataindex%3Dempty/chrome/install/ChromeStandaloneSetup64.exe",
        "Downloads",
    )
    .unwrap();
    dbg!(&file);

    // Wrap the File2Dl instance in an Arc
    let file = Arc::new(file);
    let closure_clone = file.clone();

    enable_raw_mode().unwrap();

    // Spawning a thread to listen for 'p' key press to toggle the status
    std::thread::spawn(move || loop {
        if let Ok(Event::Key(KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })) = read()
        {
            let current_status = closure_clone
                .status
                .load(std::sync::atomic::Ordering::Relaxed);
            closure_clone
                .status
                .store(!current_status, std::sync::atomic::Ordering::Relaxed);
            println!("toggled");
        }
        sleep(Duration::from_millis(100));
    });

    // Start single-threaded download (single_thread_dl)
    file.clone().single_thread_dl().unwrap();
    dbg!(&file);
}
