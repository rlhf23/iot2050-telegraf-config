use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dirs;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Frame, Terminal,
};
use sie_generate_config::{
    backend::{
        deployment::DeploymentConfig,
        opcua_poller::OpcUaPoller,
        ConfigGenerator, ServiceType,
    },
    TelegrafConfig, WorkerCommand, WorkerHandle, WorkerResponse,
};
use std::collections::HashMap;
use std::{
    fs,
    io,
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq)]
enum Tab {
    Folder,
    Files,
    OpcUaConfig,
    IoTConfig,
    Device,
    Config,
    Actions,
}

#[derive(Debug, Clone, PartialEq)]
enum InputMode {
    Normal,
    Editing,
}

#[derive(Default, Clone)]
struct XmlFileConfig {
    pub namespace: String,
    pub interval_ms: String,
    pub ip: String,
}

#[derive(Debug, Clone, PartialEq)]
enum EditField {
    OpcUaIp,
    OpcUaUsername,
    OpcUaPassword,
    IoTHost,
    IoTUsername,
    IoTPassword,
    DeviceName,
    KeyFile,
    GitBranch,
    FileNamespace(usize),
    FileIp(usize),
    FileInterval(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum OpcUaConfigField {
    Ip,
    Username,
    Password,
    Anonymous,
    TestInputs,
    OpcuaDiagnostics,
}

#[derive(Debug, Clone, PartialEq)]
enum IoTConfigField {
    Host,
    Username,
    Password,
}

#[derive(Debug, Clone, PartialEq)]
enum ActionsField {
    GenerateConfig,
    SendConfig,
    ClearMessages,
    TelegrafStatus,
    TelegrafLogs,
    RestartTelegraf,
    ServiceStatus,
    BackupGrafana,
}

#[derive(Debug, Clone, PartialEq)]
enum DeviceField {
    Provision,
    Setup,
    Update,
    Start,
    Stop,
    Status,
    Backup,
    Restore,
    SyncTime,
    DeviceName,
    KeyFile,
    GitBranch,
    Minimal,
    LocalTransfer,
}

impl OpcUaConfigField {
    fn count() -> usize {
        6 // Ip, Username, Password, Anonymous, TestInputs, OpcuaDiagnostics
    }

    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Ip,
            1 => Self::Username,
            2 => Self::Password,
            3 => Self::Anonymous,
            4 => Self::TestInputs,
            5 => Self::OpcuaDiagnostics,
            _ => Self::Ip, // Default fallback
        }
    }
}

impl IoTConfigField {
    fn count() -> usize {
        3 // Host, Username, Password
    }

    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Host,
            1 => Self::Username,
            2 => Self::Password,
            _ => Self::Host, // Default fallback
        }
    }
}

impl ActionsField {
    fn count() -> usize {
        8 // GenerateConfig, SendConfig, ClearMessages, TelegrafStatus, TelegrafLogs, RestartTelegraf, ServiceStatus, BackupGrafana
    }

    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::GenerateConfig,
            1 => Self::SendConfig,
            2 => Self::ClearMessages,
            3 => Self::TelegrafStatus,
            4 => Self::TelegrafLogs,
            5 => Self::RestartTelegraf,
            6 => Self::ServiceStatus,
            7 => Self::BackupGrafana,
            _ => Self::GenerateConfig,
        }
    }
}

impl DeviceField {
    fn count() -> usize {
        14 // Provision, Setup, Update, Start, Stop, Status, Backup, Restore, SyncTime, DeviceName, KeyFile, GitBranch, Minimal, LocalTransfer
    }

    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Provision,
            1 => Self::Setup,
            2 => Self::Update,
            3 => Self::Start,
            4 => Self::Stop,
            5 => Self::Status,
            6 => Self::Backup,
            7 => Self::Restore,
            8 => Self::SyncTime,
            9 => Self::DeviceName,
            10 => Self::KeyFile,
            11 => Self::GitBranch,
            12 => Self::Minimal,
            13 => Self::LocalTransfer,
            _ => Self::Provision,
        }
    }
}

struct App {
    // Navigation
    current_tab: Tab,
    input_mode: InputMode,
    current_edit_field: Option<EditField>,

    // Configuration
    config: TelegrafConfig,

    // File management
    xml_files: Vec<String>,
    selected_files: Vec<bool>,
    selected_listener_files: Vec<bool>,
    file_list_state: ListState,

    // Folder navigation
    current_directory: PathBuf,
    directory_entries: Vec<PathBuf>,
    directory_list_state: ListState,

    // Status and messages
    status_messages: Vec<String>,
    status_list_state: ListState,
    show_help: bool,

    // Worker for async operations
    worker: Option<WorkerHandle>,
    is_working: bool,

    // Anonymous mode
    anonymous_mode: bool,

    // Per-file configurations
    file_configs: HashMap<String, XmlFileConfig>,

    // Generated configuration
    generated_config: Option<String>,
    config_scroll: u16,
    config_horizontal_scroll: u16,

    // Temporary input buffer
    input_buffer: String,

    // Config tab selection states
    opcua_config_selection: usize,
    iot_config_selection: usize,
    actions_selection: usize,
    device_selection: usize,

    // Device deployment fields
    device_name: String,
    key_file: String,
    git_branch: String,
    minimal: bool,
    local_transfer: bool,

    // Confirmation for destructive operations
    pending_confirmation: Option<String>,
}

impl App {
    fn new() -> App {
        let mut app = App {
            current_tab: Tab::Folder,
            input_mode: InputMode::Normal,
            current_edit_field: None,
            config: TelegrafConfig {
                folder: PathBuf::from("."),
                ip: env!("DEFAULT_IP").to_string(),
                username: env!("DEFAULT_USERNAME").to_string(),
                password: env!("DEFAULT_PASSWORD").to_string(),
                iot_host: env!("DEFAULT_IOT_IP").to_string(),
                iot_username: env!("DEFAULT_IOT_USERNAME").to_string(),
                iot_password: env!("DEFAULT_IOT_PASSWORD").to_string(),
                listener_files: Vec::new(),
                output_format: Some("influxdb".to_string()),
                include_test_inputs: false,
                include_opcua_diagnostics: false,
                selected_opcua_nodes: Vec::new(),
                use_source_timestamp: false,
                ship_display_name: None,
                ship_hostname: None,
            },
            xml_files: Vec::new(),
            selected_files: Vec::new(),
            selected_listener_files: Vec::new(),
            file_list_state: ListState::default(),
            current_directory: PathBuf::from("."),
            directory_entries: Vec::new(),
            directory_list_state: ListState::default(),
            status_messages: Vec::new(),
            status_list_state: ListState::default(),
            show_help: false,
            worker: Some(WorkerHandle::new()),
            is_working: false,
            anonymous_mode: false,
            file_configs: HashMap::new(),
            generated_config: None,
            config_scroll: 0,
            config_horizontal_scroll: 0,
            input_buffer: String::new(),
            opcua_config_selection: 0,
            iot_config_selection: 0,
            actions_selection: 0,
            device_selection: 0,
            device_name: String::new(),
            key_file: String::new(),
            git_branch: "master".to_string(),
            minimal: false,
            local_transfer: false,
            pending_confirmation: None,
        };

        app.load_directory_entries();
        app.refresh_files();
        app
    }

    fn refresh_files(&mut self) {
        self.config.folder = self.current_directory.clone();
        self.xml_files = sie_generate_config::discover_xml_files(&self.config.folder);
        self.selected_files = vec![false; self.xml_files.len()];
        self.selected_listener_files = vec![false; self.xml_files.len()];

        // Initialize configs for new files
        for file in &self.xml_files {
            self.file_configs.entry(file.clone()).or_default();
        }

        if self.xml_files.is_empty() {
            self.add_status_message("No XML files found in current directory".to_string());
        } else {
            self.add_status_message(format!("Found {} XML files", self.xml_files.len()));
        }
    }

    fn load_directory_entries(&mut self) {
        self.directory_entries.clear();

        // Add parent directory entry if not at root
        if let Some(parent) = self.current_directory.parent() {
            self.directory_entries.push(parent.to_path_buf());
        }

        // Read directory entries
        if let Ok(entries) = fs::read_dir(&self.current_directory) {
            let mut dirs: Vec<PathBuf> = Vec::new();
            let mut files: Vec<PathBuf> = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    files.push(path);
                }
            }

            // Sort directories and files separately
            dirs.sort();
            files.sort();

