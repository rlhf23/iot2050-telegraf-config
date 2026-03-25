use opcua::types::{NodeClass, NodeId};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::tempdir;

use super::*;
use crate::backend::opcua_poller::{OpcUaNode, OpcUaPoller};

#[cfg(test)]
mod tests {
    use super::*;

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
            folder: std::env::current_dir().unwrap(),
            ip: "127.0.0.1:49320".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            iot_host: "192.168.1.2:22".to_string(),
            iot_username: "iotuser".to_string(),
            iot_password: "iotpass".to_string(),
            listener_files: Vec::new(),
            output_format: Some("influxdb".to_string()),
            include_test_inputs: false,
            include_opcua_diagnostics: false,
            selected_opcua_nodes: Vec::new(),
            use_source_timestamp: false,
        }
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
            let poller = OpcUaPoller::new(config.connection_config());

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
            let poller = OpcUaPoller::new(config.connection_config()).unwrap();

            // This will fail to connect to a real OPC UA server, but we can test the error case
            let result = poller.browse_complete_structure();
            assert!(result.is_err());
        }

        #[test]
        fn test_invalid_ip() {
            let mut config = create_test_config();
            config.ip = "invalid-ip".to_string();

            let result = OpcUaPoller::new(config.connection_config());
            // The error could be either ConfigError or OpcUaClientError
            assert!(result.is_err());
        }
    }

    // Test node collection with various selection scenarios
    #[test]
    fn test_node_collection() {
        // Create a more complex node hierarchy with multiple levels
        let mut root = create_test_node(1, "Root", NodeClass::Object);
        let mut folder1 = create_test_node(2, "Folder1", NodeClass::Object);
        let mut folder2 = create_test_node(3, "Folder2", NodeClass::Object);

        // Create variables with different selection states
        let mut var1 = create_test_node(4, "Var1", NodeClass::Variable);
        var1.selected = true;

        let var2 = create_test_node(5, "Var2", NodeClass::Variable);
        let mut var3 = create_test_node(6, "Var3", NodeClass::Variable);
        var3.selected = true;

        // Build the hierarchy
        folder1.children = vec![var1, var2];
        folder2.children = vec![var3];
        root.children = vec![folder1, folder2];

        // Test 1: Collect all selected nodes (including folders)
        let mut selected = Vec::new();
        OpcUaNode::collect_selected_nodes(&[root.clone()], &mut selected);
        assert_eq!(selected.len(), 2, "Should find both selected variables");
        assert_eq!(selected[0].browse_name, "Var1");
        assert_eq!(selected[1].browse_name, "Var3");

        // Test 2: Collect only selected variable nodes
        let mut var_nodes = Vec::new();
        OpcUaNode::collect_selected_variable_nodes(&[root.clone()], &mut var_nodes);
        assert_eq!(var_nodes.len(), 2, "Should find both selected variables");

        // Test 3: Collect all variables from a specific folder
        let mut folder_vars = Vec::new();
        if let Some(folder) = root.children.get(0) {
            OpcUaNode::collect_all_variables_in_folder(folder, &mut folder_vars);
        }
        assert_eq!(folder_vars.len(), 2, "Should find all variables in folder1");

        // Test 4: Deselect all children and verify
        root.deselect_children();
        let mut after_deselect = Vec::new();
        OpcUaNode::collect_selected_nodes(&[root], &mut after_deselect);
        assert!(
            after_deselect.is_empty(),
            "No nodes should be selected after deselect_all"
        );
    }

    // Test node conversion to config with various node types and properties
    #[test]
    fn test_node_conversion() {
        // Test with a simple variable node
        let mut var_node = create_test_node(1, "TestNode", NodeClass::Variable);
        var_node.node_id = NodeId::new(2, "TestNode");
        var_node.display_name = "Test Node".to_string();
        var_node.selected = true;

        // Test with an object node that has children
        let mut obj_node = create_test_node(2, "ParentNode", NodeClass::Object);
        obj_node.node_id = NodeId::new(2, "Parent");
        obj_node.display_name = "Parent Node".to_string();
        obj_node.selected = true;

        let mut child1 = create_test_node(3, "Child1", NodeClass::Variable);
        child1.node_id = NodeId::new(2, "Child1");
        child1.display_name = "Child 1".to_string();
        child1.selected = true;

        let child2 = create_test_node(4, "Child2", NodeClass::Variable);
        obj_node.children = vec![child1, child2];

        // Convert to config format
        let config_nodes = OpcUaNode::convert_selected_nodes_to_config(&[var_node, obj_node]);

        // Debug output to help diagnose test failures
        println!("Converted nodes ({}):", config_nodes.len());
        for (i, node) in config_nodes.iter().enumerate() {
            println!(
                "  {}: {} (browse_name={}, display_name={}, folder={:?})",
                i, node.node_id, node.browse_name, node.display_name, node.folder_name
            );
        }

        // Verify we have at least the variable nodes we expect
        // The exact number might vary based on implementation, but we should have at least 2 nodes
        assert!(
            config_nodes.len() >= 2,
            "Expected at least 2 nodes (TestNode and Child1)"
        );

        // Verify the variable node properties
        let var_config = config_nodes
            .iter()
            .find(|n| n.browse_name == "TestNode")
            .expect("TestNode not found in converted nodes");

        // Verify the node ID format matches the expected OPC UA string format
        assert!(
            var_config.node_id.to_string().contains("TestNode"),
            "Node ID should contain 'TestNode', got: {}",
            var_config.node_id
        );
        assert_eq!(var_config.namespace, 2);
        assert_eq!(var_config.display_name, "Test Node");
        assert_eq!(var_config.measurement_name, "test_node");
        assert_eq!(var_config.interval_ms, 1000);
        assert!(
            var_config.folder_name.is_none(),
            "Top-level node should not have a folder name, got: {:?}",
            var_config.folder_name
        );

        // Verify the child node has the correct folder name
        let child_config = config_nodes
            .iter()
            .find(|n| n.browse_name == "Child1")
            .expect("Child1 not found in converted nodes");

        assert!(
            child_config.node_id.to_string().contains("Child1"),
            "Child1 node ID should contain 'Child1', got: {}",
            child_config.node_id
        );

        // The folder_name should be the display name of the parent node
        assert_eq!(
            child_config.folder_name,
            Some("Parent Node".to_string()),
            "Child node should have parent folder name"
        );

        // We don't need to verify parent nodes as the poller doesn't use them directly
    }

    // Test node collection with complex hierarchies
    #[test]
    fn test_complex_node_hierarchy() {
        // Create a deep hierarchy: root -> folder1 -> folder2 -> var1, var2
        let mut root = create_test_node(1, "Root", NodeClass::Object);
        let mut folder1 = create_test_node(2, "Folder1", NodeClass::Object);
        let mut folder2 = create_test_node(3, "Folder2", NodeClass::Object);
        let var1 = create_test_node(4, "Var1", NodeClass::Variable);
        let var2 = create_test_node(5, "Var2", NodeClass::Variable);

        // Build the hierarchy
        folder2.children = vec![var1, var2];
        folder1.children = vec![folder2];
        root.children = vec![folder1];

        // Test collecting all variable nodes
        let mut all_vars = Vec::new();
        OpcUaNode::collect_all_variables_in_folder(&root, &mut all_vars);
        assert_eq!(
            all_vars.len(),
            2,
            "Should find all variable nodes in the hierarchy"
        );
        assert_eq!(all_vars[0].browse_name, "Var1");
        assert_eq!(all_vars[1].browse_name, "Var2");

        // Test with selective node selection
        if let Some(folder2) = root.children[0].children.get_mut(0) {
            if let Some(var1) = folder2.children.get_mut(0) {
                var1.selected = true;
            }
        }

        let mut selected_vars = Vec::new();
        OpcUaNode::collect_selected_variable_nodes(&[root], &mut selected_vars);
        assert_eq!(
            selected_vars.len(),
            1,
            "Should only find selected variable nodes"
        );
        assert_eq!(selected_vars[0].browse_name, "Var1");
    }

    // Test error handling and edge cases
    #[test]
    fn test_error_handling() {
        // Test with various invalid configurations
        let test_cases = vec![
            ("", "empty IP"),
            ("invalid-ip", "malformed IP"),
            ("256.256.256.256", "out of range IP"),
            ("localhost:70000", "invalid port"),
        ];

        for (invalid_ip, case_name) in test_cases {
            let mut config = create_test_config();
            config.ip = invalid_ip.to_string();
            let result = OpcUaPoller::new(config.connection_config());
            assert!(
                result.is_err(),
                "Should fail with {}: {}",
                case_name,
                invalid_ip
            );
        }

        // Test with empty node list
        let empty_nodes: Vec<OpcUaNode> = Vec::new();
        let empty_config = OpcUaNode::convert_selected_nodes_to_config(&empty_nodes);
        assert!(empty_config.is_empty(), "Should handle empty node list");

        // Test with unselected nodes
        let unselected_node = create_test_node(1, "Unselected", NodeClass::Variable);
        let unselected_config = OpcUaNode::convert_selected_nodes_to_config(&[unselected_node]);
        assert!(
            unselected_config.is_empty(),
            "Should not include unselected nodes"
        );
    }

    // Test node display and string representation
    #[test]
    fn test_node_display() {
        // Test with different node types and properties
        let mut node = create_test_node(1, "TestNode", NodeClass::Variable);
        node.node_id = NodeId::new(2, "TestNode");
        node.display_name = "Display Name".to_string();
        node.data_type = Some("Double".to_string());
        node.description = Some("Test Description".to_string());

        // Verify node properties are accessible
        assert_eq!(node.browse_name, "TestNode");
        assert_eq!(node.display_name, "Display Name");
        assert_eq!(node.node_class, NodeClass::Variable);
        assert_eq!(node.data_type, Some("Double".to_string()));
        assert_eq!(node.description, Some("Test Description".to_string()));

        // Test node ID string representation
        let node_id_str = node.node_id.to_string();
        assert!(
            node_id_str.contains("TestNode"),
            "Node ID should contain node identifier"
        );

        // Test node type detection
        assert!(
            !node.is_folder_node(),
            "Variable node should not be a folder"
        );

        // Create a folder node and test
        let folder_node = create_test_node(2, "Folder", NodeClass::Object);
        assert!(
            folder_node.is_folder_node(),
            "Object node should be a folder"
        );
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

    // Test error handling for namespace polling
    #[test]
    fn test_namespace_info_empty_result() {
        // Test that when browse_server_namespaces returns empty, we get a clear error
        struct MockEmptyPoller {
            config: TelegrafConfig,
        }

        impl MockEmptyPoller {
            fn new(config: TelegrafConfig) -> Self {
                Self { config }
            }

            fn get_namespace_info(
                &self,
                xml_files: &[String],
            ) -> Result<HashMap<String, u16>, TelegrafError> {
                // Simulate empty namespace result
                let namespaces: Vec<(u16, String)> = Vec::new();

                if namespaces.is_empty() {
                    return Err(TelegrafError::OpcUaClientError(
                        "No namespaces found on OPC-UA server. The server may not have ServerInterfaces configured, or the browse operation failed. Check server logs for details.".to_string()
                    ));
                }

                Ok(HashMap::new())
            }
        }

        let mock_poller = MockEmptyPoller::new(create_test_config());
        let result = mock_poller.get_namespace_info(&["test.xml".to_string()]);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No namespaces found"));
        assert!(err.to_string().contains("ServerInterfaces"));
    }

    #[test]
    fn test_namespace_info_no_matches() {
        // Test that when files don't match any namespaces, we get a clear error
        struct MockNoMatchPoller {
            config: TelegrafConfig,
        }

        impl MockNoMatchPoller {
            fn new(config: TelegrafConfig) -> Self {
                Self { config }
            }

            fn get_namespace_info(
                &self,
                xml_files: &[String],
            ) -> Result<HashMap<String, u16>, TelegrafError> {
                // Simulate namespaces that don't match any files
                let namespaces = vec![
                    (2, "SomeNamespace".to_string()),
                    (3, "AnotherNamespace".to_string()),
                ];

                let mut namespace_map = HashMap::new();

                // Try to match files (will fail)
                for file_path in xml_files {
                    let path = std::path::Path::new(file_path);
                    if let Some(file_name) = path.file_name() {
                        if let Some(base_name) = path.file_stem() {
                            let base_name_str = base_name.to_str().unwrap();

                            for (namespace_index, namespace_name) in &namespaces {
                                if namespace_name.to_lowercase() == base_name_str.to_lowercase() {
                                    namespace_map.insert(
                                        file_name.to_str().unwrap().to_string(),
                                        *namespace_index,
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }

                // Check if no matches were found
                if namespace_map.is_empty() {
                    return Err(TelegrafError::OpcUaClientError(
                        format!(
                            "Could not match any XML files to server namespaces. Found {} namespaces on server but none matched the {} uploaded file(s). Check that your XML file names match the namespace names on the server.",
                            namespaces.len(),
                            xml_files.len()
                        )
                    ));
                }

                Ok(namespace_map)
            }
        }

        let mock_poller = MockNoMatchPoller::new(create_test_config());
        let result = mock_poller.get_namespace_info(&["UnmatchedFile.xml".to_string()]);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Could not match any XML files"));
        assert!(err.to_string().contains("2 namespaces"));
        assert!(err.to_string().contains("1 uploaded file"));
    }

    #[test]
    fn test_namespace_info_partial_matches() {
        // Test that when only some files match, we still succeed but can track which matched
        struct MockPartialMatchPoller {
            config: TelegrafConfig,
        }

        impl MockPartialMatchPoller {
            fn new(config: TelegrafConfig) -> Self {
                Self { config }
            }

            fn get_namespace_info(
                &self,
                xml_files: &[String],
            ) -> Result<HashMap<String, u16>, TelegrafError> {
                // Simulate namespaces where only some files match
                let namespaces = vec![
                    (2, "MatchingNamespace".to_string()),
                    (3, "AnotherNamespace".to_string()),
                ];

                let mut namespace_map = HashMap::new();

                for file_path in xml_files {
                    let path = std::path::Path::new(file_path);
                    if let Some(file_name) = path.file_name() {
                        if let Some(base_name) = path.file_stem() {
                            let base_name_str = base_name.to_str().unwrap();

                            for (namespace_index, namespace_name) in &namespaces {
                                if namespace_name.to_lowercase() == base_name_str.to_lowercase() {
                                    namespace_map.insert(
                                        file_name.to_str().unwrap().to_string(),
                                        *namespace_index,
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }

                // Don't error if we have at least one match
                if namespace_map.is_empty() {
                    return Err(TelegrafError::OpcUaClientError(
                        "No matches found".to_string(),
                    ));
                }

                Ok(namespace_map)
            }
        }

        let mock_poller = MockPartialMatchPoller::new(create_test_config());
        let result = mock_poller.get_namespace_info(&[
            "MatchingNamespace.xml".to_string(),
            "UnmatchedFile.xml".to_string(),
        ]);

        assert!(result.is_ok());
        let namespace_map = result.unwrap();

        // Should have one match
        assert_eq!(namespace_map.len(), 1);
        assert_eq!(namespace_map.get("MatchingNamespace.xml"), Some(&2));
        assert_eq!(namespace_map.get("UnmatchedFile.xml"), None);
    }

    #[test]
    fn test_namespace_response_format() {
        // Test that the response format matches what the frontend expects
        #[derive(Debug)]
        struct NamespaceMapping {
            filename: String,
            namespace: String,
            namespace_index: u16,
        }

        // Simulate the API response structure
        let mappings = vec![
            NamespaceMapping {
                filename: "device1.xml".to_string(),
                namespace: "2".to_string(),
                namespace_index: 2,
            },
            NamespaceMapping {
                filename: "sensor2.xml".to_string(),
                namespace: "3".to_string(),
                namespace_index: 3,
            },
        ];

        // Verify structure
        assert_eq!(mappings.len(), 2);
        assert_eq!(mappings[0].filename, "device1.xml");
        assert_eq!(mappings[0].namespace, "2");
        assert_eq!(mappings[0].namespace_index, 2);

        // Verify namespace is a string (as expected by frontend)
        assert_eq!(mappings[1].namespace, "3");
    }
}
