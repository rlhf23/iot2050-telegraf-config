use crate::{error::TelegrafError, SelectedOpcUaNode, TelegrafConfig};

use std::fs::File;
use std::io::Write;

#[cfg(test)]
mod config_generator_test;
mod format;
#[cfg(test)]
mod format_test;
pub mod opcua_poller;
#[cfg(test)]
mod opcua_poller_test;
pub mod opcua_test_server;
mod ssh_utils;
#[cfg(test)]
mod ssh_utils_test;

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
            };

            let config_string = format::parse_xml(&config, file, &mut namespace_numbers)
                .map_err(|e| TelegrafError::ConfigError(format!("Failed to parse XML: {}", e)))?;
            config_strings.push(config_string);
        }

        // Generate configuration strings for selected OPC UA nodes
        if !self.config.selected_opcua_nodes.is_empty() {
            // Group nodes by namespace for efficient configuration
            let mut namespace_groups: std::collections::HashMap<u16, Vec<&SelectedOpcUaNode>> =
                std::collections::HashMap::new();

            for node in &self.config.selected_opcua_nodes {
                namespace_groups
                    .entry(node.namespace)
                    .or_default()
                    .push(node);
            }

            // Create configuration for each namespace group
            for (namespace, nodes) in namespace_groups {
                // Track the namespace
                let namespace_str = namespace.to_string();
                let namespace_info = format::NamespaceInfo {
                    number: namespace_str.clone(),
                    file_name: format!("opcua_browser_{}", namespace_str),
                };

                // Check if this namespace is already in the list
                if !namespace_numbers
                    .iter()
                    .any(|info| info.number == namespace_str)
                {
                    namespace_numbers.push(namespace_info);
                }

                // Create configuration string for this namespace group
                let mut node_configs = Vec::new();

                for node in nodes {
                    // Extract the identifier and determine identifier_type based on the NodeId type
                    let (identifier, identifier_type) = match &node.node_id.identifier {
                        // For String type identifiers (type "s")
                        opcua::types::Identifier::String(ua_string) => {
                            // Need to extract the actual String from UAString
                            let value = if let Some(val) = &ua_string.value() {
                                val.clone() // Already a String
                            } else {
                                String::new() // Empty string as fallback
                            };
                            (value, "s")
                        },
                        // For numeric identifiers (type "i")
                        opcua::types::Identifier::Numeric(i) => {
                            (i.to_string(), "i")
                        },
                        // For GUID identifiers (type "g")
                        opcua::types::Identifier::Guid(guid) => {
                            (format!("{:?}", guid), "g")
                        },
                        // For ByteString identifiers (type "b")
                        opcua::types::Identifier::ByteString(bytes) => {
                            (format!("{:?}", bytes), "b")
                        },
                        // For other types, use debug formatting as fallback
                        _ => (format!("{:?}", node.node_id.identifier), "s") // Default to string type
                    };
                    
                    // Escape any quotes in the identifier for TOML format
                    let escaped_identifier = identifier.replace('"', "\\\"");
                    
                    let node_config = format!(
                        "    # {{0}}\n    [[inputs.opcua.nodes]]\n      name = \"{}\"\n      namespace = \"{}\"\n      identifier_type = \"{}\"\n      identifier = \"{}\"\n      interval = \"{}ms\"\n",
                        node.measurement_name,
                        node.namespace,
                        identifier_type,
                        escaped_identifier,
                        node.interval_ms
                    );

                    node_configs.push(node_config);
                }

                // Create the full config for this namespace using format_browsed_config for the header
                let opcua_config = format::OpcuaConfig {
                    ip: &self.config.ip,
                    username: &self.config.username,
                    password: &self.config.password,
                    is_listener: false,
                    group_name: &format!("opcua_browser_ns{}", namespace),
                    namespace_number: &namespace.to_string(),
                    interval_ms: 1000, // Default interval
                };
                
                let config_string = format::format_browsed_config(&opcua_config, &node_configs.join("\n"));

                config_strings.push(config_string);
            }
        }

        // Get the influx token if not already set (only needed for InfluxDB output)
        let influx_token = if self.output_format == OutputFormat::InfluxDB {
            self.config
                .influx_token
                .as_ref()
                .ok_or_else(|| TelegrafError::ConfigError("InfluxDB token not set".to_string()))?
        } else {
            // For Prometheus, we don't need an influx token, so use empty string
            ""
        };

        // Generate the final config content
        let config_content = format::format_config_header(
            influx_token,
            &self.config.bucket_name,
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

    pub fn send_config(&self) -> Result<(), TelegrafError> {
        let config_path = self.config.folder.join("telegraf.conf");
        if !config_path.exists() {
            return Err(TelegrafError::ConfigError(
                "telegraf.conf file does not exist in the specified folder".to_string(),
            ));
        }

        ssh_utils::send_and_restart_telegraf(
            &config_path,
            "/etc/telegraf/telegraf.conf",
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
        )
    }

    pub fn backup_influx(&self) -> Result<(), TelegrafError> {
        // Ensure we have an InfluxDB token
        // Get token from config if available, otherwise it will be read from /etc/default/telegraf
        let influx_token = self.config.influx_token.as_deref();

        ssh_utils::backup_influxdb(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            None,
        )
    }

    pub fn backup_grafana(&self) -> Result<(), TelegrafError> {
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
    pub fn check_service_status(
        &self,
        service_url: &str,
        service_type: ssh_utils::ServiceType,
        timeout_seconds: u64,
    ) -> Result<bool, TelegrafError> {
        ssh_utils::check_service_status(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            service_url,
            service_type,
            timeout_seconds,
        )
    }

    pub fn check_influxdb_status(
        &self,
        service_url: &str,
        service_type: ssh_utils::ServiceType,
        timeout_seconds: u64,
    ) -> Result<bool, TelegrafError> {
        ssh_utils::check_influxdb_status(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            5,
        )
    }
}
