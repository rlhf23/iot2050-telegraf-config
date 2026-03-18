//! SSH utilities for remote IoT device communication
//!
//! This module provides functions for establishing SSH connections to IoT devices,
//! transferring files, executing commands, and managing services like Telegraf.
//!
//! # Examples
//!
//! ```no_run
//! // Example usage is available through the public API in ConfigGenerator
//! use std::path::Path;
//! // Public functions are re-exported through the backend module
//! ```

use crate::error::TelegrafError;
use log::debug;
use ssh2::Session;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

/// Checks if a service is running in a Docker container
fn is_service_containerized(session: &Session, service_name: &str) -> Result<bool, TelegrafError> {
    let check_cmd = format!("if command -v docker >/dev/null 2>&1 && docker ps --format '{{{{.Names}}}}' | grep -q '^{}$'; then echo 'containerized'; fi", service_name);
    let output = execute_ssh_command(session, &check_cmd)?;
    Ok(output.contains("containerized"))
}

/// Checks if Telegraf is running in a Docker container
fn is_telegraf_containerized(session: &Session) -> Result<bool, TelegrafError> {
    is_service_containerized(session, "telegraf")
}

/// Checks if InfluxDB is running in a Docker container
fn is_influxdb_containerized(session: &Session) -> Result<bool, TelegrafError> {
    is_service_containerized(session, "influxdb")
}
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

/// SSH connection configuration with separate timeout values
#[derive(Debug, Clone)]
pub struct SshConfig {
    /// Timeout for establishing TCP connection (seconds)
    pub connect_timeout: u64,
    /// Timeout for SSH operations like file transfer, command execution (seconds)  
    pub operation_timeout: u64,
    /// Timeout for reading/writing data streams (seconds)
    pub stream_timeout: u64,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            connect_timeout: 3, // Host is either there or it isn't - keep connection attempts short
            operation_timeout: 60, // SSH operations like file transfer, command execution
            stream_timeout: 15, // Data stream read/write operations
        }
    }
}

/// Validates that a host string follows the required hostname:port format
///
/// # Arguments
/// * `host` - The host string to validate (e.g., "192.168.1.2:22")
///
/// # Returns
/// * `Ok(())` if the host format is valid
/// * `Err(TelegrafError::HostFormatError)` if the format is invalid
///
/// # Examples
/// ```
/// # // Internal function - examples would require module to be public
/// # // assert!(validate_host_format("192.168.1.2:22").is_ok());
/// # // assert!(validate_host_format("invalid").is_err());
/// ```
fn validate_host_format(host: &str) -> Result<(), TelegrafError> {
    // IPv6 with brackets: [::1]:22
    if host.starts_with('[') {
        // Find closing bracket
        if let Some(end_bracket) = host.find(']') {
            // Must be followed by colon and port
            if host.len() > end_bracket + 1 && &host[end_bracket + 1..end_bracket + 2] == ":" {
                let port_part = &host[end_bracket + 2..];
                match port_part.parse::<u16>() {
                    Ok(port) if port > 0 => return Ok(()),
                    _ => {
                        return Err(TelegrafError::HostFormatError(format!(
                            "Invalid port in host: '{}'. Port must be a number between 1-65535",
                            host
                        )))
                    }
                }
            } else {
                return Err(TelegrafError::HostFormatError(format!(
                    "Invalid IPv6 host format: '{}'. Expected format: [IPv6]:port (e.g., [::1]:22)",
                    host
                )));
            }
        } else {
            return Err(TelegrafError::HostFormatError(format!(
                "Invalid IPv6 host format: '{}'. Missing closing bracket.",
                host
            )));
        }
    } else if host.contains(':') {
        // If more than one colon and not in brackets, treat as invalid (unbracketed IPv6)
        let colon_count = host.chars().filter(|&c| c == ':').count();
        if colon_count > 1 {
            return Err(TelegrafError::HostFormatError(format!(
                "Invalid host format: '{}'. IPv6 addresses must be enclosed in brackets.",
                host
            )));
        }
        // Split by colon and validate format
        let host_parts: Vec<&str> = host.split(':').collect();
        if host_parts.len() != 2 {
            return Err(TelegrafError::HostFormatError(format!(
                "Invalid host format: '{}'. Expected format: hostname:port (e.g., 192.168.0.1:22)",
                host
            )));
        }
        // Validate that the port is a valid number
        match host_parts[1].parse::<u16>() {
            Ok(port) if port > 0 => Ok(()),
            _ => Err(TelegrafError::HostFormatError(format!(
                "Invalid port in host: '{}'. Port must be a number between 1-65535",
                host
            ))),
        }
    } else {
        Err(TelegrafError::HostFormatError(format!(
            "Missing port specification in host '{}'. Expected format: hostname:port (e.g., 192.168.0.1:22)",
            host
        )))
    }
}

/// Exposed version of validate_host_format for testing
#[cfg(test)]
pub fn validate_host_format_exposed_for_testing(host: &str) -> Result<(), TelegrafError> {
    validate_host_format(host)
}

