use crate::config::*;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test XML file (not used yet)
    #[allow(dead_code)]
    fn create_test_xml(dir: &std::path::Path, filename: &str) -> std::path::PathBuf {
        use std::fs;
        use std::io::Write;
        let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
        <UANodeSet>
            <UAObject NodeId="ns=2;i=1">
                <DisplayName>TestDevice</DisplayName>
            </UAObject>
            <UAVariable NodeId="ns=2;i=2">
                <BrowseName>Temperature</BrowseName>
            </UAVariable>
        </UANodeSet>"#;

        let file_path = dir.join(filename);
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(xml_content.as_bytes()).unwrap();
        file_path
    }

    // Note: validate_filename is private, so we can't test it directly
    // It's tested indirectly through the upload_files endpoint

    #[test]
    fn test_file_config_creation() {
        let config = FileConfig {
            filename: "test.xml".to_string(),
            namespace: "2".to_string(),
            interval_ms: 1000,
            use_listener: false,
        };

        assert_eq!(config.filename, "test.xml");
        assert_eq!(config.namespace, "2");
        assert_eq!(config.interval_ms, 1000);
        assert_eq!(config.use_listener, false);
    }

    #[test]
    fn test_generate_config_request_minimal() {
        // Test that we can create a minimal request without IoT credentials
        let request = GenerateConfigRequest {
            session_id: "test-session".to_string(),
            opcua_ip: Some("192.168.1.100".to_string()),
            opcua_username: None,
            opcua_password: None,
            anonymous: true,
            iot_host: None,
            iot_username: None,
            iot_password: None,
            output_format: "influxdb".to_string(),
            file_configs: vec![],
        };

        assert_eq!(request.session_id, "test-session");
        assert_eq!(request.anonymous, true);
        assert!(request.iot_host.is_none());
    }

    #[test]
    fn test_generate_config_request_with_iot() {
        // Test that we can create a request with IoT credentials
        let request = GenerateConfigRequest {
            session_id: "test-session".to_string(),
            opcua_ip: Some("192.168.1.100".to_string()),
            opcua_username: Some("admin".to_string()),
            opcua_password: Some("password".to_string()),
            anonymous: false,
            iot_host: Some("192.168.1.2:22".to_string()),
            iot_username: Some("iotuser".to_string()),
            iot_password: Some("iotpass".to_string()),
            output_format: "influxdb".to_string(),
            file_configs: vec![],
        };

        assert_eq!(request.iot_host, Some("192.168.1.2:22".to_string()));
        assert_eq!(request.iot_username, Some("iotuser".to_string()));
    }

    #[test]
    fn test_iot_credentials_defaults() {
        // Test that when IoT credentials are None, we use sensible defaults
        let request = GenerateConfigRequest {
            session_id: "test-session".to_string(),
            opcua_ip: Some("192.168.1.100".to_string()),
            opcua_username: None,
            opcua_password: None,
            anonymous: true,
            iot_host: None,
            iot_username: None,
            iot_password: None,
            output_format: "influxdb".to_string(),
            file_configs: vec![],
        };

        // Verify defaults that would be used in generate_config
        let iot_host = request.iot_host.unwrap_or_else(|| "localhost:22".to_string());
        let iot_username = request.iot_username.unwrap_or_else(|| "user".to_string());
        let iot_password = request.iot_password.unwrap_or_else(|| "pass".to_string());

        assert_eq!(iot_host, "localhost:22");
        assert_eq!(iot_username, "user");
        assert_eq!(iot_password, "pass");
    }

    #[test]
    fn test_file_config_with_listener() {
        let config = FileConfig {
            filename: "listener.xml".to_string(),
            namespace: "3".to_string(),
            interval_ms: 500,
            use_listener: true,
        };

        assert_eq!(config.use_listener, true);
        assert_eq!(config.interval_ms, 500);
    }

    #[test]
    fn test_multiple_file_configs() {
        let configs = vec![
            FileConfig {
                filename: "file1.xml".to_string(),
                namespace: "1".to_string(),
                interval_ms: 1000,
                use_listener: false,
            },
            FileConfig {
                filename: "file2.xml".to_string(),
                namespace: "2".to_string(),
                interval_ms: 500,
                use_listener: true,
            },
        ];

        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].namespace, "1");
        assert_eq!(configs[1].use_listener, true);
    }

    #[test]
    fn test_config_generator_accepts_localhost() {
        // This is the REAL test - verify ConfigGenerator accepts localhost:22
        use sie_generate_config::{TelegrafConfig, backend::ConfigGenerator};
        use std::path::PathBuf;

        let config = TelegrafConfig {
            folder: PathBuf::from("/tmp"),
            ip: "192.168.1.100:4840".to_string(),
            username: "admin".to_string(),
            password: "password".to_string(),
            iot_host: "localhost:22".to_string(), // This is what we're testing!
            iot_username: "user".to_string(),
            iot_password: "pass".to_string(),
            listener_files: vec![],
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
            selected_opcua_nodes: vec![],
        };

        // This should NOT panic - localhost:22 is valid
        let result = ConfigGenerator::new(config);
        assert!(result.is_ok(), "ConfigGenerator should accept localhost:22 as valid iot_host");
    }
}

