use std::path::Path;

use dl::file2dl::Download;
use eframe::egui::{
    Checkbox, Color32, Image, ImageButton, Label, ProgressBar, RichText, Rounding, Separator,
    TextWrapMode, Vec2,
};
use egui_extras::{Column, TableBuilder};
use tokio::runtime::Runtime;

use crate::{MyApp, Threading, ICON, PAUSE, RESUME};

pub fn display_interface(
    interface: &mut MyApp,
    ui: &mut eframe::egui::Ui,
    ctx: &eframe::egui::Context,
) {
    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .auto_shrink(true)
        .scroll_bar_visibility(eframe::egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .column(Column::auto().resizable(false))
        .column(Column::auto().resizable(true).at_least(200.0))
        .column(Column::auto().resizable(true).at_least(150.0))
        .column(Column::auto().resizable(true).at_least(80.0))
        .column(Column::auto().resizable(true).at_least(130.0))
        .column(Column::remainder().resizable(true).at_least(120.0))
        .column(Column::auto().resizable(false).at_least(80.0))
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
                ui.heading("Limiter");
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
            header.col(|ui| {
                ui.heading("Transfer rate");
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
            header.col(|ui| {
                ui.heading("Time left");
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
            header.col(|ui| {
                ui.heading("");
                ui.add(Separator::grow(Separator::default(), ui.available_width()));
            });
        })
        .body(|mut body| {
            for core in interface.inner.iter_mut() {
                let status = *core.file.status.1.borrow();
                let connected = *interface.connected_to_net.connected.lock();
                let done = core
                    .file
                    .complete
                    .load(std::sync::atomic::Ordering::Relaxed);
                let progress = core
                .file
                .size_on_disk
                .load(std::sync::atomic::Ordering::Relaxed);
                body.row(25.0, |mut row| {
                    row.col(|ui| {
                        ui.add(Checkbox::without_text(&mut core.selected));
                    });
                    row.col(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let label = Label::new(&core.file.name_on_disk)
                                    .wrap_mode(TextWrapMode::Truncate);
                                let res = ui.add(label);
                                if res.hovered(){
                                    let text = format!("Url: {}\n(Double click to open file)",core.file.url.link);
                                    res.show_tooltip_text(text);
                                };
                                if res.double_clicked(){
                                    let path = format!("{}/{}",core.file.dir,core.file.name_on_disk);
                                    match opener::open(path){
                                        Ok(_) => {}
                                        Err(e) => {
                                            interface.popus.error.value = e.to_string();
                                            interface.popus.error.show = true;
                                        }
                                    }
                                }
                            });
                        });
                    });
                    row.col(|ui| {
                        let progress_fraction = progress as f32 / core.file.url.total_size as f32;
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let percentage = format!("{:.2}%", progress_fraction * 100.0);
                                let mbs = core
                                    .file
                                    .size_on_disk
                                    .load(std::sync::atomic::Ordering::Relaxed)
                                    as f64
                                    / 1024.0
                                    / 1024.0;
                                let total_mbs = core.file.url.total_size as f64 / 1024.0 / 1024.0;
                                let text = format!("{:.3}MB/{:.3}MB", mbs, total_mbs);
                                let progress_bar = {
                                    if !done {
                                        if status {
                                            ProgressBar::new(progress_fraction)
                                                .desired_width(130.0)
                                                .text(
                                                    RichText::new(percentage)
                                                        .color(Color32::DARK_GRAY)
                                                        .size(13.0)
                                                        .strong(),
                                                )
                                                .fill(Color32::LIGHT_GREEN)
                                                .rounding(Rounding::ZERO)
                                        } else {
                                            ProgressBar::new(progress_fraction)
                                                .desired_width(130.0)
                                                .text(
                                                    RichText::new(percentage)
                                                        .color(Color32::DARK_GRAY)
                                                        .size(13.0)
                                                        .strong(),
                                                )
                                                .fill(Color32::YELLOW)
                                                .rounding(Rounding::ZERO)
                                        }
                                    } else {
                                        ProgressBar::new(progress_fraction)
                                            .desired_width(130.0)
                                            .text(
                                                RichText::new(percentage)
                                                    .color(Color32::BLACK)
                                                    .size(13.0)
                                                    .strong(),
                                            )
                                            .fill(Color32::DARK_GREEN)
                                            .rounding(Rounding::ZERO)
                                    }
                                };
                                let pb_ui = ui.add(progress_bar);
                                if pb_ui.hovered() {
                                    pb_ui.show_tooltip_text(text);
                                }
                                ctx.request_repaint_of(pb_ui.ctx.viewport_id());
                            })
                        });
                    });
                    row.col(|ui| {
                        if status && !core.started {
                            let rt = Runtime::new().unwrap();
                            let mut file_clone = core.file.clone();
                            let tx = core.channel.0.clone();
                            let threads = core.threads;
                            if core.threading.clone() == Threading::Single && Path::new(&core.file.dir).join(&core.file.name_on_disk).is_file(){
                                std::thread::spawn(move ||{
                                    rt.block_on(async move {
                                        loop{
                                            match file_clone.single_thread_dl().await {
                                                Ok(_) => break,
                                                Err(e) => {
                                                    let error = format!("{:?}",e);
                                                    if !error.contains("ConnectError"){
                                                        tx.send(error).unwrap()
                                                    }
                                                },
                                            }
                                        }
                                    });
                                });
                            }
                            else {
                                std::thread::spawn(move ||{
                                    rt.block_on(async move {
                                        loop{
                                            match file_clone.multi_thread_dl(threads).await {
                                                Ok(_) => break,
                                                Err(e) => {
                                                    let error = format!("{:?}",e);
                                                    if !error.contains("ConnectError"){
                                                        tx.send(error).unwrap()
                                                    }
                                                },
                                            }
                                        }
                                    });
                                });
                            }
                            core.started = true;
                        }
                        if !connected{
                            ui.colored_label(Color32::RED, "Disconnected");
                        }
                        else if !done && status {
                            if let Ok(e) = core.channel.1.try_recv() {
                                println!("{e}");
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
                        let bandwidth = core.file
                        .bandwidth_chosen
                        .load(std::sync::atomic::Ordering::Relaxed);
                        let text = if bandwidth == 0 {
                            "Unlimited".to_string()
                        } else if bandwidth >= 500_000_000 {
                            format!("{:.4} Gbps", bandwidth as f64 / 1_000_000_000.0)
                        } else if bandwidth >= 500_000 {
                            format!("{:.4} Mbps", bandwidth as f64 / 1_000_000.0)
                        } else {
                            format!("{:.4} Kbps", bandwidth as f64 / 1_000.0)
                        };
                        let res = ui.label(text);
                        if res.hovered() {
                            res.show_tooltip_text("Bandwidth limiter\n(Click twice to change)");
                        }
                        if res.double_clicked() {
                            interface.popus.bandwidth.show = true;
                            interface.popus.bandwidth.to_edit = core.file.name_on_disk.clone();
                        }
                    });
                    row.col(|ui| {
                        let transfer_rate = if !status || done || !connected {
                            0
                        } else {
                            core.file.transfer_rate.load(std::sync::atomic::Ordering::Relaxed)
                        };
                        let res = if transfer_rate == 0 {
                            ui.colored_label(Color32::YELLOW,format!("{:.4}MB/s",transfer_rate as f64 / 1024.0 / 1024.0))
                        } else if transfer_rate >= 500_000_000 {
                            ui.colored_label(Color32::GREEN,format!("{:.4} Gbps", transfer_rate as f64 / 1_000_000_000.0))
                        } else if transfer_rate >= 500_000 {
                            ui.colored_label(Color32::GREEN,format!("{:.4} Mbps", transfer_rate as f64 / 1_000_000.0))
                        } else {
                            ui.colored_label(Color32::GREEN,format!("{:.4} Kbps", transfer_rate as f64 / 1_000.0))
                        };
                        ctx.request_repaint_of(res.ctx.viewport_id());
                    });
                    row.col(|ui|{
                        let transfer_rate = core.file.transfer_rate.load(std::sync::atomic::Ordering::Relaxed);
                        let total_size = core.file.url.total_size;
                        let time_left = if transfer_rate == 0 && !done{
                            "Unknown".to_string()
                        } else {
                            let time_left = (total_size as f64 - progress as f64) / transfer_rate as f64;
                            let hours = time_left as u64 / 3600;
                            let minutes = (time_left as u64 % 3600) / 60;
                            let seconds = time_left as u64 % 60;
                            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
                        };
                        ui.label(time_left);
                    });
                    row.col(|ui| {
                        let icon = Image::from_bytes("bytes://PAUSE", ICON);
                        let img_butt = ImageButton::new(icon);
                        let supposed_name = core.file.name_on_disk.clone();
                        let supposed_path = Path::new(&core.file.dir).join(&supposed_name);
                        let mut name = core.file.name_on_disk.clone();
                        name.insert(0, '.');
                        ui.vertical_centered(|ui|{
                            if !done {
                                if ui.add(img_butt.clone()).clicked(){
                                    core.file.switch_status().unwrap();
                                }
                            } else if Path::new(&name).is_dir() && supposed_path.exists() {
                                let res = ui.add(img_butt);
                                if res.clicked() {
                                    core.file.switch_status().unwrap();
                                }
                                if res.hovered() {
                                    res.show_tooltip_text("File has already finished downloading, but the data was not merged to the download file");
                                }
                            }
                            else{
                                ui.add_enabled(false,img_butt);
                            }
                        });
                    });
                });
            }
        });
}