/// Establishes an SSH connection with configurable timeouts
///
/// # Arguments
/// * `host` - The remote host in hostname:port format (e.g., "192.168.1.2:22")
/// * `username` - SSH username for authentication
/// * `password` - SSH password for authentication  
/// * `config` - SSH configuration containing timeout values
///
/// # Returns
/// * `Ok(Session)` - Successfully established SSH session
/// * `Err(TelegrafError)` - Connection failed with specific error details
fn connect_ssh_with_config(
    host: &str,
    username: &str,
    password: &str,
    config: &SshConfig,
) -> Result<Session, TelegrafError> {
    // First validate host format before attempting connection
    // Use the dedicated validation helper
    validate_host_format(host)?;

    // Use the provided host (already has port)
    let host_with_port = host.to_string();

    debug!(
        "Connecting to {} with connect_timeout={}s, operation_timeout={}s, stream_timeout={}s",
        host_with_port, config.connect_timeout, config.operation_timeout, config.stream_timeout
    );

    // Set up TCP connection with timeout
    let socket_addrs = match host_with_port.to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(e) => {
            return Err(TelegrafError::HostFormatError(format!(
                "Failed to resolve hostname '{}': {}",
                host, e
            )))
        }
    };

    for socket_addr in socket_addrs {
        match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(config.connect_timeout))
        {
            Ok(tcp) => {
                // Configure TCP stream with stream timeout (separate from connection timeout)
                if let Err(e) =
                    tcp.set_read_timeout(Some(Duration::from_secs(config.stream_timeout)))
                {
                    debug!("Failed to set read timeout: {}", e);
                    continue;
                }

                if let Err(e) =
                    tcp.set_write_timeout(Some(Duration::from_secs(config.stream_timeout)))
                {
                    debug!("Failed to set write timeout: {}", e);
                    continue;
                }

                // Create and configure SSH session
                let mut session = match Session::new() {
                    Ok(s) => s,
                    Err(e) => {
                        debug!("Failed to create SSH session: {}", e);
                        continue;
                    }
                };

                session.set_tcp_stream(tcp);
                // Use operation timeout for SSH session operations (convert to milliseconds)
                session.set_timeout((config.operation_timeout * 1000).try_into().unwrap());

                // Perform handshake and authentication
                match session.handshake() {
                    Ok(_) => match session.userauth_password(username, password) {
                        Ok(_) => return Ok(session),
                        Err(e) => {
                            debug!("Authentication failed: {}", e);
                            return Err(TelegrafError::SshError(crate::error::SshError::Auth(
                                crate::error::SshAuthError::InvalidCredentials(e),
                            )));
                        }
                    },
                    Err(e) => {
                        debug!("Handshake failed: {}", e);
                        continue;
                    }
                }
            }
            Err(e) => {
                debug!("Connection timeout: {}", e);
                // Connection failed, try next address
            }
        }
    }

    // If we get here, all connection attempts failed
    Err(TelegrafError::SshError(crate::error::SshError::Connection(
        crate::error::SshConnectionError::NetworkUnreachable(ssh2::Error::new(
            ssh2::ErrorCode::Session(-1),
            "Connection failed",
        )),
    )))
}

/// Backward-compatible wrapper for connect_ssh_with_config using default timeouts
///
/// # Arguments  
/// * `host` - The remote host in hostname:port format
/// * `username` - SSH username for authentication
/// * `password` - SSH password for authentication
/// * `timeout_seconds` - Timeout in seconds (used for all timeout types)
///
/// # Returns
/// * `Ok(Session)` - Successfully established SSH session
/// * `Err(TelegrafError)` - Connection failed with specific error details
fn connect_ssh_with_timeout(
    host: &str,
    username: &str,
    password: &str,
    timeout_seconds: u64,
) -> Result<Session, TelegrafError> {
    let config = SshConfig {
        connect_timeout: timeout_seconds,
        operation_timeout: timeout_seconds,
        stream_timeout: timeout_seconds,
    };
    connect_ssh_with_config(host, username, password, &config)
}

/// Sends a Telegraf configuration file to an IoT device and restarts the service
///
/// This is a convenience function that combines file transfer and service restart
/// in a single operation for deploying Telegraf configurations.
///
/// # Arguments
/// * `config_path` - Local path to the Telegraf configuration file
/// * `remote_path` - Remote destination path for the configuration file
/// * `iot_host` - IoT device host in hostname:port format
/// * `iot_username` - SSH username for the IoT device
/// * `iot_password` - SSH password for the IoT device
///
/// # Returns
/// * `Ok(String)` - Configuration deployed and service restarted successfully with detailed output
/// * `Err(TelegrafError)` - File transfer or service restart failed
pub fn send_and_restart_telegraf_with_progress(
    config_path: &Path,
    remote_path: &str,
    iot_host: &str,
    iot_username: &str,
    iot_password: &str,
    progress_sender: Sender<String>,
) -> Result<(), TelegrafError> {
    // Try to send the progress update, but if it fails, continue to try the SSH operation first
    let _ = progress_sender.send("Sending configuration file...".to_string());
    // Try SSH operation, return SSH error if it fails
    if let Err(e) = send_file_over_ssh(
        config_path,
        remote_path,
        iot_host,
        iot_username,
        iot_password,
    ) {
        return Err(e);
    }
    let _ = progress_sender.send("Configuration file sent successfully.".to_string());
    let _ = progress_sender.send("Restarting Telegraf service...".to_string());
    // Try SSH restart, return SSH error if it fails
    let restart_output = match restart_telegraf_over_ssh(iot_host, iot_username, iot_password) {
        Ok(out) => out,
        Err(e) => return Err(e),
    };
    let _ = progress_sender.send(restart_output);
    Ok(())
}

pub fn send_and_restart_telegraf(
    config_path: &Path,
    remote_path: &str,
    iot_host: &str,
    iot_username: &str,
    iot_password: &str,
) -> Result<String, TelegrafError> {
    let mut output = Vec::new();

    // Send the telegraf.conf file to the IOT box
    output.push("Sending configuration file...".to_string());
    send_file_over_ssh(
        config_path,
        remote_path,
        iot_host,
        iot_username,
        iot_password,
    )?;
    output.push("Configuration file sent successfully.".to_string());

    // Restart the telegraf service on the IOT box
    let restart_output = restart_telegraf_over_ssh(iot_host, iot_username, iot_password)?;
    output.push(restart_output);

    Ok(output.join("\n"))
}