            // Add directories first, then files
            self.directory_entries.extend(dirs);
            self.directory_entries.extend(files);
        }

        // Reset selection
        self.directory_list_state.select(Some(0));
    }

    fn navigate_to_directory(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.current_directory = path;
            self.load_directory_entries();
            self.refresh_files();
            self.add_status_message(format!(
                "Changed to directory: {}",
                self.current_directory.display()
            ));
        }
    }

    fn enter_selected_directory(&mut self) {
        if let Some(selected) = self.directory_list_state.selected() {
            if selected < self.directory_entries.len() {
                let selected_path = self.directory_entries[selected].clone();

                // Handle parent directory (..) navigation
                if selected == 0 && self.current_directory.parent().is_some() {
                    if let Some(parent) = self.current_directory.parent() {
                        self.navigate_to_directory(parent.to_path_buf());
                    }
                } else if selected_path.is_dir() {
                    self.navigate_to_directory(selected_path);
                }
            }
        }
    }

    fn add_status_message(&mut self, message: String) {
        self.status_messages.push(message);
        // Increased buffer size to accommodate larger outputs like logs
        if self.status_messages.len() > 50 {
            self.status_messages.remove(0);
        }
        // Auto-scroll to the latest message
        if !self.status_messages.is_empty() {
            self.status_list_state
                .select(Some(self.status_messages.len() - 1));
        }
    }

    fn process_worker_responses(&mut self) {
        let responses: Vec<WorkerResponse> = if let Some(worker) = &self.worker {
            let mut responses = Vec::new();
            while let Some(response) = worker.try_get_response() {
                responses.push(response);
            }
            responses
        } else {
            return;
        };

        for response in responses {
            if !matches!(response, WorkerResponse::ProgressUpdate(_)) {
                self.is_working = false;
            }

            match response {
                WorkerResponse::SshCommandOutput(output) => {
                    self.add_status_message(format!(
                        "✅ Command completed successfully:\n{}",
                        output
                    ));
                }
                WorkerResponse::SshError(err) => {
                    self.add_status_message(format!("❌ SSH error: {}", err));
                }
                WorkerResponse::ProgressUpdate(progress) => {
                    self.add_status_message(progress);
                }
                WorkerResponse::Error(err) => {
                    self.add_status_message(format!("❌ Error: {}", err));
                }
                WorkerResponse::ConfirmationNeeded(message) => {
                    self.add_status_message(format!("⚠️  {}", message));
                }
                _ => {
                    self.add_status_message("✅ Operation completed".to_string());
                }
            }
        }
    }

    fn toggle_file_selection(&mut self) {
        if let Some(selected) = self.file_list_state.selected() {
            if selected < self.selected_files.len() {
                self.selected_files[selected] = !self.selected_files[selected];
            }
        }
    }

    fn toggle_listener_selection(&mut self) {
        if let Some(selected) = self.file_list_state.selected() {
            if selected < self.selected_listener_files.len() {
                self.selected_listener_files[selected] = !self.selected_listener_files[selected];
                let filename = self
                    .xml_files
                    .get(selected)
                    .map(|f| {
                        std::path::Path::new(f)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(f)
                    })
                    .unwrap_or("file");

                if self.selected_listener_files[selected] {
                    self.add_status_message(format!(
                        "{} marked as listener (500ms default)",
                        filename
                    ));
                } else {
                    self.add_status_message(format!(
                        "{} unmarked as listener (1000ms default)",
                        filename
                    ));
                }
            }
        }
    }

    fn toggle_anonymous_mode(&mut self) {
        self.anonymous_mode = !self.anonymous_mode;
        if self.anonymous_mode {
            self.add_status_message("Anonymous mode enabled".to_string());
        } else {
            self.add_status_message("Anonymous mode disabled".to_string());
        }
    }

    fn generate_config(&mut self) {
        // Update config based on anonymous mode
        if self.anonymous_mode {
            self.config.username = String::new();
            self.config.password = String::new();
        }

        // Get selected files
        let selected_xml_files: Vec<String> = self
            .xml_files
            .iter()
            .enumerate()
            .filter(|(i, _)| *self.selected_files.get(*i).unwrap_or(&false))
            .map(|(_, file)| file.clone())
            .collect();

        if selected_xml_files.is_empty() && !self.config.include_test_inputs {
            self.add_status_message("No files selected and test inputs not enabled".to_string());
            return;
        }

        match ConfigGenerator::new(self.config.clone()) {
            Ok(mut generator) => {
                // Set configurations for each file (like the GUI does)
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

                // Build listener files list from selected files
                let listener_files: Vec<String> = self
                    .xml_files
                    .iter()
                    .enumerate()
                    .filter_map(|(i, file)| {
                        if i < self.selected_listener_files.len() && self.selected_listener_files[i]
                        {
                            Some(file.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                match generator.generate_config(&self.xml_files, &listener_files) {
                    Ok(result) => {
                        self.generated_config = Some(result.config_content.clone());
                        self.add_status_message(format!(
                            "Config generated with {} measurement(s)",
                            result.measurements.len()
                        ));
                    }
                    Err(e) => {
                        self.generated_config = None;
                        self.add_status_message(format!("Failed to generate config: {}", e));
                    }
                }
            }
            Err(e) => {
                self.generated_config = None;
                self.add_status_message(format!("Failed to create generator: {}", e));
            }
        }
    }

    fn send_config(&mut self) {
        if self.config.iot_host.is_empty() {
            self.add_status_message("IoT host not configured".to_string());
            return;
        }

        match ConfigGenerator::new(self.config.clone()) {
            Ok(mut generator) => {
                // Set configurations for each file (like the GUI does)
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

                match generator.send_config() {
                    Ok(_) => {
                        self.add_status_message("Configuration sent successfully!".to_string());
                    }
                    Err(e) => {
                        self.add_status_message(format!("Failed to send config: {}", e));
                    }
                }
            }
            Err(e) => {
                self.add_status_message(format!("Failed to create generator: {}", e));
            }
        }
    }

    fn start_editing(&mut self, field: EditField) {
        self.input_mode = InputMode::Editing;
        self.current_edit_field = Some(field.clone());

        // Pre-populate input buffer with current value
        self.input_buffer = match field {
            EditField::OpcUaIp => self.config.ip.clone(),
            EditField::OpcUaUsername => self.config.username.clone(),
            EditField::OpcUaPassword => self.config.password.clone(),
            EditField::IoTHost => self.config.iot_host.clone(),
            EditField::IoTUsername => self.config.iot_username.clone(),
            EditField::IoTPassword => self.config.iot_password.clone(),
            EditField::DeviceName => self.device_name.clone(),
            EditField::KeyFile => self.key_file.clone(),
            EditField::GitBranch => self.git_branch.clone(),
            EditField::FileNamespace(idx) => {
                if let Some(file) = self.xml_files.get(idx) {
                    self.file_configs
                        .get(file)
                        .map(|c| c.namespace.clone())
                        .unwrap_or_else(|| "2".to_string())
                } else {
                    "2".to_string()
                }
            }
            EditField::FileIp(idx) => {
                if let Some(file) = self.xml_files.get(idx) {
                    self.file_configs
                        .get(file)
                        .map(|c| c.ip.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            }
            EditField::FileInterval(idx) => {
                if let Some(file) = self.xml_files.get(idx) {
                    self.file_configs
                        .get(file)
                        .map(|c| c.interval_ms.clone())
                        .unwrap_or_else(|| "1000".to_string())
                } else {
                    "1000".to_string()
                }
            }
        };
    }

    fn poll_namespaces(&mut self) {
        if self.xml_files.is_empty() {
            self.add_status_message("No XML files loaded to poll namespaces for".to_string());
            return;
        }

        if self.config.ip.is_empty() {
            self.add_status_message("OPC-UA IP not configured. Set IP first.".to_string());
            return;
        }

        self.add_status_message("Connecting to OPC-UA server to poll namespaces...".to_string());

        // Create OPC-UA poller with current config (relies on internal timeouts)
        match OpcUaPoller::new(self.config.connection_config()) {
            Ok(poller) => {
                match poller.get_namespace_info(&self.xml_files) {
                    Ok(namespace_map) => {
                        if namespace_map.is_empty() {
                            self.add_status_message("No matching namespaces found. Check XML filenames match namespace names.".to_string());
                        } else {
                            let mut updated_count = 0;
                            for (file_name, namespace_index) in namespace_map {
                                // Find the full file path that matches this filename
                                if let Some(full_path) = self.xml_files.iter().find(|path| {
                                    std::path::Path::new(path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .map(|n| n == file_name)
                                        .unwrap_or(false)
                                }) {
                                    // Update the namespace for this file
                                    let config = self
                                        .file_configs
                                        .entry(full_path.clone())
                                        .or_insert_with(|| XmlFileConfig::default());
                                    config.namespace = namespace_index.to_string();
                                    updated_count += 1;
                                }
                            }
                            self.add_status_message(format!(
                                "Successfully updated namespaces for {} file(s)!",
                                updated_count
                            ));
                        }
                    }
                    Err(e) => {
                        self.add_status_message(format!("Failed to poll namespaces: {}", e));
                    }
                }
            }
            Err(e) => {
                self.add_status_message(format!("Failed to create OPC-UA poller: {}", e));
            }
        }
    }

    fn finish_editing(&mut self) {
        if let Some(field) = &self.current_edit_field {
            match field {
                EditField::OpcUaIp => self.config.ip = self.input_buffer.clone(),
                EditField::OpcUaUsername => self.config.username = self.input_buffer.clone(),
                EditField::OpcUaPassword => self.config.password = self.input_buffer.clone(),
                EditField::IoTHost => self.config.iot_host = self.input_buffer.clone(),
                EditField::IoTUsername => self.config.iot_username = self.input_buffer.clone(),
                EditField::IoTPassword => self.config.iot_password = self.input_buffer.clone(),
                EditField::DeviceName => self.device_name = self.input_buffer.clone(),
                EditField::KeyFile => self.key_file = self.input_buffer.clone(),
                EditField::GitBranch => self.git_branch = self.input_buffer.clone(),
                EditField::FileNamespace(idx) => {
                    if let Some(file) = self.xml_files.get(*idx) {
                        let config = self
                            .file_configs
                            .entry(file.clone())
                            .or_insert_with(|| XmlFileConfig::default());
                        config.namespace = self.input_buffer.clone();
                    }
                }
                EditField::FileIp(idx) => {
                    if let Some(file) = self.xml_files.get(*idx) {
                        let config = self
                            .file_configs
                            .entry(file.clone())
                            .or_insert_with(|| XmlFileConfig::default());
                        config.ip = self.input_buffer.clone();
                    }
                }
                EditField::FileInterval(idx) => {
                    if let Some(file) = self.xml_files.get(*idx) {
                        let config = self
                            .file_configs
                            .entry(file.clone())
                            .or_insert_with(|| XmlFileConfig::default());
                        config.interval_ms = self.input_buffer.clone();
                    }
                }
            }
        }

        self.input_mode = InputMode::Normal;
        self.current_edit_field = None;
        self.input_buffer.clear();
    }

    fn cancel_editing(&mut self) {
        self.input_mode = InputMode::Normal;
        self.current_edit_field = None;
        self.input_buffer.clear();
    }

    // Operational commands for Actions tab
    fn get_telegraf_status(&mut self) {
        if self.worker.is_some() {
            self.is_working = true;
            self.add_status_message("🔍 Retrieving Telegraf status...".to_string());
            if let Some(worker) = &self.worker {
                if let Err(e) = worker.send_command(WorkerCommand::GetTelegrafStatus {
                    config: self.config.clone(),
                }) {
                    self.add_status_message(format!("❌ Failed to send command: {}", e));
                    self.is_working = false;
                }
            }
        } else {
            self.add_status_message("❌ Worker not available".to_string());
        }
    }

    fn get_telegraf_logs(&mut self) {
        if self.worker.is_some() {
            self.is_working = true;
            self.add_status_message("📋 Retrieving Telegraf logs (last 30 lines)...".to_string());
            if let Some(worker) = &self.worker {
                if let Err(e) = worker.send_command(WorkerCommand::GetTelegrafLogs {
                    config: self.config.clone(),
                    lines: 30,
                }) {
                    self.add_status_message(format!("❌ Failed to send command: {}", e));
                    self.is_working = false;
                }
            }
        } else {
            self.add_status_message("❌ Worker not available".to_string());
        }
    }

    fn restart_telegraf(&mut self) {
        let host = self.config.iot_host.clone();
        let username = self.config.iot_username.clone();
        let password = self.config.iot_password.clone();

        if host.is_empty() || username.is_empty() || password.is_empty() {
            self.add_status_message("⚠️ IoT device credentials not configured".to_string());
            return;
        }

        if self.worker.is_some() {
            self.is_working = true;
            self.add_status_message("🔄 Restarting Telegraf service...".to_string());
            if let Some(worker) = &self.worker {
                if let Err(e) = worker.send_command(WorkerCommand::RestartTelegraf {
                    host,
                    username,
                    password,
                }) {
                    self.add_status_message(format!("❌ Failed to send command: {}", e));
                    self.is_working = false;
                }
            }
        } else {
            self.add_status_message("❌ Worker not available".to_string());
        }
    }

    fn check_service_status(&mut self) {
        let is_prometheus =
            self.config.output_format.as_deref().unwrap_or("influxdb") == "prometheus";

        let service_name = if is_prometheus {
            "Prometheus"
        } else {
            "InfluxDB"
        };
        self.add_status_message(format!("Checking {} status...", service_name));

        match ConfigGenerator::new(self.config.clone()) {
            Ok(generator) => {
                if is_prometheus {
                    match generator.check_service_status(
                        &self.config.iot_host,
                        ServiceType::Prometheus,
                        5,
                    ) {
                        Ok((is_healthy, status)) => {
                            let health_status = if is_healthy { "Healthy" } else { "Unhealthy" };
                            self.add_status_message(format!(
                                "Prometheus Status: {}\n{}",
                                health_status, status
                            ));
                        }
                        Err(e) => {
                            self.add_status_message(format!(
                                "Failed to check Prometheus status: {}",
                                e
                            ));
                        }
                    }
                } else {
                    match generator.check_influxdb_status() {
                        Ok((is_healthy, status)) => {
                            let health_status = if is_healthy { "Healthy" } else { "Unhealthy" };
                            self.add_status_message(format!(
                                "InfluxDB Status: {}\n{}",
                                health_status, status
                            ));
                        }
                        Err(e) => {
                            self.add_status_message(format!(
                                "Failed to check InfluxDB status: {}",
                                e
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                self.add_status_message(format!("Configuration error: {}", e));
            }
        }
    }

    fn backup_grafana(&mut self) {
        self.add_status_message("Backing up Grafana...".to_string());

        match ConfigGenerator::new(self.config.clone()) {
            Ok(generator) => match generator.backup_grafana() {
                Ok(backup_info) => {
                    self.add_status_message(format!("Grafana backup successful:\n{}", backup_info));
                }
                Err(e) => {
                    self.add_status_message(format!("Failed to backup Grafana: {}", e));
                }
            },
            Err(e) => {
                self.add_status_message(format!("Configuration error: {}", e));
            }
        }
    }

    fn sync_time(&mut self) {
        let host = self.config.iot_host.clone();
        let username = self.config.iot_username.clone();
        let password = self.config.iot_password.clone();

        if host.is_empty() || username.is_empty() || password.is_empty() {
            self.add_status_message("⚠️ IoT device credentials not configured".to_string());
            return;
        }

        if self.worker.is_some() {
            self.is_working = true;
            self.add_status_message("🕐 Syncing system time to device...".to_string());
            if let Some(worker) = &self.worker {
                if let Err(e) = worker.send_command(WorkerCommand::SyncTime {
                    host,
                    username,
                    password,
                }) {
                    self.add_status_message(format!("❌ Failed to send command: {}", e));
                    self.is_working = false;
                }
            }
        } else {
            self.add_status_message("❌ Worker not available".to_string());
        }
    }

    fn device_provision(&mut self) {
        if self.worker.is_none() {
            self.add_status_message("❌ Worker not available".to_string());
            return;
        }
        self.is_working = true;
        self.add_status_message("🚀 Provisioning device...".to_string());
        if let Some(worker) = &self.worker {
            let config = self.build_deployment_config();
            if let Err(e) = worker.send_command(WorkerCommand::DeviceProvision {
                config,
                local_transfer: self.local_transfer,
            }) {
                self.add_status_message(format!("❌ Failed to send command: {}", e));
                self.is_working = false;
            }
        }
    }

    fn device_setup(&mut self) {
        if self.worker.is_none() {
            self.add_status_message("❌ Worker not available".to_string());
            return;
        }
        self.is_working = true;
        self.add_status_message("🔧 Setting up device...".to_string());
        if let Some(worker) = &self.worker {
            let config = self.build_deployment_config();
            if let Err(e) = worker.send_command(WorkerCommand::DeviceSetup {
                config,
                minimal: self.minimal,
            }) {
                self.add_status_message(format!("❌ Failed to send command: {}", e));
                self.is_working = false;
            }
        }
    }

    fn device_update(&mut self) {
        if self.worker.is_none() {
            self.add_status_message("❌ Worker not available".to_string());
            return;
        }
        self.is_working = true;
        self.add_status_message("📦 Updating device...".to_string());
        if let Some(worker) = &self.worker {
            let config = self.build_deployment_config();
            if let Err(e) = worker.send_command(WorkerCommand::DeviceUpdate {
                config,
                use_local: self.local_transfer,
            }) {
                self.add_status_message(format!("❌ Failed to send command: {}", e));
                self.is_working = false;
            }
        }
    }

    fn device_start(&mut self) {
        if self.worker.is_none() {
            self.add_status_message("❌ Worker not available".to_string());
            return;
        }
        self.is_working = true;
        self.add_status_message("▶️ Starting monitoring stack...".to_string());
        if let Some(worker) = &self.worker {
            let config = self.build_deployment_config();
            if let Err(e) = worker.send_command(WorkerCommand::DeviceStart { config }) {
                self.add_status_message(format!("❌ Failed to send command: {}", e));
                self.is_working = false;
            }
        }
    }

    fn device_stop(&mut self) {
        if self.worker.is_none() {
            self.add_status_message("❌ Worker not available".to_string());
            return;
        }

        if self.pending_confirmation.is_some() {
            self.add_status_message("⚠️ Another confirmation is pending".to_string());
            return;
        }

        self.pending_confirmation = Some("stop_normal".to_string());
        self.add_status_message("⚠️ Stop monitoring stack? Press 'y' to confirm or 'n' to cancel. Press 'v' to stop and remove volumes.".to_string());
    }

    fn device_stop_with_volumes(&mut self) {
        if self.worker.is_none() {
            self.add_status_message("❌ Worker not available".to_string());
            return;
        }

        if self.pending_confirmation.is_some() {
            self.add_status_message("⚠️ Another confirmation is pending".to_string());
            return;
        }

        self.pending_confirmation = Some("stop_volumes".to_string());
        self.add_status_message("⚠️ Stop and REMOVE ALL DATA? Press 'y' to confirm or 'n' to cancel.".to_string());
    }

    fn device_stop_confirmed(&mut self, remove_volumes: bool) {
        self.pending_confirmation = None;
        self.is_working = true;
        if remove_volumes {
            self.add_status_message("⏹️ Stopping monitoring stack and removing volumes...".to_string());
        } else {
            self.add_status_message("⏹️ Stopping monitoring stack...".to_string());
        }
        if let Some(worker) = &self.worker {
            let config = self.build_deployment_config();
            if let Err(e) = worker.send_command(WorkerCommand::DeviceStop {
                config,
                remove_volumes,
            }) {
                self.add_status_message(format!("❌ Failed to send command: {}", e));
                self.is_working = false;
            }
        }
    }

    fn device_status(&mut self) {
        if self.worker.is_none() {
            self.add_status_message("❌ Worker not available".to_string());
            return;
        }
        self.is_working = true;
        self.add_status_message("📊 Checking device status...".to_string());
        if let Some(worker) = &self.worker {
            let config = self.build_deployment_config();
            if let Err(e) = worker.send_command(WorkerCommand::DeviceStatus { config }) {
                self.add_status_message(format!("❌ Failed to send command: {}", e));
                self.is_working = false;
            }
        }
    }

    fn device_backup(&mut self) {
        if self.worker.is_none() {
            self.add_status_message("❌ Worker not available".to_string());
            return;
        }
        self.is_working = true;
        self.add_status_message("💾 Backing up device...".to_string());
        if let Some(worker) = &self.worker {
            let config = self.build_deployment_config();
            if let Err(e) = worker.send_command(WorkerCommand::DeviceBackup {
                config,
                output_dir: None,
            }) {
                self.add_status_message(format!("❌ Failed to send command: {}", e));
                self.is_working = false;
            }
        }
    }

    fn device_restore(&mut self) {
        if self.worker.is_none() {
            self.add_status_message("❌ Worker not available".to_string());
            return;
        }

        if self.pending_confirmation.is_some() {
            self.add_status_message("⚠️ Another confirmation is pending".to_string());
            return;
        }

        self.pending_confirmation = Some("restore".to_string());
        self.add_status_message("⚠️ Restore from backup? This will overwrite existing data. Press 'y' to confirm, 'n' to cancel.".to_string());
    }

    fn device_restore_confirmed(&mut self) {
        self.pending_confirmation = None;
        self.add_status_message("⚠️ Restore requires an archive path. Use the CLI for restore with a specific archive file.".to_string());
    }

    fn build_deployment_config(&self) -> DeploymentConfig {
        let host = self.config.iot_host.clone();
        let user = self.config.iot_username.clone();

        let (hostname, port) = if host.contains(':') {
            let parts: Vec<&str> = host.splitn(2, ':').collect();
            let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(22);
            (parts[0].to_string(), port)
        } else {
            (host, 22)
        };

        let mut config = DeploymentConfig::new(hostname, user).with_port(port);

        if !self.config.iot_password.is_empty() {
            config = config.with_password(self.config.iot_password.clone());
        }

        if !self.key_file.is_empty() {
            config = config.with_key_file(self.key_file.clone());
        }

        if !self.git_branch.is_empty() {
            config = config.with_git_branch(self.git_branch.clone());
        }

        if !self.device_name.is_empty() {
            let hostname_slug =
                sie_generate_config::backend::ships::derive_hostname(&self.device_name);
            config = config.with_ship_name(self.device_name.clone(), hostname_slug);
        }

        config
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        app.process_worker_responses();

        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => {
                        if app.pending_confirmation.is_some() {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    let confirmation = app.pending_confirmation.clone();
                                    match confirmation.as_deref() {
                                        Some("stop_normal") => app.device_stop_confirmed(false),
                                        Some("stop_volumes") => app.device_stop_confirmed(true),
                                        Some("restore") => app.device_restore_confirmed(),
                                        _ => {
                                            app.pending_confirmation = None;
                                            app.add_status_message("Confirmation cancelled.".to_string());
                                        }
                                    }
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    app.pending_confirmation = None;
                                    app.add_status_message("Action cancelled.".to_string());
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Char('h') | KeyCode::F(1) => app.show_help = !app.show_help,
                                KeyCode::PageUp => {
                                    if let Some(selected) = app.status_list_state.selected() {
                                        if selected > 0 {
                                            app.status_list_state.select(Some(selected - 1));
                                        }
                                    } else if !app.status_messages.is_empty() {
                                        app.status_list_state
                                            .select(Some(app.status_messages.len() - 1));
                                    }
                                }
                                KeyCode::PageDown => {
                                    if let Some(selected) = app.status_list_state.selected() {
                                        if selected < app.status_messages.len().saturating_sub(1) {
                                            app.status_list_state.select(Some(selected + 1));
                                        }
                                    } else if !app.status_messages.is_empty() {
                                        app.status_list_state.select(Some(0));
                                    }
                                }
                                KeyCode::Tab | KeyCode::Right => {
                                    app.current_tab = match app.current_tab {
                                        Tab::Folder => Tab::Files,
                                        Tab::Files => Tab::OpcUaConfig,
                                        Tab::OpcUaConfig => Tab::IoTConfig,
                                        Tab::IoTConfig => Tab::Config,
                                        Tab::Config => Tab::Actions,
                                        Tab::Actions => Tab::Device,
                                        Tab::Device => Tab::Folder,
                                    };
                                }
                                KeyCode::Left => {
                                    app.current_tab = match app.current_tab {
                                        Tab::Folder => Tab::Device,
                                        Tab::Files => Tab::Folder,
                                        Tab::OpcUaConfig => Tab::Files,
                                        Tab::IoTConfig => Tab::OpcUaConfig,
                                        Tab::Config => Tab::IoTConfig,
                                        Tab::Actions => Tab::Config,
                                        Tab::Device => Tab::Actions,
                                    };
                                }
                                KeyCode::Char('1') => app.current_tab = Tab::Folder,
                                KeyCode::Char('2') => app.current_tab = Tab::Files,
                                KeyCode::Char('3') => app.current_tab = Tab::OpcUaConfig,
                                KeyCode::Char('4') => app.current_tab = Tab::IoTConfig,
                                KeyCode::Char('5') => app.current_tab = Tab::Config,
                                KeyCode::Char('6') => app.current_tab = Tab::Actions,
                                KeyCode::Char('7') => app.current_tab = Tab::Device,
                                _ => match app.current_tab {
                                    Tab::Folder => handle_folder_input(&mut app, key.code),
                                    Tab::Files => handle_files_input(&mut app, key.code),
                                    Tab::OpcUaConfig => handle_opcua_input(&mut app, key.code),
                                    Tab::IoTConfig => handle_iot_input(&mut app, key.code),
                                    Tab::Device => handle_device_input(&mut app, key.code),
                                    Tab::Config => handle_config_input(&mut app, key.code),
                                    Tab::Actions => handle_actions_input(&mut app, key.code),
                                },
                            }
                        }
                    }
                        InputMode::Editing => match key.code {
                            KeyCode::Enter => app.finish_editing(),
                            KeyCode::Esc => app.cancel_editing(),
                            KeyCode::Char(c) => app.input_buffer.push(c),
                            KeyCode::Backspace => {
                                app.input_buffer.pop();
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    }
}

fn handle_folder_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up => {
            let i = match app.directory_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        app.directory_entries.len().saturating_sub(1)
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            app.directory_list_state.select(Some(i));
        }
        KeyCode::Down => {
            let i = match app.directory_list_state.selected() {
                Some(i) => {
                    if i >= app.directory_entries.len().saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            app.directory_list_state.select(Some(i));
        }
        KeyCode::Enter => app.enter_selected_directory(),
        KeyCode::Char('r') => app.load_directory_entries(),
        KeyCode::Char('h') => {
            // Go to home directory
            if let Some(home) = dirs::home_dir() {
                app.navigate_to_directory(home);
            }
        }
        _ => {}
    }
}

fn handle_files_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up => {
            let i = match app.file_list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        app.xml_files.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            app.file_list_state.select(Some(i));
        }
        KeyCode::Down => {
            let i = match app.file_list_state.selected() {
                Some(i) => {
                    if i >= app.xml_files.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            app.file_list_state.select(Some(i));
        }
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_file_selection(),
        KeyCode::Char('l') => app.toggle_listener_selection(),
        KeyCode::Char('r') => app.refresh_files(),
        KeyCode::Char('n') => {
            if let Some(selected) = app.file_list_state.selected() {
                if selected < app.xml_files.len() {
                    app.start_editing(EditField::FileNamespace(selected));
                }
            }
        }
        KeyCode::Char('i') => {
            if let Some(selected) = app.file_list_state.selected() {
                if selected < app.xml_files.len() {
                    app.start_editing(EditField::FileIp(selected));
                }
            }
        }
        KeyCode::Char('t') => {
            if let Some(selected) = app.file_list_state.selected() {
                if selected < app.xml_files.len() {
                    app.start_editing(EditField::FileInterval(selected));
                }
            }
        }
        KeyCode::Char('p') => {
            app.poll_namespaces();
        }
        _ => {}
    }
}

fn handle_opcua_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up => {
            if app.opcua_config_selection > 0 {
                app.opcua_config_selection -= 1;
            }
        }
        KeyCode::Down => {
            if app.opcua_config_selection < OpcUaConfigField::count() - 1 {
                app.opcua_config_selection += 1;
            }
        }
        KeyCode::Enter => {
            let field = OpcUaConfigField::from_index(app.opcua_config_selection);
            match field {
                OpcUaConfigField::Ip => app.start_editing(EditField::OpcUaIp),
                OpcUaConfigField::Username => app.start_editing(EditField::OpcUaUsername),
                OpcUaConfigField::Password => app.start_editing(EditField::OpcUaPassword),
                OpcUaConfigField::Anonymous => app.toggle_anonymous_mode(),
                OpcUaConfigField::TestInputs => {
                    app.config.include_test_inputs = !app.config.include_test_inputs
                }
                OpcUaConfigField::OpcuaDiagnostics => {
                    app.config.include_opcua_diagnostics = !app.config.include_opcua_diagnostics
                }
            }
        }
        // Keep some legacy hotkeys for now (can be removed later)
        KeyCode::Char('i') => app.start_editing(EditField::OpcUaIp),
        KeyCode::Char('u') => app.start_editing(EditField::OpcUaUsername),
        KeyCode::Char('p') => app.start_editing(EditField::OpcUaPassword),
        KeyCode::Char('o') => {
            // Toggle between influxdb and prometheus
            let current = app.config.output_format.as_deref().unwrap_or("influxdb");
            app.config.output_format = Some(if current == "influxdb" {
                "prometheus".to_string()
            } else {
                "influxdb".to_string()
            });
        }
        KeyCode::Char('a') => app.toggle_anonymous_mode(),
        KeyCode::Char('t') => app.config.include_test_inputs = !app.config.include_test_inputs,
        KeyCode::Char('d') => {
            app.config.include_opcua_diagnostics = !app.config.include_opcua_diagnostics
        }
        _ => {}
    }
}

fn handle_iot_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up => {
            if app.iot_config_selection > 0 {
                app.iot_config_selection -= 1;
            }
        }
        KeyCode::Down => {
            if app.iot_config_selection < IoTConfigField::count() - 1 {
                app.iot_config_selection += 1;
            }
        }
        KeyCode::Enter => {
            let field = IoTConfigField::from_index(app.iot_config_selection);
            match field {
                IoTConfigField::Host => app.start_editing(EditField::IoTHost),
                IoTConfigField::Username => app.start_editing(EditField::IoTUsername),
                IoTConfigField::Password => app.start_editing(EditField::IoTPassword),
            }
        }
        // Keep some legacy hotkeys for now (can be removed later)
        KeyCode::Char('h') => app.start_editing(EditField::IoTHost),
        KeyCode::Char('u') => app.start_editing(EditField::IoTUsername),
        KeyCode::Char('p') => app.start_editing(EditField::IoTPassword),
        _ => {}
    }
}

fn handle_config_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up => {
            app.config_scroll = app.config_scroll.saturating_sub(1);
        }
        KeyCode::Down => {
            app.config_scroll = app.config_scroll.saturating_add(1);
        }
        KeyCode::Left => {
            app.config_horizontal_scroll = app.config_horizontal_scroll.saturating_sub(1);
        }
        KeyCode::Right => {
            app.config_horizontal_scroll = app.config_horizontal_scroll.saturating_add(1);
        }
        KeyCode::PageUp => {
            app.config_scroll = app.config_scroll.saturating_sub(10);
        }
        KeyCode::PageDown => {
            app.config_scroll = app.config_scroll.saturating_add(10);
        }
        KeyCode::Home => {
            app.config_scroll = 0;
            app.config_horizontal_scroll = 0;
        }
        KeyCode::Char('g') => app.generate_config(),
        KeyCode::Char('s') => app.send_config(),
        KeyCode::Char('r') => {
            app.generated_config = None;
            app.config_scroll = 0;
            app.config_horizontal_scroll = 0;
        }
        _ => {}
    }
}

fn handle_actions_input(app: &mut App, key: KeyCode) {
    match key {
        // Arrow key navigation
        KeyCode::Up => {
            if app.actions_selection > 0 {
                app.actions_selection -= 1;
            } else {
                app.actions_selection = ActionsField::count() - 1;
            }
        }
        KeyCode::Down => {
            app.actions_selection = (app.actions_selection + 1) % ActionsField::count();
        }
        KeyCode::Enter => {
            let selected_field = ActionsField::from_index(app.actions_selection);
            match selected_field {
                ActionsField::GenerateConfig => app.generate_config(),
                ActionsField::SendConfig => app.send_config(),
                ActionsField::ClearMessages => app.status_messages.clear(),
                ActionsField::TelegrafStatus => app.get_telegraf_status(),
                ActionsField::TelegrafLogs => app.get_telegraf_logs(),
                ActionsField::RestartTelegraf => app.restart_telegraf(),
                ActionsField::ServiceStatus => app.check_service_status(),
                ActionsField::BackupGrafana => app.backup_grafana(),
            }
        }
        // Legacy hotkeys for backward compatibility
        KeyCode::Char('g') => app.generate_config(),
        KeyCode::Char('s') => app.send_config(),
        KeyCode::Char('c') => app.status_messages.clear(),
        KeyCode::Char('t') => app.get_telegraf_status(),
        KeyCode::Char('l') => app.get_telegraf_logs(),
        KeyCode::Char('r') => app.restart_telegraf(),
        KeyCode::Char('v') => app.check_service_status(),
        KeyCode::Char('b') => app.backup_grafana(),
        _ => {}
    }
}

fn handle_device_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up => {
            if app.device_selection > 0 {
                app.device_selection -= 1;
            } else {
                app.device_selection = DeviceField::count() - 1;
            }
        }
        KeyCode::Down => {
            app.device_selection = (app.device_selection + 1) % DeviceField::count();
        }
        KeyCode::Enter => {
            let selected_field = DeviceField::from_index(app.device_selection);
            match selected_field {
                DeviceField::Provision => app.device_provision(),
                DeviceField::Setup => app.device_setup(),
                DeviceField::Update => app.device_update(),
                DeviceField::Start => app.device_start(),
                DeviceField::Stop => app.device_stop(),
                DeviceField::Status => app.device_status(),
                DeviceField::Backup => app.device_backup(),
                DeviceField::Restore => app.device_restore(),
                DeviceField::SyncTime => app.sync_time(),
                DeviceField::DeviceName => app.start_editing(EditField::DeviceName),
                DeviceField::KeyFile => app.start_editing(EditField::KeyFile),
                DeviceField::GitBranch => app.start_editing(EditField::GitBranch),
                DeviceField::Minimal => {
                    app.minimal = !app.minimal;
                    app.add_status_message(format!(
                        "Minimal mode: {}",
                        if app.minimal { "enabled" } else { "disabled" }
                    ));
                }
                DeviceField::LocalTransfer => {
                    app.local_transfer = !app.local_transfer;
                    app.add_status_message(format!(
                        "Local transfer: {}",
                        if app.local_transfer { "enabled" } else { "disabled" }
                    ));
                }
            }
        }
        KeyCode::Char('p') => app.device_provision(),
        KeyCode::Char('s') => app.device_setup(),
        KeyCode::Char('u') => app.device_update(),
        KeyCode::Char('g') => app.device_start(),
        KeyCode::Char('v') => app.device_stop_with_volumes(),
        KeyCode::Char('n') => {
            app.start_editing(EditField::DeviceName);
        }
        _ => {}
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Tabs
            Constraint::Min(0),    // Main content (full height)
        ])
        .split(f.area());

    // Render tabs
    let tab_title = if app.is_working {
        "IoT2050 Config TUI ⏳ Working..."
    } else if app.pending_confirmation.is_some() {
        "IoT2050 Config TUI ⚠️ Confirm?"
    } else {
        "IoT2050 Config TUI"
    };
    let tab_titles = vec![
        "Folder",
        "Files",
        "OPC-UA Config",
        "IoT Config",
        "Config",
        "Actions",
        "Device",
    ];
    let selected_tab = match app.current_tab {
        Tab::Folder => 0,
        Tab::Files => 1,
        Tab::OpcUaConfig => 2,
        Tab::IoTConfig => 3,
        Tab::Config => 4,
        Tab::Actions => 5,
        Tab::Device => 6,
    };

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(tab_title),
        )
        .select(selected_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Black),
        );
    f.render_widget(tabs, chunks[0]);

    // Render main content based on current tab
    match app.current_tab {
        Tab::Folder => render_folder_tab(f, app, chunks[1]),
        Tab::Files => render_files_tab(f, app, chunks[1]),
        Tab::OpcUaConfig => render_opcua_tab(f, app, chunks[1]),
        Tab::IoTConfig => render_iot_tab(f, app, chunks[1]),
        Tab::Device => render_device_tab(f, app, chunks[1]),
        Tab::Config => render_config_tab(f, app, chunks[1]),
        Tab::Actions => render_actions_tab(f, app, chunks[1]),
    }

    // Render help popup if needed
    if app.show_help {
        render_help_popup(f, app);
    }

    // Render confirmation overlay if needed
    if app.pending_confirmation.is_some() {
        render_confirmation_overlay(f, app);
    }
}

fn render_folder_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(chunks[1]);

    // Directory listing
    let items: Vec<ListItem> = app
        .directory_entries
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let display_name = if i == 0 && app.current_directory.parent().is_some() {
                ".. (Parent Directory)".to_string()
            } else {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<invalid>")
                    .to_string();

                if path.is_dir() {
                    format!("📁 {}/", name)
                } else {
                    format!("📄 {}", name)
                }
            };
            ListItem::new(display_name)
        })
        .collect();

    let current_dir_display = app.current_directory.display().to_string();
    let directory_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Directory: {}", current_dir_display)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(directory_list, chunks[0], &mut app.directory_list_state);

    // Instructions
    let instructions = vec![
        Line::from("Folder Navigation:"),
        Line::from(""),
        Line::from("↑/↓    - Navigate entries"),
        Line::from("Enter  - Enter directory"),
        Line::from("r      - Refresh listing"),
        Line::from("h      - Go to home directory"),
        Line::from("Tab    - Next tab"),
        Line::from(""),
        Line::from("Current folder will be used"),
        Line::from("for XML file discovery."),
    ];

    let help_block = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(help_block, right_chunks[0]);

    render_status_messages(f, app, right_chunks[1]);
}

