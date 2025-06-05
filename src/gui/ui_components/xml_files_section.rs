use eframe::egui;
use super::super::ConfigManager;

pub struct XmlFilesSection;

impl XmlFilesSection {
    pub fn show(ui: &mut egui::Ui, config_manager: &mut ConfigManager) {
        ui.heading("XML Files Configuration");
        
        if config_manager.xml_files.is_empty() {
            ui.label("No XML files found in the selected folder");
        } else {
            ui.label("Configure XML files (check for listeners/subscribers):");
            
            for (i, file) in config_manager.xml_files.iter().enumerate() {
                ui.group(|ui| {
                    // Clone the file name to avoid borrowing issues
                    let file_clone = file.clone();
                    
                    // Get or create config for this file
                    let file_config = config_manager.file_configs.entry(file_clone.clone()).or_default();
                    
                    // Store the current IP for change detection
                    let old_ip = file_config.ip.clone();
                    
                    // File name and listener checkbox
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut config_manager.selected_listener_files[i], "Listener");
                        ui.strong(&file_clone);
                    });

                    // Use grid layout for all fields in the XML file configuration
                    egui::Grid::new(&format!("xml_file_grid_{}", i))
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            // Namespace input
                            ui.label("Namespace:");
                            let text_edit = egui::TextEdit::singleline(&mut file_config.namespace);
                            if config_manager.form_state.show_namespace_error {
                                egui::Frame::NONE
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 0, 0)))
                                    .show(ui, |ui| ui.add(text_edit));
                            } else {
                                ui.add(text_edit);
                            }
                            ui.end_row();

                            // IP Address input
                            ui.label("OPC IP:");


                            // Check if we have a validation error for this file
                            let has_error = config_manager.form_state.ip_errors.get(&file_clone).copied().unwrap_or(false);

                            // Show the field with appropriate styling
                            let response = if has_error {
                                // If there's an error, show red border
                                let response = egui::Frame::NONE
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 0, 0)))
                                    .show(ui, |ui| {
                                        ui.add(egui::TextEdit::singleline(&mut file_config.ip)
                                            .hint_text(&config_manager.config.ip))
                                    })
                                    .inner;
                                response.on_hover_text("Invalid IP format. Must be four numbers 0-255 separated by dots (e.g., 192.168.1.1)")
                            } else {
                                // No error, show normal text field
                                let response = ui.add(egui::TextEdit::singleline(&mut file_config.ip)
                                    .hint_text(&config_manager.config.ip));
                                response.on_hover_text("Override the default OPC IP address for this file")
                            };

                            // Store the new IP for validation after the UI is rendered
                            let ip_changed = response.changed() && old_ip != file_config.ip;
                            
                            // We'll validate the IP after the UI is rendered to avoid borrowing issues
                            ui.end_row();

                            // Store the file and IP for validation after the UI is rendered
                            if ip_changed {
                                let file = file_clone.clone();
                                let new_ip = file_config.ip.clone();
                                ui.ctx().request_repaint_after(std::time::Duration::from_millis(0));
                                let id = egui::Id::new(("validate_ip", &file_clone));
                                let _ = ui.memory_mut(|mem| {
                                    mem.data.insert_temp(id, (file, new_ip));
                                });
                            }

                            // Interval input
                            let is_listener = config_manager.selected_listener_files[i];
                            let label = if is_listener {
                                "Sampling Interval (ms):"
                            } else {
                                "Interval (ms):"
                            };
                            ui.label(label);

                            let default_interval = if is_listener { "500" } else { "1000" };
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut file_config.interval_ms)
                                    .hint_text(default_interval),
                            );

                            response.on_hover_text(if is_listener {
                                "Default: 500ms for listeners"
                            } else {
                                "Default: 1000ms for regular files"
                            });
                            ui.end_row();
                        });
                });
                ui.add_space(4.0);
            }
        }

        // Process any pending IP validations for each file
        let files_to_validate: Vec<_> = config_manager.xml_files.iter().cloned().collect();
        for file in files_to_validate {
            let id = egui::Id::new(("validate_ip", &file));
            if let Some((validate_file, ip)) = ui.memory_mut(|mem| {
                mem.data.get_temp::<(String, String)>(id)
            }) {
                config_manager.validate_ip_for_file(&validate_file, &ip);
                // Remove the temp data after processing
                ui.memory_mut(|mem| mem.data.remove::<(String, String)>(id));
            }
        }

        // Bucket Configuration - Only show when using InfluxDB
        if !config_manager.is_prometheus_format() {
            ui.horizontal(|ui| {
                ui.label("Bucket Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut config_manager.config.bucket_name)
                        .hint_text("line"),
                );
            });
        }
    }
}