/// Sends a file over SSH to a specified remote host using SCP
///
/// # Arguments
/// * `local_path` - Path to the local file to transfer
/// * `remote_path` - Destination path on the remote host
/// * `remote_host` - Remote host in hostname:port format
/// * `username` - SSH username for authentication
/// * `password` - SSH password for authentication
///
/// # Returns
/// * `Ok(())` - File transferred successfully
/// * `Err(TelegrafError)` - Connection failed or file transfer failed
pub fn send_file_over_ssh(
    local_path: &Path,
    remote_path: &str,
    remote_host: &str,
    username: &str,
    password: &str,
) -> Result<(), TelegrafError> {
    debug!("Sending file over SSH to {}", remote_host);

    // Connect to SSH with appropriate timeouts for file transfer operations
    let config = SshConfig {
        connect_timeout: 3, // Quick connection check - host is either there or it isn't
        operation_timeout: 120, // Allow time for file transfer operations
        stream_timeout: 30, // Reasonable timeout for file data transfer
    };
    let session = connect_ssh_with_config(remote_host, username, password, &config)?;

    // Open a new SCP session and send the file
    let mut remote_file = session.scp_send(
        Path::new(remote_path),
        0o644,
        local_path.metadata()?.len(),
        None,
    )?;
    let mut local_file = std::fs::File::open(local_path)?;

    // Read local file content
    let mut contents = Vec::new();
    local_file.read_to_end(&mut contents)?;

    // Write content to remote file
    debug!("Uploading file ({} bytes)...", contents.len());
    remote_file.write_all(&contents)?;

    debug!("File upload completed successfully");
    Ok(())
}

/// Executes a command on an established SSH session and returns the output
///
/// # Arguments
/// * `session` - Active SSH session to execute the command on
/// * `command` - Shell command to execute on the remote host
///
/// # Returns
/// * `Ok(String)` - Command output if execution successful (exit status 0)
/// * `Err(TelegrafError)` - Command failed or returned non-zero exit status
///
/// # Examples
/// ```no_run
/// # // Internal function - examples would require module to be public
/// # // let session = todo!(); // Assume we have an established session
/// # // let output = execute_ssh_command(&session, "ls -la")?;
/// # // debug!("Directory listing: {}", output);
/// ```
fn execute_ssh_command(session: &Session, command: &str) -> Result<String, TelegrafError> {
    let mut channel = session.channel_session()?;
    channel.exec(command)?;

    // Read stdout
    let mut output = String::new();
    channel.read_to_string(&mut output)?;

    // Check exit status
    channel.wait_close()?;
    let exit_status = channel.exit_status()?;

    // If command failed, return error with exit status
    if exit_status != 0 {
        return Err(crate::error::ssh_error_from_string(format!(
            "Command failed with exit status {}: {}",
            exit_status, output
        )));
    }

    // Print for debugging
    debug!("SSH command output length: {}", output.len());
    debug!(
        "First 100 chars: {}",
        if output.len() > 100 {
            &output[..100]
        } else {
            &output
        }
    );

    Ok(output)
}

/// Restarts Telegraf when running as a Docker container on the remote host
pub fn restart_telegraf_docker_over_ssh(session: &Session) -> Result<String, TelegrafError> {
    debug!("Attempting to restart Telegraf Docker container...");
    let command = "docker restart telegraf";
    match execute_ssh_command(session, command) {
        Ok(output) => {
            debug!(
                "Telegraf Docker container restart command executed. Output: {}",
                output
            );
            Ok(format!(
                "Telegraf Docker container restarted successfully.\nOutput: {}",
                output
            ))
        }
        Err(e) => {
            eprintln!("Failed to restart Telegraf Docker container: {:?}", e);
            // Propagate the error from execute_ssh_command, which is already a TelegrafError
            Err(e)
        }
    }
}