fn render_files_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(10)])
        .split(chunks[1]);

    // File list with per-file configuration
    let items: Vec<ListItem> = app
        .xml_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let checkbox = if *app.selected_files.get(i).unwrap_or(&false) {
                "[x]"
            } else {
                "[ ]"
            };
            let filename = std::path::Path::new(file)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file);

            // Get per-file configuration
            let config = app.file_configs.get(file);
            let namespace = config.map(|c| c.namespace.as_str()).unwrap_or("2");
            let ip = config.map(|c| c.ip.as_str()).unwrap_or("");
            let interval = config.map(|c| c.interval_ms.as_str()).unwrap_or("");

            // Show different default interval for listeners (500ms vs 1000ms)
            let is_listener = *app.selected_listener_files.get(i).unwrap_or(&false);
            let default_interval = if is_listener { "500" } else { "1000" };
            let display_interval = if interval.is_empty() {
                default_interval
            } else {
                interval
            };

            let listener_checkbox = if is_listener { "[L]" } else { "[ ]" };

            // Format: [x] filename.xml (first line)
            // Format:   [L] Listener | NS:2 | IP:192.168.1.100 | INT:1000ms (second line)
            let config_info = format!(
                "{} Listener | NS:{} | IP:{} | INT:{}ms",
                listener_checkbox,
                if namespace.is_empty() { "2" } else { namespace },
                if ip.is_empty() { "default" } else { ip },
                display_interval
            );

            ListItem::new(vec![
                Line::from(format!("{} {}", checkbox, filename)),
                Line::from(format!("  {}", config_info)).style(Style::default().fg(Color::Gray)),
            ])
        })
        .collect();

    let files_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("XML Files"))
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(files_list, chunks[0], &mut app.file_list_state);

    // Right panel - show editing input if in edit mode, otherwise show instructions
    if app.input_mode == InputMode::Editing {
        // Show editing interface
        let edit_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(right_chunks[0]);

        // Determine what we're editing
        let edit_title = if let Some(ref field) = app.current_edit_field {
            match field {
                EditField::FileNamespace(_) => "Edit Namespace",
                EditField::FileIp(_) => "Edit IP Address",
                EditField::FileInterval(_) => "Edit Interval (ms)",
                _ => "Edit Field",
            }
        } else {
            "Edit Field"
        };

        // Input field
        let input = Paragraph::new(app.input_buffer.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(edit_title));
        f.render_widget(input, edit_chunks[0]);

        // Edit instructions
        let edit_instructions = vec![
            Line::from("Editing Mode:"),
            Line::from(""),
            Line::from("Enter  - Save changes"),
            Line::from("Esc    - Cancel editing"),
            Line::from(""),
            Line::from("Type new value and press Enter"),
        ];

        let edit_help = Paragraph::new(edit_instructions)
            .block(Block::default().borders(Borders::ALL).title("Edit Help"));
        f.render_widget(edit_help, edit_chunks[1]);
    } else {
        // Show normal instructions
        let instructions = vec![
            Line::from("Controls:"),
            Line::from(""),
            Line::from("↑/↓    - Navigate files"),
            Line::from("Space  - Toggle selection"),
            Line::from("r      - Refresh file list"),
            Line::from(""),
            Line::from("Per-file config:"),
            Line::from("n      - Edit namespace"),
            Line::from("i      - Edit IP address"),
            Line::from("t      - Edit interval"),
            Line::from("p      - Poll namespaces from server"),
            Line::from(""),
            Line::from("Tab    - Next tab"),
            Line::from("h/F1   - Help"),
            Line::from("q      - Quit"),
            Line::from(""),
            Line::from(format!("Working Directory:")),
        ];

        let help_block = Paragraph::new(instructions)
            .block(Block::default().borders(Borders::ALL).title("Controls"));
        f.render_widget(help_block, right_chunks[0]);
    }

    render_status_messages(f, app, right_chunks[1]);
}

