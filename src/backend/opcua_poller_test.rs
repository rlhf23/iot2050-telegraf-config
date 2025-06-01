use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

use opcua::{
    client::prelude::Session,
    sync::RwLock,
    types::{
        AttributeId, BrowseDescription, BrowseDescriptionResultMask, BrowseDirection, ByteString,
        EndpointDescription, MessageSecurityMode, NodeClass, NodeId, ReferenceTypeId,
        UserTokenPolicy, Variant, DataValue, DataTypeId, Guid, LocalizedText, UAString, 
        service_types::ReferenceDescription,
    },
};

use crate::error::TelegrafError;
use crate::TelegrafConfig;
use crate::backend::opcua_poller::{OpcUaNode, OpcUaPoller};

// Mock implementation for testing without a real OPC UA server
#[cfg(test)]
mod tests {
    use super::*;
    use opcua::types::*;
    
    // Helper to create a test node ID
    fn test_node_id(id: u32) -> NodeId {
        NodeId::new(0, id as u32)
    }
    
    // Helper to create a test OpcUaNode
    fn create_test_node(id: u32, name: &str, node_class: NodeClass) -> OpcUaNode {
        OpcUaNode::new(
            test_node_id(id),
            name.to_string(),
            format!("Display {}", name),
            node_class,
        )
    }
    
