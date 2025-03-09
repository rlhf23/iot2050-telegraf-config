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
    pub fn validate_ip(&self) -> Result<(), String> {
        let ip_valid = self
            .ip
            .split('.')
            .filter(|part| part.parse::<u8>().is_ok())
            .count()
            == 4;

        if !ip_valid {
            return Err(format!(
                "Invalid IP address format for '{}', expecting something like: 192.168.0.1",
                self.ip
            ));
        }
        Ok(())
    }

    pub fn validate_iot_host(&self) -> Result<(), String> {
        // First check if the host string contains a colon (required for host:port format)
        if !self.iot_host.contains(':') {
            return Err(format!(
                "Missing port specification in IOT host '{}'. Expected format: hostname:port (e.g., 192.168.0.1:22)",
                self.iot_host
            ));
        }

        // Split by colon and validate format
        let iot_host_parts: Vec<&str> = self.iot_host.split(':').collect();

        // Check that we have exactly two parts (host and port)
        if iot_host_parts.len() != 2 {
            return Err(format!(
                "Invalid IOT host format '{}'. Expected format: hostname:port (e.g., 192.168.0.1:22)",
                self.iot_host
            ));
        }

        // Validate that the port is a valid number greater than 0
        match iot_host_parts[1].parse::<u16>() {
            Ok(port) if port > 0 => {}
            _ => {
                return Err(format!(
                    "Invalid port '{}' in IOT host. Port must be a number between 1-65535",
                    iot_host_parts[1]
                ));
            }
        }

        Ok(())
    }
}

pub mod backend;
pub mod error;
