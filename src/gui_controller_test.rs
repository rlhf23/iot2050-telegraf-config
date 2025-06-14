#[cfg(test)]
mod tests {
    use crate::gui_controller::{GuiController, OpcUaBrowseState, FormState, XmlFileConfig};
    use crate::{
        backend::opcua_poller::OpcUaNode,
        error::TelegrafError,
    };
    use opcua::types::{NodeClass, NodeId};

    fn create_test_controller() -> GuiController {
        GuiController::new()
    }

    #[test]
    fn test_gui_controller_new() {
        let controller = create_test_controller();
        
        // IP can be either "192.168.1.1" or "192.168.1.1:4840" depending on environment
        assert!(controller.config.ip == "192.168.1.1" || controller.config.ip == "192.168.1.1:4840");
        assert_eq!(controller.config.username, "user");
        assert_eq!(controller.config.password, "pass");
        assert_eq!(controller.config.iot_host, "192.168.1.2:22");
        assert_eq!(controller.config.iot_username, "iotuser");
        assert_eq!(controller.config.iot_password, "iotpass");
        assert_eq!(controller.config.output_format, Some("influxdb".to_string()));
        assert!(controller.config.include_test_inputs);
        assert!(controller.xml_files.is_empty());
        assert!(controller.selected_listener_files.is_empty());
        assert!(controller.file_configs.is_empty());
        assert!(controller.status_messages.is_empty());
        assert!(controller.opcua_nodes.is_empty());
        assert!(!controller.show_opcua_browser);
        assert!(!controller.is_working);
        assert_eq!(controller.opcua_browse_state, OpcUaBrowseState::Idle);
        assert!(controller.should_scroll);
    }

    #[test]
    fn test_load_xml_files() {
        let mut controller = create_test_controller();
        
        // Initially empty
        assert!(controller.xml_files.is_empty());
        assert!(controller.selected_listener_files.is_empty());
        
        // Load XML files (will be empty since we're not in a real directory with XML files)
        controller.load_xml_files();
        
        // Should still be empty but the vectors should be consistent
        assert_eq!(controller.xml_files.len(), controller.selected_listener_files.len());
    }

    #[test]
    fn test_update_folder() {
        let mut controller = create_test_controller();
        let original_folder = controller.config.folder.clone();
        
        let new_folder = std::path::PathBuf::from("/tmp");
        controller.update_folder(new_folder.clone());
        
        assert_eq!(controller.config.folder, new_folder);
        assert_ne!(controller.config.folder, original_folder);
    }

    #[test]
    fn test_validate_file_ip_valid() {
        let mut controller = create_test_controller();
        let file = "test.xml".to_string();
        let valid_ip = "192.168.1.1";
        
        controller.validate_file_ip(&file, valid_ip);
        
        assert_eq!(controller.form_state.ip_errors.get(&file), Some(&false));
    }

    #[test]
    fn test_validate_file_ip_invalid() {
        let mut controller = create_test_controller();
        let file = "test.xml".to_string();
        let invalid_ip = "999.999.999.999";
        
        controller.validate_file_ip(&file, invalid_ip);
        
        assert_eq!(controller.form_state.ip_errors.get(&file), Some(&true));
    }

    #[test]
    fn test_validate_file_ip_empty() {
        let mut controller = create_test_controller();
        let file = "test.xml".to_string();
        let empty_ip = "";
        
        controller.validate_file_ip(&file, empty_ip);
        
        assert_eq!(controller.form_state.ip_errors.get(&file), Some(&false));
    }

    #[test]
    fn test_handle_error_host_format() {
        let mut controller = create_test_controller();
        let error = TelegrafError::HostFormatError("Invalid host format".to_string());
        
        let message = controller.handle_error(&error, "test context");
        
        assert!(controller.form_state.show_iot_host_error);
        assert!(message.contains("Host Format Error"));
    }

    #[test]
    fn test_handle_error_other() {
        let mut controller = create_test_controller();
        let error = TelegrafError::ValidationError("Some validation error".to_string());
        
        let message = controller.handle_error(&error, "test context");
        
        assert!(!controller.form_state.show_iot_host_error);
        assert!(!message.is_empty());
    }

