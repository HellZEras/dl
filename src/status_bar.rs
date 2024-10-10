use std::{net::TcpStream, thread::sleep, time::Duration};

use eframe::egui::{Align, Color32, Layout, Response, Separator, Ui};

use crate::MyApp;

pub fn display_status_bar(ctx: &eframe::egui::Context, app: &mut MyApp) {
    eframe::egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
        let transfer_rate = {
            let mut total = 0;
            for core in app.inner.iter() {
                let status = *core.file.status.1.borrow();
                if status
                    && !core
                        .file
                        .complete
                        .load(std::sync::atomic::Ordering::Relaxed)
                {
                    total += core
                        .file
                        .transfer_rate
                        .load(std::sync::atomic::Ordering::Relaxed);
                }
            }
            total
        };
        if !app.connected_to_net.started {
            let safe = app.connected_to_net.connected.clone();
            std::thread::spawn(move || loop {
                let connected = is_connected();
                *safe.lock() = connected;
                sleep(Duration::from_secs(2));
            });
        }
        app.connected_to_net.started = true;
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            // Left side: connection status
            let status = {
                let mut state: bool = false;
                for core in app.inner.iter() {
                    let status = *core.file.status.1.borrow();
                    if status {
                        state = true;
                        break;
                    }
                }
                state
            };
            let connected = *app.connected_to_net.connected.lock();
            ui.add_space(20.0);
            display_transfer_rate(ui, transfer_rate, status, connected);
            ui.add_space(20.0);
            ui.add(Separator::grow(Separator::default(), ui.available_height()));
            ui.add_space(ui.available_width() - 100.0);
            ui.add(Separator::grow(Separator::default(), ui.available_height()));
            ui.add_space(15.0);
            display_connection_status(ui, connected);
        });
    });
}

fn is_connected() -> bool {
    TcpStream::connect_timeout(&("8.8.8.8:53").parse().unwrap(), Duration::from_secs(2)).is_ok()
}

// Function to display the connection status
fn display_connection_status(ui: &mut Ui, connected: bool) {
    if connected {
        ui.colored_label(Color32::GREEN, "Connected");
    } else {
        ui.colored_label(Color32::RED, "Disconnected");
    }
}

// Function to display the transfer rate
fn display_transfer_rate(
    ui: &mut Ui,
    transfer_rate: usize,
    status: bool,
    connected: bool,
) -> Response {
    if transfer_rate == 0 || !connected {
        let text = format!("{:.4} MB/s", 0);
        ui.colored_label(Color32::YELLOW, text)
    } else if transfer_rate >= 500_000_000 && status {
        let text = format!("{:.4} Gbps", transfer_rate as f64 / 1_000_000_000.0);
        ui.colored_label(Color32::GREEN, text)
    } else if transfer_rate >= 500_000 && status {
        let text = format!("{:.4} Mbps", transfer_rate as f64 / 1_000_000.0);
        ui.colored_label(Color32::GREEN, text)
    } else {
        let text = format!("{:.4} Kbps", transfer_rate as f64 / 1_000.0);
        ui.colored_label(Color32::GREEN, text)
    }
}
