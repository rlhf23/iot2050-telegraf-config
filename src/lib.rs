use opcua::types::NodeId;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SelectedOpcUaNode {
    pub node_id: NodeId,
    pub namespace: u16,
    pub browse_name: String,
    pub display_name: String,
    pub measurement_name: String,
    pub interval_ms: u32,
    pub folder_name: Option<String>, // Optional folder name for grouping nodes
}

/// Configuration for OPC-UA server connection
/// Used by OpcUaPoller for browsing and namespace operations
#[derive(Debug, Clone)]
pub struct OpcUaConnectionConfig {
    pub ip: String,
    pub username: String,
    pub password: String,
}

/// Configuration for IoT device deployment
/// Used for SSH operations like deploying configs, backups, etc.
#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub iot_host: String,
    pub iot_username: String,
    pub iot_password: String,
}

/// Full Telegraf configuration for generating and deploying configs
#[derive(Debug, Clone)]
pub struct TelegrafConfig {
    pub folder: PathBuf,
    pub ip: String,
    pub username: String,
    pub password: String,
    pub iot_host: String,
    pub iot_username: String,
    pub iot_password: String,
    pub listener_files: Vec<String>,
    pub output_format: Option<String>, // "influxdb" or "prometheus"
    pub include_test_inputs: bool,     // Include CPU, disk, mem inputs for testing
    pub include_opcua_diagnostics: bool, // Include OPC UA server diagnostics
    pub selected_opcua_nodes: Vec<SelectedOpcUaNode>, // Selected OPC UA nodes from browser
    pub use_source_timestamp: bool,    // Use "source" instead of "gather" for timestamp
    pub ship_display_name: Option<String>, // Full Culture ship name (e.g. "ROU Killing Time")
    pub ship_hostname: Option<String>,    // Short hostname slug (e.g. "killing-time")
}

impl OpcUaConnectionConfig {
    /// Create from TelegrafConfig
    pub fn from_telegraf_config(config: &TelegrafConfig) -> Self {
        Self {
            ip: config.ip.clone(),
            username: config.username.clone(),
            password: config.password.clone(),
        }
    }
}

impl DeploymentConfig {
    /// Create from TelegrafConfig
    pub fn from_telegraf_config(config: &TelegrafConfig) -> Self {
        Self {
            iot_host: config.iot_host.clone(),
            iot_username: config.iot_username.clone(),
            iot_password: config.iot_password.clone(),
        }
    }
}

impl TelegrafConfig {
    /// Extract OPC-UA connection config
    pub fn connection_config(&self) -> OpcUaConnectionConfig {
        OpcUaConnectionConfig::from_telegraf_config(self)
    }

    /// Extract deployment config
    pub fn deployment_config(&self) -> DeploymentConfig {
        DeploymentConfig::from_telegraf_config(self)
    }