fn render_opcua_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(24), // Config fields (6 x 3 + border)
            Constraint::Length(12), // Controls
            Constraint::Min(0),     // Status messages
        ])
        .split(area);

    // Configuration fields
    let config_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(chunks[0]);

    // Render each field with selection highlighting
    let selected_field = OpcUaConfigField::from_index(app.opcua_config_selection);

    render_config_field_with_selection(
        f,
        "OPC-UA Server IP",
        &app.config.ip,
        matches!(app.current_edit_field, Some(EditField::OpcUaIp)),
        &app.input_buffer,
        config_chunks[0],
        matches!(selected_field, OpcUaConfigField::Ip),
    );

    let username_display = if app.anonymous_mode {
        "[Anonymous Mode]".to_string()
    } else {
        app.config.username.clone()
    };
    render_config_field_with_selection(
        f,
        "Username",
        &username_display,
        matches!(app.current_edit_field, Some(EditField::OpcUaUsername)),
        &app.input_buffer,
        config_chunks[1],
        matches!(selected_field, OpcUaConfigField::Username),
    );

    let password_display = if app.anonymous_mode {
        "[Anonymous Mode]".to_string()
    } else {
        "*".repeat(app.config.password.len())
    };
    render_config_field_with_selection(
        f,
        "Password",
        &password_display,
        matches!(app.current_edit_field, Some(EditField::OpcUaPassword)),
        &app.input_buffer,
        config_chunks[2],
        matches!(selected_field, OpcUaConfigField::Password),
    );

    let anonymous_status = if app.anonymous_mode {
        "Enabled"
    } else {
        "Disabled"
    };
    render_config_field_with_selection(
        f,
        "Anonymous Mode",
        anonymous_status,
        false,
        "",
        config_chunks[3],
        matches!(selected_field, OpcUaConfigField::Anonymous),
    );

    let test_inputs_status = if app.config.include_test_inputs {
        "Enabled"
    } else {
        "Disabled"
    };
    render_config_field_with_selection(
        f,
        "Test Inputs",
        test_inputs_status,
        false,
        "",
        config_chunks[4],
        matches!(selected_field, OpcUaConfigField::TestInputs),
    );

    let opcua_diagnostics_status = if app.config.include_opcua_diagnostics {
        "Enabled"
    } else {
        "Disabled"
    };
    render_config_field_with_selection(
        f,
        "OPC-UA Diagnostics",
        opcua_diagnostics_status,
        false,
        "",
        config_chunks[5],
        matches!(selected_field, OpcUaConfigField::OpcuaDiagnostics),
    );

    // Instructions
    let instructions = vec![
        Line::from("Controls:"),
        Line::from(""),
        Line::from("↑/↓   - Navigate fields"),
        Line::from("Enter - Edit selected field"),
        Line::from("Esc   - Cancel edit"),
        Line::from(""),
        Line::from("Legacy hotkeys:"),
        Line::from("i - Edit IP address"),
        Line::from("u - Edit username"),
        Line::from("p - Edit password"),
        Line::from("o - Toggle output format"),
        Line::from("a - Toggle anonymous mode"),
        Line::from("t - Toggle test inputs"),
        Line::from("d - Toggle OPC-UA diagnostics"),
    ];

    let help_block = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(help_block, chunks[1]);

    // Status messages
    render_status_messages(f, app, chunks[2]);
}

