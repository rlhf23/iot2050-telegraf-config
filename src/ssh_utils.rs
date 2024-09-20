use ssh2::Session;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::thread;
use std::time::Duration;

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

pub fn restart_telegraf_over_ssh(
    remote_host: &str,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Restarting telegraf service on the remote host ..");
    let tcp = TcpStream::connect(remote_host)?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_password(username, password)?;

    // Restart the service
    let mut channel = session.channel_session()?;
    channel.exec("sudo systemctl restart telegraf")?;
    channel.send_eof()?;
    channel.wait_eof()?;
    channel.wait_close()?;

    // Wait for a few seconds to allow the service to start
    println!("Waiting for the service to start ..");
    thread::sleep(Duration::from_secs(5));

    // Check the status of the service
    let mut status_channel = session.channel_session()?;
    status_channel
        .exec("systemctl is-active --quiet telegraf && echo 'active' || echo 'failed'")?;

    let mut status = String::new();
    status_channel.read_to_string(&mut status)?;
    status_channel.wait_close()?;

    let status = status.trim();

    if status == "active" {
        println!(
            "Telegraf service restarted successfully. Current status: {}",
            status
        );
    } else {
        println!(
            "Telegraf service restarted, but it's not active. Current status: {}",
            status
        );

        // Get more detailed status information
        let mut detailed_status_channel = session.channel_session()?;
        detailed_status_channel.exec("sudo systemctl status telegraf")?;

        let mut detailed_status = String::new();
        detailed_status_channel.read_to_string(&mut detailed_status)?;
        detailed_status_channel.wait_close()?;

        println!("Detailed Telegraf status:\n(.__. )\n{}", detailed_status);

        // Get the last 20 log entries for the Telegraf service
        println!("Fetching recent logs for the Telegraf service ..");
        let mut log_channel = session.channel_session()?;
        log_channel.exec("tail -n 20 /var/log/telegraf/telegraf.log")?;

        let mut logs = String::new();
        log_channel.read_to_string(&mut logs)?;
        log_channel.wait_close()?;

        println!("Recent Telegraf logs:\n( .__.)\n\n{}", logs);

        // Get the last error entry for the Telegraf service
        let mut error_channel = session.channel_session()?;
        error_channel.exec("tail -n 10 /var/log/telegraf/telegraf.log | grep 'E!'")?;

        let mut error_logs = String::new();
        error_channel.read_to_string(&mut error_logs)?;
        error_channel.wait_close()?;

        if !error_logs.is_empty() {
            println!("Latest Telegraf error logs:\n( *__*)\n\n{}", error_logs);
        } else {
            println!("No recent error logs found for Telegraf.");
        }
    }

    Ok(())
}

pub fn backup_influxdb(
    iot_host: &str,
    iot_username: &str,
    iot_password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let backup_folder = format!("/tmp/influx_backup_{}", date);
    let backup_command = format!("influx backup -p /var/lib/influxdb2 {}", backup_folder);

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

    // Assuming Grafana config is stored in /etc/grafana/grafana.ini
    let remote_path = Path::new("/etc/grafana/grafana.ini");
    let local_path = "grafana_backup.ini";

    // Create an SFTP session
    let sftp = session.sftp()?;

    // Download the file
    let mut remote_file = sftp.open(remote_path)?;
    let mut contents = Vec::new();
    remote_file.read_to_end(&mut contents)?;

    // Write to local file
    let mut local_file = File::create(local_path)?;
    local_file.write_all(&contents)?;

    println!("Grafana configuration backed up to {}", local_path);

    Ok(())
}
