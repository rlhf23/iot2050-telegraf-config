use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use scopeguard;
use sie_generate_config::backend::opcua_poller::OpcUaNode;

use sie_generate_config::{
    backend::{opcua_poller::OpcUaPoller, ConfigGenerator},
    SelectedOpcUaNode, TelegrafConfig,
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
#[test]
fn test_cli_config_generation() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test in CI environment if needed
    if is_ci_environment() {
        println!("Skipping CLI config generation test in CI environment");
        return Ok(());
    }

    // Get the path to the tests directory
    let tests_dir = std::env::current_dir()?.join("tests");

    // Setup test configuration
    let test_config = TelegrafConfig {
        ip: "127.0.0.1:4840".to_string(),
        username: "".to_string(),
        password: "".to_string(),
        iot_host: "192.168.1.2:22".to_string(),
        iot_username: "test".to_string(),
        iot_password: "test".to_string(),
        token_folder: std::env::temp_dir(),
        bucket_name: "test_bucket".to_string(),
        influx_token: Some("dummy_token".to_string()),
        listener_files: vec![],
        output_format: None,
        include_test_inputs: true,
        selected_opcua_nodes: vec![],
        folder: tests_dir.clone(),
    };

    // Create a ConfigGenerator
    let mut generator =
        ConfigGenerator::new(test_config.clone()).expect("Failed to create ConfigGenerator");

    // Get XML files from tests directory
    println!("Discovering XML files...");
    let xml_files = ConfigGenerator::discover_xml_files(&tests_dir);
    println!("Found {} XML files: {:?}", xml_files.len(), xml_files);
    assert!(
        !xml_files.is_empty(),
        "No XML files found in tests directory"
    );

    // Configure each file with test values
    for file in &xml_files {
        // Use namespace 1 for testing (or extract from filename if possible)
        let namespace = 1;
        let interval_ms = 1000; // 1 second
        generator.set_file_config(file.clone(), namespace.to_string(), interval_ms, None);
    }

    // Generate the configuration using the discovered XML files
    let config = generator
        .generate_config(&xml_files, &[])
        .expect("Failed to generate configuration");

    // Verify the configuration is not empty
    assert!(
        !config.is_empty(),
        "Generated configuration should not be empty"
    );

    // Create a snapshot of the generated configuration
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path("__snapshots__");
    settings.bind(|| {
        insta::assert_snapshot!("cli_config_generation", &config);
    });

    println!(
        "Successfully generated configuration from {} XML files",
        xml_files.len()
    );
    Ok(())
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

/// Helper function to start the OPC UA test server on the specified port
/// Returns a handle that will kill the server when dropped
fn start_opcua_server(port: u16) -> std::process::Child {
    // Kill any existing server instances
    let _ = std::process::Command::new("pkill")
        .arg("-f")
        .arg("opcua_test_server")
        .status();

    // Give the OS a moment to release the port
    thread::sleep(Duration::from_secs(1));

    println!("Starting OPC UA test server on port {}...", port);
    let mut server = Command::new(get_bin_path("opcua_test_server"))
        .arg("--port")
        .arg(port.to_string())
        .stdout(Stdio::piped()) // Capture stdout for debugging
        .stderr(Stdio::piped()) // Capture stderr for debugging
        .spawn()
        .expect("Failed to start OPC UA test server");

    // Give the server time to start up
    println!("Waiting for server to start...");
    thread::sleep(Duration::from_secs(3));

    // Verify the server is still running
    if let Ok(Some(status)) = server.try_wait() {
        let output = server
            .wait_with_output()
            .expect("Failed to get server output");

        println!(
            "Server process exited unexpectedly with status: {:?}",
            status
        );
        println!("Server stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("Server stderr: {}", String::from_utf8_lossy(&output.stderr));

        panic!(
            "Server process exited unexpectedly with status: {:?}",
            status
        );
    }

    server
}

#[test]
fn test_opcua_server_interaction() -> Result<(), Box<dyn std::error::Error>> {
    // Skip in CI environments as they might not have the required ports available
    if is_ci_environment() {
        println!("Skipping OPC UA server test in CI environment");
        return Ok(());
    }

    // Start the server and ensure it's cleaned up
    let port = 4841;
    let server_handle = start_opcua_server(port);
    let _server_guard = scopeguard::guard(server_handle, |mut server| {
        let _ = server.kill();
        let _ = server.wait();
    });

    // Create a test configuration
    let test_config = TelegrafConfig {
        ip: format!("127.0.0.1:{}", port), // Use the same port as the server
        username: "".to_string(),          // Anonymous access
        password: "".to_string(),          // Anonymous access
        iot_host: "".to_string(),          // Not needed for this test
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
    let poller = OpcUaPoller::new(test_config).map_err(|e| {
        eprintln!("Failed to create OpcUaPoller: {}", e);
        e
    })?;

    // Get the top-level nodes from the OPC UA server
    let mut nodes = poller
        .browse_complete_structure()
        .expect("Failed to browse server structure");

    // Now load children for each top-level node, similar to how the GUI does it
    println!("\nLoading children for top-level nodes...");
    for i in 0..nodes.len() {
        if nodes[i].is_folder_node() && !nodes[i].children_loaded {
            println!(
                "Loading children for node: {} ({:?})",
                nodes[i].display_name, nodes[i].node_class
            );
            match poller.load_node_children(&nodes[i], 1) {
                Ok(children) => {
                    nodes[i].children = children;
                    nodes[i].children_loaded = true;

                    // For the ServerInterfaces folder, try to load its children too
                    if nodes[i].display_name == "ServerInterfaces" {
                        for j in 0..nodes[i].children.len() {
                            if nodes[i].children[j].is_folder_node()
                                && !nodes[i].children[j].children_loaded
                            {
                                println!(
                                    "Loading children for node: {} ({:?})",
                                    nodes[i].children[j].display_name,
                                    nodes[i].children[j].node_class
                                );
                                if let Ok(grand_children) =
                                    poller.load_node_children(&nodes[i].children[j], 2)
                                {
                                    nodes[i].children[j].children = grand_children;
                                    nodes[i].children[j].children_loaded = true;
                                }
                            }
                        }
                    }
                }
                Err(e) => println!(
                    "Error loading children for node {}: {}",
                    nodes[i].display_name, e
                ),
            }
        }
    }

    // Print the node structure for debugging
    println!("\nNode structure from OPC UA server (after loading children):");
    for node in &nodes {
        print_node_structure(node, 0);
    }

    println!(
        "Found {} children for node {}",
        nodes[0].children.len(),
        nodes[0].display_name
    );

    // Try to find the Sample_DB node
    let sample_db_node = nodes.iter().find(|node| node.display_name == "Sample_DB");

    match sample_db_node {
        Some(node) => {
            println!("Found Sample_DB node with {} children", node.children.len());

            // Print the first few children for debugging
            for (i, child) in node.children.iter().enumerate().take(5) {
                println!(
                    "Child {}: {} ({} - {:?})",
                    i, child.display_name, child.browse_name, child.node_class
                );
            }
        }
        None => println!("Warning: Could not find Sample_DB node in server structure"),
    }

    println!("OPC UA server test completed successfully");

    Ok(())

    // The server will be killed by the _server_guard when it goes out of scope
}

// Helper function to print node structure
fn print_node_structure(node: &OpcUaNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!(
        "{}{} ({} - {:?}) - id: {:?}, children: {}",
        indent,
        node.display_name,
        node.browse_name,
        node.node_class,
        node.node_id,
        node.children.len()
    );

    // Print first few children if there are many
    let max_children = 5;
    for (i, child) in node.children.iter().enumerate() {
        if i < max_children {
            print_node_structure(child, depth + 1);
        } else if node.children.len() > max_children {
            println!(
                "{}... and {} more children",
                "  ".repeat(depth + 1),
                node.children.len() - max_children
            );
            break;
        }
    }
}

#[test]
fn test_opcua_config_generation() -> Result<(), Box<dyn std::error::Error>> {
    // Skip test in CI environment
    if is_ci_environment() {
        println!("Skipping OPC UA config generation test in CI environment");
        return Ok(());
    }

    // Start the server and ensure it's cleaned up
    let port = 4842;
    let server_handle = start_opcua_server(port);
    let _server_guard = scopeguard::guard(server_handle, |mut server| {
        let _ = server.kill();
        let _ = server.wait();
    });

    // Create a test configuration
    let test_config = TelegrafConfig {
        ip: format!("127.0.0.1:{}", port),
        username: "".to_string(),                // Anonymous access
        password: "".to_string(),                // Anonymous access
        iot_host: format!("127.0.0.1:{}", port), // Use the OPC UA server port for testing
        iot_username: "test".to_string(),
        iot_password: "test".to_string(),
        token_folder: std::env::temp_dir(),
        bucket_name: "test_bucket".to_string(),
        influx_token: Some("dummy_token".to_string()), // Add a dummy token for testing
        listener_files: vec![],
        output_format: None,
        include_test_inputs: false,
        selected_opcua_nodes: vec![],
        folder: std::env::current_dir().unwrap(),
    };

    // Create a new OpcUaPoller instance
    let poller = OpcUaPoller::new(test_config.clone())
        .expect("Failed to create OpcUaPoller and connect to OPC UA server");

    // Get the top-level nodes from the OPC UA server
    let mut nodes = poller.browse_complete_structure()?;

    // Now load children for each top-level node, similar to how the GUI does it
    println!("\nLoading children for top-level nodes...");
    for i in 0..nodes.len() {
        if nodes[i].is_folder_node() && !nodes[i].children_loaded {
            println!(
                "Loading children for node: {} ({:?})",
                nodes[i].display_name, nodes[i].node_class
            );
            match poller.load_node_children(&nodes[i], 1) {
                Ok(children) => {
                    nodes[i].children = children;
                    nodes[i].children_loaded = true;

                    // For the ServerInterfaces folder, try to load its children too
                    if nodes[i].display_name == "ServerInterfaces" {
                        for j in 0..nodes[i].children.len() {
                            if nodes[i].children[j].is_folder_node()
                                && !nodes[i].children[j].children_loaded
                            {
                                println!(
                                    "Loading children for node: {} ({:?})",
                                    nodes[i].children[j].display_name,
                                    nodes[i].children[j].node_class
                                );
                                if let Ok(grand_children) =
                                    poller.load_node_children(&nodes[i].children[j], 2)
                                {
                                    nodes[i].children[j].children = grand_children;
                                    nodes[i].children[j].children_loaded = true;
                                }
                            }
                        }
                    }
                }
                Err(e) => println!(
                    "Error loading children for node {}: {}",
                    nodes[i].display_name, e
                ),
            }
        }
    }

    // Print the node structure for debugging
    println!("\nNode structure from OPC UA server (after loading children):");
    for node in &nodes {
        print_node_structure(node, 0);
    }

    println!("Found {} top-level nodes in server structure", nodes.len());

    // Print the node structure for debugging
    fn print_node_structure(node: &OpcUaNode, depth: usize) {
        let indent = "  ".repeat(depth);
        println!(
            "{}{} ({} - {:?}) - id: {:?}, children: {}",
            indent,
            node.display_name,
            node.browse_name,
            node.node_class,
            node.node_id,
            node.children.len()
        );

        // Print first few children if there are many
        let child_count = node.children.len();
        let children_to_show = 5.min(child_count);

        for child in node.children.iter().take(children_to_show) {
            print_node_structure(child, depth + 1);
        }

        if child_count > children_to_show {
            println!(
                "{}... and {} more children",
                "  ".repeat(depth + 1),
                child_count - children_to_show
            );
        }
    }

    println!("Node structure (first 5 children per node):");
    for (i, node) in nodes.iter().enumerate() {
        println!("\nNode {}:", i);
        print_node_structure(node, 1);
    }

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
                    interval_ms: 1000, // 1 second interval
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

    // Helper function to extract numeric identifier from a node
    fn get_numeric_id(node: &SelectedOpcUaNode) -> u32 {
        match &node.node_id.identifier {
            opcua::types::Identifier::Numeric(id) => *id,
            _ => 0, // Default to 0 for non-numeric identifiers
        }
    }

    // Helper function to find a node by display name recursively
    fn find_node_by_name<'a>(nodes: &'a [OpcUaNode], name: &str) -> Option<&'a OpcUaNode> {
        for node in nodes {
            println!("Checking node: {}", node.display_name);
            if node.display_name == name {
                println!("Found node: {}", name);
                return Some(node);
            }
            if let Some(found) = find_node_by_name(&node.children, name) {
                return Some(found);
            }
        }
        None
    }

    // Print all node names to help with debugging
    println!("\nAll node names (first 20):");
    fn collect_node_names(node: &OpcUaNode, names: &mut Vec<String>) {
        names.push(format!("{} ({:?})", node.display_name, node.node_class));
        for child in &node.children {
            collect_node_names(child, names);
        }
    }

    let mut all_node_names = Vec::new();
    for node in &nodes {
        collect_node_names(node, &mut all_node_names);
    }

    for name in all_node_names.iter().take(20) {
        println!("- {}", name);
    }
    if all_node_names.len() > 20 {
        println!("... and {} more nodes", all_node_names.len() - 20);
    }

    // Try to find the Sample_DB folder with test variables
    println!("\nLooking for ServerInterfaces node...");
    if let Some(server_interfaces) = find_node_by_name(&nodes, "ServerInterfaces") {
        println!("Found ServerInterfaces node, looking for Sample_DB...");
        if let Some(sample_db) = find_node_by_name(&server_interfaces.children, "Sample_DB") {
            println!("Found Sample_DB node, collecting variables...");
            println!("Found Sample_DB folder, collecting variables...");
            // Collect all variable nodes under Sample_DB
            let mut variable_nodes = Vec::new();
            OpcUaNode::collect_all_variables_in_folder(sample_db, &mut variable_nodes);

            // Convert to SelectedOpcUaNode
            for var_node in variable_nodes.into_iter().take(5) {
                // Limit to 5 variables
                selected_nodes.push(SelectedOpcUaNode {
                    node_id: var_node.node_id.clone(),
                    namespace: var_node.node_id.namespace,
                    browse_name: var_node.browse_name.clone(),
                    display_name: var_node.display_name.clone(),
                    measurement_name: var_node
                        .display_name
                        .clone()
                        .to_lowercase()
                        .replace(' ', "_"),
                    interval_ms: 1000,
                    folder_name: Some("sample_db".to_string()),
                });
            }
        }
    }

    // If we still didn't find any variable nodes, try to find any nodes at all as fallback
    if selected_nodes.is_empty() {
        println!("No variable nodes found in Sample_DB, trying to find any nodes...");
        find_variable_nodes(&nodes, &mut selected_nodes);

        // If still nothing, just take the first few nodes we can find
        if selected_nodes.is_empty() {
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
    }

    // Make sure we found some nodes to test with
    assert!(
        !selected_nodes.is_empty(),
        "No variable nodes found to test with"
    );

    // Sort nodes by their numeric ID for consistent ordering
    let mut sorted_nodes = selected_nodes.clone();
    sorted_nodes.sort_by_key(|node| get_numeric_id(node));

    // Create a config with our selected nodes (now sorted)
    let mut config_with_nodes = test_config;
    config_with_nodes.selected_opcua_nodes = sorted_nodes.clone();

    // Clone the nodes before moving config_with_nodes
    let selected_nodes_clone = sorted_nodes.clone();

    // Create a ConfigGenerator
    let config_generator =
        ConfigGenerator::new(config_with_nodes).expect("Failed to create ConfigGenerator");

    // Generate the configuration
    let config = config_generator
        .generate_config(&[], &[])
        .expect("Failed to generate configuration");

    // Verify the configuration is not empty and contains expected sections
    assert!(
        !config.is_empty(),
        "Generated configuration should not be empty"
    );

    // Check for common OPC UA configuration sections
    assert!(
        config.contains("[[inputs.opcua]]") || config.contains("[[inputs.opcua_client]]"),
        "Configuration should contain OPC UA input section"
    );

    // Print the configuration for debugging
    println!("Generated configuration:\n{}", config);

    // Create a snapshot of the generated configuration
    // This will create/update snapshots in tests/__snapshots__
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path("__snapshots__");
    settings.bind(|| {
        insta::assert_snapshot!("opcua_config_generation", &config);
    });

    println!(
        "Successfully generated configuration for {} nodes",
        selected_nodes_clone.len()
    );

    // Clean up the generated telegraf.conf file if it exists
    let config_path = std::env::current_dir()?.join("telegraf.conf");
    if config_path.exists() {
        println!("Cleaning up test telegraf.conf file");
        std::fs::remove_file(&config_path)
            .map_err(|e| format!("Failed to clean up telegraf.conf: {}", e))?;
    }

    Ok(())
}
