use std::fs::File;

use crate::file2dl::File2Dl;
use eframe::egui::{self, Color32, Separator, Vec2};
use file2dl::Download;
use gui::{
    dl_display::display_interface,
    extern_windows::{show_confirm_window, show_input_window},
    menu_bar::init_menu_bar,
    select::select_all,
};

mod errors;
mod file2dl;
mod gui;
mod tmp;
mod url;
mod utils;
#[derive(Default)]
struct DownloadInterface {
    error: String,
    url: String,
    bandwidth: String,
    show: bool,
}
#[derive(Default)]
struct ErrorInterface {
    value: String,
    show: bool,
}

#[derive(Default)]
struct PopUps {
    error: ErrorInterface,
    confirm: ConfirmInterface,
    download: DownloadInterface,
}
#[derive(Default)]
struct ConfirmInterface {
    value: String,
    color: Color32,
    show: bool,
}
struct Core {
    file: File2Dl,
    started: bool,
    selected: bool,
    channel: (
        std::sync::mpsc::Sender<String>,
        std::sync::mpsc::Receiver<String>,
    ),
}
struct MyApp {
    inner: Vec<Core>,
    popus: PopUps,
    select_all: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        let collection = match File2Dl::from("Downloads") {
            Ok(collection) => collection,
            Err(e) => {
                let popus = PopUps {
                    error: ErrorInterface {
                        value: e.to_string(),
                        show: true,
                    },
                    confirm: ConfirmInterface::default(),
                    download: DownloadInterface::default(),
                };
                return Self {
                    inner: Vec::default(),
                    popus: popus,
                    select_all: false,
                };
            }
        };
        let core_collection = collection
            .iter()
            .map(|file| Core {
                file: file.to_owned(),
                started: false,
                selected: false,
                channel: std::sync::mpsc::channel(),
            })
            .collect::<Vec<Core>>();
        Self {
            inner: core_collection,
            popus: PopUps::default(),
            select_all: false,
        }
    }
}

fn main() -> eframe::Result<()> {
    // let file = File2Dl::new(
    //     "https://filesampleshub.com/download/document/txt/sample2.txt",
    //     "Downloads",
    //     0.0,
    // )
    // .unwrap();
    // file.switch_status();
    // file.single_thread_dl().unwrap();
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Download Manager",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            init_menu_bar(self, ui);
            ui.add(Separator::grow(Separator::default(), ui.available_width()));
            display_interface(self, ui);
        });
        if self.popus.download.show {
            show_input_window(ctx, self);
        }
        if self.popus.confirm.show {
            show_confirm_window(
                ctx,
                self,
                &self.popus.confirm.value.clone(),
                self.popus.confirm.color,
            );
        }
        select_all(self);
        ctx.request_repaint();
    }
}
