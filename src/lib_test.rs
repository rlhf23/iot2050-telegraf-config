use crate::{backend::ConfigGenerator, TelegrafConfig};
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ip() {
        // Valid IP
        let config = TelegrafConfig {
            folder: PathBuf::new(),
            ip: "192.168.1.1".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            iot_host: "192.168.1.2:22".to_string(),
            iot_username: "iot_user".to_string(),
            iot_password: "iot_pass".to_string(),
            token_folder: PathBuf::new(),
            bucket_name: "bucket".to_string(),
            influx_token: None,
            listener_files: Vec::new(),
            output_format: None,
            include_test_inputs: false,
        };

        assert!(config.validate_ip().is_ok());

        // Invalid IP
        let invalid_config = TelegrafConfig {
            ip: "invalid.ip".to_string(),
            ..config.clone()
        };

        assert!(invalid_config.validate_ip().is_err());
    }

    #[test]
    fn test_validate_iot_host() {
        // Valid host:port
        let config = TelegrafConfig {
            folder: PathBuf::new(),
            ip: "192.168.1.1".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            iot_host: "192.168.1.2:22".to_string(),
            iot_username: "iot_user".to_string(),
            iot_password: "iot_pass".to_string(),
            token_folder: PathBuf::new(),
            bucket_name: "bucket".to_string(),
            influx_token: None,
            listener_files: Vec::new(),
            output_format: None,
            include_test_inputs: false,
        };

        assert!(config.validate_iot_host().is_ok());

        // Invalid host (no port)
        let invalid_config = TelegrafConfig {
            iot_host: "192.168.1.2".to_string(),
            ..config.clone()
        };

        assert!(invalid_config.validate_iot_host().is_err());

        // Invalid port (zero)
        let invalid_port_config = TelegrafConfig {
            iot_host: "192.168.1.2:0".to_string(),
            ..config.clone()
        };

        assert!(invalid_port_config.validate_iot_host().is_err());
    }

    #[test]
    fn test_config_generator_new() {
        // Valid config
        let config = TelegrafConfig {
            folder: PathBuf::new(),
            ip: "192.168.1.1".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            iot_host: "192.168.1.2:22".to_string(),
            iot_username: "iot_user".to_string(),
            iot_password: "iot_pass".to_string(),
            token_folder: PathBuf::new(),
            bucket_name: "bucket".to_string(),
            influx_token: Some("token".to_string()),
            listener_files: Vec::new(),
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
        };

        let generator_result = ConfigGenerator::new(config);
        assert!(generator_result.is_ok());

        // Invalid config (bad IP)
        let invalid_config = TelegrafConfig {
            folder: PathBuf::new(),
            ip: "not.an.ip".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            iot_host: "192.168.1.2:22".to_string(),
            iot_username: "iot_user".to_string(),
            iot_password: "iot_pass".to_string(),
            token_folder: PathBuf::new(),
            bucket_name: "bucket".to_string(),
            influx_token: None,
            listener_files: Vec::new(),
            output_format: None,
            include_test_inputs: false,
        };

        let invalid_result = ConfigGenerator::new(invalid_config);
        assert!(invalid_result.is_err());
    }
}
