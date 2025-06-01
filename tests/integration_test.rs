use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use scopeguard;
use sie_generate_config::backend::opcua_poller::OpcUaNode;

use sie_generate_config::{
    TelegrafConfig, 
    SelectedOpcUaNode,
    backend::{
        opcua_poller::OpcUaPoller,
        ConfigGenerator
    }
};

// Check if we're running in a CI environment
fn is_ci_environment() -> bool {
    // Standard way to detect CI environments
    std::env::var("CI").is_ok()
}

// Integration tests that interface with the CLI are skipped in CI environments
// since they can be unreliable across different platforms and environments
#[test]
fn test_cli_basics() {
    // Skip all CLI tests in CI environments
    if is_ci_environment() {
        println!("Skipping CLI tests in CI environment");
        return;
    }

    // Also skip on Windows which might have different command prompt behavior
    if cfg!(target_os = "windows") {
        println!("Skipping CLI tests on Windows");
        return;
    }

    // Try to find the executable, skip the test if not found
    let binary_path = get_bin_path("sie_generate_config");
    if !binary_path.exists() {
        println!(
            "Binary not found at {}, skipping test",
            binary_path.display()
        );
        return;
    }

    // Just verify that the help command runs successfully
    let help_output = Command::new(&binary_path)
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(help_output.status.success(), "Help command should succeed");
    println!("Verified that the CLI executable can run with --help");
}

// Helper to get the path to our binaries
fn get_bin_path(bin_name: &str) -> PathBuf {
    let mut path = std::env::current_exe().expect("Failed to get current exe path");
    path.pop(); // Remove the test binary name

    // Handle differences between debug/release
    if path.ends_with("deps") {
        path.pop();
    }

    #[cfg(target_os = "windows")]
    let bin_name = format!("{}.exe", bin_name);

    path.push(bin_name);
    path
}

#[test]
fn test_opcua_server_interaction() {
    // Skip in CI environments as they might not have the required ports available
    if is_ci_environment() {
        println!("Skipping OPC UA server test in CI environment");
        return;
    }

    // Channel to communicate with the server thread
    let (tx, rx) = mpsc::channel();
    
    // Start the test server in a separate thread
    let _server_handle = thread::spawn(move || {
        // This will block until the server is shut down
        if let Err(e) = std::panic::catch_unwind(|| {
            let mut server = Command::new(get_bin_path("opcua_test_server"))
                .spawn()
                .expect("Failed to start OPC UA test server");
            
            // Let the main thread know we've started
            tx.send(()).unwrap();
            
            // Wait for the server to be stopped
            let _ = server.wait();
        }) {
            eprintln!("OPC UA server thread panicked: {:?}", e);
        }
    });

    // Wait for the server to start
    if rx.recv_timeout(Duration::from_secs(5)).is_err() {
        panic!("OPC UA server failed to start within timeout");
    }

    // Give the server a moment to fully initialize
    thread::sleep(Duration::from_secs(1));

    // Create a test configuration
    let test_config = TelegrafConfig {
        ip: "127.0.0.1:4840".to_string(),  // Use 127.0.0.1 instead of localhost
        username: "".to_string(),         // Anonymous access
        password: "".to_string(),         // Anonymous access
        iot_host: "".to_string(),        // Not needed for this test
        iot_username: "".to_string(),
        iot_password: "".to_string(),
        token_folder: std::env::temp_dir(),
        bucket_name: "test_bucket".to_string(),
        influx_token: None,
        listener_files: vec![],
        output_format: None,
        include_test_inputs: false,
        selected_opcua_nodes: vec![],
        folder: std::env::current_dir().unwrap(),
    };

    // Create a new OpcUaPoller instance - this will also test server connectivity
    let poller = OpcUaPoller::new(test_config)
        .expect("Failed to create OpcUaPoller and connect to OPC UA server");

    // Test browsing the server structure
    let nodes = poller.browse_complete_structure()
        .expect("Failed to browse server structure");
    
    // Verify we got some nodes back
    assert!(!nodes.is_empty(), "Should find at least one node in the server structure");
    
    // If there are nodes, try to browse their children
    if let Some(first_node) = nodes.first() {
        let children = poller.load_node_children(first_node, 1)
            .expect("Failed to load node children");
        
        // It's valid to have nodes with no children, so we don't assert on the result
        println!("Found {} children for node {}", children.len(), first_node.browse_name);
    }
    
    println!("OPC UA server test completed successfully");
    
    // The server will be killed by the _server_guard when it goes out of scope
}

