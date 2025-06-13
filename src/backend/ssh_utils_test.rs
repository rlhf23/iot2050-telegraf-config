use crate::backend::ssh_utils::{self, ServiceType, SshConfig};
use crate::error::TelegrafError;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    // Helper function to create a test file with content
    fn create_test_file(dir: &PathBuf, filename: &str, content: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    #[test]
    fn test_ssh_config_default() {
        let config = SshConfig::default();
        assert_eq!(
            config.connect_timeout, 3,
            "Default connect timeout should be 3 seconds"
        );
        assert_eq!(
            config.operation_timeout, 60,
            "Default operation timeout should be 60 seconds"
        );
        assert_eq!(
            config.stream_timeout, 15,
            "Default stream timeout should be 15 seconds"
        );
    }

    #[test]
    fn test_ssh_config_clone() {
        let config1 = SshConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.connect_timeout, config2.connect_timeout);
        assert_eq!(config1.operation_timeout, config2.operation_timeout);
        assert_eq!(config1.stream_timeout, config2.stream_timeout);
    }

    #[test]
    fn test_ssh_config_custom() {
        let config = SshConfig {
            connect_timeout: 5,
            operation_timeout: 120,
            stream_timeout: 45,
        };
        assert_eq!(config.connect_timeout, 5);
        assert_eq!(config.operation_timeout, 120);
        assert_eq!(config.stream_timeout, 45);
    }

    #[test]
    fn test_service_type_debug() {
        let influx = ServiceType::InfluxDB;
        let prometheus = ServiceType::Prometheus;

        // Ensure Debug trait is implemented
        assert!(format!("{:?}", influx).contains("InfluxDB"));
        assert!(format!("{:?}", prometheus).contains("Prometheus"));
    }

    #[test]
    fn test_service_type_clone_copy() {
        let influx1 = ServiceType::InfluxDB;
        let influx2 = influx1; // Copy trait
        let influx3 = influx1.clone(); // Clone trait

        // All should be the same
        assert!(matches!(influx1, ServiceType::InfluxDB));
        assert!(matches!(influx2, ServiceType::InfluxDB));
        assert!(matches!(influx3, ServiceType::InfluxDB));
    }

    // Test host validation logic
    #[test]
    fn test_validate_host_format() {
        // Test valid host format
        let validate_host_format = ssh_utils::validate_host_format_exposed_for_testing;
        
        // Valid host:port format
        assert!(validate_host_format("192.168.1.1:22").is_ok());
        assert!(validate_host_format("localhost:22").is_ok());
        assert!(validate_host_format("example.com:8022").is_ok());
        
        // Missing port
        let result = validate_host_format("192.168.1.1");
        assert!(result.is_err());
        match result {
            Err(TelegrafError::HostFormatError(msg)) => {
                assert!(msg.contains("Missing port specification"));
            }
            _ => panic!("Expected HostFormatError for missing port"),
        }
        
        // Invalid port (not a number)
        let result = validate_host_format("192.168.1.1:abc");
        assert!(result.is_err());
        match result {
            Err(TelegrafError::HostFormatError(msg)) => {
                assert!(msg.contains("Invalid port"));
            }
            _ => panic!("Expected HostFormatError for invalid port"),
        }
        
        // Too many colons
        let result = validate_host_format("192.168.1.1:22:33");
        assert!(result.is_err());
        match result {
            Err(TelegrafError::HostFormatError(msg)) => {
                assert!(msg.contains("Invalid host format"));
            }
            _ => panic!("Expected HostFormatError for too many colons"),
        }
        
        // Empty host
        let result = validate_host_format("");
        assert!(result.is_err());
        
        // Port zero (invalid)
        let result = validate_host_format("192.168.1.1:0");
        assert!(result.is_err());
        match result {
            Err(TelegrafError::HostFormatError(msg)) => {
                assert!(msg.contains("Port must be a number between 1-65535"));
            }
            _ => panic!("Expected HostFormatError for port zero"),
        }
    }

    // Test host validation logic (without actual network connections)
    mod host_validation_tests {
        use super::*;

        #[test]
        fn test_check_service_status_prometheus_not_implemented() {
            // Test that Prometheus service check returns appropriate status
            let result = ssh_utils::check_service_status(
                "127.0.0.1:1", // Invalid host will fail quickly
                "user",
                "pass",
                "http://localhost:9090",
                ServiceType::Prometheus,
                1,
            );

            // Should return Ok with false status and a message
            match result {
                Ok((false, msg)) => {
                    assert!(msg.contains("not yet implemented"), "Expected 'not yet implemented' message");
                },
                Ok((true, _)) => panic!("Expected service check to fail"),
                Err(_) => panic!("Expected Ok with false status, got error"),
            }
        }

        #[test]
        fn test_check_service_status_influxdb_delegates() {
            // Test that InfluxDB service check delegates to check_influxdb_status
            let result = ssh_utils::check_service_status(
                "127.0.0.1:1", // Invalid host will fail connection
                "user",
                "pass",
                "http://localhost:8086",
                ServiceType::InfluxDB,
                1,
            );

            // Should return an error due to connection failure
            // The actual implementation may return Ok((false, message)) or Err depending on the error case
            match result {
                Ok((false, _)) => (), // Accept Ok with false status
                Err(_) => (),         // Also accept error for connection failures
                Ok((true, _)) => panic!("Expected service check to fail or return error"),
            }
        }
    }

    // Test file operations (without actual SSH)
    mod file_operation_tests {
        use super::*;
        use crate::error::TelegrafError;

        #[test]
        fn test_send_file_over_ssh_missing_file() {
            let nonexistent_path = std::path::Path::new("/tmp/nonexistent_file_test_12345");

            // Note: send_file_over_ssh tries SSH connection BEFORE reading file,
            // so with invalid host it fails at connection stage, not file stage
            let result = ssh_utils::send_file_over_ssh(
                &nonexistent_path,
                "/remote/path",
                "127.0.0.1:1", // Invalid port will cause connection failure first
                "user",
                "pass",
            );

            assert!(result.is_err());
            // With invalid host, expect SSH/connection error, not file error
            match result.unwrap_err() {
                TelegrafError::SshError(_) | TelegrafError::HostFormatError(_) => (), // Expected
                _ => panic!("Expected SSH connection error for invalid host"),
            }
        }

        #[test]
        fn test_send_file_over_ssh_with_valid_file() {
            let dir = tempdir().unwrap();
            let file_path =
                create_test_file(&dir.path().to_path_buf(), "test.conf", "test content");

            // This should fail at SSH connection stage, not file reading stage
            let result = ssh_utils::send_file_over_ssh(
                &file_path,
                "/remote/path",
                "127.0.0.1:1", // Invalid port
                "user",
                "pass",
            );

            assert!(result.is_err());
            // Should be SSH connection error, not file error
            match result.unwrap_err() {
                TelegrafError::SshError(_) | TelegrafError::HostFormatError(_) => (), // Expected
                TelegrafError::IoError(_) => panic!("Should not be IoError since file exists"),
                _ => (), // Other error types are acceptable
            }
        }
    }

    // Test progress reporting functionality
    mod progress_reporting_tests {
        use super::*;
        use std::sync::mpsc;

        #[test]
        fn test_send_and_restart_telegraf_with_progress() {
            let dir = tempdir().unwrap();
            let file_path = create_test_file(&dir.path().to_path_buf(), "telegraf.conf", "test content");
            
            // Create a channel to receive progress updates
            let (sender, receiver) = mpsc::channel();
            
            // Call the function with progress reporting
            let result = ssh_utils::send_and_restart_telegraf_with_progress(
                &file_path,
                "/etc/telegraf/telegraf.conf",
                "127.0.0.1:1", // Invalid host that will fail
                "user",
                "pass",
                sender
            );
            
            // The function should fail due to invalid host
            assert!(result.is_err());
            
            // But we should have received at least one progress message
            let messages: Vec<String> = receiver.try_iter().collect();
            assert!(!messages.is_empty(), "Expected at least one progress message");
            
            // First message should be about sending the file
            if !messages.is_empty() {
                assert!(messages[0].contains("Sending configuration file"), 
                       "Expected first message to be about sending the file");
            }
        }
    }

    // We can't easily test actual SSH connections in unit tests
    // So we'll test the error handling for missing files and invalid paths

    #[test]
    fn test_send_file_over_ssh_invalid_host() {
        let dir = tempdir().unwrap();
        let file_path = create_test_file(&dir.path().to_path_buf(), "test.conf", "test content");

        // Use an invalid host that will fail to connect (port 1 is unlikely to have SSH)
        let result = ssh_utils::send_file_over_ssh(
            &file_path,
            "/remote/path",
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_backup_influxdb_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::backup_influxdb(
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
            Some("token"),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_backup_grafana_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::backup_grafana_config(
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_get_telegraf_status_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::get_telegraf_status(
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_get_telegraf_logs_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::get_telegraf_logs(
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
            10,
        );

        assert!(result.is_err());
    }
    
    #[test]
    fn test_execute_command_over_ssh_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::execute_command_over_ssh(
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
            "echo test"
        );

        assert!(result.is_err());
    }
    
    #[test]
    fn test_copy_directory_over_ssh_invalid_host() {
        // Test with invalid host
        let result = ssh_utils::copy_directory_over_ssh(
            "127.0.0.1:1", // Invalid port
            "user",
            "pass",
            "/remote/dir",
            "/local/dir"
        );

        assert!(result.is_err());
    }
}
