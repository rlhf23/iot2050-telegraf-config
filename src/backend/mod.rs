use crate::{error::TelegrafError, TelegrafConfig};
use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Arc;

// Public re-exports
pub use crate::TelegrafConfig;

pub mod format;
pub mod opcua_poller;
pub mod ssh_utils;

// Include tests
#[cfg(test)]
mod format_test;
#[cfg(test)]
mod opcua_poller_test;
#[cfg(test)]
mod config_generator_test;
#[cfg(test)]
mod ssh_utils_test;

// Property-based tests
#[cfg(test)]
mod format_proptest;
#[cfg(test)]
mod opcua_poller_proptest;

pub struct ConfigGenerator {
    config: TelegrafConfig,
}

impl ConfigGenerator {
    pub fn new(config: TelegrafConfig) -> Result<Self, TelegrafError> {
        // Basic validation here
        config.validate_ip()?;
        config.validate_iot_host()?;

        Ok(Self { config })
    }

    pub fn generate_config(&self) -> Result<String, TelegrafError> {
        if !self.config.folder.exists() {
            return Err(TelegrafError::FolderNotFoundError(
                self.config.folder.to_string_lossy().to_string(),
            ));
        }

        // Determine the output format (defaults to InfluxDB)
        let format = match &self.config.output_format {
            Some(f) if f.to_lowercase() == "prometheus" => format::OutputFormat::Prometheus,
            _ => format::OutputFormat::InfluxDB,
        };

        // Generate mandatory config sections
        let mut config = format::format_config_header();

        // Generate inputs section

        // First, find all XML files in the specified folder
        let xml_files = crate::discover_xml_files(&self.config.folder);

        if xml_files.is_empty() {
            return Err(TelegrafError::XmlFilesNotFoundError(
                self.config.folder.to_string_lossy().to_string(),
            ));
        }

        // OPC UA inputs: regular polling
        let mut namespace_infos = Vec::new();
        for file in &xml_files {
            if self.config.listener_files.contains(&file.to_string()) {
                continue; // Skip listener files, they'll be processed separately
            }

            let opcua_config = format::OpcuaConfig {
                endpoint: &format!("opc.tcp://{}:4840/", self.config.ip),
                username: &self.config.username,
                password: &self.config.password,
                is_listener: false,
                group_name: "",
                namespace_number: "2", // Default namespace, will be updated if needed
                interval_ms: 1000,     // Default interval, will be updated if needed
            };

            match format::parse_xml(&opcua_config, file, &mut namespace_infos) {
                Ok(section) => {
                    config.push_str(&section);
                    config.push('\n');
                }
                Err(e) => {
                    // Log the error but continue with other files
                    eprintln!("Error processing file {}: {}", file, e);
                }
            }
        }

        // OPC UA inputs: listeners (if any)
        for file in &self.config.listener_files {
            let opcua_config = format::OpcuaConfig {
                endpoint: &format!("opc.tcp://{}:4840/", self.config.ip),
                username: &self.config.username,
                password: &self.config.password,
                is_listener: true,
                group_name: "",
                namespace_number: "2", // Default namespace
                interval_ms: 1000,     // Default interval
            };

            match format::parse_xml(&opcua_config, file, &mut namespace_infos) {
                Ok(section) => {
                    config.push_str(&section);
                    config.push('\n');
                }
                Err(e) => {
                    // Log the error but continue with other files
                    eprintln!("Error processing listener file {}: {}", file, e);
                }
            }
        }

        // Add test inputs if requested
        if self.config.include_test_inputs {
            config.push_str("\n");
            config.push_str("# Test inputs\n");
            config.push_str("[[inputs.cpu]]\n");
            config.push_str("  percpu = true\n");
            config.push_str("  totalcpu = true\n");
            config.push_str("  collect_cpu_time = false\n");
            config.push_str("  report_active = false\n");
            config.push_str("\n");
            config.push_str("[[inputs.mem]]\n");
            config.push_str("\n");
            config.push_str("[[inputs.disk]]\n");
            config.push_str("  ignore_fs = [\"tmpfs\", \"devtmpfs\", \"devfs\", \"iso9660\", \"overlay\", \"aufs\", \"squashfs\"]\n");
        }

        // Generate outputs section based on format
        match format {
            format::OutputFormat::InfluxDB => {
                let token = match &self.config.influx_token {
                    Some(token) => token.clone(),
                    None => {
                        // When token is not provided directly, we should look for it in a file
                        match &self.config.token_folder {
                            folder if folder.exists() => {
                                // Try to read token from a file in the specified folder
                                let mut token_path = folder.clone();
                                token_path.push("influx_token.txt");

                                match fs::read_to_string(&token_path) {
                                    Ok(content) => content.trim().to_string(),
                                    Err(_) => {
                                        return Err(TelegrafError::TokenError(
                                            "Token file not found".to_string(),
                                        ))
                                    }
                                }
                            }
                            _ => {
                                return Err(TelegrafError::TokenError(
                                    "Token folder not found".to_string(),
                                ))
                            }
                        }
                    }
                };

                config.push_str(&format!(
                    r#"
# Output configuration
[[outputs.influxdb_v2]]
  urls = ["http://127.0.0.1:8086"]
  token = "{}"
  organization = "siemens"
  bucket = "{}"
  timeout = "5s"
"#,
                    token, self.config.bucket_name
                ));
            }
            format::OutputFormat::Prometheus => {
                config.push_str(
                    r#"
# Output configuration
[[outputs.prometheus_client]]
  listen = ":9273"
  path = "/metrics"
"#,
                );
            }
        }

        Ok(config)
    }

    /// Transfer generated config to the IoT device
    pub fn transfer_config(&self, config_content: &str) -> Result<(), TelegrafError> {
        ssh_utils::transfer_config(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            config_content,
        )
    }

    /// Restart the Telegraf service on the IoT device
    pub fn restart_telegraf(&self) -> Result<(), TelegrafError> {
        ssh_utils::restart_telegraf(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
        )
    }

    /// Check the status of the Telegraf service
    pub fn check_telegraf_status(&self) -> Result<bool, TelegrafError> {
        ssh_utils::check_telegraf_status(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
        )
    }

    pub fn backup_influxdb(&self) -> Result<PathBuf, TelegrafError> {
        ssh_utils::backup_influxdb(
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

    /// Checks if InfluxDB is responding
    pub fn check_influxdb_status(
        &self,
        influx_url: &str,
        timeout_seconds: u64,
    ) -> Result<bool, TelegrafError> {
        ssh_utils::check_service_status(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            influx_url,
            ssh_utils::ServiceType::InfluxDB,
            timeout_seconds,
        )
    }

    /// Checks if Prometheus is responding
    pub fn check_prometheus_status(
        &self,
        prometheus_url: &str,
        timeout_seconds: u64,
    ) -> Result<bool, TelegrafError> {
        ssh_utils::check_service_status(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            prometheus_url,
            ssh_utils::ServiceType::Prometheus,
            timeout_seconds,
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
}
