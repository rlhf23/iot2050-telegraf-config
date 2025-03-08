use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

// This is a basic integration test that tests the CLI application with arguments
#[test]
fn test_cli_help() {
    let output = Command::new(get_bin_path("sie_generate_config"))
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Check that help output contains expected information
    assert!(stdout.contains("IOT2050 config handler"));
    assert!(stdout.contains("--folder"));
    assert!(stdout.contains("--ip"));
    assert!(stdout.contains("--output-format"));
}

// Test the CLI with a simple XML file
#[test]
fn test_cli_with_xml_file() {
    // Create a temporary directory for our test
    let dir = tempdir().expect("Failed to create temp dir");
    let xml_path = dir.path().join("test.xml");
    
    // Create a test XML file
    let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
    <UANodeSet>
        <UAObject NodeId="ns=2;i=1">
            <DisplayName>TestDevice</DisplayName>
        </UAObject>
        <UAVariable NodeId="ns=2;i=2">
            <BrowseName>Temperature</BrowseName>
        </UAVariable>
    </UANodeSet>"#;
    
    let mut file = File::create(&xml_path).expect("Failed to create XML file");
    file.write_all(xml_content.as_bytes()).expect("Failed to write XML content");
    
    // Create a token file
    let token_path = dir.path().join("token.txt");
    let mut token_file = File::create(&token_path).expect("Failed to create token file");
    token_file.write_all(b"test_token").expect("Failed to write token");
    
    // Run the CLI with our test files
    // Note: We're using --test-inputs so we don't need actual XML files to generate a valid config
    let output = Command::new(get_bin_path("sie_generate_config"))
        .arg("-f")
        .arg(dir.path())
        .arg("-t")
        .arg(dir.path())
        .arg("-x") // Enable test inputs
        .arg("-o")
        .arg("prometheus") // Use prometheus output so we don't need a token
        .output()
        .expect("Failed to execute command");
    
    // The command will exit with code 1 because it will ask for confirmation
    // and we're not providing input, but we can still check if it processed the arguments
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Check that it found our XML file
    assert!(stdout.contains("Found the following XML files in the folder"));
    assert!(stdout.contains("test.xml"));
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
