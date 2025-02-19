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
        let iot_host_parts: Vec<&str> = self.iot_host.split(':').collect();
        let iot_host_valid = iot_host_parts.len() == 2
            && iot_host_parts[1]
                .parse::<u16>()
                .map_or(false, |port| port > 0);

        if !iot_host_valid {
            return Err(format!(
                "Invalid IOT host format for '{}', expecting something like: 192.168.0.1:22",
                self.iot_host
            ));
        }
        Ok(())
    }
}

pub mod backend;
pub mod error;