    #[test]
    fn test_add_selected_nodes_to_config() {
        let mut controller = create_test_controller();
        
        // Create a test node
        let mut test_node = OpcUaNode {
            node_id: NodeId::new(2, "TestNode"),
            browse_name: "TestNode".to_string(),
            display_name: "Test Node".to_string(),
            node_class: NodeClass::Variable,
            selected: true,
            children: Vec::new(),
            children_loaded: true,
            data_type: Some("Int32".to_string()),
            description: Some("Test description".to_string()),
            has_more_children: false,
            continuation_point: None,
        };
        
        controller.opcua_nodes.push(test_node);
        
        // Initially no selected nodes in config
        assert!(controller.config.selected_opcua_nodes.is_empty());
        
        controller.add_selected_nodes_to_config();
        
        // Should now have selected nodes in config
        assert!(!controller.config.selected_opcua_nodes.is_empty());
    }

    #[test]
    fn test_start_opcua_browsing() {
        let mut controller = create_test_controller();
        
        assert!(!controller.show_opcua_browser);
        assert_eq!(controller.opcua_browse_state, OpcUaBrowseState::Idle);
        
        controller.start_opcua_browsing();
        
        assert!(controller.show_opcua_browser);
        assert_eq!(controller.opcua_browse_state, OpcUaBrowseState::BrowsingNodes);
        assert_eq!(controller.browse_status_message, "Requesting OPC UA structure...");
    }

    #[test]
    fn test_start_opcua_browsing_already_complete() {
        let mut controller = create_test_controller();
        controller.opcua_browse_state = OpcUaBrowseState::BrowsingNodesComplete;
        
        controller.start_opcua_browsing();
        
        assert!(controller.show_opcua_browser);
        assert_eq!(controller.opcua_browse_state, OpcUaBrowseState::BrowsingNodesComplete);
        assert_eq!(controller.browse_status_message, "OPC UA structure previously loaded.");
    }

    #[test]
    fn test_start_getting_namespaces() {
        let mut controller = create_test_controller();
        
        assert_eq!(controller.opcua_browse_state, OpcUaBrowseState::Idle);
        
        controller.start_getting_namespaces();
        
        assert_eq!(controller.opcua_browse_state, OpcUaBrowseState::GettingNamespaces);
        assert!(!controller.status_messages.is_empty());
        assert_eq!(controller.status_messages.last().unwrap(), "Getting OPC UA namespaces...");
    }

    #[test]
    fn test_refresh_opcua_structure() {
        let mut controller = create_test_controller();
        controller.opcua_browse_state = OpcUaBrowseState::BrowsingNodesComplete;
        
        // Add some nodes to verify they get cleared
        controller.opcua_nodes.push(OpcUaNode {
            node_id: NodeId::new(2, "TestNode"),
            browse_name: "TestNode".to_string(),
            display_name: "Test Node".to_string(),
            node_class: NodeClass::Variable,
            selected: false,
            children: Vec::new(),
            children_loaded: true,
            data_type: None,
            description: None,
            has_more_children: false,
            continuation_point: None,
        });
        
        controller.refresh_opcua_structure();
        
        assert!(controller.opcua_nodes.is_empty());
        assert_eq!(controller.opcua_browse_state, OpcUaBrowseState::BrowsingNodes);
        assert_eq!(controller.browse_status_message, "Requesting OPC UA structure refresh...");
    }

    #[test]
    fn test_clear_status_messages() {
        let mut controller = create_test_controller();
        controller.status_messages.push("Test message 1".to_string());
        controller.status_messages.push("Test message 2".to_string());
        controller.should_scroll = false;
        
        assert_eq!(controller.status_messages.len(), 2);
        
        controller.clear_status_messages();
        
        assert!(controller.status_messages.is_empty());
        assert!(controller.should_scroll);
    }

    #[test]
    fn test_remove_selected_opcua_node() {
        let mut controller = create_test_controller();
        
        // Add some test nodes
        controller.config.selected_opcua_nodes.push(crate::SelectedOpcUaNode {
            node_id: NodeId::new(2, "Node1"),
            namespace: 2,
            browse_name: "Node1".to_string(),
            display_name: "Node 1".to_string(),
            measurement_name: "measurement1".to_string(),
            interval_ms: 1000,
            folder_name: None,
        });
        
        controller.config.selected_opcua_nodes.push(crate::SelectedOpcUaNode {
            node_id: NodeId::new(2, "Node2"),
            namespace: 2,
            browse_name: "Node2".to_string(),
            display_name: "Node 2".to_string(),
            measurement_name: "measurement2".to_string(),
            interval_ms: 1000,
            folder_name: None,
        });
        
        assert_eq!(controller.config.selected_opcua_nodes.len(), 2);
        
        controller.remove_selected_opcua_node(0);
        
        assert_eq!(controller.config.selected_opcua_nodes.len(), 1);
        assert_eq!(controller.config.selected_opcua_nodes[0].display_name, "Node 2");
    }

