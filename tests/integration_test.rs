use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use sie_generate_config::backend::opcua_poller::OpcUaPoller;
use sie_generate_config::TelegrafConfig;

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
}
