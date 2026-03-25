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
            custom_ip: None,
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
        let iot_host = request
            .iot_host
            .unwrap_or_else(|| "localhost:22".to_string());
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
            custom_ip: None,
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
                custom_ip: None,
            },
            FileConfig {
                filename: "file2.xml".to_string(),
                namespace: "2".to_string(),
                interval_ms: 500,
                use_listener: true,
                custom_ip: Some("192.168.1.55".to_string()),
            },
        ];

        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0].namespace, "1");
        assert_eq!(configs[1].use_listener, true);
    }

    #[test]
    fn test_config_generator_accepts_localhost() {
        // This is the REAL test - verify ConfigGenerator accepts localhost:22
        use sie_generate_config::{backend::ConfigGenerator, TelegrafConfig};
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
            include_opcua_diagnostics: false,
            selected_opcua_nodes: vec![],
            use_source_timestamp: false,
        };

        // This should NOT panic - localhost:22 is valid
        let result = ConfigGenerator::new(config);
        assert!(
            result.is_ok(),
            "ConfigGenerator should accept localhost:22 as valid iot_host"
        );
    }

    #[test]
    fn test_config_generator_with_ip_defaults() {
        // Test that the defaults used by the API pass validation
        use sie_generate_config::{backend::ConfigGenerator, TelegrafConfig};
        use std::path::PathBuf;

        let config = TelegrafConfig {
            folder: PathBuf::from("/tmp"),
            ip: "127.0.0.1".to_string(), // Default OPC-UA IP
            username: "".to_string(),
            password: "".to_string(),
            iot_host: "127.0.0.1:22".to_string(), // Default IoT host
            iot_username: "user".to_string(),
            iot_password: "pass".to_string(),
            listener_files: vec![],
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
            include_opcua_diagnostics: false,
            selected_opcua_nodes: vec![],
            use_source_timestamp: false,
        };

        // This should pass - verifies our defaults are valid
        let result = ConfigGenerator::new(config);
        assert!(
            result.is_ok(),
            "ConfigGenerator should accept default IP addresses: {}",
            result.err().map(|e| e.to_string()).unwrap_or_default()
        );
    }

    #[test]
    fn test_config_generator_rejects_invalid_opcua_ip() {
        // Test that invalid OPC-UA IPs are rejected
        use sie_generate_config::{backend::ConfigGenerator, TelegrafConfig};
        use std::path::PathBuf;

        let config = TelegrafConfig {
            folder: PathBuf::from("/tmp"),
            ip: "localhost".to_string(), // Invalid - not an IP address
            username: "admin".to_string(),
            password: "password".to_string(),
            iot_host: "127.0.0.1:22".to_string(),
            iot_username: "user".to_string(),
            iot_password: "pass".to_string(),
            listener_files: vec![],
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
            include_opcua_diagnostics: false,
            selected_opcua_nodes: vec![],
            use_source_timestamp: false,
        };

        // This should FAIL - localhost is not a valid IP format
        let result = ConfigGenerator::new(config);
        assert!(
            result.is_err(),
            "ConfigGenerator should reject 'localhost' as OPC-UA IP"
        );

        let error_msg = result.err().unwrap().to_string();
        assert!(
            error_msg.contains("4 parts") || error_msg.contains("IP"),
            "Error should mention IP validation: {}",
            error_msg
        );
    }

    #[test]
    fn test_config_generator_with_file_configs() {
        // Test that ConfigGenerator accepts file configs and generates successfully
        use sie_generate_config::{backend::ConfigGenerator, TelegrafConfig};
        use std::fs;

        // Create a temp directory for the test
        let temp_dir = std::env::temp_dir().join(format!("test_config_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        // Create a simple test XML file with valid OPC-UA nodes
        let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<UANodeSet xmlns="http://opcfoundation.org/UA/2011/03/UANodeSet.xsd">
    <NamespaceUris>
        <Uri>http://test.example.com</Uri>
    </NamespaceUris>
    <UAVariable NodeId="ns=2;i=1001" BrowseName="2:Temperature" DataType="Double">
        <DisplayName>Temperature</DisplayName>
        <Description>Test temperature variable</Description>
    </UAVariable>
</UANodeSet>"#;
        fs::write(temp_dir.join("test.xml"), xml_content).unwrap();

        let config = TelegrafConfig {
            folder: temp_dir.clone(),
            ip: "127.0.0.1:4840".to_string(),
            username: "admin".to_string(),
            password: "password".to_string(),
            iot_host: "127.0.0.1:22".to_string(),
            iot_username: "user".to_string(),
            iot_password: "pass".to_string(),
            listener_files: vec![],
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
            include_opcua_diagnostics: false,
            selected_opcua_nodes: vec![],
            use_source_timestamp: false,
        };

        // Create ConfigGenerator
        let mut generator = ConfigGenerator::new(config).unwrap();

        // Get full path to XML file
        let xml_path = temp_dir.join("test.xml");
        let xml_path_str = xml_path.to_string_lossy().to_string();

        // Set file config (simulating what the API does)
        generator.set_file_config(xml_path_str.clone(), "2".to_string(), 1000, None);

        // Generate the config
        let xml_files = vec![xml_path_str];
        let listener_files: Vec<String> = vec![];
        let result = generator.generate_config(&xml_files, &listener_files);

        // Verify it succeeded
        assert!(
            result.is_ok(),
            "Config generation should succeed: {:?}",
            result.err()
        );

        // Verify telegraf.conf was created
        let config_path = temp_dir.join("telegraf.conf");
        assert!(config_path.exists(), "telegraf.conf should be created");

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_config_generator_with_null_opcua_ip() {
        // Test that when opcua_ip is null, per-file IPs default to 127.0.0.1
        use sie_generate_config::{backend::ConfigGenerator, TelegrafConfig};
        use std::fs;

        let temp_dir = std::env::temp_dir().join(format!("test_null_ip_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<UANodeSet xmlns="http://opcfoundation.org/UA/2011/03/UANodeSet.xsd">
    <UAVariable NodeId="ns=2;i=1001" BrowseName="2:TestVar" DataType="Double">
        <DisplayName>TestVar</DisplayName>
    </UAVariable>
</UANodeSet>"#;
        fs::write(temp_dir.join("test.xml"), xml_content).unwrap();

        // Use 127.0.0.1 as the default (what the API uses when opcua_ip is null)
        let config = TelegrafConfig {
            folder: temp_dir.clone(),
            ip: "127.0.0.1".to_string(), // This is the default when null
            username: "".to_string(),
            password: "".to_string(),
            iot_host: "127.0.0.1:22".to_string(),
            iot_username: "user".to_string(),
            iot_password: "pass".to_string(),
            listener_files: vec![],
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
            include_opcua_diagnostics: false,
            selected_opcua_nodes: vec![],
            use_source_timestamp: false,
        };

        let mut generator = ConfigGenerator::new(config).unwrap();

        let xml_path = temp_dir.join("test.xml");
        let xml_path_str = xml_path.to_string_lossy().to_string();

        // Set file config with 127.0.0.1 as custom IP (the default)
        generator.set_file_config(
            xml_path_str.clone(),
            "2".to_string(),
            1000,
            Some("127.0.0.1".to_string()), // This should NOT be "localhost"
        );

        let result = generator.generate_config(&[xml_path_str], &[]);

        fs::remove_dir_all(&temp_dir).ok();

        assert!(
            result.is_ok(),
            "Config generation with 127.0.0.1 default should succeed: {:?}",
            result.err()
        );
    }
}