fn render_iot_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Config fields (3 x 3 + title)
            Constraint::Length(8),  // Controls
            Constraint::Min(0),    // Status messages
        ])
        .split(area);

    // Configuration fields
    let config_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(chunks[0]);

    // Render each field with selection highlighting
    let selected_field = IoTConfigField::from_index(app.iot_config_selection);

    render_config_field_with_selection(
        f,
        "IoT Device Host",
        &app.config.iot_host,
        matches!(app.current_edit_field, Some(EditField::IoTHost)),
        &app.input_buffer,
        config_chunks[0],
        matches!(selected_field, IoTConfigField::Host),
    );

    render_config_field_with_selection(
        f,
        "IoT Username",
        &app.config.iot_username,
        matches!(app.current_edit_field, Some(EditField::IoTUsername)),
        &app.input_buffer,
        config_chunks[1],
        matches!(selected_field, IoTConfigField::Username),
    );

    let password_display = "*".repeat(app.config.iot_password.len());
    render_config_field_with_selection(
        f,
        "IoT Password",
        &password_display,
        matches!(app.current_edit_field, Some(EditField::IoTPassword)),
        &app.input_buffer,
        config_chunks[2],
        matches!(selected_field, IoTConfigField::Password),
    );

    // Instructions
    let instructions = vec![
        Line::from("Controls:"),
        Line::from(""),
        Line::from("↑/↓   - Navigate fields"),
        Line::from("Enter - Edit selected field"),
        Line::from("Esc   - Cancel edit"),
        Line::from(""),
        Line::from("Legacy hotkeys:"),
        Line::from("h - Edit IoT host"),
        Line::from("u - Edit IoT username"),
        Line::from("p - Edit IoT password"),
    ];

    let help_block = Paragraph::new(instructions)
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(help_block, chunks[1]);

    // Status messages
    render_status_messages(f, app, chunks[2]);
}