pub fn restart_telegraf_over_ssh(
    remote_host: &str,
    username: &str,
    password: &str,
) -> Result<String, TelegrafError> {
    // Connect to SSH
    let config = SshConfig {
        connect_timeout: 3,
        operation_timeout: 60,
        stream_timeout: 15,
    };
    let session = connect_ssh_with_config(remote_host, username, password, &config)?;

    // Check if Telegraf is containerized
    if is_telegraf_containerized(&session)? {
        debug!("Telegraf is containerized. Using Docker restart logic.");
        return restart_telegraf_docker_over_ssh(&session);
    }

    debug!("Telegraf is not containerized or check failed. Using system service restart logic.");
    let mut output = Vec::new();
    output.push("Detected system Telegraf".to_string());
    output.push("Restarting telegraf service on the remote host...".to_string());
    let start_time = Instant::now();

    // Define commands for system service
    let (check_cmd, stop_cmd, start_cmd, status_cmd, log_path) = (
        "pgrep telegraf || echo 'not_running'",
        format!("echo '{}' | sudo -S service telegraf stop", password),
        format!("echo '{}' | sudo -S service telegraf start", password),
        "service telegraf status | head -n15",
        "/var/log/telegraf/telegraf.log",
    );

    // Step 1: Try graceful stop first with timeout
    output.push("Stopping telegraf service gracefully...".to_string());

    // First check if telegraf is actually running
    let check_result = execute_ssh_command(&session, check_cmd)?;

    if check_result.trim() == "not_running" {
        output.push("Telegraf is not currently running. Proceeding to start.".to_string());
    } else {
        // Attempt graceful stop
        match execute_ssh_command(&session, &stop_cmd) {
            Ok(_) => output.push("Service stop command issued".to_string()),
            Err(e) => output.push(format!("Warning: Error issuing stop command: {}", e)),
        }

        // Wait up to 10 seconds for telegraf to stop gracefully
        output.push("Waiting up to 10 seconds for telegraf to stop...".to_string());
        let mut stopped = false;
        for i in 0..10 {
            thread::sleep(Duration::from_secs(1));
            let check_result =
                execute_ssh_command(&session, &format!("{} || echo 'stopped'", check_cmd))?;

            if check_result.trim() == "stopped" {
                output.push(format!(
                    "Telegraf stopped gracefully after {} seconds",
                    i + 1
                ));
                stopped = true;
                break;
            }
        }

        // If still running after 10 seconds, forcefully kill it
        if !stopped {
            output.push(
                "Telegraf didn't stop gracefully within 10 seconds. Forcing stop...".to_string(),
            );
            let kill_cmd = format!("echo '{}' | sudo -S pkill -9 telegraf", password);

            match execute_ssh_command(&session, &kill_cmd) {
                Ok(_) => output.push("Telegraf process killed forcefully".to_string()),
                Err(e) => output.push(format!("Warning: Error killing telegraf: {}", e)),
            }

            // Give a moment for the kill to take effect
            thread::sleep(Duration::from_secs(1));
        }
    }

    // Step 2: Start the service
    output.push("Starting telegraf service...".to_string());
    execute_ssh_command(&session, &start_cmd)?;

    // Step 3: Wait for service to initialize
    output.push("Waiting for service to initialize...".to_string());
    thread::sleep(Duration::from_secs(5));

    // Step 4: Check service status
    output.push("Checking service status...".to_string());
    let status = execute_ssh_command(&session, status_cmd)?;

    let elapsed_time = start_time.elapsed();

    // Check if the process is actually running
    let process_check =
        execute_ssh_command(&session, &format!("{} || echo 'not_running'", check_cmd))?;
    let is_running = process_check.trim() != "not_running";

    // For containerized, we'll consider it successful if the container is running
    // For non-containerized, success is determined by the 'status' command output.
    // 'is_running' (from pgrep) is also implicitly checked by these status strings.
    let is_success = status.contains("active (running)") // Systemd status
        || status.contains("is running") // Init.d status
        || (!status.contains("not running") && !status.contains("dead") && !status.contains("inactive")); // General check for other init systems

    if is_success {
        output.push(format!(
            "✓ Telegraf restart successful ({:.2?})",
            elapsed_time
        ));
        output.push(format!("Status:\n{}", status));
    } else {
        output.push(format!("✗ Telegraf restart failed ({:.2?})", elapsed_time));
        output.push(format!("Status:\n{}", status));

        if !is_running {
            output.push("WARNING: The telegraf process is not running!".to_string());
        }

        // Get recent logs if service failed
        output.push("\nRecent logs:".to_string());
        let logs_cmd = format!("tail -n 10 {}", log_path);

        match execute_ssh_command(&session, &logs_cmd) {
            Ok(logs) => output.push(logs),
            Err(e) => output.push(format!("Could not retrieve logs: {}", e)),
        }

        // Get error logs if any
        output.push("\nRecent errors:".to_string());
        // Since we are in the non-containerized path, is_containerized is effectively false.
        let errors_cmd = format!("echo '{}' | sudo -S grep -E 'E!' {} 2>/dev/null | tail -n 10 || echo 'No error logs found'", password, log_path);

        match execute_ssh_command(&session, &errors_cmd) {
            Ok(errors) => output.push(errors),
            Err(e) => output.push(format!("Could not retrieve error logs: {}", e)),
        }
    }

    Ok(output.join("\n"))
}

pub fn backup_influxdb(
    iot_host: &str,
    iot_username: &str,
    iot_password: &str,
    token: Option<&str>,
) -> Result<String, TelegrafError> {
    let mut output = Vec::new();

    // Get token from parameter or read from /etc/default/telegraf
    let token = if let Some(token_value) = token {
        output.push("Using provided InfluxDB token".to_string());
        token_value.to_string()
    } else {
        output.push("Reading InfluxDB token from remote host...".to_string());
        // Read token from the environment file via SSH
        let command = "cat /etc/default/telegraf | grep INFLUX_TOKEN=";
        let config = SshConfig {
            connect_timeout: 3,    // Quick connection check - host is either there or it isn't
            operation_timeout: 30, // Reading token file should be quick
            stream_timeout: 10,    // Small file read doesn't need long timeout
        };
        let session = connect_ssh_with_config(iot_host, iot_username, iot_password, &config)?;
        let command_output = execute_ssh_command(&session, command)?;

        // Parse the token from the output (format: token=value)
        let token_value = command_output
            .trim()
            .strip_prefix("INFLUX_TOKEN=")
            .ok_or_else(|| {
                TelegrafError::SshError(crate::error::SshError::Other(ssh2::Error::new(
                    ssh2::ErrorCode::Session(-1),
                    "Token not found in /etc/default/telegraf",
                )))
            })?;

        output.push("Token retrieved successfully.".to_string());
        token_value.to_string()
    };

    let date = chrono::Utc::now().format("%Y-%m-%d-%H-%M").to_string();
    let backup_folder = format!("/tmp/influx_backup_{}", date);
    let backup_command = format!("influx backup -t {} {}", token, backup_folder);

    output.push(format!("Backing up InfluxDB to {}", backup_folder));
    let backup_output =
        execute_command_over_ssh(iot_host, iot_username, iot_password, &backup_command)?;
    output.push(backup_output);

    let local_backup_path = format!("./influx_backup_{}", date);
    std::fs::create_dir_all(&local_backup_path)?;

    output.push(format!(
        "Downloading backup files to local directory: {}",
        local_backup_path
    ));
    copy_directory_over_ssh(
        iot_host,
        iot_username,
        iot_password,
        &backup_folder,
        &local_backup_path,
    )?;

    output.push(format!(
        "Backup completed successfully. Files are located at: {}",
        local_backup_path
    ));
    Ok(output.join("\n"))
}

