use eframe::egui;
use super::super::ConfigManager;

pub struct ConfigForm;

impl ConfigForm {
    pub fn show(ui: &mut egui::Ui, config_manager: &mut ConfigManager) {
        ui.collapsing("Configuration", |ui| {
            // Folder selection
            ui.horizontal(|ui| {
                ui.label("XML Folder:");
                if ui.button("Browse").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        config_manager.update_folder(path);
                    }
                }
                ui.label(config_manager.config.folder.to_string_lossy().to_string());
            });

            // Main Configuration using Grid
            egui::Grid::new("config_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .show(ui, |ui| {
                    // IP Configuration
                    ui.label("OPC IP:");
                    ui.text_edit_singleline(&mut config_manager.config.ip);
                    ui.end_row();

                    // IOT Host with validation
                    ui.label("IOT Host:");
                    let text_edit = egui::TextEdit::singleline(&mut config_manager.config.iot_host);
                    if config_manager.form_state.show_iot_host_error {
                        egui::Frame::NONE
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 0, 0)))
                            .show(ui, |ui| ui.add(text_edit));
                    } else {
                        ui.add(text_edit);
                    }
                    ui.end_row();
                });

            // Test Inputs Toggle
            ui.horizontal(|ui| {
                ui.label("Include Test Inputs:");
                ui.checkbox(
                    &mut config_manager.config.include_test_inputs,
                    "CPU, Disk, Memory, of the IOT device",
                );
            });

            // Credentials
            ui.collapsing("Credentials", |ui| {
                egui::Grid::new("credentials_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // OPC Username
                        ui.label("OPC Username:");
                        ui.text_edit_singleline(&mut config_manager.config.username);
                        ui.end_row();
                        
                        // OPC Password
                        ui.label("OPC Password:");
                        ui.add(egui::TextEdit::singleline(&mut config_manager.config.password).password(true));
                        ui.end_row();
                        
                        // IOT Username
                        ui.label("IOT Username:");
                        ui.text_edit_singleline(&mut config_manager.config.iot_username);
                        ui.end_row();
                        
                        // IOT Password
                        ui.label("IOT Password:");
                        ui.add(egui::TextEdit::singleline(&mut config_manager.config.iot_password).password(true));
                        ui.end_row();
                    });

                // Output Format Toggle
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Output Format:");

                    let is_prometheus = config_manager.is_prometheus_format();
                    let toggle_text = if is_prometheus { "Prometheus" } else { "InfluxDB" };
                    
                    if ui.button(toggle_text).clicked() {
                        config_manager.toggle_output_format();
                    }

                    ui.label(if is_prometheus {
                        "(exposes metrics via HTTP)"
                    } else {
                        "(sends to InfluxDB)"
                    });
                });

                // Only show InfluxDB token options when using InfluxDB
                if !config_manager.is_prometheus_format() {
                    ui.separator();
                    let mut token = config_manager.config.influx_token.clone().unwrap_or_default();

                    egui::Grid::new("influxdb_options_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            // Token input
                            ui.label("InfluxDB Token:");
                            if ui.text_edit_singleline(&mut token).changed() {
                                config_manager.config.influx_token = Some(token);
                            }
                            ui.end_row();

                            // Token file path
                            ui.label("Token File:");
                            ui.horizontal(|ui| {
                                if ui.button("Browse").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Text files", &["txt"])
                                        .set_file_name("token.txt")
                                        .pick_file()
                                    {
                                        config_manager.update_token_file(path);
                                    }
                                }
                                ui.label(config_manager.token_file_path.to_string_lossy().to_string());
                            });
                            ui.end_row();
                        });
                }
            });
        });
    }
}
