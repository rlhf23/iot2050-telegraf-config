use std::path::PathBuf;

#[derive(Clone)]
pub struct TelegrafConfig {
    pub folder: PathBuf,
    pub ip: String,
    pub username: String,
    pub password: String,
    pub iot_host: String,
    pub iot_username: String,
    pub iot_password: String,
    pub token_folder: PathBuf,
    pub bucket_name: String,
    pub influx_token: Option<String>,
    pub listener_files: Vec<String>,
    pub output_format: Option<String>, // "influxdb" or "prometheus"
    pub include_test_inputs: bool,     // Include CPU, disk, mem inputs for testing
}

impl TelegrafConfig {
    pub fn validate_ip(&self) -> Result<(), crate::error::TelegrafError> {
        // Check if empty
        if self.ip.is_empty() {
            return Err(crate::error::TelegrafError::ValidationError(
                "IP address cannot be empty".to_string(),
            ));
        }

        // Split by dots and check that we have exactly 4 parts
        let parts: Vec<&str> = self.ip.split('.').collect();
        if parts.len() != 4 {
            return Err(crate::error::TelegrafError::ValidationError(format!(
                "Invalid IP address format for '{}', must have exactly 4 parts separated by dots",
                self.ip
            )));
        }

        // Check that each part is a valid number between 0-255
        for part in &parts {
            // Check if part is empty
            if part.is_empty() {
                return Err(crate::error::TelegrafError::ValidationError(format!(
                    "Invalid IP address format for '{}', empty segment found",
                    self.ip
                )));
            }

            // Check if part contains only digits
            if !part.chars().all(|c| c.is_ascii_digit()) {
                return Err(crate::error::TelegrafError::ValidationError(format!(
                    "Invalid IP address format for '{}', segment '{}' contains non-digit characters",
                    self.ip, part
                )));
            }

            // Check if part is a valid u8 (0-255)
            match part.parse::<u8>() {
                Ok(_) => {} // Valid
                Err(_) => {
                    return Err(crate::error::TelegrafError::ValidationError(format!(
                        "Invalid IP address format for '{}', segment '{}' must be between 0-255",
                        self.ip, part
                    )));
                }
            }
        }

        Ok(())
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
            // Create a temporary config with this hostname as the IP for validation
            let temp_config = TelegrafConfig {
                ip: hostname.to_string(),
                ..self.clone()
            };
            
            // Use our existing IP validation logic
            if let Err(e) = temp_config.validate_ip() {
                return Err(crate::error::TelegrafError::HostFormatError(format!(
                    "Invalid IP format in IOT host: {}", e
                )));
            }
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

pub mod backend;
pub mod error;

#[cfg(test)]
mod lib_test;