/// Executes a command on a remote host via SSH
///
/// This is a convenience function that establishes an SSH connection,
/// executes a command, and returns the output. For more control over
/// the session lifecycle, use `connect_ssh_with_config` and `execute_ssh_command`.
///
/// # Arguments
/// * `remote_host` - The remote host in hostname:port format
/// * `username` - SSH username for authentication
/// * `password` - SSH password for authentication  
/// * `command` - Shell command to execute on the remote host
///
/// # Returns
/// * `Ok(String)` - Command executed successfully with output
/// * `Err(TelegrafError)` - Connection failed or command execution failed
pub fn execute_command_over_ssh(
    remote_host: &str,
    username: &str,
    password: &str,
    command: &str,
) -> Result<String, TelegrafError> {
    // Connect to SSH with appropriate timeouts for potentially long-running commands
    let config = SshConfig {
        connect_timeout: 3, // Quick connection check - host is either there or it isn't
        operation_timeout: 360, // Allow long time for potentially long-running commands
        stream_timeout: 60, // Extended stream timeout for large command output
    };
    let session = connect_ssh_with_config(remote_host, username, password, &config)?;

    // Execute the command using the centralized execution function
    let output = execute_ssh_command(&session, command)?;
    Ok(format!(
        "Command executed successfully.\nOutput: {}",
        output
    ))
}

pub fn copy_directory_over_ssh(
    remote_host: &str,
    username: &str,
    password: &str,
    remote_directory: &str,
    local_directory: &str,
) -> Result<(), TelegrafError> {
    // Connect to SSH with appropriate timeouts for directory operations
    let config = SshConfig {
        connect_timeout: 3, // Quick connection check - host is either there or it isn't
        operation_timeout: 180, // Directory operations may take longer depending on size
        stream_timeout: 60, // File transfers need reasonable stream timeout
    };
    let session = connect_ssh_with_config(remote_host, username, password, &config)?;

    // Execute a command to list files in the remote directory
    let mut channel = session.channel_session()?;
    let list_command = format!("ls {}", remote_directory);
    channel.exec(&list_command)?;
    let mut file_list = String::new();
    channel.read_to_string(&mut file_list)?;
    channel.wait_close()?;
    let file_list: Vec<&str> = file_list.lines().collect();

    // Iterate over each file name and copy it to the local directory
    for file_name in file_list {
        let remote_file_path = format!("{}/{}", remote_directory, file_name);
        let local_file_path = Path::new(local_directory).join(file_name);

        // Start SCP download for the remote file
        let (mut remote_file, stat) = session.scp_recv(Path::new(&remote_file_path))?;
        let mut local_file = File::create(local_file_path)?;

        // Copy the file content
        std::io::copy(&mut remote_file, &mut local_file)?;

        debug!("Copied {} ({} bytes)", file_name, stat.size());
    }

    Ok(())
}

pub fn backup_grafana_config(
    host: &str,
    username: &str,
    password: &str,
) -> Result<String, TelegrafError> {
    let mut output = Vec::new();

    // Connect to SSH with appropriate timeouts for file backup operations
    let config = SshConfig {
        connect_timeout: 3,    // Quick connection check - host is either there or it isn't
        operation_timeout: 60, // File backup should be reasonably quick
        stream_timeout: 20,    // Configuration files are usually small
    };
    let session = connect_ssh_with_config(host, username, password, &config)?;

    // Since /etc/grafana/grafana.ini might require sudo access,
    // first copy it to a temp location with sudo, then download it
    output.push("Copying Grafana config to a temporary location...".to_string());
    let temp_path = "/tmp/grafana_backup.ini";
    let copy_cmd = format!(
        "echo '{}' | sudo -S cp /etc/grafana/grafana.ini {}",
        password, temp_path
    );

    // Execute the copy command
    execute_ssh_command(&session, &copy_cmd)?;

    // Fix permissions on the temp file so we can read it
    let chmod_cmd = format!("echo '{}' | sudo -S chmod 644 {}", password, temp_path);
    execute_ssh_command(&session, &chmod_cmd)?;

    // Now use SFTP to download the accessible temp file
    let local_path = "grafana_backup.ini";
    output.push(format!("Downloading Grafana config to: {}", local_path));

    // Create an SFTP session
    let sftp = session.sftp()?;

    // Download the file from temp location
    let mut remote_file = sftp.open(Path::new(temp_path))?;
    let mut contents = Vec::new();
    remote_file.read_to_end(&mut contents)?;

    // Write to local file
    let mut local_file = File::create(local_path)?;
    local_file.write_all(&contents)?;

    // Clean up the temp file
    let cleanup_cmd = format!("echo '{}' | sudo -S rm {}", password, temp_path);
    match execute_ssh_command(&session, &cleanup_cmd) {
        Ok(_) => output.push("Temporary file cleaned up".to_string()),
        Err(e) => output.push(format!("Warning: Could not clean up temporary file: {}", e)),
    }

    output.push(format!("Grafana configuration backed up to {}", local_path));

    Ok(output.join("\n"))
}

