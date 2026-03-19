use crate::{error::TelegrafError, SelectedOpcUaNode, TelegrafConfig};

use std::fs::File;
use std::io::Write;
use std::path::Path;

pub mod backup;
#[cfg(test)]
mod backup_test;
#[cfg(test)]
mod config_generator_test;
pub mod dashboard;
pub mod deployment;
#[cfg(test)]
mod deployment_test;
mod format;
#[cfg(test)]
mod format_test;
pub mod opcua_poller;
#[cfg(test)]
mod opcua_poller_test;
pub mod ssh_utils;
#[cfg(test)]
mod ssh_utils_test;

pub use dashboard::{generate_dashboard, sanitize_uid, DashboardConfig};
pub use format::OutputFormat;
pub use ssh_utils::{check_service_status, ServiceType};

#[derive(Default)]
pub struct FileConfig {
    pub namespace: String,
    pub interval_ms: u64,
    pub ip: Option<String>,
}

pub struct ConfigGenerator {
    config: TelegrafConfig,
    file_configs: std::collections::HashMap<String, FileConfig>,
    output_format: OutputFormat,
    include_test_inputs: bool,
}

impl ConfigGenerator {
    pub fn new(config: TelegrafConfig) -> Result<Self, TelegrafError> {
        // Validate configuration
        config.validate_ip()?;
        config.validate_iot_host()?;

        // Determine output format from config or default to InfluxDB
        let output_format = match config.output_format.as_deref() {
            Some("prometheus") => OutputFormat::Prometheus,
            _ => OutputFormat::InfluxDB, // Default to InfluxDB for None or any other value
        };

        Ok(Self {
            include_test_inputs: config.include_test_inputs,
            config,
            file_configs: std::collections::HashMap::new(),
            output_format,
        })
    }

    pub fn set_output_format(&mut self, output_format: OutputFormat) {
        self.output_format = output_format;
    }

    pub fn set_include_test_inputs(&mut self, include_test_inputs: bool) {
        self.include_test_inputs = include_test_inputs;
    }

    pub fn set_file_config(
        &mut self,
        file_path: String,
        namespace: String,
        interval_ms: u64,
        ip: Option<String>,
    ) {
        self.file_configs.insert(
            file_path,
            FileConfig {
                namespace,
                interval_ms,
                ip,
            },
        );
    }

