#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
use std::{
    path::Path,
    sync::{mpsc, Arc},
};

use dl::{file2dl::File2Dl, utils::count_files};
use dl_display::display_interface;
use eframe::egui::{self, mutex::Mutex, Color32, Separator};
use extern_windows::{
    show_bandwidth_edit_window, show_confirm_window, show_error_window, show_input_window,
};
use menu_bar::init_menu_bar;
use select::select_all;
use status_bar::display_status_bar;
mod dl_display;
mod extern_windows;
mod menu_bar;
mod select;
mod status_bar;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum Threading {
    #[default]
    Single,
    Multi,
}

struct DownloadInterface {
    error: String,
    url: String,
    bandwidth: String,
    show: bool,
    threading: Threading,
    threads: String,
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
            threading: Threading::default(),
            threads: String::default(),
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
    threading: Threading,
    threads: usize,
}
struct Connected {
    connected: Arc<Mutex<bool>>,
    started: bool,
}
impl Default for Connected {
    fn default() -> Self {
        Self {
            connected: Arc::new(Mutex::new(false)),
            started: false,
        }
    }
}

struct MyApp {
    inner: Vec<Core>,
    popus: PopUps,
    select_all: bool,
    connected_to_net: Connected,
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
                    connected_to_net: Connected::default(),
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
                threading: {
                    if Path::new(&file.dir).join(&file.name_on_disk).is_dir() {
                        Threading::Multi
                    } else {
                        Threading::Single
                    }
                },
                threads: count_files(&format!("{}/.{}", file.dir, file.name_on_disk))
                    .unwrap_or_default(),
                channel: mpsc::channel(),
            })
            .collect::<Vec<Core>>();
        Self {
            inner: core_collection,
            popus: PopUps::default(),
            connected_to_net: Connected::default(),
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
        if self.popus.bandwidth.show {
            show_bandwidth_edit_window(ctx, self, &self.popus.bandwidth.to_edit.clone());
        }
        if self.popus.error.show {
            show_error_window(ctx, self, &self.popus.error.value.clone());
        }
        select_all(self);
    }
}
