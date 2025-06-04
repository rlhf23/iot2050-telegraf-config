use eframe::egui;
use sie_generate_config::{
    backend::{
        opcua_poller::{OpcUaNode, OpcUaPoller},
        ConfigGenerator, ServiceType,
    },
    error::{TelegrafError, XmlFileValidation},
    TelegrafConfig, WorkerHandle, WorkerCommand, WorkerResponse,
};

#[derive(Default)]
struct XmlFileConfig {
    namespace: String,
    interval_ms: String,
    ip: String,
}

#[derive(Default)]
struct FormState {
    show_namespace_error: bool, // Track if we should show namespace errors
    show_iot_host_error: bool,  // Track if IOT host is invalid
    ip_errors: std::collections::HashMap<String, bool>, // Track file IP validation errors
}

struct TelegrafApp {
    config: TelegrafConfig,
    xml_files: Vec<String>,
    selected_listener_files: Vec<bool>, // Checkboxes for listener selection
    file_configs: std::collections::HashMap<String, XmlFileConfig>,
    status_message: String,
    token_file_path: std::path::PathBuf, // Store the complete token file path
    form_state: FormState,               // Validation state for the form
    opcua_nodes: Vec<OpcUaNode>,         // Store the complete OPC UA node hierarchy
    show_opcua_browser: bool,            // Toggle for showing the OPC UA browser
    browse_status_message: String,       // Status message for OPC UA browsing
    worker: Option<WorkerHandle>,        // Background worker for async operations
    is_working: bool,                   // Whether a background operation is in progress
    opcua_browse_state: OpcUaBrowseState, // State for OPC UA browsing operations
}

#[derive(Default, Debug, PartialEq, Clone)]
enum OpcUaBrowseState {
    #[default]
    Idle,
    BrowsingNodes,
    BrowsingNodesFailed(String),
    BrowsingNodesComplete,
    GettingNamespaces,
    GettingNamespacesFailed(String),
    GettingNamespacesComplete,
}

impl TelegrafApp {
    /// Helper method to send a command to the worker and update UI state
    fn send_worker_command(&mut self, command: WorkerCommand, working_message: &str) {
        if let Some(worker) = &self.worker {
            if let Err(e) = worker.send_command(command) {
                self.status_message = format!("Failed to start operation: {}", e);
            } else {
                self.is_working = true;
                self.status_message = working_message.to_string();
            }
        } else {
            self.status_message = "Worker not initialized".to_string();
        }
    }

    fn load_token(&mut self) {
        if let Ok(token_content) = std::fs::read_to_string(&self.token_file_path) {
            self.config.influx_token = Some(token_content.trim().to_string());
        }
    }

    fn load_xml_files(&mut self) {
        // Use the shared function from lib.rs
        self.xml_files = sie_generate_config::discover_xml_files(&self.config.folder);
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
            self.form_state.show_iot_host_error = true;
        }

        // Additional flags can be set here as needed

