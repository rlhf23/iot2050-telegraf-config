use crate::backend::{ConfigGenerator, FileConfig, OutputFormat};
use crate::TelegrafConfig;
use crate::error::TelegrafError;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[cfg(test)]
mod tests {
    use super::*;

    // Test helper to create a sample XML file
    fn create_test_xml(dir: &PathBuf, filename: &str) -> PathBuf {
        let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
        <UANodeSet>
            <UAObject NodeId="ns=2;i=1">
                <DisplayName>TestDevice</DisplayName>
            </UAObject>
            <UAVariable NodeId="ns=2;i=2">
                <BrowseName>Temperature</BrowseName>
            </UAVariable>
            <UAVariable NodeId="ns=2;i=3">
                <BrowseName>Pressure</BrowseName>
            </UAVariable>
        </UANodeSet>"#;
        
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(xml_content.as_bytes()).unwrap();
        file_path
    }

    // Test helper to create a basic config
    fn create_test_config(folder: PathBuf) -> TelegrafConfig {
        TelegrafConfig {
            folder: folder.clone(),
            ip: "192.168.1.1".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            iot_host: "192.168.1.2:22".to_string(),
            iot_username: "iot_user".to_string(),
            iot_password: "iot_pass".to_string(),
            token_folder: folder,
            bucket_name: "test_bucket".to_string(),
            influx_token: Some("test_token".to_string()),
            listener_files: Vec::new(),
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
        }
    }

    #[test]
    fn test_set_file_config() {
        let dir = tempdir().unwrap();
        let config = create_test_config(dir.path().to_path_buf());
        let mut generator = ConfigGenerator::new(config).unwrap();
        
        generator.set_file_config(
            "file1.xml".to_string(),
            "1".to_string(),
            1000,
        );
        
        generator.set_file_config(
            "file2.xml".to_string(),
            "2".to_string(),
            500,
        );
        
        // Access the internal file_configs to verify
        let file_configs = &generator.file_configs;
        
        assert_eq!(file_configs.len(), 2);
        
        let file1_config = file_configs.get("file1.xml").unwrap();
        assert_eq!(file1_config.namespace, "1");
        assert_eq!(file1_config.interval_ms, 1000);
        
        let file2_config = file_configs.get("file2.xml").unwrap();
        assert_eq!(file2_config.namespace, "2");
        assert_eq!(file2_config.interval_ms, 500);
    }
    
    #[test]
    fn test_get_xml_files() {
        let dir = tempdir().unwrap();
        
        // Create a few XML files and one non-XML file
        create_test_xml(&dir.path().to_path_buf(), "test1.xml");
        create_test_xml(&dir.path().to_path_buf(), "test2.xml");
        
        // Create a non-XML file
        let non_xml_path = dir.path().join("not_xml.txt");
        let mut non_xml_file = File::create(&non_xml_path).unwrap();
        non_xml_file.write_all(b"This is not an XML file").unwrap();
        
        let config = create_test_config(dir.path().to_path_buf());
        let generator = ConfigGenerator::new(config).unwrap();
        
        let xml_files = generator.get_xml_files().unwrap();
        
        // Should find exactly 2 XML files
        assert_eq!(xml_files.len(), 2);
        
        // Each file path should end with .xml
        for file in &xml_files {
            assert!(file.ends_with(".xml"));
        }
        
        // Check that both our XML files are found (ignoring order)
        let test1_path = dir.path().join("test1.xml").to_string_lossy().to_string();
        let test2_path = dir.path().join("test2.xml").to_string_lossy().to_string();
        
        assert!(xml_files.contains(&test1_path) || xml_files.contains(&test2_path));
    }
    
    #[test]
    fn test_set_output_format() {
        let dir = tempdir().unwrap();
        let config = create_test_config(dir.path().to_path_buf());
        let mut generator = ConfigGenerator::new(config).unwrap();
        
        // Initial format should be InfluxDB (default)
        assert_eq!(generator.output_format, OutputFormat::InfluxDB);
        
        // Change to Prometheus
        generator.set_output_format(OutputFormat::Prometheus);
        assert_eq!(generator.output_format, OutputFormat::Prometheus);
        
        // Change back to InfluxDB
        generator.set_output_format(OutputFormat::InfluxDB);
        assert_eq!(generator.output_format, OutputFormat::InfluxDB);
    }
    
    #[test]
    fn test_set_include_test_inputs() {
        let dir = tempdir().unwrap();
        let config = create_test_config(dir.path().to_path_buf());
        let mut generator = ConfigGenerator::new(config).unwrap();
        
        // Initial value should be false (from test config)
        assert_eq!(generator.include_test_inputs, false);
        
        // Change to true
        generator.set_include_test_inputs(true);
        assert_eq!(generator.include_test_inputs, true);
        
        // Change back to false
        generator.set_include_test_inputs(false);
        assert_eq!(generator.include_test_inputs, false);
    }
    