    /// Validate the entire configuration
    /// Returns a tuple of (is_valid, validation_errors)
    /// This checks for:
    /// 1. Valid IP format
    /// 2. Valid IoT host
    /// 3. Duplicate namespaces across files with the same IP
    pub fn validate_config(
        &self,
        file_configs: &std::collections::HashMap<String, crate::error::XmlFileValidation>,
    ) -> Result<(), Vec<crate::error::TelegrafError>> {
        let mut errors = Vec::new();

        // 1. Validate IP
        if let Err(e) = self.validate_ip() {
            errors.push(e);
        }

        // 2. Validate IoT host
        if let Err(e) = self.validate_iot_host() {
            errors.push(e);
        }

        // 3. Check for duplicate namespaces
        let mut namespace_ip_map: std::collections::HashMap<(String, String), Vec<String>> =
            std::collections::HashMap::new();

        for (file, config) in file_configs {
            // Validate namespace - it's required for all files
            if config.namespace.is_empty() {
                errors.push(crate::error::TelegrafError::ValidationError(format!(
                    "Missing namespace in file: {}",
                    file
                )));
                continue;
            } else if let Err(e) = self.validate_namespace(&config.namespace) {
                errors.push(e);
                continue;
            }

            // Validate interval if provided
            if !config.interval_ms.is_empty() {
                if let Err(e) = self.validate_interval(&config.interval_ms) {
                    errors.push(e);
                    continue;
                }
            }

            // Validate custom IP if provided
            let ip = if !config.ip.is_empty() {
                // Create a temporary config with this IP for validation
                let temp_config = TelegrafConfig {
                    ip: config.ip.clone(),
                    ..self.clone()
                };

                if let Err(e) = temp_config.validate_ip() {
                    errors.push(e);
                    continue;
                }

                config.ip.clone()
            } else {
                self.ip.clone()
            };

            // Track namespace + IP combinations to detect duplicates
            if !config.namespace.is_empty() {
                let key = (ip, config.namespace.clone());
                namespace_ip_map.entry(key).or_default().push(file.clone());
            }
        }

        // Check for duplicate namespaces on the same IP
        for ((ip, namespace), files) in namespace_ip_map {
            if files.len() > 1 {
                errors.push(crate::error::TelegrafError::ValidationError(format!(
                    "Duplicate namespace {} on IP {}: {}",
                    namespace,
                    ip,
                    files.join(", ")
                )));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    pub fn validate_ip_for_file(&self, ip: &str) -> Result<(), crate::error::TelegrafError> {
        // Create a temporary config with the provided IP
        let temp_config = TelegrafConfig {
            ip: ip.to_string(),
            ..self.clone()
        };

        // Use the existing validation logic
        temp_config.validate_ip()
    }

    pub fn validate_ip(&self) -> Result<(), crate::error::TelegrafError> {
        // Check if empty
        if self.ip.is_empty() {
            return Err(crate::error::TelegrafError::ValidationError(
                "IP address cannot be empty".to_string(),
            ));
        }

        // Check if we have IP:PORT format
        let (ip_part, port_part) = if self.ip.contains(':') {
            // Find the last colon (in case there are colons in the IP, like in IPv6)
            // For IPv4, there will only be one colon separating IP and port
            let last_colon_pos = self.ip.rfind(':').unwrap();
            let ip_addr = &self.ip[0..last_colon_pos];
            let port = &self.ip[last_colon_pos + 1..];
            (ip_addr, Some(port))
        } else {
            // No port specified
            (self.ip.as_str(), None)
        };

        // Check if IP is empty
        if ip_part.is_empty() {
            return Err(crate::error::TelegrafError::ValidationError(
                "IP address cannot be empty".to_string(),
            ));
        }

        // Validate IP portion
        self.validate_ip_portion(ip_part)?;

        // If we have a port, validate it
        if let Some(port) = port_part {
            // Validate that the port is a valid number
            match port.parse::<u16>() {
                Ok(_) => {} // Valid port
                Err(_) => {
                    return Err(crate::error::TelegrafError::ValidationError(format!(
                        "Invalid port '{}' in IP address. Port must be a number between 0-65535",
                        port
                    )));
                }
            }
        }

        Ok(())
    }

    /// Helper function to validate just the IP portion
    fn validate_ip_portion(&self, ip: &str) -> Result<(), crate::error::TelegrafError> {
        // Split by dots and check that we have exactly 4 parts
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return Err(crate::error::TelegrafError::ValidationError(format!(
                "Invalid IP address format for '{}', must have exactly 4 parts separated by dots",
                ip
            )));
        }

        // Check that each part is a valid number between 0-255
        for part in &parts {
            // Check if part is empty
            if part.is_empty() {
                return Err(crate::error::TelegrafError::ValidationError(format!(
                    "Invalid IP address format for '{}', empty segment found",
                    ip
                )));
            }

            // Check if part contains only digits
            if !part.chars().all(|c| c.is_ascii_digit()) {
                return Err(crate::error::TelegrafError::ValidationError(format!(
                    "Invalid IP address format for '{}', segment '{}' contains non-digit characters",
                    ip, part
                )));
            }

            // Check if part is a valid u8 (0-255)
            match part.parse::<u8>() {
                Ok(_) => {} // Valid
                Err(_) => {
                    return Err(crate::error::TelegrafError::ValidationError(format!(
                        "Invalid IP address format for '{}', segment '{}' must be between 0-255",
                        ip, part
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn validate_namespace(&self, namespace: &str) -> Result<(), crate::error::TelegrafError> {
        // Check if namespace is empty
        if namespace.trim().is_empty() {
            return Err(crate::error::TelegrafError::ValidationError(
                "Namespace cannot be empty".to_string(),
            ));
        }

        // Check if namespace is numeric (valid for OPC UA)
        if !namespace.chars().all(|c| c.is_ascii_digit()) {
            return Err(crate::error::TelegrafError::ValidationError(format!(
                "Invalid namespace '{}': Must contain only digits",
                namespace
            )));
        }

        Ok(())
    }

    pub fn validate_interval(&self, interval: &str) -> Result<(), crate::error::TelegrafError> {
        // Check if interval is empty - empty is OK as we use defaults
        if interval.trim().is_empty() {
            return Ok(());
        }

        // Check if interval is numeric
        if !interval.chars().all(|c| c.is_ascii_digit()) {
            return Err(crate::error::TelegrafError::ValidationError(format!(
                "Invalid interval '{}': Must contain only digits",
                interval
            )));
        }

        // Check if interval is greater than zero
        match interval.parse::<u32>() {
            Ok(val) if val > 0 => Ok(()),
            _ => Err(crate::error::TelegrafError::ValidationError(format!(
                "Invalid interval '{}': Must be greater than zero",
                interval
            ))),
        }
    }

    pub fn validate_iot_host(&self) -> Result<(), crate::error::TelegrafError> {
        // First check if the host string contains a colon (required for host:port format)
        if !self.iot_host.contains(':') {
            return Err(crate::error::TelegrafError::HostFormatError(format!(
                "Missing port specification in IOT host '{}'. Expected format: hostname:port (e.g., 192.168.0.1:22)",
                self.iot_host
            )));
        }

        // Split by colon and validate format
        let iot_host_parts: Vec<&str> = self.iot_host.split(':').collect();

        // Check that we have exactly two parts (host and port)
        if iot_host_parts.len() != 2 {
            return Err(crate::error::TelegrafError::HostFormatError(format!(
                "Invalid IOT host format '{}'. Expected format: hostname:port (e.g., 192.168.0.1:22)",
                self.iot_host
            )));
        }

        // Get the hostname and port
        let hostname = iot_host_parts[0];
        let port = iot_host_parts[1];

        // Check if hostname is empty
        if hostname.is_empty() {
            return Err(crate::error::TelegrafError::HostFormatError(
                "Hostname cannot be empty".to_string(),
            ));
        }

        // Try to validate as an IP address if:
        // 1. It contains only digits and dots
        // 2. It contains at least one dot (to separate IP segments)
        let only_digits_and_dots = hostname.chars().all(|c| c.is_ascii_digit() || c == '.');
        let has_dots = hostname.contains('.');

        if only_digits_and_dots && has_dots {
            // Validate IP portion directly
            self.validate_ip_portion(hostname)?;
        }
        // If not an IP-like pattern or validation passed, allow other hostname formats (DNS names, etc.)

        // Validate that the port is a valid number greater than 0
        match port.parse::<u16>() {
            Ok(port_num) if port_num > 0 => {}
            _ => {
                return Err(crate::error::TelegrafError::HostFormatError(format!(
                    "Invalid port '{}' in IOT host. Port must be a number between 1-65535",
                    port
                )));
            }
        }

        Ok(())
    }
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

pub mod backend;
pub mod error;
pub mod worker;

#[cfg(feature = "gui")]
pub mod gui_controller;
#[cfg(feature = "gui")]
pub mod gui_renderer;

pub use worker::{WorkerCommand, WorkerHandle, WorkerResponse};

#[cfg(test)]
mod error_test;
#[cfg(test)]
mod lib_test;
#[cfg(test)]
mod worker_tests;