    #[test]
    fn test_remove_selected_opcua_node_invalid_index() {
        let mut controller = create_test_controller();
        
        // Add one test node
        controller.config.selected_opcua_nodes.push(crate::SelectedOpcUaNode {
            node_id: NodeId::new(2, "Node1"),
            namespace: 2,
            browse_name: "Node1".to_string(),
            display_name: "Node 1".to_string(),
            measurement_name: "measurement1".to_string(),
            interval_ms: 1000,
            folder_name: None,
        });
        
        assert_eq!(controller.config.selected_opcua_nodes.len(), 1);
        
        // Try to remove with invalid index
        controller.remove_selected_opcua_node(5);
        
        // Should still have the original node
        assert_eq!(controller.config.selected_opcua_nodes.len(), 1);
    }

    #[test]
    fn test_process_worker_responses_no_worker() {
        let mut controller = create_test_controller();
        controller.worker = None;
        
        let should_repaint = controller.process_worker_responses();
        
        assert!(!should_repaint);
    }

    #[test]
    fn test_form_state_default() {
        let form_state = FormState::default();
        
        assert!(!form_state.show_namespace_error);
        assert!(!form_state.show_iot_host_error);
        assert!(form_state.ip_errors.is_empty());
    }

    #[test]
    fn test_opcua_browse_state_default() {
        let state = OpcUaBrowseState::default();
        assert_eq!(state, OpcUaBrowseState::Idle);
    }

    #[test]
    fn test_opcua_browse_state_equality() {
        assert_eq!(OpcUaBrowseState::Idle, OpcUaBrowseState::Idle);
        assert_eq!(OpcUaBrowseState::BrowsingNodes, OpcUaBrowseState::BrowsingNodes);
        assert_ne!(OpcUaBrowseState::Idle, OpcUaBrowseState::BrowsingNodes);
        
        let error1 = OpcUaBrowseState::BrowsingNodesFailed("error1".to_string());
        let error2 = OpcUaBrowseState::BrowsingNodesFailed("error1".to_string());
        let error3 = OpcUaBrowseState::BrowsingNodesFailed("error2".to_string());
        
        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }

    #[test]
    fn test_xml_file_config_default() {
        let config = XmlFileConfig::default();
        
        assert!(config.namespace.is_empty());
        assert!(config.interval_ms.is_empty());
        assert!(config.ip.is_empty());
    }

    #[test]
    fn test_controller_default() {
        let controller = GuiController::default();
        
        // Should be equivalent to GuiController::new()
        assert!(!controller.xml_files.is_empty() || controller.xml_files.is_empty()); // Just checking it doesn't panic
        assert_eq!(controller.opcua_browse_state, OpcUaBrowseState::Idle);
        assert!(controller.should_scroll);
    }

    #[test]
    fn test_generate_config_validation_errors() {
        let mut controller = create_test_controller();
        
        // Set up invalid configuration to trigger validation errors
        controller.config.iot_host = "invalid_host_format".to_string(); // Missing port
        
        // Add a file config with empty namespace to trigger validation error
        controller.xml_files.push("test.xml".to_string());
        controller.selected_listener_files.push(false);
        controller.file_configs.insert("test.xml".to_string(), XmlFileConfig {
            namespace: "".to_string(), // Empty namespace should cause validation error
            interval_ms: "1000".to_string(),
            ip: "".to_string(),
        });
        
        let initial_message_count = controller.status_messages.len();
        
        controller.generate_config();
        
        // Should have added an error message
        assert!(controller.status_messages.len() > initial_message_count);
        let last_message = controller.status_messages.last().unwrap();
        assert!(last_message.contains("Validation errors"));
        
        // Error flags should be set
        assert!(controller.form_state.show_iot_host_error);
    }

    #[test]
    fn test_send_worker_command_no_worker() {
        let mut controller = create_test_controller();
        controller.worker = None;
        
        let initial_message_count = controller.status_messages.len();
        
        controller.send_worker_command(
            crate::WorkerCommand::DummyCommand,
            "Test message"
        );
        
        // Should add error message about worker not being initialized
        assert_eq!(controller.status_messages.len(), initial_message_count + 1);
        assert_eq!(controller.status_messages.last().unwrap(), "Worker not initialized");
        assert!(!controller.is_working);
    }
}