fn render_config_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    // Configuration display
    let config_content = if let Some(ref config) = app.generated_config {
        config.clone()
    } else {
        "No configuration generated yet.\n\nPress 'g' to generate configuration based on selected files and settings.".to_string()
    };

    let config_paragraph = Paragraph::new(config_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Generated Configuration"),
        )
        .scroll((app.config_scroll, app.config_horizontal_scroll));

    f.render_widget(config_paragraph, chunks[0]);

    // Controls and status
    let control_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Controls
    let controls = vec![
        Line::from("Controls:"),
        Line::from(""),
        Line::from("g      - Generate config"),
        Line::from("s      - Send to IoT device"),
        Line::from("r      - Clear config"),
        Line::from("↑/↓    - Scroll vertical"),
        Line::from("←/→    - Scroll horizontal"),
        Line::from("PgUp/Dn- Fast scroll"),
        Line::from("Home   - Reset scroll"),
    ];

    let controls_block =
        Paragraph::new(controls).block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(controls_block, control_chunks[0]);

    render_status_messages(f, app, control_chunks[1]);
}

fn render_device_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(14), Constraint::Min(0)])
        .split(chunks[1]);

    let selected_field = DeviceField::from_index(app.device_selection);

    let confirmation_warning = if app.pending_confirmation.is_some() {
        vec![
            Line::from(""),
            Line::from("⚠️  CONFIRMATION REQUIRED ⚠️".to_string())
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Line::from("Press 'y' to confirm or 'n' to cancel".to_string())
                .style(Style::default().fg(Color::Yellow)),
        ]
    } else {
        vec![]
    };

    let actions = vec![
        Line::from("Device Management (↑/↓ navigate, Enter execute):"),
        Line::from(""),
        Line::from("Deployment Actions:"),
        render_action_item(
            "🚀 Provision Device",
            matches!(selected_field, DeviceField::Provision),
            "p",
        ),
        render_action_item(
            "🔧 Setup Device",
            matches!(selected_field, DeviceField::Setup),
            "s",
        ),
        render_action_item(
            "📦 Update Device",
            matches!(selected_field, DeviceField::Update),
            "u",
        ),
        Line::from(""),
        Line::from("Service Control:"),
        render_action_item(
            "▶️ Start Monitoring Stack",
            matches!(selected_field, DeviceField::Start),
            "g",
        ),
        render_action_item(
            "⏹️ Stop Monitoring Stack",
            matches!(selected_field, DeviceField::Stop),
            "",
        ),
        render_action_item(
            "📊 Device Status",
            matches!(selected_field, DeviceField::Status),
            "",
        ),
        Line::from(""),
        Line::from("Data Management:"),
        render_action_item(
            "💾 Backup All Data",
            matches!(selected_field, DeviceField::Backup),
            "",
        ),
        render_action_item(
            "♻️ Restore from Backup",
            matches!(selected_field, DeviceField::Restore),
            "",
        ),
        render_action_item(
            "🕐 Sync Device Time",
            matches!(selected_field, DeviceField::SyncTime),
            "",
        ),
    ];

    let mut lines = actions;
    lines.extend(confirmation_warning);

    let actions_block =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Device"));
    f.render_widget(actions_block, chunks[0]);

    // Configuration panel
    let minimal_status = if app.minimal { "Enabled" } else { "Disabled" };
    let local_transfer_status = if app.local_transfer { "Enabled" } else { "Disabled" };

    let device_name_display = if app.device_name.is_empty() {
        "(random Culture ship name)"
    } else {
        &app.device_name
    };
    let key_file_display = if app.key_file.is_empty() {
        "(default)"
    } else {
        &app.key_file
    };

    let config = vec![
        Line::from("Device Configuration:"),
        Line::from(""),
        Line::from(format!("Device Name: {}", device_name_display)),
        Line::from(format!("Key File: {}", key_file_display)),
        Line::from(format!("Git Branch: {}", app.git_branch)),
        Line::from(format!("Minimal Mode: {}", minimal_status)),
        Line::from(format!("Local Transfer: {}", local_transfer_status)),
        Line::from(""),
        Line::from("IoT Connection (from IoT Config tab):"),
        Line::from(format!("  Host: {}", app.config.iot_host)),
        Line::from(format!("  User: {}", app.config.iot_username)),
        Line::from(""),
        Line::from("Controls:"),
        Line::from("  ↑/↓   - Navigate"),
        Line::from("  Enter - Execute / Edit"),
        Line::from("  n     - Edit device name"),
    ];

    let config_block =
        Paragraph::new(config).block(Block::default().borders(Borders::ALL).title("Config"));
    f.render_widget(config_block, right_chunks[0]);

    render_status_messages(f, app, right_chunks[1]);
}