        message
    }

    // Render the OPC UA node tree recursively with support for lazy loading
    fn render_node_tree(
        &mut self,
        ui: &mut egui::Ui,
        nodes: &mut [OpcUaNode],
        indent_level: usize,
    ) {
        for node in nodes.iter_mut() {
            // Calculate indentation
            let indent = (indent_level as f32) * 20.0; // 20 pixels per indent level
            ui.horizontal(|ui| {
                ui.add_space(indent);

                // Determine if this is a folder-like node
                let is_folder = node.is_folder_node();
                
                // Show checkboxes for variables that can be selected and for folders
                // Skip checkbox for the root node (at indent_level 0)
                if (node.node_class == opcua::types::NodeClass::Variable || is_folder) && indent_level > 0 {
                    if ui.checkbox(&mut node.selected, "").changed() {
                        // For variables: If a node is deselected, also deselect all its children
                        if !node.selected && node.node_class == opcua::types::NodeClass::Variable {
                            node.deselect_children();
                        }
                        
                        // For folders: When selected, ensure children are loaded
                        if node.selected && is_folder && !node.children_loaded {
                            // Clone to avoid borrow issues
                            let node_clone = node.clone();
                            
                            // Create a new poller to load children
                            if let Ok(poller) = OpcUaPoller::new(self.config.clone()) {
                                match poller.load_node_children(&node_clone, indent_level) {
                                    Ok(children) => {
                                        // Update the node with loaded children
                                        node.children = children;
                                        node.children_loaded = true;
                                        
                                        // Force a redraw
                                        ui.ctx().request_repaint();
                                    }
                                    Err(e) => {
                                        // Log the error but don't display it in the UI to avoid disrupting the layout
                                        eprintln!("Error loading folder children when selecting checkbox: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }

                // Node display - show different icons based on node type
                let node_icon = match node.node_class {
                    opcua::types::NodeClass::Object => "[O] ",
                    opcua::types::NodeClass::Variable => "[V] ",
                    opcua::types::NodeClass::Method => "[M] ",
                    opcua::types::NodeClass::ObjectType => "[T] ",
                    opcua::types::NodeClass::VariableType => "[VT] ",
                    opcua::types::NodeClass::ReferenceType => "[R] ",
                    opcua::types::NodeClass::DataType => "[D] ",
                    opcua::types::NodeClass::View => "[~] ",
                    _ => "[?] ",
                };

                // Check if this is a folder-like node that can have children
                if is_folder {
                    
                    let label = format!("{}{} ({:?})", node_icon, node.display_name, node.node_class);
                    
                    // Use collapsing header to show node and its children
                    let header = ui.collapsing(label, |ui| {
                        // Show additional node information
                        if let Some(data_type) = &node.data_type {
                            ui.label(format!("Data Type: {}", data_type));
                        }
                        if let Some(description) = &node.description {
                            ui.label(format!("Description: {}", description));
                        }

                        // Check if children need to be loaded
                        if !node.children_loaded && node.children.is_empty() {
                            // Show loading indicator
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Loading children...");
                            });

                            // Clone to avoid borrow issues
                            let node_clone = node.clone();

                            // Create a new poller to load children
                            if let Ok(poller) = OpcUaPoller::new(self.config.clone()) {
                                match poller.load_node_children(&node_clone, indent_level) {
                                    Ok(children) => {
                                        // Update the node with loaded children
                                        node.children = children;
                                        node.children_loaded = true;

                                        // Force a redraw
                                        ui.ctx().request_repaint();
                                    }
                                    Err(e) => {
                                        ui.label(format!("Error loading children: {}", e));
                                    }
                                }
                            }
                        } else {
                            // Render children recursively
                            self.render_node_tree(ui, &mut node.children, indent_level + 1);
                        }
                    });

                    // If the header is expanded, we don't need to show the hover text
                    if !header.fully_open() {
                        header.header_response.on_hover_text(format!(
                            "NodeId: {:?}\nNamespace: {}\nBrowse Name: {}{}",
                            node.node_id,
                            node.node_id.namespace,
                            node.browse_name,
                            if let Some(data_type) = &node.data_type {
                                format!("\nData Type: {}", data_type)
                            } else {
                                String::new()
                            }
                        ));
                    }
                } else {
                    // Leaf node without children
                    let label =
                        format!("{}{} ({:?})", node_icon, node.display_name, node.node_class);
                    ui.label(label).on_hover_text(format!(
                        "NodeId: {:?}\nNamespace: {}\nBrowse Name: {}{}",
                        node.node_id,
                        node.node_id.namespace,
                        node.browse_name,
                        if let Some(data_type) = &node.data_type {
                            format!("\nData Type: {}", data_type)
                        } else {
                            String::new()
                        }
                    ));
                }
            });
        }
    }

    // Add selected nodes to the configuration
    fn add_selected_nodes_to_config(&mut self) {
        // Clear any existing selections first
        self.config.selected_opcua_nodes.clear();

        // Use the new utility function to convert selected nodes to configuration
        let selected_nodes = OpcUaNode::convert_selected_nodes_to_config(&self.opcua_nodes);
        self.config.selected_opcua_nodes = selected_nodes;
    }

    fn default() -> Self {
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // Remove the executable name
        let token_file_path = path.join("token.txt");

        let worker = Some(WorkerHandle::new());
        
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
                influx_token: Some("${INFLUX_TOKEN}".to_string()), // moved to telegraf env var
                listener_files: Vec::new(),
                output_format: Some("influxdb".to_string()),
                include_test_inputs: false,
                selected_opcua_nodes: Vec::new(),
            },
            xml_files: Vec::new(),
            selected_listener_files: Vec::new(),
            file_configs: std::collections::HashMap::new(),
            status_message: String::new(),
            token_file_path, // Default to token.txt in the current directory
            form_state: FormState::default(),
            opcua_nodes: Vec::new(),
            show_opcua_browser: false,
            browse_status_message: String::new(),
            worker,
            is_working: false,
            opcua_browse_state: OpcUaBrowseState::default(),
        };
        
        // Load initial data
        app.load_xml_files();
        app.load_token();
        
        app
    }
}

impl eframe::App for TelegrafApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for worker responses
        if let Some(worker) = &self.worker {
            // First, check if we have any responses
            if let Some(response) = worker.try_get_response() {
                // Process progress updates separately to maintain working state
                if let WorkerResponse::ProgressUpdate(progress) = &response {
                    self.status_message = progress.clone();
                } else {
                    self.is_working = false;
                }
                
                // Process the response
                match response {
                    WorkerResponse::DummyResponse => {
                        self.status_message = "Dummy operation completed!".to_string();
                    }
                    WorkerResponse::SshCommandOutput(output) => {
                        self.status_message = format!("SSH command output:\n{}", output);
                    }
                    WorkerResponse::SshError(err) => {
                        self.status_message = format!("SSH error: {}", err);
                    }
                    WorkerResponse::FileTransferComplete => {
                        self.status_message = "File transfer completed successfully".to_string();
                    }
                    WorkerResponse::FileTransferError(err) => {
                        self.status_message = format!("File transfer error: {}", err);
                    }
                    WorkerResponse::ProgressUpdate(_) => {
                        // Already handled above to maintain working state
                    }
                    WorkerResponse::OpcUaNodes(nodes) => {
                        self.opcua_nodes = nodes;
                        self.opcua_browse_state = OpcUaBrowseState::BrowsingNodesComplete;
                        self.browse_status_message = "OPC UA structure loaded successfully.".to_string();
                    }
                    WorkerResponse::OpcUaNamespaces(namespace_map) => {
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
                        self.opcua_browse_state = OpcUaBrowseState::GettingNamespacesComplete;
                    }
                    WorkerResponse::OpcUaError(err) => {
                        self.status_message = format!("OPC UA error: {}", err);
                        match self.opcua_browse_state {
                            OpcUaBrowseState::BrowsingNodes => {
                                self.opcua_browse_state = OpcUaBrowseState::BrowsingNodesFailed(err.clone());
                                self.browse_status_message = err;
                            }
                            OpcUaBrowseState::GettingNamespaces => {
                                self.opcua_browse_state = OpcUaBrowseState::GettingNamespacesFailed(err);
                            }
                            _ => {
                                // Other states are not expected to transition here on a generic OpcUaError,
                                // or the error is not specific to an ongoing browse/get_namespace operation.
                                // We've already set a general self.status_message.
                            }
                        }
                            _ => {}
                        }
                    }
                }
                // Always request a repaint when we have a response
                ctx.request_repaint();
            } else if self.is_working {
                // If we're working but don't have a response yet, request a repaint
                // to keep the spinner animating
                ctx.request_repaint();
            }
        }

        // Show loading indicator next to Command Output when working
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

                // Main Configuration using Grid
                egui::Grid::new("config_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        // IP Configuration
                        ui.label("OPC IP:");
                        ui.text_edit_singleline(&mut self.config.ip);
                        ui.end_row();

                        // IOT Host with validation
                        ui.label("IOT Host:");
                        let text_edit = egui::TextEdit::singleline(&mut self.config.iot_host);
                        if self.form_state.show_iot_host_error {
                            egui::Frame::NONE
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    egui::Color32::from_rgb(255, 0, 0),
                                ))
                                .show(ui, |ui| ui.add(text_edit));
                        } else {
                            ui.add(text_edit);
                        }
                        ui.end_row();
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
                    egui::Grid::new("credentials_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // OPC Username
                            ui.label("OPC Username:");
                            ui.text_edit_singleline(&mut self.config.username);
                            ui.end_row();
                            // OPC Password
                            ui.label("OPC Password:");
                            ui.add(egui::TextEdit::singleline(&mut self.config.password).password(true));
                            ui.end_row();
                            // IOT Username
                            ui.label("IOT Username:");
                            ui.text_edit_singleline(&mut self.config.iot_username);
                            ui.end_row();
                            // IOT Password
                            ui.label("IOT Password:");
                            ui.add(egui::TextEdit::singleline(&mut self.config.iot_password).password(true));
                            ui.end_row();
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

                        egui::Grid::new("influxdb_options_grid")
                            .num_columns(2)
                            .spacing([40.0, 4.0])
                            .show(ui, |ui| {
                                // Token input
                                ui.label("InfluxDB Token:");
                                if ui.text_edit_singleline(&mut token).changed() {
                                    self.config.influx_token = Some(token);
                                }
                                ui.end_row();

                                // Token file path
                                ui.label("Token File:");
                                ui.horizontal(|ui| {
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
                                ui.end_row();
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

                        // Use grid layout for all fields in the XML file configuration
                        egui::Grid::new(&format!("xml_file_grid_{}", i))
                            .num_columns(2)
                            .spacing([40.0, 4.0])
                            .show(ui, |ui| {
                                // Namespace input
                                ui.label("Namespace:");
                                let text_edit = egui::TextEdit::singleline(&mut file_config.namespace);
                                if self.form_state.show_namespace_error {
                                    egui::Frame::NONE
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgb(255, 0, 0),
                                        ))
                                        .show(ui, |ui| ui.add(text_edit));
                                } else {
                                    ui.add(text_edit);
                                }
                                ui.end_row();

                                // IP Address input
                                ui.label("OPC IP:");

                                // Check if we have a validation error for this file
                                let has_error = self.form_state.ip_errors.get(file).unwrap_or(&false);

                                // Show the field with appropriate styling
                                let response = if *has_error {
                                    // If there's an error, show red border
                                    let response = egui::Frame::NONE
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
                                        // Use the backend validation logic
                                        let validation_result = self.config.validate_ip_for_file(&file_config.ip);

                                        // Update error state
                                        self.form_state.ip_errors.insert(file.clone(), validation_result.is_err());
                                    } else {
                                        // Empty IP means no error (will use default)
                                        self.form_state.ip_errors.insert(file.clone(), false);
                                    }
                                }
                                ui.end_row();

                                // Interval input
                                let is_listener = self.selected_listener_files[i];
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
                ui.horizontal(|ui| {
                    // OPC UA Browser button
                    if ui.button("Browse OPC UA Structure").clicked() {
                        self.show_opcua_browser = true;
                        if self.opcua_nodes.is_empty() && self.opcua_browse_state == OpcUaBrowseState::Idle {
                            self.opcua_browse_state = OpcUaBrowseState::BrowsingNodes;
                            self.browse_status_message = "Browsing OPC UA structure...".to_string();
                            
                            // Send command to worker
                            let command = WorkerCommand::BrowseOpcUaNodes {
                                config: self.config.clone(),
                            };
                            self.send_worker_command(command, "Browsing OPC UA structure...");
                        }
                    }
                
                    if ui.button("Get OPC UA Namespaces").clicked() {
                        self.status_message = "Getting OPC UA namespaces...".to_string();
                        self.opcua_browse_state = OpcUaBrowseState::GettingNamespaces;
                        
                        // Send command to worker
                        let command = WorkerCommand::GetOpcUaNamespaces {
                            config: self.config.clone(),
                            xml_files: self.xml_files.clone(),
                        };
                        self.send_worker_command(command, "Getting OPC UA namespaces...");
                    }
                    
                });
                
                if ui.button("Generate Config").clicked() {
                    // Reset validation state
                    self.form_state.show_namespace_error = false;
                    self.form_state.show_iot_host_error = false;

                    // Convert our file_configs to XmlFileValidation for backend validation
                    let validation_configs: std::collections::HashMap<String, XmlFileValidation> =
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

                            self.status_message = format!("Validation errors: {}", error_messages.join("; "));
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
                    let config = self.config.clone();
                    self.send_worker_command(
                        WorkerCommand::SendTelegrafConfig { config },
                        "Sending configuration..."
                    );
                }
            });

            // Other Commands Section
            ui.collapsing("Other Commands", |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Backup InfluxDB").clicked() {
                        let config = self.config.clone();
                        self.send_worker_command(
                            WorkerCommand::BackupInfluxDB { config },
                            "Backing up InfluxDB..."
                        );
                    }

                    if ui.button("Backup Grafana").clicked() {
                        let config = self.config.clone();
                        self.send_worker_command(
                            WorkerCommand::BackupGrafana { config },
                            "Backing up Grafana..."
                        );
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("Get Telegraf Status").clicked() {
                        let config = self.config.clone();
                        self.send_worker_command(
                            WorkerCommand::GetTelegrafStatus { config },
                            "Retrieving Telegraf status..."
                        );
                    }

                    if ui.button("Get Telegraf Logs").clicked() {
                        let config = self.config.clone();
                        self.send_worker_command(
                            WorkerCommand::GetTelegrafLogs { config, lines: 30 },
                            "Retrieving Telegraf logs..."
                        );
                    }
                });

                // Add a new row for service status check
                ui.horizontal(|ui| {
                    // Calculate is_prometheus value
                    let is_prometheus = self
                        .config
                        .output_format
                        .clone()
                        .unwrap_or_else(|| "influxdb".to_string())
                        == "prometheus";

                    // Service name based on output format
                    let service_name = if is_prometheus { "Prometheus" } else { "InfluxDB" };

                    if ui.button(format!("Check {} Status", service_name)).clicked() {
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
                });
            });

            // Status Message
            if !self.status_message.is_empty() {
                // Add a header to make it more visible
                ui.separator();
                ui.horizontal(|ui| {
                    ui.heading("Command Output:");
                    if self.is_working {
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

            // OPC UA Browser Panel
            if self.show_opcua_browser {
                ui.separator();
                ui.heading("OPC UA Browser");
                
                ui.horizontal(|ui| {
                        ui.label("Status: ");
                        ui.label(&self.browse_status_message);
                        
                        if ui.button("Close Browser").clicked() {
                            self.show_opcua_browser = false;
                        }
                    });
                    
                    // Display the node tree with checkboxes
                    if !self.opcua_nodes.is_empty() {
                        // Create a clone of opcua_nodes to avoid borrowing issues
                        let mut nodes_clone = self.opcua_nodes.clone();
                        ui.push_id("opcua_browser_area", |ui| {
                            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                                // Render using the cloned nodes
                                self.render_node_tree(ui, &mut nodes_clone, 0);
                            });
                        });
                        
                        // Update the original nodes with any changes from UI
                        self.opcua_nodes = nodes_clone;
                        
                        // Add selected nodes button
                        if ui.button("Add Selected Nodes to Configuration").clicked() {
                            self.add_selected_nodes_to_config();
                            self.status_message = "Selected nodes added to configuration.".to_string();
                        }
                    } else if self.is_browsing_opcua {
                        ui.spinner();
                        ui.label("Loading OPC UA structure... This may take a moment.");
                    } else {
                        ui.label("No OPC UA structure available. Click 'Browse OPC UA Structure' to load.");
                    }
                    
                    // Display currently selected nodes
                    if !self.config.selected_opcua_nodes.is_empty() {
                        ui.heading("Selected Nodes");
                        ui.push_id("selected_nodes_area", |ui| {
                            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                egui::Grid::new("selected_opcua_nodes_grid")
                                .num_columns(5)
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.label("Node Name");
                                    ui.label("Namespace");
                                    ui.label("Measurement Name");
                                    ui.label("Interval (ms)");
                                    ui.label("Actions");
                                    ui.end_row();
                                    
                                    let mut nodes_to_remove = Vec::new();
                                    
                                    for (i, node) in self.config.selected_opcua_nodes.iter_mut().enumerate() {
                                        ui.label(&node.display_name);
                                        ui.label(&node.namespace.to_string());
                                        
                                        // Allow editing the measurement name
                                        let mut measurement_name = node.measurement_name.clone();
                                        if ui.text_edit_singleline(&mut measurement_name).changed() {
                }
                OpcUaBrowseState::BrowsingNodesFailed(err) => {
                    ui.colored_label(egui::Color32::RED, format!("Failed to browse OPC UA structure: {}", err));
                }
                OpcUaBrowseState::Idle if !self.opcua_nodes.is_empty() => { // Show cached nodes if available
                     egui::ScrollArea::vertical().show(ui, |ui| {
                        self.render_node_tree(ui, &mut self.opcua_nodes, 0);
                    });
                }
                _ => { // Idle and no nodes, or other states not directly related to browsing display
                    ui.label(&self.browse_status_message);
                }
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Add Selected to Config").clicked() {
                    self.add_selected_nodes_to_config();
                    self.status_message = "Selected OPC UA nodes added to configuration.".to_string();
                    self.show_opcua_browser = false; // Close browser after adding
                }
                if ui.button("Refresh Structure").clicked() {
                    self.opcua_nodes.clear(); // Clear existing nodes
                    self.opcua_browse_state = OpcUaBrowseState::BrowsingNodes;
                    self.browse_status_message = "Refreshing OPC UA structure...".to_string();
                    let command = WorkerCommand::BrowseOpcUaNodes {
                        config: self.config.clone(),
                    };
                    self.send_worker_command(command, "Refreshing OPC UA structure...");
                }
                if ui.button("Close").clicked() {
                    self.show_opcua_browser = false;
                }
            });
        });
}

// ... (rest of the code remains the same)
    // let native_options = eframe::NativeOptions {
    //     viewport: Some(egui::vec2(800.0, 1000.0)),
    //     ..Default::default()
    // };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 1000.0]) // wide enough for the drag-drop overlay text
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Telegraf Config Generator",
        options,
        Box::new(|_cc| Ok(Box::new(TelegrafApp::default()))),
    )
}
