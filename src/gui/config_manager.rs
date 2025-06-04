use eframe::egui;
use sie_generate_config::{
    backend::ConfigGenerator,
    error::{TelegrafError, XmlFileValidation},
    TelegrafConfig,
};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct XmlFileConfig {
    pub namespace: String,
    pub interval_ms: String,
    pub ip: String,
}

#[derive(Default, Debug)]
pub struct FormState {
    pub show_namespace_error: bool,
    pub show_iot_host_error: bool,
    pub ip_errors: HashMap<String, bool>,
}

pub struct ConfigManager {
    pub config: TelegrafConfig,
    pub xml_files: Vec<String>,
    pub selected_listener_files: Vec<bool>,
    pub file_configs: HashMap<String, XmlFileConfig>,
    pub token_file_path: PathBuf,
    pub form_state: FormState,
}

impl ConfigManager {
    pub fn new() -> Self {
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // Remove the executable name
        let token_file_path = path.join("token.txt");

        let mut manager = Self {
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
                influx_token: Some("${INFLUX_TOKEN}".to_string()),
                listener_files: Vec::new(),
                output_format: Some("influxdb".to_string()),
                include_test_inputs: false,
                selected_opcua_nodes: Vec::new(),
            },
            xml_files: Vec::new(),
            selected_listener_files: Vec::new(),
            file_configs: HashMap::new(),
            token_file_path,
            form_state: FormState::default(),
        };

        manager.load_xml_files();
        manager.load_token();
        manager
    }

    pub fn load_token(&mut self) {
        if let Ok(token_content) = std::fs::read_to_string(&self.token_file_path) {
            self.config.influx_token = Some(token_content.trim().to_string());
        }
    }

    pub fn load_xml_files(&mut self) {
        self.xml_files = sie_generate_config::discover_xml_files(&self.config.folder);
        self.selected_listener_files = vec![false; self.xml_files.len()];

        // Initialize configs for new files
        for file in &self.xml_files {
            self.file_configs.entry(file.clone()).or_default();
        }
    }

    pub fn update_folder(&mut self, new_folder: PathBuf) {
        self.config.folder = new_folder.clone();
        self.config.token_folder = new_folder.clone();
        self.token_file_path = new_folder.join("token.txt");
        self.load_token();
        self.load_xml_files();
    }

    pub fn update_token_file(&mut self, new_path: PathBuf) {
        self.token_file_path = new_path.clone();
        self.config.token_folder = new_path.parent().unwrap_or(&new_path).to_path_buf();
        self.load_token();
    }

    pub fn validate_ip_for_file(&mut self, file: &str, ip: &str) {
        if !ip.is_empty() {
            let validation_result = self.config.validate_ip_for_file(ip);
            self.form_state.ip_errors.insert(file.to_string(), validation_result.is_err());
        } else {
            self.form_state.ip_errors.insert(file.to_string(), false);
        }
    }

    pub fn handle_error(&mut self, error: &TelegrafError, context: &str) -> String {
        let message = error.user_friendly_message(context);

        // Set UI error flags based on the error message
        if message.contains("Host Format Error") {
            self.form_state.show_iot_host_error = true;
        }

        message
    }

    pub fn validate_and_generate_config(&mut self) -> Result<String, String> {
        // Reset validation state
        self.form_state.show_namespace_error = false;
        self.form_state.show_iot_host_error = false;

        // Convert file_configs to XmlFileValidation for backend validation
        let validation_configs: HashMap<String, XmlFileValidation> = self
            .file_configs
            .iter()
            .map(|(file, config)| {
                (
                    file.clone(),
                    XmlFileValidation {
                        namespace: config.namespace.clone(),
                        interval_ms: config.interval_ms.clone(),
                        ip: config.ip.clone(),
                    },
                )
            })
            .collect();

        // Perform comprehensive backend validation
        if let Err(errors) = self.config.validate_config(&validation_configs) {
            let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();

            // Set appropriate error flags
            for error in &errors {
                match error {
                    TelegrafError::ValidationError(msg) if msg.contains("namespace") => {
                        self.form_state.show_namespace_error = true;
                    }
                    TelegrafError::HostFormatError(_) => {
                        self.form_state.show_iot_host_error = true;
                    }
                    _ => {}
                }
            }

            return Err(format!("Validation errors: {}", error_messages.join("; ")));
        }

        // Set bucket name default if empty
        if self.config.bucket_name.is_empty() {
            self.config.bucket_name = "line".to_string();
        }

        // Update listener files
        self.config.listener_files = self
            .xml_files
            .iter()
            .zip(self.selected_listener_files.iter())
            .filter(|(_, &selected)| selected)
            .map(|(file, _)| file.clone())
            .collect();

        // Generate configuration
        match ConfigGenerator::new(self.config.clone()) {
            Ok(mut generator) => {
                // Set configurations for each file
                for file in &self.xml_files {
                    if let Some(file_config) = self.file_configs.get(file) {
                        let is_listener = self.config.listener_files.contains(file);
                        let default_interval = if is_listener { 500 } else { 1000 };

                        let interval_ms = file_config
                            .interval_ms
                            .parse()
                            .unwrap_or(default_interval);

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

                match generator.generate_config(&self.xml_files, &self.config.listener_files) {
                    Ok(_) => Ok("Configuration generated successfully!".to_string()),
                    Err(e) => Err(self.handle_error(&e, "generating config")),
                }
            }
            Err(e) => Err(self.handle_error(&e, "generating config")),
        }
    }

    pub fn is_prometheus_format(&self) -> bool {
        self.config
            .output_format
            .clone()
            .unwrap_or_else(|| "influxdb".to_string())
            == "prometheus"
    }

    pub fn toggle_output_format(&mut self) {
        let is_prometheus = self.is_prometheus_format();
        self.config.output_format = Some(if is_prometheus {
            "influxdb".to_string()
        } else {
            "prometheus".to_string()
        });
    }
}