/// Gets the status of Telegraf, checking both containerized and non-containerized instances
///
/// # Arguments
/// * `remote_host` - The remote host in hostname:port format
/// * `username` - SSH username for authentication
/// * `password` - SSH password for authentication
///
/// # Returns
/// * `Ok(String)` - Detailed status information about Telegraf
/// * `Err(TelegrafError)` - If there was an error checking the status
pub fn get_telegraf_status(
    remote_host: &str,
    username: &str,
    password: &str,
) -> Result<String, TelegrafError> {
    debug!("Starting telegraf status retrieval from {}", remote_host);

    // Connect to SSH with appropriate timeouts for status check operations
    let config = SshConfig {
        connect_timeout: 3,    // Quick connection check - host is either there or it isn't
        operation_timeout: 30, // Status checks should be quick
        stream_timeout: 10,    // Status output is usually small
    };
    let session = connect_ssh_with_config(remote_host, username, password, &config)?;
    debug!("SSH connection established, checking Telegraf status");

    // Check if Telegraf is running in a container
    let is_containerized = is_telegraf_containerized(&session).unwrap_or(false);

    if is_containerized {
        debug!("Detected containerized Telegraf");
        let (is_healthy, status_msg) = check_containerized_telegraf_status(&session)?;
        let status = if is_healthy { "running" } else { "degraded" };
        Ok(format!("Telegraf container is {}\n{}", status, status_msg))
    } else {
        debug!("Checking non-containerized Telegraf service");
        // Use service command with non-interactive sudo for non-containerized Telegraf
        let command = format!(
            "echo '{}' | sudo -S service telegraf status || true",
            password
        );
        let status = execute_ssh_command(&session, &command)?;

        if status.is_empty() {
            debug!("Warning: Empty status returned!");
            Ok("Telegraf service status unknown (empty response)".to_string())
        } else {
            Ok(status)
        }
    }
}

/// Fetches logs from Telegraf when running as a Docker container
fn get_telegraf_logs_docker(session: &Session, lines: usize) -> Result<String, TelegrafError> {
    debug!("Fetching last {} lines from Telegraf container...", lines);

    // Try with timestamps first, fall back to basic logs if that fails
    let command = format!("docker logs --tail={} --timestamps telegraf 2>&1 || docker logs --tail={} telegraf 2>&1 || true", lines, lines);

    match execute_ssh_command(session, &command) {
        Ok(output) if !output.trim().is_empty() => {
            debug!("Successfully retrieved {} bytes of logs", output.len());
            Ok(output)
        }
        Ok(_) => {
            let msg = "Received empty log output from container";
            eprintln!("{}", msg);
            Err(TelegrafError::SshError(crate::error::SshError::Other(
                ssh2::Error::new(ssh2::ErrorCode::Session(-1), msg),
            )))
        }
        Err(e) => {
            eprintln!("Failed to fetch Docker logs: {:?}", e);
            Err(e)
        }
    }
}

pub fn get_telegraf_logs(
    remote_host: &str,
    username: &str,
    password: &str,
    lines: usize,
) -> Result<String, TelegrafError> {
    debug!("Starting telegraf logs retrieval from {}", remote_host);

    // Connect to SSH with appropriate timeouts for log retrieval operations
    let config = SshConfig {
        connect_timeout: 3,    // Quick connection check - host is either there or it isn't
        operation_timeout: 45, // Log retrieval may take a bit longer for large logs
        stream_timeout: 20,    // Log output can be moderate in size
    };
    let session = connect_ssh_with_config(remote_host, username, password, &config)?;

    debug!("SSH connection established, retrieving logs");

    // Check if Telegraf is containerized
    if is_telegraf_containerized(&session)? {
        debug!("Telegraf is containerized. Using Docker log retrieval.");
        return get_telegraf_logs_docker(&session, lines);
    }

    debug!("Telegraf is not containerized or check failed. Using system service log retrieval.");
    // Try to check if the log file exists first with non-interactive sudo
    debug!("Checking if log file exists for system service Telegraf");
    let check_cmd = format!("echo '{}' | sudo -S test -f /var/log/telegraf/telegraf.log && echo 'exists' || echo 'missing'", password);
    let check_result = execute_ssh_command(&session, &check_cmd)?;

    if check_result.trim() == "missing" {
        // Try an alternative location
        let alt_check = format!(
            "echo '{}' | sudo -S test -f /var/log/telegraf.log && echo 'exists' || echo 'missing'",
            password
        );
        let alt_result = execute_ssh_command(&session, &alt_check)?;

        if alt_result.trim() == "exists" {
            debug!("Found log file in alternative location");
            // Get last n lines from alternative log location
            let alt_command = format!(
                "echo '{}' | sudo -S tail -n {} /var/log/telegraf.log",
                password, lines
            );
            let logs = execute_ssh_command(&session, &alt_command)?;

            debug!(
                "Telegraf logs retrieved from alternative location, length: {}",
                logs.len()
            );
            return Ok(logs);
        }

        debug!("Log file not found in standard locations!");
        return Err(TelegrafError::SshError(crate::error::SshError::Other(
            ssh2::Error::new(
                ssh2::ErrorCode::Session(-1),
                "Telegraf log file not found at expected locations",
            ),
        )));
    }

    // Get last n lines from telegraf log with non-interactive sudo
    let command = format!(
        "echo '{}' | sudo -S tail -n {} /var/log/telegraf/telegraf.log",
        password, lines
    );
    debug!("Executing command to retrieve logs");
    let logs = execute_ssh_command(&session, &command)?;

    debug!(
        "Telegraf logs retrieved successfully, length: {}",
        logs.len()
    );
    if logs.is_empty() {
        debug!("Warning: Empty logs returned!");
    }

    Ok(logs)
}

/// Service type to check for health status
#[derive(Debug, Clone, Copy)]
pub enum ServiceType {
    /// InfluxDB service
    InfluxDB,
    /// Prometheus service
    Prometheus,
}