fn render_actions_tab(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(chunks[1]);

    // Actions with selection bar
    let selected_field = ActionsField::from_index(app.actions_selection);

    let actions = vec![
        Line::from("Available Actions (↑/↓ to navigate, Enter to execute):"),
        Line::from(""),
        render_action_item(
            "Generate Configuration",
            matches!(selected_field, ActionsField::GenerateConfig),
            "g",
        ),
        render_action_item(
            "Send Config to IoT Device",
            matches!(selected_field, ActionsField::SendConfig),
            "s",
        ),
        render_action_item(
            "Clear Status Messages",
            matches!(selected_field, ActionsField::ClearMessages),
            "c",
        ),
        Line::from(""),
        Line::from("Operational Commands:"),
        render_action_item(
            "🔍 Get Telegraf Status",
            matches!(selected_field, ActionsField::TelegrafStatus),
            "t",
        ),
        render_action_item(
            "📋 Get Telegraf Logs",
            matches!(selected_field, ActionsField::TelegrafLogs),
            "l",
        ),
        render_action_item(
            "🔄 Restart Telegraf",
            matches!(selected_field, ActionsField::RestartTelegraf),
            "r",
        ),
        render_action_item(
            &format!(
                "✅ Check {} Status",
                if app.config.output_format.as_deref().unwrap_or("influxdb") == "prometheus" {
                    "Prometheus"
                } else {
                    "InfluxDB"
                }
            ),
            matches!(selected_field, ActionsField::ServiceStatus),
            "v",
        ),
        render_action_item(
            "📊 Backup Grafana",
            matches!(selected_field, ActionsField::BackupGrafana),
            "b",
        ),
    ];

    let actions_block =
        Paragraph::new(actions).block(Block::default().borders(Borders::ALL).title("Actions"));
    f.render_widget(actions_block, chunks[0]);

    // Current configuration summary
    let selected_count = app.selected_files.iter().filter(|&&x| x).count();
    let auth_mode = if app.anonymous_mode {
        "Anonymous"
    } else {
        "Username/Password"
    };

    let summary = vec![
        Line::from("Current Configuration:"),
        Line::from(""),
        Line::from(format!("Selected Files: {}", selected_count)),
        Line::from(format!("OPC-UA Server: {}", app.config.ip)),
        Line::from(format!("Authentication: {}", auth_mode)),
        Line::from(format!("IoT Device: {}", app.config.iot_host)),
        Line::from(format!(
            "Output Format: {}",
            app.config
                .output_format
                .as_ref()
                .unwrap_or(&"influxdb".to_string())
        )),
        Line::from(format!(
            "Test Inputs: {}",
            if app.config.include_test_inputs {
                "Yes"
            } else {
                "No"
            }
        )),
    ];

    let summary_block =
        Paragraph::new(summary).block(Block::default().borders(Borders::ALL).title("Summary"));
    f.render_widget(summary_block, right_chunks[0]);

    render_status_messages(f, app, right_chunks[1]);
}

