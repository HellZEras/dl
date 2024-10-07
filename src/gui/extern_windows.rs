use std::sync::mpsc::channel;

use crate::{file2dl::File2Dl, Core, MyApp};
use eframe::egui::{self, Button, Color32, Pos2, TextEdit, Vec2};
pub fn show_input_window(ctx: &eframe::egui::Context, interface: &mut MyApp) {
    let window_size = egui::vec2(250.0, 200.0);
    let center = calc_center(ctx, window_size);
    egui::Window::new("Add Download")
        .default_size(window_size)
        .default_pos(center)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Add Download").strong());
            });
            ui.separator();
            ui.label("URL:");
            if !interface.popus.download.error.is_empty() {
                ui.colored_label(Color32::RED, &interface.popus.download.error);
            }
            ui.text_edit_singleline(&mut interface.popus.download.url);
            ui.label("Bandwidth in Mbs: (Will be ignored if empty)");
            ui.text_edit_singleline(&mut interface.popus.download.bandwidth);
            ui.add_space(5f32);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Confirm").clicked() {
                        if interface.popus.download.bandwidth.is_empty() {
                            interface.popus.download.bandwidth = "0.0".to_string();
                        }
                        let bandwidth = match interface.popus.download.bandwidth.parse::<f64>() {
                            Ok(bandwidth) => bandwidth,
                            Err(_) => {
                                interface.popus.download.error =
                                    String::from("Enter a valid number");
                                return;
                            }
                        };
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let tx = interface.popus.download.error_channel.0.clone();
                        let file_tx = interface.file_channel.0.clone();
                        let link = interface.popus.download.url.clone();
                        rt.block_on(async move {
                            match File2Dl::new(&link, "Downloads", bandwidth).await {
                                Ok(file) => file_tx.send(file).unwrap(),
                                Err(e) => {
                                    let error = format!("{:?}", e);
                                    println!("{}", error);
                                    tx.send(e.to_string()).unwrap();
                                }
                            };
                        });
                        if let Ok(val) = interface.popus.download.error_channel.1.try_recv() {
                            interface.popus.download.error = val;
                            return;
                        }
                        let mut file = match interface.file_channel.1.try_recv() {
                            Ok(file) => file,
                            Err(e) => {
                                interface.popus.download.error = e.to_string();
                                return;
                            }
                        };
                        for core in interface.inner.iter() {
                            if core.file.url.link == file.url.link
                                && core
                                    .file
                                    .size_on_disk
                                    .load(std::sync::atomic::Ordering::Relaxed)
                                    < core.file.url.total_size
                            {
                                interface.popus.download.error =
                                    "Download already exists,simply resume it".to_string();
                                return;
                            }
                        }
                        file.switch_status().unwrap();
                        let core = Core {
                            file,
                            started: false,
                            selected: false,
                            channel: channel(),
                        };
                        interface.inner.push(core);
                        interface.popus.download.show = false;
                        interface.popus.download.error = String::default();
                    }
                    ui.add_space(180.0);
                    if ui.button("Cancel").clicked() {
                        interface.popus.download.show = false;
                        interface.popus.download.error = String::default();
                    }
                });
            });
        });
}

pub fn show_error_window(ctx: &eframe::egui::Context, interface: &mut MyApp, error: &str) {
    let window_size = egui::vec2(250.0, 200.0);
    let center = calc_center(ctx, window_size);
    egui::Window::new("Confirm")
        .default_size(window_size)
        .default_pos(center)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.colored_label(Color32::RED, "Error!");
            });
            ui.separator();
            ui.label(error);
            ui.separator();
            ui.add_space(10.0);
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                if ui
                    .add_sized(Vec2::new(40.0, 30.0), Button::new("Ok"))
                    .clicked()
                {
                    interface.popus.error.show = false;
                }
            });
            ui.add_space(10.0);
        });
}
fn calc_center(ctx: &eframe::egui::Context, size: Vec2) -> Pos2 {
    let screen_rect = ctx.screen_rect();
    screen_rect.center() - 0.5 * size
}

