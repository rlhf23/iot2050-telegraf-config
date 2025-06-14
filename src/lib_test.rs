use crate::{backend::ConfigGenerator, TelegrafConfig, discover_xml_files};
use std::path::PathBuf;
use tempfile::TempDir;

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
            listener_files: Vec::new(),
            selected_opcua_nodes: Vec::new(),
            output_format: None,
            include_test_inputs: false,
            include_diagnostics: false,
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
            selected_opcua_nodes: Vec::new(),
            listener_files: Vec::new(),
            output_format: None,
            include_test_inputs: false,
            include_diagnostics: false,
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
            listener_files: Vec::new(),
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
            include_diagnostics: false,
            selected_opcua_nodes: Vec::new(),
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
            selected_opcua_nodes: Vec::new(),
            listener_files: Vec::new(),
            output_format: None,
            include_test_inputs: false,
            include_diagnostics: false,
        };

        let invalid_result = ConfigGenerator::new(invalid_config);
        assert!(invalid_result.is_err());
    }

    #[test]
    fn test_validate_namespace() {
        let config = create_test_config();

        // Valid namespaces
        let valid_namespaces = vec!["0", "1", "2", "10", "100", "999"];
        for namespace in valid_namespaces {
            assert!(
                config.validate_namespace(namespace).is_ok(),
                "Namespace '{}' should be valid",
                namespace
            );
        }

        // Invalid namespaces
        let invalid_namespaces = vec![
            "",           // Empty
            " ",          // Whitespace only
            "  ",         // Multiple whitespace
            "\t",         // Tab
            "\n",         // Newline
            "abc",        // Non-numeric
            "1a",         // Mixed alphanumeric
            "a1",         // Mixed alphanumeric
            "1.0",        // Decimal
            "-1",         // Negative
            "+1",         // Plus sign
            "1 2",        // Space in middle
            "1,2",        // Comma
            "namespace1", // Text with numbers
        ];

        for namespace in invalid_namespaces {
            assert!(
                config.validate_namespace(namespace).is_err(),
                "Namespace '{}' should be invalid",
                namespace
            );
        }
    }

    #[test]
    fn test_validate_interval() {
        let config = create_test_config();

        // Valid intervals
        let valid_intervals = vec![
            "",      // Empty (should be OK as we use defaults)
            " ",     // Whitespace only (should be OK after trim)
            "1",     // Minimum valid
            "100",   // Normal value
            "1000",  // Common value
            "60000", // Large value
        ];

        for interval in valid_intervals {
            assert!(
                config.validate_interval(interval).is_ok(),
                "Interval '{}' should be valid",
                interval
            );
        }

        // Invalid intervals
        let invalid_intervals = vec![
            "0",       // Zero
            "-1",      // Negative
            "abc",     // Non-numeric
            "1.5",     // Decimal
            "1a",      // Mixed alphanumeric
            "a1",      // Mixed alphanumeric
            "+1",      // Plus sign
            "1 2",     // Space in middle
            "1,000",   // Comma
            "1000ms",  // With units
        ];

        for interval in invalid_intervals {
            assert!(
                config.validate_interval(interval).is_err(),
                "Interval '{}' should be invalid",
                interval
            );
        }
    }

    #[test]
    fn test_validate_ip_for_file() {
        let config = create_test_config();

        // Valid IPs
        let valid_ips = vec![
            "192.168.1.1",
            "127.0.0.1",
            "10.0.0.1",
            "192.168.1.1:4840", // With port
        ];

        for ip in valid_ips {
            assert!(
                config.validate_ip_for_file(ip).is_ok(),
                "IP '{}' should be valid",
                ip
            );
        }

        // Invalid IPs
        let invalid_ips = vec![
            "",
            "invalid.ip",
            "192.168.1",
            "192.168.1.300",
            "192.168.1.1:abc", // Invalid port
        ];

        for ip in invalid_ips {
            assert!(
                config.validate_ip_for_file(ip).is_err(),
                "IP '{}' should be invalid",
                ip
            );
        }
    }

    #[test]
    fn test_validate_config_success() {
        let config = create_test_config();
        let mut file_configs = std::collections::HashMap::new();
        
        // Add valid file configurations
        file_configs.insert(
            "file1.xml".to_string(),
            crate::error::XmlFileValidation {
                namespace: "1".to_string(),
                interval_ms: "1000".to_string(),
                ip: "".to_string(), // Use default IP
            },
        );
        
        file_configs.insert(
            "file2.xml".to_string(),
            crate::error::XmlFileValidation {
                namespace: "2".to_string(),
                interval_ms: "2000".to_string(),
                ip: "192.168.1.100".to_string(), // Custom IP
            },
        );

        let result = config.validate_config(&file_configs);
        assert!(result.is_ok(), "Valid config should pass validation");
    }

    #[test]
    fn test_validate_config_duplicate_namespace() {
        let config = create_test_config();
        let mut file_configs = std::collections::HashMap::new();
        
        // Add configurations with duplicate namespace on same IP
        file_configs.insert(
            "file1.xml".to_string(),
            crate::error::XmlFileValidation {
                namespace: "1".to_string(),
                interval_ms: "1000".to_string(),
                ip: "".to_string(), // Use default IP
            },
        );
        
        file_configs.insert(
            "file2.xml".to_string(),
            crate::error::XmlFileValidation {
                namespace: "1".to_string(), // Same namespace
                interval_ms: "2000".to_string(),
                ip: "".to_string(), // Same IP (default)
            },
        );

        let result = config.validate_config(&file_configs);
        assert!(result.is_err(), "Duplicate namespace should fail validation");
        
        let errors = result.unwrap_err();
        assert!(errors.len() > 0, "Should have validation errors");
        
        // Check that the error mentions duplicate namespace
        let error_msg = format!("{:?}", errors[0]);
        assert!(error_msg.contains("Duplicate namespace"), "Error should mention duplicate namespace");
    }

    #[test]
    fn test_validate_config_missing_namespace() {
        let config = create_test_config();
        let mut file_configs = std::collections::HashMap::new();
        
        // Add configuration with missing namespace
        file_configs.insert(
            "file1.xml".to_string(),
            crate::error::XmlFileValidation {
                namespace: "".to_string(), // Empty namespace
                interval_ms: "1000".to_string(),
                ip: "".to_string(),
            },
        );

        let result = config.validate_config(&file_configs);
        assert!(result.is_err(), "Missing namespace should fail validation");
        
        let errors = result.unwrap_err();
        assert!(errors.len() > 0, "Should have validation errors");
        
        // Check that the error mentions missing namespace
        let error_msg = format!("{:?}", errors[0]);
        assert!(error_msg.contains("Missing namespace"), "Error should mention missing namespace");
    }

    #[test]
    fn test_validate_config_invalid_interval() {
        let config = create_test_config();
        let mut file_configs = std::collections::HashMap::new();
        
        // Add configuration with invalid interval
        file_configs.insert(
            "file1.xml".to_string(),
            crate::error::XmlFileValidation {
                namespace: "1".to_string(),
                interval_ms: "invalid".to_string(), // Invalid interval
                ip: "".to_string(),
            },
        );

        let result = config.validate_config(&file_configs);
        assert!(result.is_err(), "Invalid interval should fail validation");
        
        let errors = result.unwrap_err();
        assert!(errors.len() > 0, "Should have validation errors");
    }

    #[test]
    fn test_validate_config_invalid_custom_ip() {
        let config = create_test_config();
        let mut file_configs = std::collections::HashMap::new();
        
        // Add configuration with invalid custom IP
        file_configs.insert(
            "file1.xml".to_string(),
            crate::error::XmlFileValidation {
                namespace: "1".to_string(),
                interval_ms: "1000".to_string(),
                ip: "invalid.ip".to_string(), // Invalid IP
            },
        );

        let result = config.validate_config(&file_configs);
        assert!(result.is_err(), "Invalid custom IP should fail validation");
        
        let errors = result.unwrap_err();
        assert!(errors.len() > 0, "Should have validation errors");
    }

    #[test]
    fn test_discover_xml_files() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path().to_path_buf();

        // Create some test files
        let xml_file1 = temp_path.join("test1.xml");
        let xml_file2 = temp_path.join("test2.xml");
        let txt_file = temp_path.join("test.txt");
        let no_ext_file = temp_path.join("noextension");

        // Write some content to the files
        std::fs::write(&xml_file1, "<xml>test1</xml>").expect("Failed to write test file");
        std::fs::write(&xml_file2, "<xml>test2</xml>").expect("Failed to write test file");
        std::fs::write(&txt_file, "not xml").expect("Failed to write test file");
        std::fs::write(&no_ext_file, "no extension").expect("Failed to write test file");

        // Test discovering XML files
        let xml_files = discover_xml_files(&temp_path);

        // Should find exactly 2 XML files
        assert_eq!(xml_files.len(), 2, "Should find exactly 2 XML files");

        // Convert to just filenames for easier comparison
        let mut filenames: Vec<String> = xml_files
            .iter()
            .map(|path| {
                std::path::Path::new(path)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        filenames.sort();

        assert_eq!(filenames, vec!["test1.xml", "test2.xml"]);
    }

    #[test]
    fn test_discover_xml_files_empty_directory() {
        // Create an empty temporary directory
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path().to_path_buf();

        // Test discovering XML files in empty directory
        let xml_files = discover_xml_files(&temp_path);

        // Should find no files
        assert_eq!(xml_files.len(), 0, "Should find no XML files in empty directory");
    }

    #[test]
    fn test_discover_xml_files_nonexistent_directory() {
        // Use a non-existent directory path
        let nonexistent_path = PathBuf::from("/this/path/does/not/exist");

        // Test discovering XML files - should fall back to current directory
        // This test verifies the fallback behavior without failing
        let xml_files = discover_xml_files(&nonexistent_path);

        // Should not panic and return a vector (may be empty or contain files from current dir)
        // Just verify it returns a valid vector without panicking
        let _ = xml_files.len();
    }

    #[test]
    fn test_discover_xml_files_case_sensitivity() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_path = temp_dir.path().to_path_buf();

        // Create files with different case extensions
        let xml_lower = temp_path.join("test.xml");
        let xml_upper = temp_path.join("test.XML");
        let xml_mixed = temp_path.join("test.Xml");

        std::fs::write(&xml_lower, "<xml>test</xml>").expect("Failed to write test file");
        // Only create uppercase and mixed case files on case-sensitive filesystems
        if cfg!(unix) {
            std::fs::write(&xml_upper, "<xml>test</xml>").expect("Failed to write test file");
            std::fs::write(&xml_mixed, "<xml>test</xml>").expect("Failed to write test file");
        }

        let xml_files = discover_xml_files(&temp_path);

        // On case-sensitive systems (Unix), should only find .xml (lowercase)
        // On case-insensitive systems (Windows), behavior may vary
        if cfg!(unix) {
            assert_eq!(xml_files.len(), 1, "Should only find lowercase .xml files on Unix");
            assert!(xml_files[0].ends_with("test.xml"), "Should find the lowercase .xml file");
        } else {
            // On Windows, just verify we get at least one file
            assert!(xml_files.len() >= 1, "Should find at least one XML file");
        }
    }

    // Helper function to create a test config
    fn create_test_config() -> TelegrafConfig {
        TelegrafConfig {
            folder: PathBuf::new(),
            ip: "192.168.1.1".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            iot_host: "192.168.1.2:22".to_string(),
            iot_username: "iot_user".to_string(),
            iot_password: "iot_pass".to_string(),
            listener_files: Vec::new(),
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
            selected_opcua_nodes: Vec::new(),
        }
    }
}
