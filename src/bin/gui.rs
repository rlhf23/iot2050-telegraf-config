use eframe::egui;
use sie_generate_config::{backend::ConfigGenerator, TelegrafConfig};

#[derive(Default)]
struct XmlFileConfig {
    namespace: String,
    interval_ms: String,
}

struct TelegrafApp {
    config: TelegrafConfig,
    xml_files: Vec<String>,
    selected_listener_files: Vec<bool>, // Checkboxes for listener selection
    file_configs: std::collections::HashMap<String, XmlFileConfig>,
    status_message: String,
    token_file_path: std::path::PathBuf, // Store the complete token file path
    show_namespace_error: bool,          // Track if we should show namespace errors
}

impl TelegrafApp {
    fn load_token(&mut self) {
        if let Ok(token_content) = std::fs::read_to_string(&self.token_file_path) {
            self.config.influx_token = Some(token_content.trim().to_string());
        }
    }

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

        // Initialize configs for new files
        for file in &self.xml_files {
            self.file_configs.entry(file.clone()).or_default();
        }
    }
}

impl Default for TelegrafApp {
    fn default() -> Self {
        let mut path = std::env::current_exe().unwrap();
        path.pop();

        let token_file_path = path.join("token.txt");

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
                bucket_name: String::new(),
                influx_token: None,
                listener_files: Vec::new(),
                output_format: Some("influxdb".to_string()),
                include_test_inputs: false,
            },
            xml_files: Vec::new(),
            selected_listener_files: Vec::new(),
            file_configs: std::collections::HashMap::new(),
            status_message: String::new(),
            token_file_path, // Default to token.txt in the current directory
            show_namespace_error: false,
        };
        app.load_xml_files();
        app.load_token();
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
                            self.config.folder = path.clone();
                            self.config.token_folder = path.clone(); // Update token folder as well
                            self.token_file_path = path.join("token.txt"); // Update
                            self.load_token(); // Read token from new folder
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

                // Test Inputs Toggle
                ui.horizontal(|ui| {
                    ui.label("Include Test Inputs:");
                    if ui
                        .checkbox(
                            &mut self.config.include_test_inputs,
                            "CPU, Disk, Memory, of the IOT device",
                        )
                        .changed()
                    {
                        // Checkbox state is automatically saved to config
                    }
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

                    // Calculate is_prometheus value once
                    let mut is_prometheus = self
                        .config
                        .output_format
                        .clone()
                        .unwrap_or_else(|| "influxdb".to_string())
                        == "prometheus";

                    // Output Format Toggle
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Output Format:");

                        let toggle_text = if is_prometheus {
                            "Prometheus"
                        } else {
                            "InfluxDB"
                        };
                        if ui.button(toggle_text).clicked() {
                            is_prometheus = !is_prometheus;
                            self.config.output_format = Some(if is_prometheus {
                                "prometheus".to_string()
                            } else {
                                "influxdb".to_string()
                            });
                        }

                        ui.label(if is_prometheus {
                            "(exposes metrics via HTTP)"
                        } else {
                            "(sends to InfluxDB)"
                        });
                    });

                    // Only show InfluxDB token options when using InfluxDB
                    if !is_prometheus {
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
                                    .set_file_name("token.txt") // Default filename suggestion
                                    .pick_file()
                                {
                                    self.token_file_path = path.clone();
                                    self.config.token_folder =
                                        path.parent().unwrap_or(&path).to_path_buf();
                                    self.load_token();
                                }
                            }
                            ui.label(self.token_file_path.to_string_lossy().to_string());
                        });
                    }
                });
            });

            // XML Files Section
            ui.heading("XML Files Configuration");
            if self.xml_files.is_empty() {
                ui.label("No XML files found in the selected folder");
            } else {
                ui.label("Configure XML files (check for listeners/subscribers):");
                for (i, file) in self.xml_files.iter().enumerate() {
                    ui.group(|ui| {
                        // Get or create config for this file
                        let file_config = self.file_configs.entry(file.clone()).or_default();

                        // File name and listener checkbox
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.selected_listener_files[i], "Listener");
                            ui.strong(file);
                        });

                        // Namespace input
                        ui.horizontal(|ui| {
                            ui.label("Namespace:");
                            let text_edit = egui::TextEdit::singleline(&mut file_config.namespace);
                            if self.show_namespace_error {
                                egui::Frame::none()
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(255, 0, 0),
                                    ))
                                    .show(ui, |ui| ui.add(text_edit));
                            } else {
                                ui.add(text_edit);
                            }
                        });

                        // Interval input
                        ui.horizontal(|ui| {
                            let is_listener = self.selected_listener_files[i];
                            let label = if is_listener {
                                "Sampling Interval (ms):"
                            } else {
                                "Interval (ms):"
                            };
                            ui.label(label);
                            let default_interval = if is_listener { "500" } else { "1000" };
                            ui.add(
                                egui::TextEdit::singleline(&mut file_config.interval_ms)
                                    .hint_text(default_interval),
                            )
                            .on_hover_text(if is_listener {
                                "Default: 500ms for listeners"
                            } else {
                                "Default: 1000ms for regular files"
                            });
                        });
                    });
                    ui.add_space(4.0);
                }
            }

            // Bucket Configuration - Only show when using InfluxDB
            // Reuse the already calculated is_prometheus value
            // Since it might have changed with the toggle, get the current value
            let is_prometheus = self
                .config
                .output_format
                .clone()
                .unwrap_or_else(|| "influxdb".to_string())
                == "prometheus";
            if !is_prometheus {
                ui.horizontal(|ui| {
                    ui.label("Bucket Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.config.bucket_name).hint_text("line"),
                    );
                });
            }

            // Main Action Buttons
            ui.horizontal(|ui| {
                if ui.button("Generate Config").clicked() {
                    self.show_namespace_error = false;
                    // Validate that all namespace numbers are unique and provided
                    let mut namespace_map: std::collections::HashMap<&str, Vec<&str>> =
                        std::collections::HashMap::new();

                    // Collect namespaces and their corresponding files
                    for (file, config) in &self.file_configs {
                        let namespace = config.namespace.trim();
                        if namespace.is_empty() {
                            self.status_message =
                                format!("Error: No namespace provided for file: {}", file);
                            self.show_namespace_error = true;
                            return;
                        }
                        namespace_map.entry(namespace).or_default().push(file);
                    }

                    // Check for duplicate namespaces
                    for (namespace, files) in &namespace_map {
                        if files.len() > 1 {
                            self.status_message = format!(
                                "Error: Namespace {} is used by multiple files: {}",
                                namespace,
                                files.join(", ")
                            );
                            self.show_namespace_error = true;
                            return;
                        }
                    }

                    self.config.bucket_name = if self.config.bucket_name.is_empty() {
                        "line".to_string()
                    } else {
                        self.config.bucket_name.clone()
                    };
                    self.config.listener_files = self
                        .xml_files
                        .iter()
                        .zip(self.selected_listener_files.iter())
                        .filter(|(_, &selected)| selected)
                        .map(|(file, _)| file.clone())
                        .collect();

                    match ConfigGenerator::new(self.config.clone()) {
                        Ok(mut generator) => {
                            // Set configurations for each file
                            for file in &self.xml_files {
                                if let Some(file_config) = self.file_configs.get(file) {
                                    let is_listener = self.config.listener_files.contains(file);
                                    let default_interval = if is_listener { 500 } else { 1000 };

                                    let interval_ms =
                                        file_config.interval_ms.parse().unwrap_or(default_interval);

                                    generator.set_file_config(
                                        file.clone(),
                                        file_config.namespace.clone(),
                                        interval_ms,
                                    );
                                }
                            }
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
            });

            // Other Commands Section
            ui.collapsing("Other Commands", |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Backup InfluxDB").clicked() {
                        if let Ok(generator) = ConfigGenerator::new(self.config.clone()) {
                            match generator.backup_influx() {
                                Ok(_) => {
                                    self.status_message = "InfluxDB backup completed!".to_string();
                                }
                                Err(e) => {
                                    self.status_message =
                                        format!("Error backing up InfluxDB: {}", e);
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
                                    self.status_message =
                                        format!("Error backing up Grafana: {}", e);
                                }
                            }
                        }
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("Get Telegraf Status").clicked() {
                        if let Ok(generator) = ConfigGenerator::new(self.config.clone()) {
                            match generator.get_telegraf_status() {
                                Ok(status) => {
                                    self.status_message = format!("Telegraf Status:\n{}", status);
                                }
                                Err(e) => {
                                    self.status_message =
                                        format!("Error getting Telegraf status: {}", e);
                                }
                            }
                        }
                    }

                    if ui.button("Get Telegraf Logs").clicked() {
                        if let Ok(generator) = ConfigGenerator::new(self.config.clone()) {
                            match generator.get_telegraf_logs(30) {
                                Ok(logs) => {
                                    self.status_message =
                                        format!("Telegraf Logs (Last 30 lines):\n{}", logs);
                                }
                                Err(e) => {
                                    self.status_message =
                                        format!("Error getting Telegraf logs: {}", e);
                                }
                            }
                        }
                    }
                });
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
        initial_window_size: Some(egui::vec2(800.0, 1000.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Telegraf Config Generator",
        native_options,
        Box::new(|_cc| Box::new(TelegrafApp::default())),
    )
}
