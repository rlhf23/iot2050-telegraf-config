use crate::{
    backend::{
        opcua_poller::{OpcUaNode, OpcUaPoller},
        ConfigGenerator, ServiceType,
    },
    error::{TelegrafError, XmlFileValidation},
    TelegrafConfig, WorkerCommand, WorkerHandle, WorkerResponse,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct XmlFileConfig {
    pub namespace: String,
    pub interval_ms: String,
    pub ip: String,
}

#[derive(Default, Debug, PartialEq, Clone)]
pub enum OpcUaBrowseState {
    #[default]
    Idle,
    BrowsingNodes,
    BrowsingNodesFailed(String),
    BrowsingNodesComplete,
    GettingNamespaces,
    GettingNamespacesFailed(String),
    GettingNamespacesComplete,
}

#[derive(Default, Debug)]
pub struct FormState {
    pub show_namespace_error: bool,
    pub show_iot_host_error: bool,
    pub ip_errors: HashMap<String, bool>,
}

/// Controller that handles all business logic for the GUI
pub struct GuiController {
    pub config: TelegrafConfig,
    pub xml_files: Vec<String>,
    pub selected_listener_files: Vec<bool>,
    pub file_configs: HashMap<String, XmlFileConfig>,
    pub status_messages: Vec<String>,
    pub form_state: FormState,
    pub opcua_nodes: Vec<OpcUaNode>,
    pub show_opcua_browser: bool,
    pub browse_status_message: String,
    pub worker: Option<WorkerHandle>,
    pub is_working: bool,
    pub opcua_browse_state: OpcUaBrowseState,
    pub should_scroll: bool,
}

impl GuiController {
    pub fn new() -> Self {
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // Remove the executable name

        let worker = Some(WorkerHandle::new());

        let mut controller = Self {
            config: TelegrafConfig {
                folder: path.clone(),
                ip: env!("DEFAULT_IP").to_string(),
                username: env!("DEFAULT_USERNAME").to_string(),
                password: env!("DEFAULT_PASSWORD").to_string(),
                iot_host: env!("DEFAULT_IOT_IP").to_string(),
                iot_username: env!("DEFAULT_IOT_USERNAME").to_string(),
                iot_password: env!("DEFAULT_IOT_PASSWORD").to_string(),
                listener_files: Vec::new(),
                output_format: Some("influxdb".to_string()),
                include_test_inputs: true,
                selected_opcua_nodes: Vec::new(),
                use_source_timestamp: false, // Default to "gather"
            },
            xml_files: Vec::new(),
            selected_listener_files: Vec::new(),
            file_configs: HashMap::new(),
            status_messages: Vec::new(),
            form_state: FormState::default(),
            opcua_nodes: Vec::new(),
            show_opcua_browser: false,
            browse_status_message: String::new(),
            worker,
            is_working: false,
            opcua_browse_state: OpcUaBrowseState::default(),
            should_scroll: true,
        };

        // Load initial data
        controller.load_xml_files();
        controller
    }

    /// Load XML files from the configured folder
    pub fn load_xml_files(&mut self) {
        self.xml_files = crate::discover_xml_files(&self.config.folder);
        self.selected_listener_files = vec![false; self.xml_files.len()];

        // Initialize configs for new files
        for file in &self.xml_files {
            self.file_configs.entry(file.clone()).or_default();
        }
    }

    /// Update folder and reload XML files
    pub fn update_folder(&mut self, new_folder: std::path::PathBuf) {
        self.config.folder = new_folder;
        self.xml_files = std::fs::read_dir(&self.config.folder)
            .unwrap()
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

    /// Send a command to the worker and update UI state
    pub fn send_worker_command(&mut self, command: WorkerCommand, working_message: &str) {
        if let Some(worker) = &self.worker {
            if let Err(e) = worker.send_command(command) {
                self.status_messages.push(format!("Failed to start operation: {}", e));
            } else {
                self.is_working = true;
                self.status_messages.push(working_message.to_string());
            }
        } else {
            self.status_messages.push("Worker not initialized".to_string());
        }
    }

    /// Process worker responses and update state accordingly
    pub fn process_worker_responses(&mut self) -> bool {
        let mut should_repaint = false;

        if let Some(worker) = &self.worker {
            if let Some(response) = worker.try_get_response() {
                // Set working state for non-progress responses
                if !matches!(response, WorkerResponse::ProgressUpdate(_)) {
                    self.is_working = false;
                    self.should_scroll = true;
                }

                // Process the response
                match response {
                    WorkerResponse::DummyResponse => {
                        self.status_messages.push("Dummy operation completed!".to_string());
                    }
                    WorkerResponse::SshCommandOutput(output) => {
                        self.status_messages.push(format!("SSH command output:\n{}", output));
                    }
                    WorkerResponse::SshError(err) => {
                        self.status_messages.push(format!("SSH error: {}", err));
                    }
                    WorkerResponse::FileTransferComplete => {
                        self.status_messages.push("File transfer completed successfully".to_string());
                    }
                    WorkerResponse::FileTransferError(err) => {
                        self.status_messages.push(format!("File transfer error: {}", err));
                    }
                    WorkerResponse::ProgressUpdate(progress) => {
                        // Update or add progress message
                        if let Some(last_msg) = self.status_messages.last_mut() {
                            if last_msg.starts_with("Progress:") {
                                *last_msg = progress;
                            } else if last_msg != &progress {
                                self.status_messages.push(progress);
                            }
                        } else {
                            self.status_messages.push(progress);
                        }
                        self.should_scroll = true;
                    }
                    WorkerResponse::OpcUaNodes(nodes) => {
                        self.opcua_nodes = nodes;
                        self.opcua_browse_state = OpcUaBrowseState::BrowsingNodesComplete;
                        self.browse_status_message = if self.opcua_nodes.is_empty() {
                            "OPC UA structure loaded, but no nodes found.".to_string()
                        } else {
                            "OPC UA structure loaded successfully.".to_string()
                        };
                    }
                    WorkerResponse::OpcUaNamespaces(namespace_map) => {
                        let mut found_count = 0;
                        for (file_name, namespace_index) in namespace_map {
                            if let Some(full_path) = self
                                .xml_files
                                .iter()
                                .find(|path| path.ends_with(&file_name))
                            {
                                if let Some(config) = self.file_configs.get_mut(full_path) {
                                    config.namespace = namespace_index.to_string();
                                    found_count += 1;
                                }
                            }
                        }
                        if found_count > 0 {
                            self.status_messages.push(
                                format!("Found namespaces for {} XML files!", found_count)
                            );
                        } else {
                            self.status_messages.push("No matching namespaces found. Check XML filenames match namespace names.".to_string());
                        }
                        self.opcua_browse_state = OpcUaBrowseState::GettingNamespacesComplete;
                    }
                    WorkerResponse::OpcUaError(err) => {
                        self.status_messages.push(format!("OPC UA operation failed: {}", err));
                        match self.opcua_browse_state {
                            OpcUaBrowseState::BrowsingNodes => {
                                self.opcua_browse_state =
                                    OpcUaBrowseState::BrowsingNodesFailed(err.clone());
                                self.browse_status_message =
                                    format!("Failed to browse OPC UA structure: {}", err);
                            }
                            OpcUaBrowseState::GettingNamespaces => {
                                self.opcua_browse_state =
                                    OpcUaBrowseState::GettingNamespacesFailed(err);
                            }
                            _ => {}
                        }
                    }
                    WorkerResponse::Error(err) => {
                        self.status_messages.push(format!("Worker error: {}", err));
                        self.is_working = false;
                    }
                }
                should_repaint = true;
            } else if self.is_working {
                should_repaint = true;
            }
        }

        should_repaint
    }

    /// Handle error and set appropriate UI flags
    pub fn handle_error(&mut self, error: &TelegrafError, context: &str) -> String {
        let message = error.user_friendly_message(context);

        if message.contains("Host Format Error") {
            self.form_state.show_iot_host_error = true;
        }

        message
    }

    /// Validate IP for a specific file
    pub fn validate_file_ip(&mut self, file: &str, ip: &str) {
        if !ip.is_empty() {
            let validation_result = self.config.validate_ip_for_file(ip);
            self.form_state.ip_errors.insert(file.to_string(), validation_result.is_err());
        } else {
            self.form_state.ip_errors.insert(file.to_string(), false);
        }
    }

    /// Load children for an OPC UA node
    pub fn load_node_children(&self, node: &OpcUaNode, indent_level: usize) -> Result<Vec<OpcUaNode>, Box<dyn std::error::Error>> {
        let poller = OpcUaPoller::new(self.config.connection_config())?;
        Ok(poller.load_node_children(node, indent_level)?)
    }

    /// Add selected nodes to configuration
    pub fn add_selected_nodes_to_config(&mut self) {
        self.config.selected_opcua_nodes.clear();
        let selected_nodes = OpcUaNode::convert_selected_nodes_to_config(&self.opcua_nodes);
        self.config.selected_opcua_nodes = selected_nodes;
    }

    /// Start OPC UA browsing
    pub fn start_opcua_browsing(&mut self) {
        self.show_opcua_browser = true;
        if self.opcua_browse_state != OpcUaBrowseState::BrowsingNodes 
            && self.opcua_browse_state != OpcUaBrowseState::BrowsingNodesComplete {
            self.opcua_nodes.clear();
            self.opcua_browse_state = OpcUaBrowseState::BrowsingNodes;
            self.browse_status_message = "Requesting OPC UA structure...".to_string();
            let command = WorkerCommand::BrowseOpcUaNodes {
                config: self.config.clone(),
            };
            self.send_worker_command(command, "Browsing OPC UA structure...");
        } else if self.opcua_browse_state == OpcUaBrowseState::BrowsingNodesComplete {
            self.browse_status_message = "OPC UA structure previously loaded.".to_string();
        }
    }

    /// Start getting OPC UA namespaces
    pub fn start_getting_namespaces(&mut self) {
        if self.opcua_browse_state != OpcUaBrowseState::GettingNamespaces {
            self.opcua_browse_state = OpcUaBrowseState::GettingNamespaces;
            self.status_messages.push("Requesting OPC UA namespaces...".to_string());
            let command = WorkerCommand::GetOpcUaNamespaces {
                config: self.config.clone(),
                xml_files: self.xml_files.clone(),
            };
            self.send_worker_command(command, "Getting OPC UA namespaces...");
        }
    }

    /// Refresh OPC UA structure
    pub fn refresh_opcua_structure(&mut self) {
        if self.opcua_browse_state != OpcUaBrowseState::BrowsingNodes {
            self.opcua_nodes.clear();
            self.opcua_browse_state = OpcUaBrowseState::BrowsingNodes;
            self.browse_status_message = "Requesting OPC UA structure refresh...".to_string();
            let command = WorkerCommand::BrowseOpcUaNodes {
                config: self.config.clone(),
            };
            self.send_worker_command(command, "Refreshing OPC UA structure...");
        }
    }

    /// Generate configuration
    pub fn generate_config(&mut self) {
        // Reset validation state
        self.form_state.show_namespace_error = false;
        self.form_state.show_iot_host_error = false;

        // Convert file_configs to XmlFileValidation for backend validation
        let validation_configs: HashMap<String, XmlFileValidation> =
            self.file_configs.iter().map(|(file, config)| {
                (file.clone(), XmlFileValidation {
                    namespace: config.namespace.clone(),
                    interval_ms: config.interval_ms.clone(),
                    ip: config.ip.clone(),
                })
            }).collect();

        // Perform comprehensive backend validation
        match self.config.validate_config(&validation_configs) {
            Ok(_) => {
                // All validations passed
            },
            Err(errors) => {
                // Handle errors and update UI state
                let error_messages: Vec<String> = errors.iter()
                    .map(|e| e.to_string())
                    .collect();

                // Set appropriate error flags
                for error in &errors {
                    match error {
                        TelegrafError::ValidationError(msg) if msg.contains("namespace") => {
                            self.form_state.show_namespace_error = true;
                        },
                        TelegrafError::HostFormatError(_) => {
                            self.form_state.show_iot_host_error = true;
                        },
                        _ => {}
                    }
                }

                self.status_messages.push(format!("Validation errors: {}", error_messages.join("; ")));
                return;
            }
        }

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
                    Ok(output_path) => {
                        self.status_messages.push(
                            format!("Successfully generated config in {:?}", output_path)
                        );
                    }
                    Err(e) => {
                        let error_message = self.handle_error(&e, "generating config");
                        self.status_messages.push(error_message);
                    }
                }
            }
            Err(e) => {
                let error_message = self.handle_error(&e, "generating config");
                self.status_messages.push(error_message);
            }
        }
    }

    /// Send configuration to IOT device
    pub fn send_config(&mut self) {
        let config = self.config.clone();
        self.send_worker_command(
            WorkerCommand::SendTelegrafConfig { config },
            "Sending configuration..."
        );
    }

    /// Backup InfluxDB
    pub fn backup_influxdb(&mut self) {
        let config = self.config.clone();
        self.send_worker_command(
            WorkerCommand::BackupInfluxDB { config },
            "Backing up InfluxDB..."
        );
    }

    /// Backup Grafana
    pub fn backup_grafana(&mut self) {
        let config = self.config.clone();
        self.send_worker_command(
            WorkerCommand::BackupGrafana { config },
            "Backing up Grafana..."
        );
    }

    /// Get Telegraf status
    pub fn get_telegraf_status(&mut self) {
        let config = self.config.clone();
        self.send_worker_command(
            WorkerCommand::GetTelegrafStatus { config },
            "Retrieving Telegraf status..."
        );
    }

    /// Get Telegraf logs
    pub fn get_telegraf_logs(&mut self) {
        let config = self.config.clone();
        self.send_worker_command(
            WorkerCommand::GetTelegrafLogs { config, lines: 30 },
            "Retrieving Telegraf logs..."
        );
    }

    /// Check service status (Prometheus or InfluxDB)
    pub fn check_service_status(&mut self) {
        let is_prometheus = self
            .config
            .output_format
            .clone()
            .unwrap_or_else(|| "influxdb".to_string())
            == "prometheus";

        let service_name = if is_prometheus { "Prometheus" } else { "InfluxDB" };
        let service_url = self.config.iot_host.clone();
        let config = self.config.clone();
        
        if is_prometheus {
            self.send_worker_command(
                WorkerCommand::CheckServiceStatus {
                    config,
                    service_url: service_url.clone(),
                    service_type: ServiceType::Prometheus,
                    timeout_secs: 5,
                },
                &format!("Checking {} status at {}...", service_name, service_url)
            );
        } else {
            self.send_worker_command(
                WorkerCommand::CheckInfluxDbStatus { config },
                &format!("Checking InfluxDB status at {}...", service_url)
            );
        }
    }

    /// Clear status messages
    pub fn clear_status_messages(&mut self) {
        self.status_messages.clear();
        self.should_scroll = true;
    }

    /// Remove selected OPC UA node at index
    pub fn remove_selected_opcua_node(&mut self, index: usize) {
        if index < self.config.selected_opcua_nodes.len() {
            self.config.selected_opcua_nodes.remove(index);
        }
    }

    /// Restart Telegraf service on the remote device
    pub fn restart_telegraf(&mut self) {
        let host = self.config.iot_host.clone();
        let username = self.config.iot_username.clone();
        let password = self.config.iot_password.clone();
        
        self.send_worker_command(
            WorkerCommand::RestartTelegraf { host, username, password },
            "Restarting Telegraf service..."
        );
    }

    /// Sync system time to the remote device
    pub fn sync_time(&mut self) {
        let host = self.config.iot_host.clone();
        let username = self.config.iot_username.clone();
        let password = self.config.iot_password.clone();
        
        self.send_worker_command(
            WorkerCommand::SyncTime { host, username, password },
            "Syncing system time to device..."
        );
    }
}

impl Default for GuiController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "gui_controller_test.rs"]
mod gui_controller_test;