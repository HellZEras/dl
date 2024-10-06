use eframe::egui::{menu, Color32};

use crate::MyApp;

pub fn init_menu_bar(interface: &mut MyApp, ui: &mut eframe::egui::Ui) {
    menu::bar(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Add").clicked() {
                        interface.popus.download.show = true;
                    }
                    if ui.button("Remove all from list").clicked() {
                        interface.popus.confirm.color = Color32::GREEN;
                        interface.popus.confirm.value =
                            String::from("Are u sure?,this will not delete files from disk");
                        interface.popus.confirm.show = true;
                    }
                });
            });
        });
    });
}
