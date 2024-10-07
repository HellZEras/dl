use crate::MyApp;
use eframe::egui::{menu, Color32};
use std::fs::{read_dir, remove_file};

pub fn init_menu_bar(interface: &mut MyApp, ui: &mut eframe::egui::Ui) {
    menu::bar(ui, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    file_button_content(interface, ui);
                });
                ui.menu_button("Downloads", |ui| {
                    if ui.button("Add Download").clicked() {
                        interface.popus.download.show = true;
                    }
                    if ui.button("Resume all").clicked() {
                        for core in interface.inner.iter_mut() {
                            core.file.status.0.send(true).unwrap();
                        }
                    }
                    if ui.button("Pause all").clicked() {
                        for core in interface.inner.iter_mut() {
                            core.file.status.0.send(false).unwrap();
                        }
                    }
                    if ui.button("Delete all completed").clicked() {
                        interface.inner.retain(|core| {
                            !core
                                .file
                                .complete
                                .load(std::sync::atomic::Ordering::Relaxed)
                        });
                    }
                    if ui.button("Delete all incomplete").clicked() {
                        interface.inner.retain(|core| {
                            core.file
                                .complete
                                .load(std::sync::atomic::Ordering::Relaxed)
                        });
                    }
                });
            });
        });
    });
}

fn file_button_content(interface: &mut MyApp, ui: &mut eframe::egui::Ui) {
    if ui.button("Remove selected from list").clicked() {
        interface.popus.confirm.color = Color32::GREEN;
        interface.popus.confirm.task = Box::new(|| {
            Box::new(move |app: &mut MyApp| {
                app.inner.retain(|core| !core.selected);
            })
        });
        interface.popus.confirm.show = true;
    }
    if ui.button("Remove all from list").clicked() {
        interface.popus.confirm.color = Color32::GREEN;
        interface.popus.confirm.text = "This will not delete files from disk".to_string();
        interface.popus.confirm.task = Box::new(|| {
            Box::new(move |app: &mut MyApp| {
                app.inner.clear();
            })
        });
        interface.popus.confirm.show = true;
    }
    if ui.button("Remove selected from disk").clicked() {
        interface.popus.confirm.color = Color32::RED;
        interface.popus.confirm.task = Box::new(|| {
            Box::new(move |app: &mut MyApp| {
                remove_selected_from_disk(app);
            })
        });
        interface.popus.confirm.show = true;
    }
    if ui.button("Remove all from disk").clicked() {
        interface.popus.confirm.color = Color32::RED;
        interface.popus.confirm.task = Box::new(|| {
            Box::new(move |app: &mut MyApp| {
                delete_all_files_from_disk(app);
            })
        });
        interface.popus.confirm.show = true;
    }
}
fn delete_all_files_from_disk(interface: &mut MyApp) {
    let dir = match read_dir("Downloads") {
        Ok(dir) => dir,
        Err(e) => {
            interface.popus.error.value = e.to_string();
            interface.popus.error.show = true;
            return;
        }
    };
    for file in dir {
        let path = match file {
            Ok(file) => file.path(),
            Err(e) => {
                interface.popus.error.value = e.to_string();
                interface.popus.error.show = true;
                return;
            }
        };
        remove_file(path).unwrap();
    }
    interface.inner.clear();
}
fn remove_selected_from_disk(app: &mut MyApp) {
    app.inner.retain(|core| {
        if core.selected {
            let path = format!("Downloads/{}", core.file.name_on_disk);
            let tmp_path = format!("Downloads/.{}.metadata", core.file.name_on_disk);
            match remove_file(path) {
                Ok(_) => {}
                Err(e) => {
                    app.popus.error.value = e.to_string();
                    app.popus.error.show = true;
                }
            }
            match remove_file(tmp_path) {
                Ok(_) => {}
                Err(e) => {
                    app.popus.error.value = e.to_string();
                    app.popus.error.show = true;
                }
            }
            return false;
        }
        true
    });
}