/// Checks if a service (InfluxDB or Prometheus) is responding
///
/// # Arguments
/// * `remote_host` - The remote host in hostname:port format
/// * `username` - SSH username for authentication
/// * `password` - SSH password for authentication
/// * `service_url` - Service URL (currently unused, kept for API compatibility)
/// * `service_type` - Type of service to check
/// * `timeout_seconds` - Timeout for the SSH connection and command execution
///
/// # Returns
/// * `Ok((bool, String))` - A tuple with health status and status message
/// * `Err(TelegrafError)` - SSH connection or command execution failed
pub fn check_service_status(
    remote_host: &str,
    username: &str,
    password: &str,
    _service_url: &str, // Kept for API compatibility but not used
    service_type: ServiceType,
    timeout_seconds: u64,
) -> Result<(bool, String), TelegrafError> {
    match service_type {
        ServiceType::InfluxDB => {
            // Delegate to specialized InfluxDB status check
            check_influxdb_status(remote_host, username, password, timeout_seconds)
        }
        ServiceType::Prometheus => {
            // TODO: Implement proper Prometheus health check when needed
            // For now, return a failure status with a message
            Ok((
                false,
                "Prometheus health check not yet implemented".to_string(),
            ))
        }
    }
}

/// Checks if InfluxDB is responding on the remote host
///
/// # Arguments
/// * `remote_host` - The remote host in hostname:port format
/// * `username` - SSH username for authentication
/// * `password` - SSH password for authentication
/// * `timeout_seconds` - Timeout for the SSH connection and command execution
///
/// # Returns
/// * `Ok((bool, String))` - A tuple with health status and status message
/// * `Err(TelegrafError)` - SSH connection or command execution failed
pub fn check_influxdb_status(
    remote_host: &str,
    username: &str,
    password: &str,
    timeout_seconds: u64,
) -> Result<(bool, String), TelegrafError> {
    debug!("Checking InfluxDB status at {}", remote_host);

    // Connect to the remote host
    let session = connect_ssh_with_timeout(remote_host, username, password, timeout_seconds)?;

    // Check if InfluxDB is running in a container
    let is_containerized = is_influxdb_containerized(&session).unwrap_or(false);

    if is_containerized {
        debug!("Detected containerized InfluxDB");
        return check_containerized_influxdb_status(&session);
    }

    debug!("Checking non-containerized InfluxDB");
    let command = "influx ping";

    // Execute the command
    match execute_ssh_command(&session, command) {
        Ok(output) if output.trim() == "OK" => {
            Ok((true, "InfluxDB is responding normally".to_string()))
        }
        Ok(output) => Ok((
            false,
            format!("InfluxDB returned unexpected output: {}", output.trim()),
        )),
        Err(e) => Ok((false, format!("Failed to check InfluxDB status: {}", e))),
    }
}

/// Checks the status of a containerized service
///
/// # Arguments
/// * `session` - Active SSH session
/// * `container_name` - Name of the container to check
/// * `health_check_cmd` - Command to run inside the container to check health (empty string for no check)
/// * `service_name` - Human-readable name of the service (for status messages)
///
/// # Returns
/// * `Ok((bool, String))` - A tuple with health status and status message
/// * `Err(TelegrafError)` - If there's an error executing the check
fn check_container_status(
    session: &Session,
    container_name: &str,
    health_check_cmd: &str,
    service_name: &str,
) -> Result<(bool, String), TelegrafError> {
    // Check if container is running
    let container_status_cmd = format!(
        "docker inspect --format='{{{{.State.Running}}}}' {} 2>/dev/null || echo 'false'",
        container_name
    );
    let container_running = execute_ssh_command(session, &container_status_cmd)?;

    if container_running.trim() != "true" {
        return Ok((false, format!("{} container is not running", service_name)));
    }

    // Get container logs for debugging
    let logs_cmd = format!(
        "docker logs --tail 5 {} 2>&1 || echo 'Failed to get logs'",
        container_name
    );
    let logs = execute_ssh_command(session, &logs_cmd)
        .unwrap_or_else(|_| "Failed to retrieve logs".to_string());

    // Run the health check command if provided
    let health_status = if !health_check_cmd.is_empty() {
        let full_cmd = format!("docker exec {} {}", container_name, health_check_cmd);
        match execute_ssh_command(session, &full_cmd) {
            Ok(output) if output.trim() == "OK" => (
                true,
                format!("{} container is running and healthy", service_name),
            ),
            Ok(output) => (
                false,
                format!(
                    "{} container is running but health check failed: {}",
                    service_name, output
                ),
            ),
            Err(e) => {
                // If health check failed, get more detailed logs
                let detailed_logs_cmd = format!("docker logs --tail 20 {} 2>&1", container_name);
                let detailed_logs = execute_ssh_command(session, &detailed_logs_cmd)
                    .unwrap_or_else(|_| "Failed to retrieve detailed logs".to_string());

                (
                    false,
                    format!(
                        "{} container health check error: {}\nRecent logs:\n{}",
                        service_name, e, detailed_logs
                    ),
                )
            }
        }
    } else {
        // If no health check command, just report the container is running
        (true, format!("{} container is running", service_name))
    };

    // Always include the recent logs in the status
    let status_message = format!("{}\nRecent logs:\n{}", health_status.1, logs);

    Ok((health_status.0, status_message))
}

/// Checks the status of a containerized InfluxDB instance
fn check_containerized_influxdb_status(session: &Session) -> Result<(bool, String), TelegrafError> {
    check_container_status(session, "influxdb", "influx ping", "InfluxDB")
}

/// Checks the status of a containerized Telegraf instance
fn check_containerized_telegraf_status(session: &Session) -> Result<(bool, String), TelegrafError> {
    check_container_status(session, "telegraf", "pgrep -x telegraf", "Telegraf")
}

