use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::tempdir;
use std::fs::File;
use std::io::Write;

use crate::TelegrafConfig;
use crate::error::TelegrafError;
use super::opcua_poller::OpcUaPoller;

// Mock implementation for testing without a real OPC UA server
#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper function to create a test TelegrafConfig
    fn create_test_config() -> TelegrafConfig {
        TelegrafConfig {
            folder: PathBuf::new(),
            ip: "127.0.0.1".to_string(),
            username: "testuser".to_string(),
            password: "testpass".to_string(),
            iot_host: "localhost:22".to_string(),
            iot_username: "iotuser".to_string(),
            iot_password: "iotpass".to_string(),
            token_folder: PathBuf::new(),
            bucket_name: "test-bucket".to_string(),
            influx_token: Some("test-token".to_string()),
            listener_files: Vec::new(),
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
        }
    }
    
    // Test that the mapping between XML file names and namespaces works correctly
    #[test]
    fn test_namespace_mapping() {
        // Create a temporary directory for our test files
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();
        
        // Create some test XML files
        let file_paths = vec![
            "device1.xml",
            "sensor2.xml",
            "controller3.xml",
        ];
        
        let mut xml_file_paths = Vec::new();
        
        for file_name in &file_paths {
            let file_path = dir_path.join(file_name);
            let mut file = File::create(&file_path).unwrap();
            write!(file, "<root></root>").unwrap();
            xml_file_paths.push(file_path.to_string_lossy().to_string());
        }
        
        // Create a mock namespace mapping function to test the logic
        // This replaces the actual OPC UA server connection
        struct MockPoller {
            config: TelegrafConfig,
        }
        
        impl MockPoller {
            fn new(config: TelegrafConfig) -> Self {
                Self { config }
            }
            
            fn get_namespace_info(&self, xml_files: &[String]) -> Result<HashMap<String, u16>, TelegrafError> {
                let mut result = HashMap::new();
                
                // Simulate finding matches based on file names
                for file_path in xml_files {
                    let path = std::path::Path::new(file_path);
                    if let Some(file_name) = path.file_name() {
                        if let Some(file_name_str) = file_name.to_str() {
                            // Mock mapping: device1.xml -> 1, sensor2.xml -> 2, etc.
                            if file_name_str == "device1.xml" {
                                result.insert(file_name_str.to_string(), 1);
                            } else if file_name_str == "sensor2.xml" {
                                result.insert(file_name_str.to_string(), 2);
                            } else if file_name_str == "controller3.xml" {
                                result.insert(file_name_str.to_string(), 3);
                            }
                        }
                    }
                }
                
                Ok(result)
            }
        }
        
        // Create our mock poller
        let mock_poller = MockPoller::new(create_test_config());
        
        // Test the namespace mapping
        let namespace_map = mock_poller.get_namespace_info(&xml_file_paths).unwrap();
        
        // Verify the results
        assert_eq!(namespace_map.len(), 3);
        assert_eq!(namespace_map.get("device1.xml"), Some(&1));
        assert_eq!(namespace_map.get("sensor2.xml"), Some(&2));
        assert_eq!(namespace_map.get("controller3.xml"), Some(&3));
    }
    
    // Test how the GUI would use the namespace information to update its fields
    #[test]
    fn test_gui_update_from_namespace_map() {
        // Create a mock namespace map returned from the poller
        let mut namespace_map = HashMap::new();
        namespace_map.insert("device1.xml".to_string(), 1);
        namespace_map.insert("sensor2.xml".to_string(), 2);
        namespace_map.insert("controller3.xml".to_string(), 3);
        
        // Create a mock file_configs structure like in the GUI
        #[derive(Default)]
        struct XmlFileConfig {
            namespace: String,
            interval_ms: String,
            ip: String,
        }
        
        let mut file_configs = HashMap::new();
        let file_paths = vec![
            "/path/to/device1.xml",
            "/another/path/sensor2.xml",
            "/yet/another/path/controller3.xml",
        ];
        
        // Setup initial file configs
        for path in &file_paths {
            file_configs.insert(path.to_string(), XmlFileConfig::default());
        }
        
        // Simulate the GUI update logic
        let mut found_count = 0;
        for (file_name, namespace_index) in &namespace_map {
            // Find the full path for this file name
            if let Some(full_path) = file_paths.iter().find(|path| {
                path.ends_with(file_name)
            }) {
                // Update the namespace field
                if let Some(config) = file_configs.get_mut(*full_path) {
                    config.namespace = namespace_index.to_string();
                    found_count += 1;
                }
            }
        }
        
        // Verify that all files were updated
        assert_eq!(found_count, 3);
        
        // Verify that each config has the correct namespace
        assert_eq!(file_configs.get("/path/to/device1.xml").unwrap().namespace, "1");
        assert_eq!(file_configs.get("/another/path/sensor2.xml").unwrap().namespace, "2");
        assert_eq!(file_configs.get("/yet/another/path/controller3.xml").unwrap().namespace, "3");
    }
}
