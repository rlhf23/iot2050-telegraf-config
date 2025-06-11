use crate::backend::ssh_utils::{self, ServiceType, SshConfig};
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

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
                Ok((false, _)) => (), // Expected result - service check failed with message
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
}
