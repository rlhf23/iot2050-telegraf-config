use ssh2::Session;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

pub fn send_and_restart_telegraf(
    config_path: &Path,
    remote_path: &str,
    iot_host: &str,
    iot_username: &str,
    iot_password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Send the telegraf.conf file to the IOT box
    send_file_over_ssh(
        config_path,
        remote_path,
        iot_host,
        iot_username,
        iot_password,
    )?;

    // Restart the telegraf service on the IOT box
    restart_telegraf_over_ssh(iot_host, iot_username, iot_password)?;

    Ok(())
}

pub fn send_file_over_ssh(
    // Sends a file over SSH to a specified remote host, path, and credentials
    local_path: &Path,
    remote_path: &str,
    remote_host: &str,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Sending file ..");
    // Establish a TCP connection to the remote host
    let tcp = TcpStream::connect(remote_host)?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;

    // Authenticate with the remote server
    session.userauth_password(username, password)?;

    // Open a new SCP session and send the file
    let mut remote_file = session.scp_send(
        Path::new(remote_path),
        0o644,
        local_path.metadata()?.len(),
        None,
    )?;
    let mut local_file = std::fs::File::open(local_path)?;

    let mut contents = Vec::new();
    local_file.read_to_end(&mut contents)?;
    remote_file.write_all(&contents)?;

    Ok(())
}

fn execute_ssh_command(
    session: &Session,
    command: &str,
) -> Result<String, Box<dyn std::error::Error>> {
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
        return Err(format!(
            "Command failed with exit status {}: {}",
            exit_status, output
        )
        .into());
    }

    // Print for debugging
    println!("SSH command output length: {}", output.len());
    println!(
        "First 100 chars: {}",
        if output.len() > 100 {
            &output[..100]
        } else {
            &output
        }
    );

    Ok(output)
}

pub fn restart_telegraf_over_ssh(
    remote_host: &str,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Restarting telegraf service on the remote host...");
    let start_time = Instant::now();

    // Establish SSH connection
    let tcp = TcpStream::connect(remote_host)?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password(username, password)?;

    // Step 1: Try graceful stop first with timeout
    println!("Stopping telegraf service gracefully...");
    
    // First check if telegraf is actually running
    let check_cmd = format!("pgrep telegraf || echo 'not_running'");
    let check_result = execute_ssh_command(&session, &check_cmd)?;
    
    if check_result.trim() == "not_running" {
        println!("Telegraf is not currently running. Proceeding to start.");
    } else {
        // Attempt graceful stop
        let stop_cmd = format!("echo '{}' | sudo -S service telegraf stop", password);
        match execute_ssh_command(&session, &stop_cmd) {
            Ok(_) => println!("Service stop command issued"),
            Err(e) => println!("Warning: Error issuing stop command: {}", e),
        }
        
        // Wait up to 10 seconds for telegraf to stop gracefully
        println!("Waiting up to 10 seconds for telegraf to stop...");
        let mut stopped = false;
        for i in 0..10 {
            thread::sleep(Duration::from_secs(1));
            let check_cmd = format!("pgrep telegraf || echo 'stopped'");
            let check_result = execute_ssh_command(&session, &check_cmd)?;
            
            if check_result.trim() == "stopped" {
                println!("Telegraf stopped gracefully after {} seconds", i + 1);
                stopped = true;
                break;
            }
        }
        
        // If still running after 10 seconds, forcefully kill it
        if !stopped {
            println!("Telegraf didn't stop gracefully within 10 seconds. Killing process...");
            let kill_cmd = format!("echo '{}' | sudo -S pkill -9 telegraf", password);
            match execute_ssh_command(&session, &kill_cmd) {
                Ok(_) => println!("Telegraf process killed forcefully"),
                Err(e) => println!("Warning: Error killing telegraf: {}", e),
            }
            
            // Give a moment for the kill to take effect
            thread::sleep(Duration::from_secs(1));
        }
    }

    // Step 2: Start the service
    println!("Starting telegraf service...");
    let start_cmd = format!("echo '{}' | sudo -S service telegraf start", password);
    execute_ssh_command(&session, &start_cmd)?;

    // Step 3: Wait for service to initialize
    println!("Waiting for service to initialize...");
    thread::sleep(Duration::from_secs(5));

    // Step 4: Check service status
    println!("Checking service status...");
    let status_cmd = format!("echo '{}' | sudo -S service telegraf status | head -n15", password);
    let status = execute_ssh_command(&session, &status_cmd)?;

    let elapsed_time = start_time.elapsed();

    // Also check if the process is actually running
    let process_check = execute_ssh_command(&session, "pgrep telegraf || echo 'not_running'")?;
    let is_running = process_check.trim() != "not_running";

    if status.contains("Active: active") && is_running {
        println!("✓ Telegraf restart successful ({:.2?})", elapsed_time);
        println!("Status:\n{}", status);
    } else {
        println!("✗ Telegraf restart failed ({:.2?})", elapsed_time);
        println!("Status:\n{}", status);
        
        if !is_running {
            println!("WARNING: The telegraf process is not running!");
        }

        // Get recent logs if service failed
        println!("\nRecent logs:");
        let logs_cmd = format!(
            "echo '{}' | sudo -S tail -n 10 /var/log/telegraf/telegraf.log 2>/dev/null || echo 'No logs found'", 
            password
        );
        match execute_ssh_command(&session, &logs_cmd) {
            Ok(logs) => println!("{}", logs),
            Err(e) => println!("Could not retrieve logs: {}", e),
        }

        // Get error logs if any
        println!("\nRecent errors:");
        let errors_cmd = format!(
            "echo '{}' | sudo -S grep -E 'E!' /var/log/telegraf/telegraf.log 2>/dev/null | tail -n 10 || echo 'No error logs found'",
            password
        );
        match execute_ssh_command(&session, &errors_cmd) {
            Ok(errors) => println!("{}", errors),
            Err(e) => println!("Could not retrieve error logs: {}", e),
        }
    }

    Ok(())
}

