use crate::{file2dl::File2Dl, Core, MyApp};
use eframe::egui::{self, Button, Color32, Label, Pos2, Vec2};
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
            ui.label("Enter URL:");
            if !interface.popus.download.error.is_empty() {
                ui.colored_label(Color32::RED, &interface.popus.download.error);
            }
            ui.text_edit_singleline(&mut interface.popus.download.url);
            ui.label("Enter Bandwidth:");
            ui.text_edit_singleline(&mut interface.popus.download.bandwidth);
            ui.add_space(5f32);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Confirm").clicked() {
                        let bandwidth = match interface.popus.download.bandwidth.parse::<f64>() {
                            Ok(bandwidth) => bandwidth,
                            Err(e) => {
                                interface.popus.download.error = e.to_string();
                                return;
                            }
                        };
                        let file = match File2Dl::new(
                            &interface.popus.download.url,
                            "Downloads",
                            bandwidth,
                        ) {
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
                        file.switch_status();
                        let core = Core {
                            file,
                            started: false,
                            selected: false,
                            channel: std::sync::mpsc::channel(),
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

pub fn show_confirm_window(
    ctx: &eframe::egui::Context,
    interface: &mut MyApp,
    title: &str,
    color: Color32,
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
                ui.label(egui::RichText::new(title).strong().color(color));
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
                        interface.inner.clear();
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