fn render_action_item(text: &str, is_selected: bool, hotkey: &str) -> Line<'static> {
    if is_selected {
        Line::from(vec![
            Span::styled(
                "► ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                text.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" ({})", hotkey), Style::default().fg(Color::Gray)),
        ])
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::raw(text.to_string()),
            Span::styled(format!(" ({})", hotkey), Style::default().fg(Color::Gray)),
        ])
    }
}

fn render_config_field_with_selection(
    f: &mut Frame,
    label: &str,
    value: &str,
    is_editing: bool,
    input_buffer: &str,
    area: Rect,
    is_selected: bool,
) {
    let display_value = if is_editing {
        format!("{}_", input_buffer)
    } else {
        value.to_string()
    };

    let style = if is_editing {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if is_selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let block_style = if is_selected {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let paragraph = Paragraph::new(display_value)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(label)
                .border_style(block_style),
        )
        .style(style);

    f.render_widget(paragraph, area);
}

fn render_status_messages(f: &mut Frame, app: &mut App, area: Rect) {
    let mut messages: Vec<ListItem> = app
        .status_messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            // Split long messages into multiple lines for better readability
            let lines: Vec<&str> = m.lines().collect();
            if lines.len() > 1 {
                // Multi-line message - show with line numbers for clarity
                ListItem::new(format!("[{}] {}", i + 1, lines.join("\n    ")))
            } else {
                ListItem::new(format!("[{}] {}", i + 1, m))
            }
        })
        .collect();

    // Add helpful message if empty
    if messages.is_empty() {
        messages.push(ListItem::new(
            "Status area ready - press 'c' to clear messages, PgUp/PgDn to scroll",
        ));
    }

    let title = format!(
        "Status Messages ({}/{}) - PgUp/PgDn to scroll",
        app.status_messages.len(),
        50
    );
    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_stateful_widget(messages_list, area, &mut app.status_list_state);
}

fn render_help_popup(f: &mut Frame, _app: &App) {
    let popup_area = centered_rect(60, 70, f.area());

    let help_text = vec![
        Line::from("IoT2050 Configuration TUI - Help"),
        Line::from(""),
        Line::from("Global Controls:"),
        Line::from("  Tab/←→/1-7 - Switch between tabs"),
        Line::from("  h/F1    - Toggle this help"),
        Line::from("  PgUp/PgDn - Scroll status messages"),
        Line::from("  q       - Quit application"),
        Line::from(""),
        Line::from("Folder Tab:"),
        Line::from("  ↑/↓     - Navigate directories"),
        Line::from("  Enter   - Enter directory"),
        Line::from("  r       - Refresh listing"),
        Line::from("  h       - Go to home directory"),
        Line::from(""),
        Line::from("Files Tab:"),
        Line::from("  ↑/↓     - Navigate files"),
        Line::from("  Space   - Toggle file selection"),
        Line::from("  l       - Toggle listener mode (OPC-UA subscriptions)"),
        Line::from("  r       - Refresh file list"),
        Line::from("  n       - Edit namespace for selected file"),
        Line::from("  i       - Edit IP for selected file"),
        Line::from("  t       - Edit interval for selected file"),
        Line::from("  p       - Poll namespaces from OPC-UA server"),
        Line::from(""),
        Line::from("Config Tabs:"),
        Line::from("  Letters - Edit corresponding fields"),
        Line::from("  Enter   - Confirm edit"),
        Line::from("  Esc     - Cancel edit"),
        Line::from(""),
        Line::from("Device Tab:"),
        Line::from("  ↑/↓     - Navigate actions/config"),
        Line::from("  Enter   - Execute action / edit field"),
        Line::from("  n       - Edit device name"),
        Line::from("  y/n     - Confirm/cancel destructive ops"),
        Line::from(""),
        Line::from("Config Tab:"),
        Line::from("  g       - Generate configuration"),
        Line::from("  s       - Send config to IoT device"),
        Line::from("  r       - Clear config"),
        Line::from("  ↑/↓     - Scroll vertical"),
        Line::from("  ←/→     - Scroll horizontal"),
        Line::from("  Home    - Reset scroll"),
        Line::from(""),
        Line::from("Actions Tab:"),
        Line::from("  g       - Generate configuration"),
        Line::from("  s       - Send config to IoT device"),
        Line::from("  c       - Clear status messages"),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .style(Style::default().fg(Color::White).bg(Color::Blue));

    f.render_widget(Clear, popup_area);
    f.render_widget(help_paragraph, popup_area);
}

fn render_confirmation_overlay(f: &mut Frame, app: &mut App) {
    let popup_area = centered_rect(60, 30, f.area());

    let action = match app.pending_confirmation.as_deref() {
        Some("stop_normal") => "Stop monitoring stack?",
        Some("stop_volumes") => "Stop monitoring stack and REMOVE ALL DATA (volumes)?",
        Some("restore") => "Restore from backup? This will overwrite existing data.",
        _ => "Confirm action?",
    };

    let warning_text = vec![
        Line::from(""),
        Line::from(format!("⚠️  {}", action))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from("Press 'y' to confirm or 'n' to cancel")
            .style(Style::default().fg(Color::Yellow)),
        Line::from(""),
        Line::from("This action cannot be undone.")
            .style(Style::default().fg(Color::DarkGray)),
    ];

    let warning_paragraph = Paragraph::new(warning_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirmation Required ")
                .border_style(Style::default().fg(Color::Red)),
        )
        .style(Style::default().bg(Color::Black));

    f.render_widget(Clear, popup_area);
    f.render_widget(warning_paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