    /// Discover all XML files in the given folder
    /// Returns a list of file paths as strings
    pub fn discover_xml_files(folder: &std::path::PathBuf) -> Vec<String> {
        std::fs::read_dir(folder)
            .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "xml") {
                    Some(path.to_str()?.to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn generate_config(
        &self,
        xml_files: &[String],
        listener_files: &[String],
    ) -> Result<String, TelegrafError> {
        let mut config_strings = Vec::new();
        let mut namespace_numbers = Vec::new();

        // Generate configuration strings for each XML file
        for file in xml_files {
            let is_listener = listener_files.contains(file);
            let file_config = self.file_configs.get(file).ok_or_else(|| {
                TelegrafError::ConfigError(format!("No configuration found for file: {}", file))
            })?;

            // Use file-specific IP if available, otherwise use the default IP
            let ip = match &file_config.ip {
                Some(ip) if !ip.is_empty() => {
                    // Validate custom IP using the same validation logic as the main config
                    // Create a temporary config with this IP for validation
                    let temp_config = TelegrafConfig {
                        ip: ip.clone(),
                        ..self.config.clone()
                    };

                    temp_config.validate_ip().map_err(|e| {
                        TelegrafError::ValidationError(format!(
                            "Invalid custom IP for file '{}': {}",
                            file, e
                        ))
                    })?;

                    ip
                }
                _ => &self.config.ip,
            };

            let config = format::OpcuaConfig {
                ip,
                username: &self.config.username,
                password: &self.config.password,
                is_listener,
                group_name: "", // This will be determined in parse_xml
                namespace_number: &file_config.namespace,
                interval_ms: file_config.interval_ms,
                identifier_type: "i", // Default to numeric identifier
                use_source_timestamp: self.config.use_source_timestamp,
            };

            let config_string = format::parse_xml(&config, file, &mut namespace_numbers)
                .map_err(|e| TelegrafError::ConfigError(format!("Failed to parse XML: {}", e)))?;
            config_strings.push(config_string);
        }

        // Generate configuration for selected OPC UA nodes from browser if available
        if !self.config.selected_opcua_nodes.is_empty() {
            // First, separate nodes into folder groups and individual nodes
            let mut folder_groups: std::collections::HashMap<String, Vec<SelectedOpcUaNode>> =
                std::collections::HashMap::new();
            let mut individual_nodes: Vec<SelectedOpcUaNode> = Vec::new();

            for node in &self.config.selected_opcua_nodes {
                if let Some(folder_name) = &node.folder_name {
                    // Add to folder group
                    folder_groups
                        .entry(folder_name.clone())
                        .or_default()
                        .push(node.clone());
                } else {
                    // Individual node (not part of a folder)
                    individual_nodes.push(node.clone());
                }
            }

            // Process folder groups first - create a config for each folder using format_regular_config
            for (folder_name, folder_nodes) in folder_groups {
                if folder_nodes.is_empty() {
                    continue;
                }

                // Get the namespace from the first node (all nodes in a folder should have the same namespace)
                let namespace = folder_nodes[0].namespace;
                let namespace_str = namespace.to_string();

                // Track the namespace
                let namespace_info = format::NamespaceInfo {
                    number: namespace_str.clone(),
                    file_name: format!("folder_{}", folder_name.replace(" ", "_").to_lowercase()),
                };

                // Add to namespace list if not already there
                if !namespace_numbers
                    .iter()
                    .any(|info| info.number == namespace_str)
                {
                    namespace_numbers.push(namespace_info);
                }

                // Create the nodes configuration string for this folder
                let mut node_configs = Vec::new();

                // Default identifier type for the group (will be overridden if we have nodes)
                let mut group_identifier_type = "i";

                for node in &folder_nodes {
                    // Extract the identifier and determine identifier_type
                    let (identifier, identifier_type) = match &node.node_id.identifier {
                        opcua::types::Identifier::String(s) => (s.to_string(), "s"),
                        opcua::types::Identifier::Numeric(i) => (i.to_string(), "i"),
                        opcua::types::Identifier::Guid(guid) => (format!("{:?}", guid), "g"),
                        opcua::types::Identifier::ByteString(bytes) => {
                            (format!("{:?}", bytes), "b")
                        }
                    };

                    // If this is the first node, use its identifier type for the group
                    if node_configs.is_empty() {
                        group_identifier_type = identifier_type;
                    }

                    // Escape any quotes in the identifier for TOML format
                    let escaped_identifier = identifier.replace('"', "\\\"");

                    // Format the node entry for the group
                    node_configs.push(format!(
                        "{{name=\"{}\", identifier=\"{}\"}}",
                        node.display_name, escaped_identifier
                    ));
                }

                // Join all node configs with commas and newlines for the group format
                let nodes_str = node_configs.join(",\n        ");

                // Create a grouped config using format_regular_config
                let opcua_config = format::OpcuaConfig {
                    ip: &self.config.ip,
                    username: &self.config.username,
                    password: &self.config.password,
                    is_listener: false,
                    group_name: &folder_name,
                    namespace_number: &namespace_str,
                    interval_ms: 1000,                      // Default interval
                    identifier_type: group_identifier_type, // Use identifier type from the first node
                    use_source_timestamp: self.config.use_source_timestamp,
                };

                let config_string = format::format_regular_config(&opcua_config, &nodes_str);
                config_strings.push(config_string);
            }

            // Now handle individual nodes (not part of a folder)
            if !individual_nodes.is_empty() {
                // Group individual nodes by namespace
                let mut namespace_groups: std::collections::HashMap<u16, Vec<SelectedOpcUaNode>> =
                    std::collections::HashMap::new();

                for node in individual_nodes {
                    namespace_groups
                        .entry(node.namespace)
                        .or_default()
                        .push(node);
                }

                // Create configuration for each namespace group of individual nodes
                for (namespace, nodes) in namespace_groups {
                    // Track the namespace
                    let namespace_str = namespace.to_string();
                    let namespace_info = format::NamespaceInfo {
                        number: namespace_str.clone(),
                        file_name: format!("opcua_browser_ns{}", namespace_str),
                    };

                    // Add to namespace list if not already there
                    if !namespace_numbers
                        .iter()
                        .any(|info| info.number == namespace_str)
                    {
                        namespace_numbers.push(namespace_info);
                    }

                    // Create individual node configurations
                    let mut node_configs = Vec::new();

                    // Default identifier type for this namespace group (will be overridden by first node)
                    let mut group_identifier_type = "i";

                    for node in nodes {
                        // Extract the identifier and determine identifier_type
                        let (identifier, identifier_type) = match &node.node_id.identifier {
                            opcua::types::Identifier::String(s) => (s.to_string(), "s"),
                            opcua::types::Identifier::Numeric(i) => (i.to_string(), "i"),
                            opcua::types::Identifier::Guid(guid) => (format!("{:?}", guid), "g"),
                            opcua::types::Identifier::ByteString(bytes) => {
                                (format!("{:?}", bytes), "b")
                            }
                        };

                        // If this is the first node, use its identifier type for the group
                        if node_configs.is_empty() {
                            group_identifier_type = identifier_type;
                        }

                        // Escape any quotes in the identifier for TOML format
                        let escaped_identifier = identifier.replace('"', "\\\"");

                        // Format individual node config
                        let node_config = format!(
                            "  # {{0}}
    [[inputs.opcua.nodes]]
      name = \"{}\"
      namespace = \"{}\"
      identifier_type = \"{}\"
      identifier = \"{}\"
      interval = \"{}ms\"",
                            node.measurement_name,
                            node.namespace,
                            identifier_type,
                            escaped_identifier,
                            node.interval_ms
                        );
                        node_configs.push(node_config);
                    }

                    // Create the full config for individual nodes using format_browsed_config
                    let opcua_config = format::OpcuaConfig {
                        ip: &self.config.ip,
                        username: &self.config.username,
                        password: &self.config.password,
                        is_listener: false,
                        group_name: &format!("opcua_browser_ns{}", namespace),
                        namespace_number: &namespace.to_string(),
                        interval_ms: 1000,                      // Default interval
                        identifier_type: group_identifier_type, // Use identifier type from the first node
                        use_source_timestamp: self.config.use_source_timestamp,
                    };

                    let config_string =
                        format::format_browsed_config(&opcua_config, &node_configs.join("\n"));
                    config_strings.push(config_string);
                }
            }
        }

        // Generate the final config content
        let config_content = format::format_config_header(
            &config_strings,
            &namespace_numbers,
            self.output_format,
            self.include_test_inputs,
        );

        // Write to file
        let config_path = self.config.folder.join("telegraf.conf");
        let mut config_file = File::create(&config_path).map_err(TelegrafError::IoError)?;

        config_file
            .write_all(config_content.as_bytes())
            .map_err(TelegrafError::IoError)?;

        Ok(config_content)
    }

    pub fn send_config(&self) -> Result<String, TelegrafError> {
        let config_path = self.config.folder.join("telegraf.conf");
        if !config_path.exists() {
            return Err(TelegrafError::ConfigError(
                "telegraf.conf file does not exist in the specified folder".to_string(),
            ));
        }

        ssh_utils::send_and_restart_telegraf(
            &config_path,
            "telegraf/telegraf.conf",
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
        )
    }

    /// Generate a Grafana dashboard from template
    ///
    /// # Arguments
    /// * `measurements` - List of measurement names (from XML parsing or browser selection)
    /// * `bucket` - InfluxDB bucket name (default: "telegraf")
    /// * `datasource_uid` - Grafana datasource UID or name (default: "InfluxDB")
    /// * `template_path` - Optional custom template path (uses default if None)
    ///
    /// # Returns
    /// * `Ok(String)` - Generated dashboard JSON
    /// * `Err(TelegrafError)` - Generation error
    pub fn generate_grafana_dashboard(
        &self,
        measurements: &[String],
        bucket: Option<&str>,
        datasource_uid: Option<&str>,
        template_path: Option<&Path>,
    ) -> Result<String, TelegrafError> {
        if measurements.is_empty() {
            return Err(TelegrafError::ConfigError(
                "At least one measurement is required for dashboard generation".to_string(),
            ));
        }

        // Use first measurement as primary for dashboard UID and title
        let primary_measurement = &measurements[0];
        let dashboard_uid = sanitize_uid(primary_measurement);

        let config = DashboardConfig {
            uid: dashboard_uid.clone(),
            title: format!("{} Monitor", primary_measurement),
            measurements: measurements.to_vec(),
            bucket: bucket.unwrap_or("telegraf").to_string(),
            datasource_uid: datasource_uid.unwrap_or("InfluxDB").to_string(),
        };

        let dashboard_json = generate_dashboard(&config, template_path)?;

        // Write to config folder
        let dashboard_path = self.config.folder.join("grafana_dashboard.json");
        let mut dashboard_file = File::create(&dashboard_path).map_err(TelegrafError::IoError)?;
        dashboard_file
            .write_all(dashboard_json.as_bytes())
            .map_err(TelegrafError::IoError)?;

        Ok(dashboard_json)
    }

    /// Deploy Grafana dashboard via API
    ///
    /// # Arguments
    /// * `dashboard_json` - Dashboard JSON string (from generate_grafana_dashboard)
    ///
    /// # Returns
    /// * `Ok(())` - Dashboard deployed successfully
    /// * `Err(TelegrafError)` - Deployment error
    pub fn deploy_grafana_dashboard(&self, dashboard_json: &str) -> Result<(), TelegrafError> {
        ssh_utils::deploy_grafana_dashboard(
            dashboard_json,
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
        )
    }

    pub fn backup_influx(&self) -> Result<String, TelegrafError> {
        ssh_utils::backup_influxdb(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            None,
        )
    }

    pub fn backup_grafana(&self) -> Result<String, TelegrafError> {
        ssh_utils::backup_grafana_config(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
        )
    }

    pub fn get_telegraf_status(&self) -> Result<String, TelegrafError> {
        ssh_utils::get_telegraf_status(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
        )
    }

    pub fn get_telegraf_logs(&self, lines: usize) -> Result<String, TelegrafError> {
        ssh_utils::get_telegraf_logs(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            lines,
        )
    }

    /// Generic method to check if a service is responding
    ///
    /// # Returns
    /// A tuple containing a boolean indicating if the service is healthy and a status message string
    pub fn check_service_status(
        &self,
        service_url: &str,
        service_type: ssh_utils::ServiceType,
        timeout_seconds: u64,
    ) -> Result<(bool, String), TelegrafError> {
        ssh_utils::check_service_status(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            service_url,
            service_type,
            timeout_seconds,
        )
    }

    /// Check if InfluxDB is responding
    ///
    /// # Returns
    /// A tuple containing a boolean indicating if InfluxDB is healthy and a status message string
    pub fn check_influxdb_status(&self) -> Result<(bool, String), TelegrafError> {
        ssh_utils::check_influxdb_status(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            5,
        )
    }
}
