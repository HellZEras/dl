use crate::file2dl::File2Dl;
use eframe::egui::{self, menu, ProgressBar, Rounding};
use file2dl::Download;

mod errors;
mod file2dl;
mod tmp;
mod url;
mod utils;

#[derive(Default)]
struct DownloadWindow {
    input: String,
    show: bool,
}
struct Core {
    file: File2Dl,
    started: bool,
}
struct MyApp {
    inner: Vec<Core>,
    download_window: DownloadWindow,
}

impl Default for MyApp {
    fn default() -> Self {
        let core_collection = File2Dl::from("Downloads")
            .unwrap_or_default()
            .iter()
            .map(|file| Core {
                file: file.to_owned(),
                started: false,
            })
            .collect::<Vec<Core>>();
        Self {
            inner: core_collection,
            download_window: DownloadWindow::default(),
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update CentralPanel as usual
        egui::CentralPanel::default().show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Add").clicked() {
                                self.download_window.show = true;
                            }
                        });
                    });
                });
            });
            ui.separator();

            for core in self.inner.iter_mut() {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(&core.file.name_on_disk);

                        let progress = core
                            .file
                            .size_on_disk
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let done = progress == core.file.url.total_size;
                        let progress_fraction = progress as f32 / core.file.url.total_size as f32;

                        // Improved ProgressBar
                        let progress_bar = ProgressBar::new(progress_fraction)
                            .desired_width(200.0)
                            .show_percentage()
                            .rounding(Rounding::ZERO);
                        ui.add(progress_bar);

                        // Fetching file status
                        let status = core.file.status.load(std::sync::atomic::Ordering::Relaxed);
                        let file_clone = core.file.clone();

                        // Ensure download thread starts only once
                        if status && !core.started {
                            std::thread::spawn(move || {
                                file_clone.single_thread_dl().unwrap();
                            });
                            core.started = true;
                        }
                        if !done {
                            if !status {
                                if ui.button("Resume").clicked() {
                                    core.file.switch_status();
                                }
                            } else {
                                if ui.button("Pause").clicked() {
                                    core.file.switch_status();
                                }
                            }
                        } else {
                            ui.label("Download complete");
                        }
                    });
                });
            }
        });

        // Download window to enter URL
        if self.download_window.show {
            let screen_rect = ctx.screen_rect();
            let window_size = egui::vec2(300.0, 200.0);
            let center = screen_rect.center() - 0.5 * window_size;
            egui::Window::new("Add Download")
                .open(&mut self.download_window.show.clone())
                .default_size(window_size)
                .default_pos(center)
                .show(ctx, |ui| {
                    ui.label("Enter URL:");
                    ui.text_edit_singleline(&mut self.download_window.input);
                    if ui.button("Confirm").clicked() {
                        let file = File2Dl::new(&self.download_window.input, "Downloads").unwrap();
                        file.switch_status();
                        let core = Core {
                            file,
                            started: false,
                        };
                        self.inner.push(core);
                        self.download_window.show = false;
                    }
                });
        }
        ctx.request_repaint();
    }
}
