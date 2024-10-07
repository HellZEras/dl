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

        // Outer layout that spans the entire bottom panel
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
            let res = display_transfer_rate(ui, transfer_rate, status, app.connected_to_net);
            let sep = ui.add(Separator::grow(Separator::default(), ui.available_height()));
            ui.add_space(ui.available_width() - res.rect.width() - sep.rect.width());
            ui.add(Separator::grow(Separator::default(), ui.available_height()));
            display_connection_status(ui, app);
        });
    });
}

// Function to display the connection status
fn display_connection_status(ui: &mut Ui, app: &MyApp) {
    if app.connected_to_net {
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
