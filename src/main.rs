#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
use std::{borrow::Borrow, net::TcpStream, time::Duration};

use crate::file2dl::File2Dl;
use eframe::egui::{self, Color32, Layout, Separator};
use gui::{
    dl_display::display_interface,
    extern_windows::{
        show_bandwidth_edit_window, show_confirm_window, show_error_window, show_input_window,
    },
    menu_bar::init_menu_bar,
    select::select_all,
    status_bar::display_status_bar,
};

mod errors;
mod file2dl;
mod gui;
mod tmp;
mod url;
mod utils;
struct DownloadInterface {
    error: String,
    url: String,
    bandwidth: String,
    show: bool,
    error_channel: (
        std::sync::mpsc::Sender<String>,
        std::sync::mpsc::Receiver<String>,
    ),
}
impl Default for DownloadInterface {
    fn default() -> Self {
        Self {
            error: String::default(),
            url: String::default(),
            bandwidth: String::default(),
            show: false,
            error_channel: std::sync::mpsc::channel(),
        }
    }
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
    bandwidth: BandwidthInterface,
}
#[derive(Debug, Default, PartialEq, Eq)]
enum BandwidthUnit {
    Kbs,
    #[default]
    Mbs,
    Gbs,
}
#[derive(Default)]
struct BandwidthInterface {
    error: String,
    value: String,
    show: bool,
    unit: BandwidthUnit,
    to_edit: String,
}
struct ConfirmInterface {
    text: String,
    color: Color32,
    show: bool,
    task: Box<dyn Fn() -> Box<dyn FnOnce(&mut MyApp)>>,
}
impl Default for ConfirmInterface {
    fn default() -> Self {
        ConfirmInterface {
            text: String::new(),
            color: Color32::default(),
            show: false,
            task: Box::new(|| Box::new(|_app: &mut MyApp| {})),
        }
    }
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
    connected_to_net: bool,
    file_channel: (
        std::sync::mpsc::Sender<File2Dl>,
        std::sync::mpsc::Receiver<File2Dl>,
    ),
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
                    bandwidth: BandwidthInterface::default(),
                };
                return Self {
                    inner: Vec::default(),
                    popus,
                    connected_to_net: false,
                    select_all: false,
                    file_channel: std::sync::mpsc::channel(),
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
            connected_to_net: false,
            select_all: false,
            file_channel: std::sync::mpsc::channel(),
        }
    }
}

fn main() -> Result<(), eframe::Error> {
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
            display_interface(self, ui, ctx);
        });
        display_status_bar(ctx, self);
        if self.popus.download.show {
            show_input_window(ctx, self);
        }
        ctx.request_repaint();
        if self.popus.confirm.show {
            let task = (self.popus.confirm.task)();
            show_confirm_window(
                ctx,
                self,
                self.popus.confirm.color,
                &self.popus.confirm.text.clone(),
                task,
            );
        }
        self.connected_to_net = is_connected();
        if self.popus.bandwidth.show {
            show_bandwidth_edit_window(ctx, self, &self.popus.bandwidth.to_edit.clone());
        }
        if self.popus.error.show {
            show_error_window(ctx, self, &self.popus.error.value.clone());
        }
        select_all(self);
    }
}

fn is_connected() -> bool {
    TcpStream::connect_timeout(&("8.8.8.8:53".parse().unwrap()), Duration::from_secs(2)).is_ok()
}
