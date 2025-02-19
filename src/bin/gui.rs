use eframe::egui;
use sie_generate_config::{backend::ConfigGenerator, TelegrafConfig};

struct TelegrafApp {
    config: TelegrafConfig,
    xml_files: Vec<String>,
    selected_listener_files: Vec<bool>, // Checkboxes for listener selection
    bucket_name: String,
    status_message: String,
}

impl TelegrafApp {
    fn load_xml_files(&mut self) {
        self.xml_files = std::fs::read_dir(&self.config.folder)
            .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "xml") {
                    Some(path.to_str()?.to_string())
                } else {
                    None
                }
            })
            .collect();
        self.selected_listener_files = vec![false; self.xml_files.len()];
    }
}

impl Default for TelegrafApp {
    fn default() -> Self {
        let mut path = std::env::current_exe().unwrap();
        path.pop();

        let mut app = Self {
            config: TelegrafConfig {
                folder: path.clone(),
                ip: env!("DEFAULT_IP").to_string(),
                username: env!("DEFAULT_USERNAME").to_string(),
                password: env!("DEFAULT_PASSWORD").to_string(),
                iot_host: env!("DEFAULT_IOT_IP").to_string(),
                iot_username: env!("DEFAULT_IOT_USERNAME").to_string(),
                iot_password: env!("DEFAULT_IOT_PASSWORD").to_string(),
                token_folder: path,
                bucket_name: "line".to_string(),
                influx_token: None,
                listener_files: Vec::new(),
            },
            xml_files: Vec::new(),
            selected_listener_files: Vec::new(),
            bucket_name: "line".to_string(),
            status_message: String::new(),
        };
        app.load_xml_files();
        app
    }
}

impl eframe::App for TelegrafApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Telegraf Configuration Generator");

            // Configuration Section
            ui.collapsing("Configuration", |ui| {
                // Folder selection
                ui.horizontal(|ui| {
                    ui.label("XML Folder:");
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            self.config.folder = path;
                            // Update XML files list
                            self.xml_files = std::fs::read_dir(&self.config.folder)
                                .unwrap()
                                .filter_map(|entry| {
                                    let path = entry.ok()?.path();
                                    if path.is_file()
                                        && path.extension().is_some_and(|ext| ext == "xml")
                                    {
                                        Some(path.to_str()?.to_string())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            self.selected_listener_files = vec![false; self.xml_files.len()];
                        }
                    }
                    ui.label(self.config.folder.to_string_lossy().to_string());
                });

                // IP Configuration
                ui.horizontal(|ui| {
                    ui.label("OPC IP:");
                    ui.text_edit_singleline(&mut self.config.ip);
                });

                ui.horizontal(|ui| {
                    ui.label("IOT Host:");
                    ui.text_edit_singleline(&mut self.config.iot_host);
                });

                // Credentials
                ui.collapsing("Credentials", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("OPC Username:");
                        ui.text_edit_singleline(&mut self.config.username);
                    });
                    ui.horizontal(|ui| {
                        ui.label("OPC Password:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.password).password(true),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("IOT Username:");
                        ui.text_edit_singleline(&mut self.config.iot_username);
                    });
                    ui.horizontal(|ui| {
                        ui.label("IOT Password:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.config.iot_password)
                                .password(true),
                        );
                    });

                    // InfluxDB Token
                    ui.separator();
                    let mut token = self.config.influx_token.clone().unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.label("InfluxDB Token:");
                        if ui.text_edit_singleline(&mut token).changed() {
                            self.config.influx_token = Some(token);
                        }
                    });

                    // Token file path
                    ui.horizontal(|ui| {
                        ui.label("Token File:");
                        if ui.button("Browse").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Text files", &["txt"])
                                .pick_file()
                            {
                                self.config.token_folder =
                                    path.parent().unwrap_or(&path).to_path_buf();
                                if let Ok(token_content) = std::fs::read_to_string(&path) {
                                    self.config.influx_token =
                                        Some(token_content.trim().to_string());
                                }
                            }
                        }
                        ui.label(self.config.token_folder.to_string_lossy().to_string());
                    });
                });
            });

            // XML Files Section
            ui.heading("XML Files");
            if self.xml_files.is_empty() {
                ui.label("No XML files found in the selected folder");
            } else {
                ui.label("Select files to be listeners (subscribers):");
                for (i, file) in self.xml_files.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.selected_listener_files[i], "");
                        ui.label(file);
                    });
                }
            }

            // Bucket Configuration
            ui.horizontal(|ui| {
                ui.label("Bucket Name:");
                ui.text_edit_singleline(&mut self.bucket_name);
            });

            // Action Buttons
            ui.horizontal(|ui| {
                if ui.button("Generate Config").clicked() {
                    self.config.bucket_name = self.bucket_name.clone();
                    self.config.listener_files = self
                        .xml_files
                        .iter()
                        .zip(self.selected_listener_files.iter())
                        .filter(|(_, &selected)| selected)
                        .map(|(file, _)| file.clone())
                        .collect();

                    // Read token
                    let token_path = self.config.token_folder.join("token.txt");
                    if let Ok(token) = std::fs::read_to_string(token_path) {
                        self.config.influx_token = Some(token.trim().to_string());
                    }

                    match ConfigGenerator::new(self.config.clone()) {
                        Ok(generator) => {
                            match generator
                                .generate_config(&self.xml_files, &self.config.listener_files)
                            {
                                Ok(_) => {
                                    self.status_message =
                                        "Configuration generated successfully!".to_string();
                                }
                                Err(e) => {
                                    self.status_message = format!("Error generating config: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message = format!("Error creating generator: {}", e);
                        }
                    }
                }

                if ui.button("Send Config").clicked() {
                    if let Ok(generator) = ConfigGenerator::new(self.config.clone()) {
                        match generator.send_config() {
                            Ok(_) => {
                                self.status_message =
                                    "Configuration sent successfully!".to_string();
                            }
                            Err(e) => {
                                self.status_message = format!("Error sending config: {}", e);
                            }
                        }
                    }
                }

                if ui.button("Backup InfluxDB").clicked() {
                    if let Ok(generator) = ConfigGenerator::new(self.config.clone()) {
                        match generator.backup_influx() {
                            Ok(_) => {
                                self.status_message = "InfluxDB backup completed!".to_string();
                            }
                            Err(e) => {
                                self.status_message = format!("Error backing up InfluxDB: {}", e);
                            }
                        }
                    }
                }

                if ui.button("Backup Grafana").clicked() {
                    if let Ok(generator) = ConfigGenerator::new(self.config.clone()) {
                        match generator.backup_grafana() {
                            Ok(_) => {
                                self.status_message = "Grafana backup completed!".to_string();
                            }
                            Err(e) => {
                                self.status_message = format!("Error backing up Grafana: {}", e);
                            }
                        }
                    }
                }
            });

            // Status Message
            if !self.status_message.is_empty() {
                ui.label(&self.status_message);
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Telegraf Config Generator",
        native_options,
        Box::new(|_cc| Box::new(TelegrafApp::default())),
    )
}
