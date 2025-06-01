use std::process::Command;
use std::sync::Once;
use std::time::Duration;
use tokio::time::sleep;

static INIT: Once = Once::new();

#[derive(Debug)]
pub struct TestEnvironment {
    pub ssh_port: u16,
    pub influx_port: u16,
    pub prometheus_port: u16,
    pub node_exporter_port: u16,
    pub container_id: String,
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Clean up the container when the test environment is dropped
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container_id])
            .output()
            .map_err(|e| eprintln!("Failed to remove container: {}", e));
    }
}

pub async fn setup_test_environment() -> TestEnvironment {
    INIT.call_once(|| {
        // Build the test container
        let build_status = Command::new("docker")
            .args(["build", "-t", "iot2050-test-env", "-f", "tests/Dockerfile.test", "."])
            .status()
            .expect("Failed to execute docker build");
        
        if !build_status.success() {
            panic!("Failed to build test container");
        }
    });

    // Remove any existing container with the same name
    let _ = Command::new("docker")
        .args(["rm", "-f", "iot2050-test-container"])
        .status();

    // Start the container
    let output = Command::new("docker")
        .args([
            "run", "-d",
            "--name", "iot2050-test-container",
            "-p", "0:22",    // Auto-assign port for SSH
            "-p", "0:8086",   // Auto-assign port for InfluxDB
            "-p", "0:9090",   // Auto-assign port for Prometheus
            "-p", "0:9273",   // Auto-assign port for Node Exporter
            "--privileged",   // Required for systemd
            "--tmpfs", "/run",
            "--tmpfs", "/run/lock",
            "--tmpfs", "/tmp",
            "--cgroupns=host",
            "--volume", "/sys/fs/cgroup:/sys/fs/cgroup:ro",
            "--health-cmd", "ss -tuln | grep -q ':22 .*LISTEN' && \
                            ss -tuln | grep -q ':8086 .*LISTEN' && \
                            ss -tuln | grep -q ':9090 .*LISTEN' && \
                            ss -tuln | grep -q ':9273 .*LISTEN' && \
                            systemctl is-active --quiet ssh && \
                            systemctl is-active --quiet influxdb && \
                            systemctl is-active --quiet prometheus",
            "--health-interval", "5s",
            "--health-timeout", "30s",
            "--health-retries", "6",
            "iot2050-test-env"
        ])
        .output()
        .expect("Failed to start test container");

    if !output.status.success() {
        panic!("Failed to start container: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Get the container ID
    let output = Command::new("docker")
        .args(["ps", "-q", "--filter", "name=iot2050-test-container"])
        .output()
        .expect("Failed to get container ID");
    
    let container_id = String::from_utf8(output.stdout)
        .expect("Invalid UTF-8 in container ID")
        .trim()
        .to_string();
    
    if container_id.is_empty() {
        panic!("Failed to get container ID");
    }

    // Wait for container to be healthy
    let mut healthy = false;
    for _ in 0..30 {  // Wait up to 30 seconds
        let status = Command::new("docker")
            .args(["inspect", "--format", "{{.State.Health.Status}}", &container_id])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            });
        
        if let Some(status) = status {
            if status == "healthy" {
                healthy = true;
                break;
            }
        }
        
        sleep(Duration::from_secs(1)).await;
    }
    
    if !healthy {
        // Get container logs for debugging
        let logs = Command::new("docker")
            .args(["logs", &container_id])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
            .unwrap_or_else(|_| "Failed to get container logs".to_string());
        
        panic!("Container did not become healthy. Logs:\n{}", logs);
    }

    // Get the mapped ports
    let get_port = |port: u16| -> u16 {
        let output = Command::new("docker")
            .args(["port", &container_id, &port.to_string()])
            .output()
            .expect("Failed to get port mapping");
        
        if !output.status.success() {
            panic!("Failed to get port mapping: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        let port_str = String::from_utf8(output.stdout)
            .expect("Invalid UTF-8 in port mapping");
        
        port_str.trim_end()
            .split(':')
            .last()
            .expect("Invalid port mapping format")
            .parse()
            .expect("Invalid port number")
    };
    
    let ssh_port = get_port(22);
    let influx_port = get_port(8086);
    let prometheus_port = get_port(9090);
    let node_exporter_port = get_port(9273);
    
    println!("Test environment started with the following ports:");
    println!("Container ID: {}", container_id);
    println!("SSH: {}", ssh_port);
    println!("InfluxDB: {}", influx_port);
    println!("Prometheus: {}", prometheus_port);
    println!("Node Exporter: {}", node_exporter_port);
    
    // Wait for services to be ready
    println!("Waiting for services to be ready...");
    thread::sleep(Duration::from_secs(10));
    
    TestEnvironment {
        ssh_port,
        influx_port,
        prometheus_port,
        node_exporter_port,
        container_id,
    }
}