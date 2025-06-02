use std::process::Command;

#[test]
fn test_container_is_running() {
    // Check if the container is running
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=iot2050-test-container", "--format", "{{.Names}}"])
        .output()
        .expect("Failed to execute docker ps");

    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        output_str.trim() == "iot2050-test-container",
        "Container is not running. Output: {}",
        output_str
    );

    // Check if SSH is accessible
    let ssh_output = Command::new("sshpass")
        .args(["-p", "testpass"])
        .arg("ssh")
        .args(["-o", "StrictHostKeyChecking=no"])
        .args(["-o", "ConnectTimeout=5"])
        .arg("testuser@localhost")
        .args(["-p", "2222"])
        .arg("echo test")
        .output();

    assert!(
        ssh_output.is_ok(),
        "Failed to connect to SSH: {:?}",
        ssh_output.err()
    );

    let ssh_output = ssh_output.unwrap();
    assert!(
        ssh_output.status.success(),
        "SSH command failed: {}",
        String::from_utf8_lossy(&ssh_output.stderr)
    );
}
