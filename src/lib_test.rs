use crate::{backend::ConfigGenerator, TelegrafConfig};
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ip() {
        // Sample config for testing
        let base_config = TelegrafConfig {
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

        // Valid IP addresses
        let valid_ips = vec![
            "192.168.1.1",
            "127.0.0.1",
            "0.0.0.0",
            "255.255.255.255",
            "10.0.0.1",
            "192.168.1.01", // Leading zeros are valid
        ];

        for valid_ip in valid_ips {
            let config = TelegrafConfig {
                ip: valid_ip.to_string(),
                ..base_config.clone()
            };
            assert!(
                config.validate_ip().is_ok(),
                "IP {} should be valid",
                valid_ip
            );
        }

        // Invalid IP addresses
        let invalid_ips = vec![
            "invalid.ip",    // Non-numeric
            "192.168.1",     // Too few segments
            "192.168.1.1.1", // Too many segments
            "192.168..1",    // Empty segment
            "192.168.1.",    // Trailing dot
            ".192.168.1.1",  // Leading dot
            "192.168.1.300", // Segment too large
            "192.168.1.a",   // Non-numeric segment
            "192..168.1.1",  // Empty segment
            "192.168.1.-1",  // Negative number
            "192.11..19.9",  // Double dot
            "",              // Empty string
        ];

        for invalid_ip in invalid_ips {
            let config = TelegrafConfig {
                ip: invalid_ip.to_string(),
                ..base_config.clone()
            };
            assert!(
                config.validate_ip().is_err(),
                "IP {} should be invalid",
                invalid_ip
            );
        }
    }

    #[test]
    fn test_validate_iot_host() {
        // Sample config for testing
        let base_config = TelegrafConfig {
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

        // Test valid hostname formats
        let valid_hosts = vec![
            "192.168.1.2:22",               // Valid IP:Port
            "10.0.0.1:8080",                // Valid IP:Port
            "localhost:22",                 // Valid hostname:Port
            "my-server.com:443",            // Valid domain:Port
            "example.org:80",               // Valid domain:Port
            "server123.domain456.com:8080", // Domain with numbers
            "machine-1.internal:22",        // Domain with hyphen
        ];

        for valid_host in valid_hosts {
            let config = TelegrafConfig {
                iot_host: valid_host.to_string(),
                ..base_config.clone()
            };
            assert!(
                config.validate_iot_host().is_ok(),
                "Host '{}' should be valid",
                valid_host
            );
        }

        // Test invalid host:port formats
        let invalid_hosts = vec![
            "192.168.1.2",       // Missing port
            "192.168.1.2:",      // Empty port
            ":22",               // Empty hostname
            "192.168.1.2:0",     // Invalid port (zero)
            "192.168.1.2:-1",    // Negative port
            "192.168.1.2:abc",   // Non-numeric port
            "192.168.1.2:22:33", // Too many colons
            "192.168..1:22",     // Invalid IP (double dot)
            "192.168.1.300:22",  // Invalid IP (segment > 255)
            "192.11..19.9:22",   // Double dot in IP
        ];

        for invalid_host in invalid_hosts {
            let config = TelegrafConfig {
                iot_host: invalid_host.to_string(),
                ..base_config.clone()
            };
            assert!(
                config.validate_iot_host().is_err(),
                "Host '{}' should be invalid",
                invalid_host
            );
        }
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