/// Sync system time from local machine to remote device
pub fn sync_time_over_ssh(
    remote_host: &str,
    username: &str,
    password: &str,
) -> Result<String, TelegrafError> {
    // Connect to SSH
    let config = SshConfig {
        connect_timeout: 3,
        operation_timeout: 30,
        stream_timeout: 15,
    };
    let session = connect_ssh_with_config(remote_host, username, password, &config)?;

    // Get local time
    let local_time = chrono::Local::now();
    let time_str = local_time.format("%Y-%m-%d %H:%M:%S").to_string();

    // Set time on device (requires sudo)
    let set_time_cmd = format!("echo '{}' | sudo -S date -s '{}'", password, time_str);

    let mut channel = session.channel_session()?;
    channel.exec(&set_time_cmd)?;

    let mut output = String::new();
    channel
        .read_to_string(&mut output)
        .map_err(|e| TelegrafError::IoError(e))?;

    channel.wait_close()?;

    // Verify the time was set
    let mut verify_channel = session.channel_session()?;
    verify_channel.exec("date")?;

    let mut device_time = String::new();
    verify_channel
        .read_to_string(&mut device_time)
        .map_err(|e| TelegrafError::IoError(e))?;

    verify_channel.wait_close()?;

    Ok(format!(
        "Local time: {}\nDevice time: {}",
        time_str,
        device_time.trim()
    ))
}

/// Run a command on the remote device with sudo password support
///
/// Prints output to stdout/stderr in real-time and returns an error if the command fails.
pub fn run_command(
    session: &Session,
    command: &str,
    description: &str,
    password: Option<&String>,
) -> Result<(), TelegrafError> {
    println!("🔧 {}", description);

    let command = if command.contains("sudo ") && password.is_some() {
        let pwd = password.unwrap();
        format!("echo '{}' | sudo -S {}", pwd, command.replace("sudo ", ""))
    } else {
        command.to_string()
    };

    let mut channel = session.channel_session()?;
    channel.exec(&command)?;

    use std::io::{BufReader, Read, Write};
    use std::thread;
    use std::time::Instant;

    let mut stdout = channel.stream(0);
    let mut stderr = channel.stderr();
    let mut stdout_reader = BufReader::new(&mut stdout);
    let mut stderr_reader = BufReader::new(&mut stderr);
    let start_time = Instant::now();
    let timeout = std::time::Duration::from_secs(300);

    let mut stdout_buf = [0u8; 1024];
    let mut stderr_buf = [0u8; 1024];
    loop {
        if start_time.elapsed() > timeout {
            println!("⏰ Command timed out after 5 minutes");
            channel.close().ok();
            return Err(TelegrafError::SshOperationError(format!(
                "Command timed out: {}",
                command
            )));
        }
        match stdout_reader.read(&mut stdout_buf) {
            Ok(n) if n > 0 => {
                let s = String::from_utf8_lossy(&stdout_buf[..n]);
                print!("{}", s);
                std::io::stdout().flush().ok();
            }
            _ => {}
        }
        match stderr_reader.read(&mut stderr_buf) {
            Ok(n) if n > 0 => {
                let s = String::from_utf8_lossy(&stderr_buf[..n]);
                eprint!("{}", s);
                std::io::stderr().flush().ok();
            }
            _ => {}
        }
        if channel.eof() {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }
    channel.wait_close()?;
    let exit_status = channel.exit_status()?;
    if exit_status != 0 {
        return Err(TelegrafError::SshOperationError(format!(
            "Command failed (exit code {}): {}",
            exit_status, command
        )));
    }
    Ok(())
}

/// Download a file from remote device to local machine via SFTP
pub fn download_file(
    session: &Session,
    remote_path: &str,
    local_path: &str,
) -> Result<(), TelegrafError> {
    use std::fs::File;
    use std::io::Write;

    let (mut remote_file, _stat) = session.scp_recv(std::path::Path::new(remote_path))?;
    let mut local_file = File::create(local_path)?;
    std::io::copy(&mut remote_file, &mut local_file)?;
    Ok(())
}

/// Download a directory from remote device to local machine via SFTP
pub fn download_directory(
    session: &Session,
    remote_path: &str,
    local_path: &str,
) -> Result<(), TelegrafError> {
    use std::fs::File;
    use std::io::Write;

    // List files in remote directory
    let mut channel = session.channel_session()?;
    channel.exec(&format!("ls {}", remote_path))?;
    let mut file_list = String::new();
    channel.read_to_string(&mut file_list)?;
    channel.wait_close()?;

    // Create local directory
    std::fs::create_dir_all(local_path)?;

    // Download each file
    for file_name in file_list.lines() {
        let remote_file = format!("{}/{}", remote_path, file_name);
        let local_file = format!("{}/{}", local_path, file_name);

        let (mut remote, _stat) = session.scp_recv(std::path::Path::new(&remote_file))?;
        let mut local = File::create(&local_file)?;
        std::io::copy(&mut remote, &mut local)?;
    }

    Ok(())
}

/// Upload a file from local machine to remote device via SFTP
pub fn upload_file(
    session: &Session,
    local_path: &str,
    remote_path: &str,
) -> Result<(), TelegrafError> {
    use std::fs::File;
    use std::io::Read;
    use std::io::Write;

    let mut local_file = File::open(local_path)?;
    let mut contents = Vec::new();
    local_file.read_to_end(&mut contents)?;

    let sftp = session.sftp()?;
    let mut remote_file = sftp.create(std::path::Path::new(remote_path))?;
    remote_file.write_all(&contents)?;

    Ok(())
}

/// Upload a directory from local machine to remote device via SFTP
pub fn upload_directory(
    session: &Session,
    local_path: &str,
    remote_path: &str,
    password: Option<&String>,
) -> Result<(), TelegrafError> {
    // Create remote directory first
    run_command(
        session,
        &format!("mkdir -p {}", remote_path),
        "Creating remote directory",
        password,
    )?;

    // Upload each file in the directory
    for entry in std::fs::read_dir(local_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().unwrap().to_string_lossy();
            let remote_file = format!("{}/{}", remote_path, file_name);
            upload_file(session, &path.to_string_lossy(), &remote_file)?;
        }
    }

    Ok(())
}