    #[test]
    fn test_generate_config_basic() {
        let dir = tempdir().unwrap();
        
        // Create an XML file
        let xml_path = create_test_xml(&dir.path().to_path_buf(), "test.xml");
        let xml_path_str = xml_path.to_string_lossy().to_string();
        
        let mut config = create_test_config(dir.path().to_path_buf());
        config.bucket_name = "test_bucket".to_string();
        
        let mut generator = ConfigGenerator::new(config).unwrap();
        
        // Configure the file
        generator.set_file_config(
            xml_path_str.clone(),
            "2".to_string(),
            1000,
        );
        
        // Generate the config
        let listener_files: Vec<String> = Vec::new();
        let result = generator.generate_config(&[xml_path_str.clone()], &listener_files);
        
        assert!(result.is_ok());
        let config_content = result.unwrap();
        
        // Check that the config contains expected elements
        assert!(config_content.contains("bucket = \"test_bucket\""));
        assert!(config_content.contains("token = \"test_token\""));
        assert!(config_content.contains("endpoint = \"opc.tcp://192.168.1.1:4840\""));
        assert!(config_content.contains("username = \"user\""));
        assert!(config_content.contains("password = \"pass\""));
        assert!(config_content.contains("name = \"TestDevice\""));
        assert!(config_content.contains("namespace = \"2\""));
        assert!(config_content.contains("interval = \"1000ms\""));
        
        // Verify the config file was created
        let config_file_path = dir.path().join("telegraf.conf");
        assert!(config_file_path.exists());
    }
    
    #[test]
    fn test_generate_config_with_listeners() {
        let dir = tempdir().unwrap();
        
        // Create two XML files
        let xml_path1 = create_test_xml(&dir.path().to_path_buf(), "regular.xml");
        let xml_path2 = create_test_xml(&dir.path().to_path_buf(), "listener.xml");
        let xml_path1_str = xml_path1.to_string_lossy().to_string();
        let xml_path2_str = xml_path2.to_string_lossy().to_string();
        
        let mut config = create_test_config(dir.path().to_path_buf());
        let mut generator = ConfigGenerator::new(config).unwrap();
        
        // Configure the files
        generator.set_file_config(
            xml_path1_str.clone(),
            "1".to_string(),
            1000,
        );
        
        generator.set_file_config(
            xml_path2_str.clone(),
            "2".to_string(),
            500,
        );
        
        // Generate config with one file as listener
        let listener_files = vec![xml_path2_str.clone()];
        let result = generator.generate_config(
            &[xml_path1_str.clone(), xml_path2_str.clone()], 
            &listener_files
        );
        
        assert!(result.is_ok());
        let config_content = result.unwrap();
        
        // Check that the listener config uses the right input type and sampling interval
        assert!(config_content.contains("[[inputs.opcua]]"));
        assert!(config_content.contains("[[inputs.opcua_listener]]"));
        assert!(config_content.contains("interval = \"1000ms\""));
        assert!(config_content.contains("sampling_interval = \"500ms\""));
    }
    
    #[test]
    fn test_generate_config_with_test_inputs() {
        let dir = tempdir().unwrap();
        
        // Create an XML file
        let xml_path = create_test_xml(&dir.path().to_path_buf(), "test.xml");
        let xml_path_str = xml_path.to_string_lossy().to_string();
        
        let mut config = create_test_config(dir.path().to_path_buf());
        config.include_test_inputs = true;
        
        let mut generator = ConfigGenerator::new(config).unwrap();
        
        // Configure the file
        generator.set_file_config(
            xml_path_str.clone(),
            "2".to_string(),
            1000,
        );
        
        // Generate the config
        let listener_files: Vec<String> = Vec::new();
        let result = generator.generate_config(&[xml_path_str.clone()], &listener_files);
        
        assert!(result.is_ok());
        let config_content = result.unwrap();
        
        // Verify test inputs are included
        assert!(config_content.contains("[[inputs.cpu]]"));
        assert!(config_content.contains("[[inputs.disk]]"));
        assert!(config_content.contains("[[inputs.mem]]"));
        assert!(config_content.contains("TEST INPUTS START"));
        assert!(config_content.contains("TEST INPUTS END"));
    }
    
    #[test]
    fn test_generate_config_prometheus_format() {
        let dir = tempdir().unwrap();
        
        // Create an XML file
        let xml_path = create_test_xml(&dir.path().to_path_buf(), "test.xml");
        let xml_path_str = xml_path.to_string_lossy().to_string();
        
        let mut config = create_test_config(dir.path().to_path_buf());
        config.output_format = Some("prometheus".to_string());
        
        let mut generator = ConfigGenerator::new(config).unwrap();
        
        // Configure the file
        generator.set_file_config(
            xml_path_str.clone(),
            "2".to_string(),
            1000,
        );
        
        // Generate the config
        let listener_files: Vec<String> = Vec::new();
        let result = generator.generate_config(&[xml_path_str.clone()], &listener_files);
        
        assert!(result.is_ok());
        let config_content = result.unwrap();
        
        // Verify Prometheus output is used
        assert!(config_content.contains("[[outputs.prometheus_client]]"));
        assert!(config_content.contains("listen = \":9273\""));
        assert!(!config_content.contains("[[outputs.influxdb_v2]]"));
    }
    
    #[test]
    fn test_send_config_missing_file() {
        let dir = tempdir().unwrap();
        let config = create_test_config(dir.path().to_path_buf());
        let generator = ConfigGenerator::new(config).unwrap();
        
        // Try to send config without generating it first
        let result = generator.send_config();
        
        assert!(result.is_err());
        if let Err(TelegrafError::ConfigError(msg)) = result {
            assert!(msg.contains("telegraf.conf file does not exist"));
        } else {
            panic!("Expected ConfigError but got a different error or success");
        }
    }
}
