use eframe::egui::{Checkbox, Color32, Label, ProgressBar, RichText, Rounding, Separator, Vec2};
use egui_extras::{Column, TableBuilder};

use crate::{file2dl::Download, MyApp};

pub fn display_interface(interface: &mut MyApp, ui: &mut eframe::egui::Ui) {
    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .auto_shrink(true)
        .scroll_bar_visibility(eframe::egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .column(Column::auto().resizable(false))
        .column(Column::remainder().resizable(false))
        .column(Column::remainder().resizable(false))
        .column(Column::remainder().resizable(false))
        .column(Column::remainder().resizable(false))
        .column(Column::remainder().resizable(false))
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.add_sized(
                    Vec2::new(20.0, 21.0),
                    Checkbox::without_text(&mut interface.select_all),
                );
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
            header.col(|ui| {
                ui.heading("Filename");
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
            header.col(|ui| {
                ui.heading("Progress");
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
            header.col(|ui| {
                ui.heading("Status");
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
            header.col(|ui| {
                ui.heading("Bandwidth Mb/s");
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
            header.col(|ui| {
                ui.heading("Action");
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
        })
        .body(|mut body| {
            for core in interface.inner.iter_mut() {
                body.row(20.0, |mut row| {
                    row.col(|ui| {
                        ui.add(Checkbox::without_text(&mut core.selected));
                    });
                    row.col(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(&core.file.name_on_disk);
                            });
                        });
                    });
                    row.col(|ui| {
                        let progress = core
                            .file
                            .size_on_disk
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let progress_fraction = progress as f32 / core.file.url.total_size as f32;
                        let progress_bar = ProgressBar::new(progress_fraction)
                            .desired_width(200.0)
                            .fill(Color32::GREEN)
                            .text(
                                RichText::new(format!("{:.2}%", progress_fraction * 100.0))
                                    .strong()
                                    .color(Color32::BLACK),
                            )
                            .rounding(Rounding::ZERO);
                        ui.add(progress_bar);
                    });
                    row.col(|ui| {
                        let status = core.file.status.load(std::sync::atomic::Ordering::Relaxed);
                        let done = core
                            .file
                            .size_on_disk
                            .load(std::sync::atomic::Ordering::Relaxed)
                            == core.file.url.total_size;
                        if status && !core.started {
                            let file_clone = core.file.clone();
                            let tx = core.channel.0.clone();
                            std::thread::spawn(move || loop {
                                match file_clone.single_thread_dl() {
                                    Ok(_) => break,
                                    Err(e) => tx.send(e.to_string()).unwrap(),
                                }
                            });
                            core.started = true;
                        }
                        if !done && status {
                            if let Ok(e) = core.channel.1.try_recv() {
                                ui.colored_label(Color32::RED, e);
                            } else {
                                ui.colored_label(Color32::GREEN, "Downloading");
                            }
                        } else if !done && !status {
                            ui.colored_label(Color32::YELLOW, "Paused");
                        } else {
                            ui.colored_label(Color32::DARK_GREEN, "Complete");
                        }
                    });
                    row.col(|ui| {
                        ui.label(
                            core.file
                                .bandwidth
                                .load(std::sync::atomic::Ordering::Relaxed)
                                .to_string(),
                        );
                    });
                    row.col(|ui| {
                        let status = core.file.status.load(std::sync::atomic::Ordering::Relaxed);
                        let done = core
                            .file
                            .size_on_disk
                            .load(std::sync::atomic::Ordering::Relaxed)
                            == core.file.url.total_size;
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
                            ui.label("Nothing to do");
                        }
                    });
                });
            }
        });
}