pub fn show_confirm_window(
    ctx: &eframe::egui::Context,
    interface: &mut MyApp,
    color: Color32,
    text: &str,
    action: Box<dyn FnOnce(&mut MyApp) + 'static>,
) {
    let window_size = egui::vec2(250.0, 200.0);
    let center = calc_center(ctx, window_size);
    egui::Window::new("Confirm")
        .default_size(window_size)
        .default_pos(center)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label("Are u sure?");
                ui.label(egui::RichText::new(text).strong().color(color));
            });
            ui.separator();
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    if ui
                        .add_sized(Vec2::new(40.0, 30.0), Button::new("Yes"))
                        .clicked()
                    {
                        action(interface);
                        interface.popus.confirm.show = false;
                    }
                    ui.add_space(125.0);
                    if ui
                        .add_sized(Vec2::new(40.0, 30.0), Button::new("No"))
                        .clicked()
                    {
                        interface.popus.confirm.show = false;
                    }
                })
            });
            ui.add_space(10.0);
        });
}

pub fn show_bandwidth_edit_window(ctx: &eframe::egui::Context, interface: &mut MyApp, name: &str) {
    let window_size = egui::vec2(250.0, 200.0);
    let center = calc_center(ctx, window_size);
    egui::Window::new("Edit Bandwidth")
        .default_size(window_size)
        .default_pos(center)
        .resizable(false)
        .title_bar(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Edit Bandwidth").strong());
            });
            ui.separator();
            if !interface.popus.bandwidth.error.is_empty() {
                ui.colored_label(Color32::RED, &interface.popus.bandwidth.error);
            }
            ui.vertical_centered(|ui| {
                ui.label("Bandwidth:(Will be ignored if empty or 0)");
            });
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.add_sized(
                        [220.0, 17.0],
                        TextEdit::singleline(&mut interface.popus.bandwidth.value),
                    );
                    let choose_currently_selected = || match interface.popus.bandwidth.unit {
                        crate::BandwidthUnit::Kbs => "Kbs",
                        crate::BandwidthUnit::Mbs => "Mbs",
                        crate::BandwidthUnit::Gbs => "Gbs",
                    };
                    egui::ComboBox::from_label("")
                        .width(40.0)
                        .height(0.0)
                        .selected_text(choose_currently_selected())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut interface.popus.bandwidth.unit,
                                crate::BandwidthUnit::Kbs,
                                "Kbs",
                            );
                            ui.selectable_value(
                                &mut interface.popus.bandwidth.unit,
                                crate::BandwidthUnit::Mbs,
                                "Mbs",
                            );
                            ui.selectable_value(
                                &mut interface.popus.bandwidth.unit,
                                crate::BandwidthUnit::Gbs,
                                "Gbs",
                            );
                        });
                })
            });
            ui.add_space(5f32);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Confirm").clicked() {
                        let bandwidth = {
                            if interface.popus.bandwidth.value == "0"
                                || interface.popus.bandwidth.value.is_empty()
                            {
                                0.0
                            } else {
                                match interface.popus.bandwidth.unit {
                                    crate::BandwidthUnit::Kbs => {
                                        match interface.popus.bandwidth.value.parse::<f64>() {
                                            Ok(bandwidth) => bandwidth * 1024.0,
                                            Err(e) => {
                                                interface.popus.bandwidth.error = e.to_string();
                                                return;
                                            }
                                        }
                                    }
                                    crate::BandwidthUnit::Mbs => {
                                        match interface.popus.bandwidth.value.parse::<f64>() {
                                            Ok(bandwidth) => bandwidth * 1024.0 * 1024.0,
                                            Err(e) => {
                                                interface.popus.bandwidth.error = e.to_string();
                                                return;
                                            }
                                        }
                                    }
                                    crate::BandwidthUnit::Gbs => {
                                        match interface.popus.bandwidth.value.parse::<f64>() {
                                            Ok(bandwidth) => bandwidth * 1024.0 * 1024.0 * 1024.0,
                                            Err(e) => {
                                                interface.popus.bandwidth.error = e.to_string();
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        };
                        for core in interface.inner.iter_mut() {
                            if core.file.name_on_disk == name {
                                core.file.bandwidth.store(
                                    bandwidth as usize,
                                    std::sync::atomic::Ordering::Relaxed,
                                );
                                interface.popus.bandwidth.show = false;
                                interface.popus.bandwidth.error = String::default();
                                return;
                            }
                        }
                    }
                    ui.add_space(190.0);
                    if ui.button("Cancel").clicked() {
                        interface.popus.bandwidth.show = false;
                        interface.popus.bandwidth.error = String::default();
                    }
                });
            });
        });
}
