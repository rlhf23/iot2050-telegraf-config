use eframe::egui;
use sie_generate_config::{backend::{ConfigGenerator, opcua_poller::OpcUaPoller}, TelegrafConfig, error::TelegrafError};

#[derive(Default)]
struct XmlFileConfig {
    namespace: String,
    interval_ms: String,
    ip: String,
}

struct TelegrafApp {
    config: TelegrafConfig,
    xml_files: Vec<String>,
    selected_listener_files: Vec<bool>, // Checkboxes for listener selection
    file_configs: std::collections::HashMap<String, XmlFileConfig>,
    status_message: String,
    token_file_path: std::path::PathBuf, // Store the complete token file path
    show_namespace_error: bool,          // Track if we should show namespace errors
    show_iot_host_error: bool,           // Track if IOT host is invalid
    ip_errors: std::collections::HashMap<String, bool>, // Track file IP validation errors
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

    // Helper function to set UI error flags and get user-friendly error message
    fn handle_error(&mut self, error: &TelegrafError, context: &str) -> String {
        let message = error.user_friendly_message(context);
        
        // Set UI error flags based on the error message
        if message.contains("Host Format Error") {
            self.show_iot_host_error = true;
        }
        
        // Additional flags can be set here as needed
        
        message
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
            show_iot_host_error: false,
            ip_errors: std::collections::HashMap::new(),
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

                    let text_edit = egui::TextEdit::singleline(&mut self.config.iot_host);
                    if self.show_iot_host_error {
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

                        // IP Address input (new)
                        ui.horizontal(|ui| {
                            ui.label("OPC IP:");

                            // Check if we have a validation error for this file
                            let has_error = self.ip_errors.get(file).unwrap_or(&false);

                            // Show the field with appropriate styling
                            let response = if *has_error {
                                // If there's an error, show red border
                                let response = egui::Frame::none()
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(255, 0, 0),
                                    ))
                                    .show(ui, |ui| {
                                        ui.add(egui::TextEdit::singleline(&mut file_config.ip)
                                            .hint_text(&self.config.ip))
                                    })
                                    .inner;
                                response.on_hover_text("Invalid IP format. Must be four numbers 0-255 separated by dots (e.g., 192.168.1.1)")
                            } else {
                                // No error, show normal text field
                                let response = ui.add(egui::TextEdit::singleline(&mut file_config.ip)
                                    .hint_text(&self.config.ip));
                                response.on_hover_text("Override the default OPC IP address for this file")
                            };

                            // Validate IP after user types
                            if response.changed() {
                                // Only validate non-empty custom IPs
                                if !file_config.ip.is_empty() {
                                    let temp_config = sie_generate_config::TelegrafConfig {
                                        ip: file_config.ip.clone(),
                                        ..self.config.clone()
                                    };

                                    // Update error state
                                    self.ip_errors.insert(
                                        file.clone(),
                                        temp_config.validate_ip().is_err()
                                    );
                                } else {
                                    // Empty IP means no error (will use default)
                                    self.ip_errors.insert(file.clone(), false);
                                }
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

                    // Check for IP validation errors
                    let ip_errors: Vec<String> = self.ip_errors
                        .iter()
                        .filter(|(_, &has_error)| has_error)
                        .map(|(file, _)| file.clone())
                        .collect();

                    if !ip_errors.is_empty() {
                        self.status_message = format!(
                            "Error: Invalid IP format in files: {}. Please correct the IP addresses.",
                            ip_errors.join(", ")
                        );
                        return;
                    }

                    // Validate that all namespace numbers are unique and provided
                    // Use a map of (IP, namespace) -> files to track duplicates
                    let mut namespace_ip_map: std::collections::HashMap<(String, &str), Vec<&str>> =
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

                        // Use the file's custom IP if provided, otherwise use the default IP
                        let ip = if config.ip.is_empty() {
                            self.config.ip.clone()
                        } else {
                            config.ip.clone()
                        };

                        // Add to map with combined (IP, namespace) key
                        namespace_ip_map.entry((ip, namespace)).or_default().push(file);
                    }

                    // Check for duplicate namespaces within the same IP
                    for ((ip, namespace), files) in &namespace_ip_map {
                        if files.len() > 1 {
                            self.status_message = format!(
                                "Error: Namespace {} is used by multiple files on IP {}: {}",
                                namespace, ip, files.join(", ")
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

                                    // Convert empty IP string to None, otherwise Some(ip)
                                    let ip_option = if file_config.ip.is_empty() {
                                        None
                                    } else {
                                        Some(file_config.ip.clone())
                                    };

                                    generator.set_file_config(
                                        file.clone(),
                                        file_config.namespace.clone(),
                                        interval_ms,
                                        ip_option,
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
                                    self.status_message = self.handle_error(&e, "generating config");
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message = self.handle_error(&e, "generating config");
                        }
                    }
                }

                if ui.button("Send Config").clicked() {
                    self.status_message = "Sending configuration...".to_string();
                    match ConfigGenerator::new(self.config.clone()) {
                        Ok(generator) => {
                            match generator.send_config() {
                                Ok(_) => {
                                    self.status_message =
                                        "Configuration sent successfully!".to_string();
                                }
                                Err(e) => {
                                    self.status_message = self.handle_error(&e, "send config");
                                }
                            }
                        }
                        Err(e) => {
                            self.status_message = self.handle_error(&e, "send config");
                        }
                    }
                }
            });

            // Other Commands Section
            ui.collapsing("Other Commands", |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Get OPC UA Namespaces").clicked() {
                        self.status_message = "Connecting to OPC UA server...".to_string();
                        match OpcUaPoller::new(self.config.clone()) {
                            Ok(poller) => {
                                // Get the list of XML files
                                let xml_files: Vec<String> = self.xml_files.clone();
                                
                                // Call the OPC UA poller to get namespace information
                                match poller.get_namespace_info(&xml_files) {
                                    Ok(namespace_map) => {
                                        // Update the namespace fields in the GUI
                                        let mut found_count = 0;
                                        for (file_name, namespace_index) in namespace_map {
                                            // Find the full path for this file name
                                            if let Some(full_path) = self.xml_files.iter().find(|path| {
                                                path.ends_with(&file_name)
                                            }) {
                                                // Update the namespace field
                                                if let Some(config) = self.file_configs.get_mut(full_path) {
                                                    config.namespace = namespace_index.to_string();
                                                    found_count += 1;
                                                }
                                            }
                                        }
                                        
                                        if found_count > 0 {
                                            self.status_message = format!("Found namespaces for {} XML files!", found_count);
                                        } else {
                                            self.status_message = "No matching namespaces found. Check XML filenames match namespace names.".to_string();
                                        }
                                    }
                                    Err(e) => {
                                        self.status_message = self.handle_error(&e, "OPC UA namespace lookup");
                                    }
                                }
                            }
                            Err(e) => {
                                self.status_message = self.handle_error(&e, "OPC UA connection");
                            }
                        }
                    }
                });
                
                ui.horizontal(|ui| {
                    if ui.button("Backup InfluxDB").clicked() {
                        self.status_message = "Backing up InfluxDB...".to_string();
                        match ConfigGenerator::new(self.config.clone()) {
                            Ok(generator) => {
                                match generator.backup_influx() {
                                    Ok(_) => {
                                        self.status_message = "InfluxDB backup completed!".to_string();
                                    }
                                    Err(e) => {
                                        self.status_message = self.handle_error(&e, "InfluxDB backup");
                                    }
                                }
                            }
                            Err(e) => {
                                self.status_message = self.handle_error(&e, "InfluxDB backup");
                            }
                        }
                    }

                    if ui.button("Backup Grafana").clicked() {
                        self.status_message = "Backing up Grafana...".to_string();
                        match ConfigGenerator::new(self.config.clone()) {
                            Ok(generator) => {
                                match generator.backup_grafana() {
                                    Ok(_) => {
                                        self.status_message = "Grafana backup completed!".to_string();
                                    }
                                    Err(e) => {
                                        let error_message = e.user_friendly_message("Grafana backup");
                                        self.update_error_flags(&error_message);
                                        self.status_message = error_message;
                                    }
                                }
                            }
                            Err(e) => {
                                let error_message = e.user_friendly_message("Grafana backup");
                                self.update_error_flags(&error_message);
                                self.status_message = error_message;
                            }
                        }
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("Get Telegraf Status").clicked() {
                        self.status_message = "Retrieving Telegraf status...".to_string();
                        match ConfigGenerator::new(self.config.clone()) {
                            Ok(generator) => {
                                match generator.get_telegraf_status() {
                                    Ok(status) => {
                                        if status.is_empty() {
                                            self.status_message = "Error: Telegraf status returned empty. Telegraf may not be running.".to_string();
                                        } else {
                                            self.status_message = format!("Telegraf Status:\n{}", status);
                                        }
                                    }
                                    Err(e) => {
                                        self.status_message = self.handle_error(&e, "status");
                                    }
                                }
                            }
                            Err(e) => {
                                self.status_message = self.format_error_message(&e.to_string(), "status");
                            }
                        }
                    }

                    if ui.button("Get Telegraf Logs").clicked() {
                        self.status_message = "Retrieving Telegraf logs...".to_string();
                        match ConfigGenerator::new(self.config.clone()) {
                            Ok(generator) => {
                                match generator.get_telegraf_logs(30) {
                                    Ok(logs) => {
                                        if logs.is_empty() {
                                            self.status_message = "Error: Telegraf logs are empty. The log file may exist but is empty.".to_string();
                                        } else {
                                            self.status_message = format!("Telegraf Logs (Last 30 lines):\n{}", logs);
                                        }
                                    }
                                    Err(e) => {
                                        self.status_message = self.handle_error(&e, "logs");
                                    }
                                }
                            }
                            Err(e) => {
                                self.status_message = self.format_error_message(&e.to_string(), "logs");
                            }
                        }
                    }
                });
            });

            // Status Message
            if !self.status_message.is_empty() {
                // Add a header to make it more visible
                ui.separator();
                ui.heading("Command Output:");
                // Create a frame with a border to make the output more visible
                let frame = egui::Frame::dark_canvas(ui.style())
                    .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE))
                    .inner_margin(egui::style::Margin::same(8.0))
                    .outer_margin(egui::style::Margin::same(4.0));
                frame.show(ui, |ui| {
                    // Use scrollable area with fixed height for multiline text
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            // Use a selectable label with monospace font for output
                            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                            let content = self.status_message.clone();
                            // Split by lines and display each line separately
                            for line in content.lines() {
                                ui.label(line);
                            }
                        });
                    // Add status message length for debugging
                    ui.separator();
                    ui.label(format!("Output length: {} characters", self.status_message.len()));
                });
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
