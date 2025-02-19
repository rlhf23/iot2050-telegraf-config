use crate::{error::TelegrafError, TelegrafConfig};
use std::path::Path;

use std::fs::{self, File};
use std::io::Write;

mod format;
mod ssh_utils;

pub struct ConfigGenerator {
    config: TelegrafConfig,
}

impl ConfigGenerator {
    pub fn new(config: TelegrafConfig) -> Result<Self, TelegrafError> {
        // Validate configuration
        config
            .validate_ip()
            .map_err(TelegrafError::ValidationError)?;
        config
            .validate_iot_host()
            .map_err(TelegrafError::ValidationError)?;

        Ok(Self { config })
    }

    pub fn get_xml_files(&self) -> Result<Vec<String>, TelegrafError> {
        fs::read_dir(&self.config.folder)
            .map_err(TelegrafError::IoError)?
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "xml") {
                    Some(path.to_str()?.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .into_iter()
            .map(Ok)
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
            let config_string = format::parse_xml(
                file,
                &self.config.ip,
                &self.config.username,
                &self.config.password,
                is_listener,
                &mut namespace_numbers,
            );
            config_strings.push(config_string);
        }

        // Get the influx token if not already set
        let influx_token = self
            .config
            .influx_token
            .as_ref()
            .ok_or_else(|| TelegrafError::ConfigError("InfluxDB token not set".to_string()))?;

        // Generate the final config content
        let config_content = format::generate_config_content(
            influx_token,
            &self.config.bucket_name,
            &config_strings,
            &namespace_numbers,
        );

        // Write to file
        let config_path = self.config.folder.join("telegraf.conf");
        let mut config_file = File::create(&config_path).map_err(|e| TelegrafError::IoError(e))?;

        config_file
            .write_all(config_content.as_bytes())
            .map_err(|e| TelegrafError::IoError(e))?;

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
        .map_err(|e| TelegrafError::SshError(e.to_string()))
    }

    pub fn backup_influx(&self) -> Result<(), TelegrafError> {
        let influx_token = self
            .config
            .influx_token
            .as_ref()
            .ok_or_else(|| TelegrafError::ConfigError("InfluxDB token not set".to_string()))?;

        ssh_utils::backup_influxdb(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
            influx_token,
        )
        .map_err(|e| TelegrafError::SshError(e.to_string()))
    }

    pub fn backup_grafana(&self) -> Result<(), TelegrafError> {
        ssh_utils::backup_grafana_config(
            &self.config.iot_host,
            &self.config.iot_username,
            &self.config.iot_password,
        )
        .map_err(|e| TelegrafError::SshError(e.to_string()))
    }
}