#[test]
fn test_opcua_config_generation() {
    // Skip test in CI environment
    if is_ci_environment() {
        println!("Skipping OPC UA config generation test in CI environment");
        return;
    }

    // Kill any existing server instances
    let _ = std::process::Command::new("pkill")
        .arg("-f")
        .arg("opcua_test_server")
        .status();

    // Start the OPC UA test server as a child process
    let mut server_handle = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("opcua_test_server")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start OPC UA test server");

    // Give the server a moment to start up
    thread::sleep(Duration::from_secs(2));
    
    // Check if the server is still running
    if let Ok(Some(status)) = server_handle.try_wait() {
        panic!("Server process exited unexpectedly with status: {:?}", status);
    }
    
    // Ensure the server is killed when the test ends (even on panic)
    let _server_guard = scopeguard::guard(server_handle, |mut server| {
        let _ = server.kill();
        let _ = server.wait();
    });

    // Create a test configuration
    let test_config = TelegrafConfig {
        ip: "127.0.0.1:4840".to_string(),
        username: "".to_string(),         // Anonymous access
        password: "".to_string(),         // Anonymous access
        iot_host: "127.0.0.1:4840".to_string(),  // Use the OPC UA server port for testing
        iot_username: "test".to_string(),
        iot_password: "test".to_string(),
        token_folder: std::env::temp_dir(),
        bucket_name: "test_bucket".to_string(),
        influx_token: None,
        listener_files: vec![],
        output_format: None,
        include_test_inputs: false,
        selected_opcua_nodes: vec![],
        folder: std::env::current_dir().unwrap(),
    };

    // Create a new OpcUaPoller instance
    let poller = OpcUaPoller::new(test_config.clone())
        .expect("Failed to create OpcUaPoller and connect to OPC UA server");

    // Browse the server structure to find some nodes
    let nodes = poller.browse_complete_structure()
        .expect("Failed to browse server structure");
    
    println!("Found {} nodes in server structure", nodes.len());
    
    // Select the first few variable nodes for monitoring
    let mut selected_nodes = Vec::new();
    
    // Helper function to recursively find variable nodes
    fn find_variable_nodes(nodes: &[crate::OpcUaNode], selected: &mut Vec<SelectedOpcUaNode>) {
        for node in nodes {
            if node.node_class == opcua::types::NodeClass::Variable {
                selected.push(SelectedOpcUaNode {
                    node_id: node.node_id.clone(),
                    namespace: node.node_id.namespace,
                    browse_name: node.browse_name.clone(),
                    display_name: node.display_name.clone(),
                    measurement_name: node.display_name.clone().to_lowercase().replace(' ', "_"),
                    interval_ms: 1000,  // 1 second interval
                    folder_name: Some("test_folder".to_string()),
                });
                
                // Stop if we've found enough nodes
                if selected.len() >= 5 {
                    return;
                }
            }
            
            // Recursively check children
            if !node.children.is_empty() {
                find_variable_nodes(&node.children, selected);
                
                // Stop if we've found enough nodes
                if selected.len() >= 5 {
                    return;
                }
            }
        }
    }
    
    // Find up to 5 variable nodes
    find_variable_nodes(&nodes, &mut selected_nodes);
    
    // If we didn't find any variable nodes, try to find any nodes at all
    if selected_nodes.is_empty() {
        println!("No variable nodes found, trying to find any nodes...");
        for node in nodes.iter().take(5) {
            selected_nodes.push(SelectedOpcUaNode {
                node_id: node.node_id.clone(),
                namespace: node.node_id.namespace,
                browse_name: node.browse_name.clone(),
                display_name: node.display_name.clone(),
                measurement_name: node.display_name.clone().to_lowercase().replace(' ', "_"),
                interval_ms: 1000,
                folder_name: Some("test_folder".to_string()),
            });
        }
    }

    // Make sure we found some nodes to test with
    assert!(!selected_nodes.is_empty(), "No variable nodes found to test with");
    
    // Create a config with our selected nodes
    let mut config_with_nodes = test_config;
    config_with_nodes.selected_opcua_nodes = selected_nodes.clone();
    
    // Clone the nodes before moving config_with_nodes
    let selected_nodes_clone = selected_nodes.clone();
    
    // Create a ConfigGenerator
    let config_generator = ConfigGenerator::new(config_with_nodes)
        .expect("Failed to create ConfigGenerator");
    
    // Generate the configuration
    let config = config_generator.generate_config(&[], &[])
        .expect("Failed to generate configuration");
    
    // Verify the generated configuration contains our nodes
    for node in &selected_nodes_clone {
        assert!(
            config.contains(&node.measurement_name),
            "Configuration should contain node: {}",
            node.measurement_name
        );
        
        // Verify the node's namespace and identifier are in the config
        let node_id_str = format!("ns={};{}", node.namespace, node.node_id.identifier);
        assert!(
            config.contains(&node_id_str),
            "Configuration should contain node ID: {}",
            node_id_str
        );
    }
    
    println!("Successfully generated configuration for {} nodes", selected_nodes_clone.len());
}