pub fn backup_influxdb(
    iot_host: &str,
    iot_username: &str,
    iot_password: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let date = chrono::Utc::now().format("%Y-%m-%d-%H-%M").to_string();
    let backup_folder = format!("/tmp/influx_backup_{}", date);
    let backup_command = format!("influx backup -t {} {}", token, backup_folder);

    println!("Backing up InfluxDB to {}", backup_folder);
    execute_command_over_ssh(iot_host, iot_username, iot_password, &backup_command)?;

    let local_backup_path = format!("./influx_backup_{}", date);
    std::fs::create_dir_all(&local_backup_path)?;
    copy_directory_over_ssh(
        iot_host,
        iot_username,
        iot_password,
        &backup_folder,
        &local_backup_path,
    )?;

    println!(
        "Backup completed successfully. Files are located at: {}",
        local_backup_path
    );
    Ok(())
}

pub fn execute_command_over_ssh(
    remote_host: &str,
    username: &str,
    password: &str,
    command: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect(remote_host)?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password(username, password)?;

    let mut channel = session.channel_session()?;
    channel.exec(command)?;
    let mut s = String::new();
    channel.read_to_string(&mut s)?;
    println!("Command output: {}", s);
    channel.send_eof()?;
    channel.wait_eof()?;
    channel.wait_close()?;
    println!("Command executed successfully.");
    Ok(())
}

pub fn copy_directory_over_ssh(
    remote_host: &str,
    username: &str,
    password: &str,
    remote_directory: &str,
    local_directory: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Establish an SSH session
    let tcp = TcpStream::connect(remote_host)?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password(username, password)?;

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

        println!("Copied {} ({} bytes)", file_name, stat.size());
    }

    Ok(())
}

pub fn backup_grafana_config(
    host: &str,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Establish SSH connection
    let tcp = TcpStream::connect(host)?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password(username, password)?;

    // Since /etc/grafana/grafana.ini might require sudo access,
    // first copy it to a temp location with sudo, then download it
    println!("Copying Grafana config to a temporary location...");
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
        Ok(_) => println!("Temporary file cleaned up"),
        Err(e) => println!("Warning: Could not clean up temporary file: {}", e),
    }

    println!("Grafana configuration backed up to {}", local_path);

    Ok(())
}

pub fn get_telegraf_status(
    remote_host: &str,
    username: &str,
    password: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("Starting telegraf status retrieval from {}", remote_host);

    // Establish SSH connection
    let tcp = TcpStream::connect(remote_host)?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password(username, password)?;

    println!("SSH connection established, running status command");

    // Use service command with non-interactive sudo
    let command = format!("echo '{}' | sudo -S service telegraf status", password);
    println!("Executing command with non-interactive sudo");

    let status = execute_ssh_command(&session, &command)?;
    println!(
        "Telegraf status retrieved successfully, length: {}",
        status.len()
    );

    if status.is_empty() {
        println!("Warning: Empty status returned!");
    }

    Ok(status)
}

pub fn get_telegraf_logs(
    remote_host: &str,
    username: &str,
    password: &str,
    lines: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("Starting telegraf logs retrieval from {}", remote_host);

    // Establish SSH connection
    let tcp = TcpStream::connect(remote_host)?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password(username, password)?;

    println!("SSH connection established, retrieving logs");

    // Try to check if the log file exists first with non-interactive sudo
    println!("Checking if log file exists");
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
            println!("Found log file in alternative location");
            // Get last n lines from alternative log location
            let alt_command = format!(
                "echo '{}' | sudo -S tail -n {} /var/log/telegraf.log",
                password, lines
            );
            let logs = execute_ssh_command(&session, &alt_command)?;

            println!(
                "Telegraf logs retrieved from alternative location, length: {}",
                logs.len()
            );
            return Ok(logs);
        }

        println!("Log file not found in standard locations!");
        return Err("Telegraf log file not found at expected locations".into());
    }

    // Get last n lines from telegraf log with non-interactive sudo
    let command = format!(
        "echo '{}' | sudo -S tail -n {} /var/log/telegraf/telegraf.log",
        password, lines
    );
    println!("Executing command to retrieve logs");
    let logs = execute_ssh_command(&session, &command)?;

    println!(
        "Telegraf logs retrieved successfully, length: {}",
        logs.len()
    );
    if logs.is_empty() {
        println!("Warning: Empty logs returned!");
    }

    Ok(logs)
}
