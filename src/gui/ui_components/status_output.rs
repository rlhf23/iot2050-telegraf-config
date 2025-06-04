use eframe::egui;
use super::super::WorkerManager;

pub struct StatusOutput;

impl StatusOutput {
    pub fn show(ui: &mut egui::Ui, worker_manager: &WorkerManager) {
        if !worker_manager.status_message.is_empty() {
            // Add a header to make it more visible
            ui.separator();
            ui.horizontal(|ui| {
                ui.heading("Command Output:");
                if worker_manager.is_working {
                    ui.add(egui::Spinner::new().size(16.0));
                    ui.label("Working...");
                }
            });
            
            // Create a frame with a border to make the output more visible
            let frame = egui::Frame::dark_canvas(ui.style())
                .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
            frame.show(ui, |ui| {
                // Use scrollable area with fixed height for multiline text
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        // Use a selectable label with monospace font for output
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                        let content = worker_manager.status_message.clone();
                        // Split by lines and display each line separately
                        for line in content.lines() {
                            ui.label(line);
                        }
                    });
                // Add status message length for debugging
                ui.separator();
                ui.label(format!("Output length: {} characters", worker_manager.status_message.len()));
            });
        }
    }
}