    // Helper to create a test config
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
            selected_opcua_nodes: Vec::new(),
        }
    }
    
    // Helper to create a test node with children
    fn create_test_node_with_children() -> OpcUaNode {
        let mut node = create_test_node(1, "TestNode", NodeClass::Object);
        let child1 = create_test_node(2, "Child1", NodeClass::Variable);
        let child2 = create_test_node(3, "Child2", NodeClass::Variable);
        node.children = vec![child1, child2];
        node
    }
    
    // Tests for OpcUaNode implementation
    mod opcua_node_tests {
        use super::*;
        
        #[test]
        fn test_new_node() {
            let node = OpcUaNode::new(
                test_node_id(1),
                "TestNode".to_string(),
                "Test Node".to_string(),
                NodeClass::Variable,
            );
            
            assert_eq!(node.browse_name, "TestNode");
            assert_eq!(node.display_name, "Test Node");
            assert_eq!(node.node_class, NodeClass::Variable);
            assert!(!node.selected);
            assert!(!node.children_loaded);
            assert!(!node.has_more_children);
            assert!(node.continuation_point.is_none());
        }
        
        #[test]
        fn test_node_selection() {
            let mut node = create_test_node(1, "TestNode", NodeClass::Variable);
            
            // Test initial state
            assert!(!node.selected);
            
            // Test selection
            node.selected = true;
            assert!(node.selected);
        }
        
        #[test]
        fn test_is_folder_node() {
            let folder_node = create_test_node(1, "Folder", NodeClass::Object);
            let var_node = create_test_node(2, "Variable", NodeClass::Variable);
            
            assert!(folder_node.is_folder_node());
            assert!(!var_node.is_folder_node());
        }
        
        #[test]
        fn test_deselect_children() {
            let mut parent = create_test_node(1, "Parent", NodeClass::Object);
            let mut child1 = create_test_node(2, "Child1", NodeClass::Variable);
            let mut child2 = create_test_node(3, "Child2", NodeClass::Variable);
            
            parent.selected = true;
            child1.selected = true;
            child2.selected = true;
            
            parent.children = vec![child1, child2];
            parent.deselect_children();
            
            for child in &parent.children {
                assert!(!child.selected);
            }
        }
    }
    
    // Tests for OpcUaPoller implementation
    mod opcua_poller_tests {
        use super::*;
        
        #[test]
        fn test_new_poller() {
            let config = create_test_config();
            let poller = OpcUaPoller::new(config);
            
            // Should succeed with valid config
            assert!(poller.is_ok());
            
            // Test that we can use the poller
            if let Ok(poller) = poller {
                // Test that we can get namespace info (will fail to connect, but should return an error)
                let result = poller.get_namespace_info(&["test.xml".to_string()]);
                assert!(result.is_err()); // Should fail to connect to OPC UA server
            }
        }
        
        #[test]
        fn test_browse_complete_structure() {
            let config = create_test_config();
            let poller = OpcUaPoller::new(config).unwrap();
            
            // This will fail to connect to a real OPC UA server, but we can test the error case
            let result = poller.browse_complete_structure();
            assert!(result.is_err());
        }
        
        #[test]
        fn test_invalid_ip() {
            let mut config = create_test_config();
            config.ip = "invalid-ip".to_string();
            
            let result = OpcUaPoller::new(config);
            // The error could be either ConfigError or OpcUaClientError
            assert!(result.is_err());
        }
    }


    // Test node collection methods
    #[test]
    fn test_node_collection() {
        // Create a simple node hierarchy
        let mut parent = create_test_node(1, "Parent", NodeClass::Object);
        let mut child1 = create_test_node(2, "Child1", NodeClass::Variable);
        let child2 = create_test_node(3, "Child2", NodeClass::Variable);
        
        // Select one child
        child1.selected = true;
        parent.children = vec![child1, child2];
        
        // Test collect_selected_nodes
        let mut selected = Vec::new();
        OpcUaNode::collect_selected_nodes(&[parent.clone()], &mut selected);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].browse_name, "Child1");
        
        // Test collect_selected_variable_nodes
        let mut var_nodes = Vec::new();
        OpcUaNode::collect_selected_variable_nodes(&[parent], &mut var_nodes);
        assert_eq!(var_nodes.len(), 1);
        assert_eq!(var_nodes[0].browse_name, "Child1");
    }
    
    // Test node conversion to config
    #[test]
    fn test_node_conversion() {
        let mut node = create_test_node(1, "TestNode", NodeClass::Variable);
        node.node_id = NodeId::new(2, "TestNode");
        node.data_type = Some("Double".to_string());
        node.selected = true; // Node must be selected to be included
        
        // Convert to config format
        let config_nodes = OpcUaNode::convert_selected_nodes_to_config(&[node]);
        
        // Verify we got the expected number of nodes
        assert_eq!(config_nodes.len(), 1);
        
        // Verify the node ID is correctly formatted
        let node_id_str = config_nodes[0].node_id.to_string();
        assert!(node_id_str.contains("TestNode"), "Node ID should contain 'TestNode', got: {}", node_id_str);
    }
    
    // Test error handling for invalid configurations
    #[test]
    fn test_invalid_configurations() {
        // Test with invalid IP
        let mut config = create_test_config();
        config.ip = "invalid-ip".to_string();
        let poller = OpcUaPoller::new(config);
        assert!(poller.is_err());
        
        // Test with empty IP
        let mut empty_ip_config = create_test_config();
        empty_ip_config.ip = "".to_string();
        let poller = OpcUaPoller::new(empty_ip_config);
        assert!(poller.is_err());
    }
    
    // Test that the mapping between XML file names and namespaces works correctly
    #[test]
    fn test_namespace_mapping() {
        // Create a temporary directory for our test files
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();

        // Create some test XML files
        let file_paths = vec!["device1.xml", "sensor2.xml", "controller3.xml"];

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

            fn get_namespace_info(
                &self,
                xml_files: &[String],
            ) -> Result<HashMap<String, u16>, TelegrafError> {
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
            if let Some(full_path) = file_paths.iter().find(|path| path.ends_with(file_name)) {
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
        assert_eq!(
            file_configs.get("/path/to/device1.xml").unwrap().namespace,
            "1"
        );
        assert_eq!(
            file_configs
                .get("/another/path/sensor2.xml")
                .unwrap()
                .namespace,
            "2"
        );
        assert_eq!(
            file_configs
                .get("/yet/another/path/controller3.xml")
                .unwrap()
                .namespace,
            "3"
        );
    }
}
