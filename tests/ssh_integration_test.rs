use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tempfile::tempdir;
use tokio::fs;

use sie_generate_config::backend::ssh_utils;
use test_utils::container_setup;

mod test_utils;

// Test configuration
const TEST_USER: &str = "testuser";
const TEST_PASSWORD: &str = "testpass";
const REMOTE_PATH: &str = "/tmp/telegraf.conf";

#[tokio::test]
async fn test_ssh_operations() {
    // Skip test if SSH tests are disabled
    if std::env::var("RUN_SSH_TESTS").is_err() {
        println!("Skipping SSH tests. Set RUN_SSH_TESTS=1 to enable.");
        return;
    }

    // Set up test environment
    let env = container_setup::setup_test_environment().await;
    
    // Test SSH connection
    if let Err(e) = test_ssh_connection(&format!("localhost:{}", env.ssh_port)).await {
        panic!("SSH connection test failed: {}", e);
    }
    
    // Test file transfer
    if let Err(e) = test_file_transfer(&format!("localhost:{}", env.ssh_port)).await {
        panic!("File transfer test failed: {}", e);
    }
}

async fn test_ssh_connection(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let host = "localhost";
    let username = TEST_USER;
    let password = TEST_PASSWORD;
    let timeout_seconds = 30;

    // Get the SSH port from the provided address
    let port = addr.split(':').nth(1).unwrap_or("22");
    let host_with_port = format!("{}:{}", host, port);
    
    println!("Testing SSH connection to {}...", host_with_port);

    // First, check if we can connect via raw TCP
    let tcp_addr = format!("{}:{}", host, port);
    let socket_addr: std::net::SocketAddr = tcp_addr.parse()?;
    
    // Try multiple times to connect as the container might need time to start
    let mut connected = false;
    for _ in 0..10 {
        match std::net::TcpStream::connect_timeout(&socket_addr, std::time::Duration::from_secs(5)) {
            Ok(_) => {
                connected = true;
                break;
            }
            Err(e) => {
                println!("TCP connection attempt failed: {}. Retrying...", e);
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    }

    if !connected {
        return Err(format!("Failed to establish TCP connection to {}", host_with_port).into());
    }

    // Now try SSH connection
    let mut last_error = None;
    for attempt in 1..=5 {
        println!("SSH connection attempt {}...", attempt);
        match ssh_utils::connect_ssh_with_timeout(&host_with_port, username, password, timeout_seconds).await {
            Ok(_) => {
                println!("Successfully connected to SSH server");
                return Ok(());
            }
            Err(e) => {
                println!("SSH connection attempt {} failed: {}", attempt, e);
                last_error = Some(e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }

    Err(format!("Failed to connect to SSH server after multiple attempts: {:?}", 
        last_error.unwrap_or_else(|| Box::new(std::io::Error::new(
            std::io::ErrorKind::Other, 
            "No error details available"
        )))
    ).into())
    ) {
        Ok(s) => s,
        Err(e) => {
            // Try to get more detailed error information
            let output = std::process::Command::new("ssh")
                .args(["-v"])
                .args(["-o", "BatchMode=yes"])
                .args(["-o", "StrictHostKeyChecking=no"])
                .args(["-p", port])
                .arg(&format!("{}@{}", TEST_USER, ip))
                .arg("echo test")
                .output()
                .unwrap_or_else(|_| std::process::Output {
                    status: std::process::ExitStatus::default(),
                    stdout: vec![],
                    stderr: b"Failed to execute SSH command".to_vec(),
                });
                
            eprintln!("SSH connection failed. Debug output:");
            eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
            return Err(e.into());
        }
    };
    
    if !session.authenticated() {
        return Err("SSH session not authenticated".into());
    }
    
    println!("SSH authentication successful");
    Ok(())
}

async fn test_file_transfer(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create a test file
    let temp_dir = tempdir()?;
    let test_content = "test content";
    let local_path = temp_dir.path().join("test.txt");
    fs::write(&local_path, test_content).await?;

    // Get just the IP part of the address
    let ip = addr.split(':').next().unwrap();
    let port = addr.split(':').nth(1).unwrap_or("22");
    
    // Create the remote directory if it doesn't exist
    let _ = Command::new("sshpass")
        .args(["-p", TEST_PASSWORD])
        .arg("ssh")
        .args(["-o", "StrictHostKeyChecking=no"])
        .args(["-p", port])
        .arg(&format!("{}@{}", TEST_USER, ip))
        .arg("mkdir -p $(dirname /tmp/telegraf.conf)")
        .output()?;

    // Transfer file
    println!("Transferring file to {}:{}...", ip, port);
    let result = ssh_utils::send_file_over_ssh(
        &local_path,
        REMOTE_PATH,
        &format!("{}:{}", ip, port),
        TEST_USER,
        TEST_PASSWORD,
    );

    if let Err(e) = &result {
        eprintln!("Failed to transfer file: {}", e);
        return result.map_err(|e| e.into());
    }

    // Verify file was transferred
    let output = Command::new("sshpass")
        .args(["-p", TEST_PASSWORD])
        .arg("ssh")
        .args(["-o", "StrictHostKeyChecking=no"])
        .args(["-p", port])
        .arg(&format!("{}@{}", TEST_USER, ip))
        .arg(&format!("cat {}", REMOTE_PATH))
        .output()?;

    if !output.status.success() {
        eprintln!("Failed to read remote file: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Failed to verify file transfer".into());
    }

    let remote_content = String::from_utf8_lossy(&output.stdout);
    if remote_content != test_content {
        eprintln!("Content mismatch. Expected: '{}', Got: '", test_content);
        eprintln!("Remote content: '{}'", remote_content);
        return Err("File content does not match".into());
    }

    println!("File transfer test passed");
    Ok(())
}
