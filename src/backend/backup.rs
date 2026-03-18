//! Backup and restore functionality for monitoring stack
//!
//! This module provides comprehensive backup and restore operations
//! for InfluxDB, Grafana, Prometheus, and configuration files.

use crate::backend::deployment::DeploymentConfig;
use crate::error::TelegrafError;
use ssh2::Session;
use std::fs;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

/// Create an SSH session from deployment configuration
fn create_ssh_session(config: &DeploymentConfig) -> Result<Session, TelegrafError> {
    let tcp = TcpStream::connect((config.host.as_str(), config.port))
        .map_err(|e| TelegrafError::SshOperationError(format!("Failed to connect: {}", e)))?;

    tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(30)))?;

    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;

    if let Some(key_file) = &config.key_file {
        let key_path = Path::new(key_file);
        if key_path.exists() {
            session.userauth_pubkey_file(&config.user, None, key_path, None)?;
        } else {
            return Err(TelegrafError::SshOperationError(format!(
                "SSH key file not found: {}",
                key_file
            )));
        }
    } else if let Some(password) = &config.password {
        session.userauth_password(&config.user, password)?;
    } else {
        return Err(TelegrafError::SshOperationError(
            "No authentication method provided".to_string(),
        ));
    }

    if !session.authenticated() {
        return Err(TelegrafError::SshOperationError(
            "SSH authentication failed".to_string(),
        ));
    }

    Ok(session)
}

/// Run a command on the remote device
fn run_command(
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
    let timeout = Duration::from_secs(300);

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
        thread::sleep(Duration::from_millis(100));
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

/// Download a file from remote device to local machine
fn download_file(
    session: &Session,
    remote_path: &str,
    local_path: &str,
) -> Result<(), TelegrafError> {
    let (mut remote_file, _stat) = session.scp_recv(Path::new(remote_path))?;
    let mut local_file = fs::File::create(local_path)?;
    std::io::copy(&mut remote_file, &mut local_file)?;
    Ok(())
}

/// Download a directory from remote device to local machine
fn download_directory(
    session: &Session,
    remote_path: &str,
    local_path: &str,
) -> Result<(), TelegrafError> {
    // List files in remote directory
    let mut channel = session.channel_session()?;
    channel.exec(&format!("ls {}", remote_path))?;
    let mut file_list = String::new();
    channel.read_to_string(&mut file_list)?;
    channel.wait_close()?;

    // Create local directory
    fs::create_dir_all(local_path)?;

    // Download each file
    for file_name in file_list.lines() {
        let remote_file = format!("{}/{}", remote_path, file_name);
        let local_file = format!("{}/{}", local_path, file_name);

        let (mut remote, _stat) = session.scp_recv(Path::new(&remote_file))?;
        let mut local = fs::File::create(&local_file)?;
        std::io::copy(&mut remote, &mut local)?;
    }

    Ok(())
}

/// Upload a file from local machine to remote device
fn upload_file(
    session: &Session,
    local_path: &str,
    remote_path: &str,
) -> Result<(), TelegrafError> {
    use std::io::Write;

    let mut local_file = fs::File::open(local_path)?;
    let mut contents = Vec::new();
    local_file.read_to_end(&mut contents)?;

    let sftp = session.sftp()?;
    let mut remote_file = sftp.create(Path::new(remote_path))?;
    remote_file.write_all(&contents)?;
    Ok(())
}

/// Backup monitoring data (InfluxDB, Grafana, Prometheus, configs)
pub fn backup(
    config: &DeploymentConfig,
    output_dir: Option<String>,
) -> Result<String, TelegrafError> {
    use chrono::Utc;
    use std::process::Command;

    println!("💾 Starting comprehensive backup...");

    let session = create_ssh_session(config)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_name = format!("monitoring_backup_{}", timestamp);
    let local_backup_dir = output_dir.unwrap_or_else(|| backup_name.clone());

    fs::create_dir_all(&local_backup_dir).map_err(|e| {
        TelegrafError::ConfigError(format!("Failed to create backup directory: {}", e))
    })?;

    println!("📂 Backup directory: {}", local_backup_dir);

    // 1. Backup InfluxDB data
    println!("\n📊 Backing up InfluxDB data...");
    let influx_backup_remote = format!("/tmp/influx_backup_{}", timestamp);

    let token_cmd = "cd ~/monitoring && grep INFLUXDB_TOKEN= .env | cut -d'=' -f2";
    let mut channel = session.channel_session()?;
    channel.exec(token_cmd)?;
    let mut token = String::new();
    channel.read_to_string(&mut token)?;
    channel.wait_close()?;
    let token = token.trim();

    let backup_cmd = format!(
        "docker exec influxdb influx backup -t {} {}",
        token, influx_backup_remote
    );
    run_command(
        &session,
        &backup_cmd,
        "Creating InfluxDB backup",
        config.password.as_ref(),
    )?;

    let copy_cmd = format!("docker cp influxdb:{} /tmp/", influx_backup_remote);
    run_command(
        &session,
        &copy_cmd,
        "Copying backup from container",
        config.password.as_ref(),
    )?;

    let influx_local = format!("{}/influxdb", local_backup_dir);
    fs::create_dir_all(&influx_local)?;
    download_directory(&session, &influx_backup_remote, &influx_local)?;

    run_command(
        &session,
        &format!("rm -rf {}", influx_backup_remote),
        "Cleaning up remote backup",
        config.password.as_ref(),
    )?;

    // 2. Backup Grafana dashboards via API
    println!("\n📈 Backing up Grafana dashboards...");
    let grafana_local = format!("{}/grafana", local_backup_dir);
    fs::create_dir_all(&grafana_local)?;

    let creds_cmd =
        "cd ~/monitoring && grep -E 'GRAFANA_ADMIN_USER|GRAFANA_ADMIN_PASSWORD' .env | head -2";
    let mut channel = session.channel_session()?;
    channel.exec(creds_cmd)?;
    let mut creds = String::new();
    channel.read_to_string(&mut creds)?;
    channel.wait_close()?;

    let mut admin_user = "admin".to_string();
    let mut admin_pass = "admin".to_string();
    for line in creds.lines() {
        if line.starts_with("GRAFANA_ADMIN_USER=") {
            admin_user = line.split('=').nth(1).unwrap_or("admin").to_string();
        } else if line.starts_with("GRAFANA_ADMIN_PASSWORD=") {
            admin_pass = line.split('=').nth(1).unwrap_or("admin").to_string();
        }
    }

    let list_cmd = format!(
        "curl -s -u {}:'{}' 'http://localhost:3000/api/search?type=dash-db'",
        admin_user, admin_pass
    );
    let mut channel = session.channel_session()?;
    channel.exec(&list_cmd)?;
    let mut dashboards_json = String::new();
    channel.read_to_string(&mut dashboards_json)?;
    channel.wait_close()?;

    let dashboards_dir = format!("{}/dashboards", grafana_local);
    fs::create_dir_all(&dashboards_dir)?;

    let uids_cmd = format!(
        "curl -s -u {}:'{}' 'http://localhost:3000/api/search?type=dash-db' | grep -oP '\"uid\":\\s*\"[^\"]+\"' | sed 's/\"uid\":\\s*\"\\([^\"]*\\)\"/\\1/'",
        admin_user, admin_pass
    );
    let mut channel = session.channel_session()?;
    channel.exec(&uids_cmd)?;
    let mut uids_output = String::new();
    channel.read_to_string(&mut uids_output)?;
    channel.wait_close()?;

    let mut dashboard_count = 0;
    for uid in uids_output.lines() {
        let uid = uid.trim();
        if uid.is_empty() {
            continue;
        }

        let export_cmd = format!(
            "curl -s -u {}:'{}' 'http://localhost:3000/api/dashboards/uid/{}'",
            admin_user, admin_pass, uid
        );
        let mut channel = session.channel_session()?;
        channel.exec(&export_cmd)?;
        let mut dashboard_json = String::new();
        channel.read_to_string(&mut dashboard_json)?;
        channel.wait_close()?;

        // Transform to import format
        let import_json = {
            let json = dashboard_json.trim();
            if let Some(dash_key_pos) = json.find("\"dashboard\":") {
                let rest = &json[dash_key_pos..];
                if let Some(brace_start) = rest.find('{') {
                    let start_pos = dash_key_pos + brace_start;
                    let mut depth = 0;
                    let mut end_pos = start_pos;
                    for (i, c) in json[start_pos..].chars().enumerate() {
                        match c {
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    end_pos = start_pos + i + 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    let dashboard_obj = &json[start_pos..end_pos];
                    format!("{{\"dashboard\": {}, \"overwrite\": true}}", dashboard_obj)
                } else {
                    json.to_string()
                }
            } else {
                json.to_string()
            }
        };

        let dashboard_file = format!("{}/{}.json", dashboards_dir, uid);
        fs::write(&dashboard_file, &import_json)?;
        dashboard_count += 1;
    }

    println!("   Exported {} dashboards", dashboard_count);

    // Export datasources
    println!("   Exporting datasources...");
    let datasources_cmd = format!(
        "curl -s -u {}:'{}' 'http://localhost:3000/api/datasources'",
        admin_user, admin_pass
    );
    let mut channel = session.channel_session()?;
    channel.exec(&datasources_cmd)?;
    let mut datasources_json = String::new();
    channel.read_to_string(&mut datasources_json)?;
    channel.wait_close()?;

    let datasources_file = format!("{}/datasources.json", grafana_local);
    fs::write(&datasources_file, &datasources_json)?;

    // 3. Backup Prometheus data
    println!("\n📉 Backing up Prometheus data...");
    let prometheus_local = format!("{}/prometheus", local_backup_dir);
    fs::create_dir_all(&prometheus_local)?;

    let prometheus_tar = format!("/tmp/prometheus_data_{}.tar.gz", timestamp);
    let export_cmd = format!(
        "docker exec prometheus tar czf - -C /prometheus . > {}",
        prometheus_tar
    );
    run_command(
        &session,
        &export_cmd,
        "Exporting Prometheus data",
        config.password.as_ref(),
    )?;

    download_file(
        &session,
        &prometheus_tar,
        &format!("{}/prometheus_data.tar.gz", prometheus_local),
    )?;

    run_command(
        &session,
        &format!("rm {}", prometheus_tar),
        "Cleaning up Prometheus backup",
        config.password.as_ref(),
    )?;

    // 4. Backup configuration files
    println!("\n⚙️  Backing up configuration files...");
    let config_local = format!("{}/config", local_backup_dir);
    fs::create_dir_all(&config_local)?;

    let mut channel = session.channel_session()?;
    channel.exec("echo $HOME")?;
    let mut home_dir = String::new();
    channel.read_to_string(&mut home_dir)?;
    channel.wait_close()?;
    let home_dir = home_dir.trim();

    download_file(
        &session,
        &format!("{}/monitoring/.env", home_dir),
        &format!("{}/env_backup", config_local),
    )?;

    // 5. Create backup metadata
    let metadata = format!(
        "Backup created: {}\nHost: {}\nUser: {}\n",
        Utc::now().to_rfc3339(),
        config.host,
        config.user
    );
    fs::write(format!("{}/backup_info.txt", local_backup_dir), metadata)?;

    // 6. Create compressed archive
    println!("\n📦 Creating compressed archive...");
    let archive_name = format!("{}.tar.gz", backup_name);
    let output = Command::new("tar")
        .args(&["-czf", &archive_name, "-C", ".", &backup_name])
        .output()
        .map_err(|e| TelegrafError::ConfigError(format!("Failed to create archive: {}", e)))?;

    if !output.status.success() {
        return Err(TelegrafError::ConfigError(format!(
            "Failed to create archive: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    println!("\n✅ Backup completed successfully!");
    println!("📦 Archive: {}", archive_name);
    println!("📂 Extracted backup: {}", local_backup_dir);

    Ok(format!("Backup saved to: {}", archive_name))
}

/// Restore monitoring data from backup archive
pub fn restore(
    config: &DeploymentConfig,
    archive_path: String,
    force: bool,
) -> Result<(), TelegrafError> {
    println!("♻️  Starting restore from backup...");
    println!("📦 Archive: {}", archive_path);
    if force {
        println!("⚠️  Force mode enabled - existing buckets will be deleted");
    }

    // Verify archive exists
    if !Path::new(&archive_path).exists() {
        return Err(TelegrafError::ConfigError(format!(
            "Backup archive not found: {}",
            archive_path
        )));
    }

    let session = create_ssh_session(config)?;

    // 1. Upload tarball to device
    println!("📤 Uploading archive to device...");
    let remote_path = "/tmp/monitoring_restore.tar.gz";
    upload_file(&session, &archive_path, remote_path)?;
    println!("✅ Archive uploaded to: {}", remote_path);

    // 2. Extract on device
    println!("📦 Extracting archive on device...");
    let backup_name = archive_path.trim_end_matches(".tar.gz");
    let backup_name = Path::new(backup_name)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let extract_dir = format!("/tmp/{}", backup_name);

    run_command(
        &session,
        &format!("rm -rf {}", extract_dir),
        "Cleaning up old extraction",
        config.password.as_ref(),
    )?;

    run_command(
        &session,
        &format!(
            "mkdir -p {} && tar -xzf {} -C /tmp",
            extract_dir, remote_path
        ),
        "Extracting archive",
        config.password.as_ref(),
    )?;

    // 3. Check if InfluxDB container is running
    println!("\n🔍 Checking if containers are running...");
    let mut channel = session.channel_session()?;
    channel.exec("docker ps --filter name=influxdb --format '{{.Names}}'")?;
    let mut container_status = String::new();
    channel.read_to_string(&mut container_status)?;
    channel.wait_close()?;

    if !container_status.trim().contains("influxdb") {
        return Err(TelegrafError::ConfigError(
            "InfluxDB container is not running. Please start the monitoring stack first"
                .to_string(),
        ));
    }
    println!("✅ InfluxDB container is running");

    // 4. Restore InfluxDB data
    println!("\n📊 Restoring InfluxDB data...");
    let influx_backup_dir = format!("{}/influxdb", extract_dir);

    let mut channel = session.channel_session()?;
    channel.exec(&format!("ls {} 2>/dev/null", influx_backup_dir))?;
    let mut influx_exists = String::new();
    channel.read_to_string(&mut influx_exists)?;
    channel.wait_close()?;

    if !influx_exists.trim().is_empty() {
        let mut channel = session.channel_session()?;
        channel.exec("cd ~/monitoring && grep -E 'INFLUXDB_TOKEN|INFLUXDB_ORG' .env")?;
        let mut env_vars = String::new();
        channel.read_to_string(&mut env_vars)?;
        channel.wait_close()?;

        let mut token = String::new();
        let mut org = String::new();
        for line in env_vars.lines() {
            if line.starts_with("INFLUXDB_TOKEN=") {
                token = line.split('=').nth(1).unwrap_or("").to_string();
            } else if line.starts_with("INFLUXDB_ORG=") {
                org = line.split('=').nth(1).unwrap_or("").to_string();
            }
        }

        if token.trim().is_empty() {
            return Err(TelegrafError::ConfigError(
                "Could not find INFLUXDB_TOKEN in .env file".to_string(),
            ));
        }

        let token = token.trim();
        let org = if org.trim().is_empty() {
            "my-org"
        } else {
            org.trim()
        };

        if force {
            println!("🔧 Deleting existing buckets in org '{}'...", org);
            let mut channel = session.channel_session()?;
            channel.exec(&format!(
                "docker exec influxdb influx bucket list -t {} -o {}",
                token, org
            ))?;
            let mut bucket_list = String::new();
            channel.read_to_string(&mut bucket_list)?;
            channel.wait_close()?;

            for line in bucket_list.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let bucket_name = parts[1];
                    if bucket_name.starts_with('_') {
                        continue;
                    }
                    println!("   Deleting bucket: {}", bucket_name);
                    let mut channel = session.channel_session()?;
                    channel.exec(&format!(
                        "docker exec influxdb influx bucket delete -t {} -n {} -o {}",
                        token, bucket_name, org
                    ))?;
                    let mut result = String::new();
                    channel.read_to_string(&mut result)?;
                    channel.wait_close()?;
                    if result.contains("Error") || result.contains("error") {
                        println!("   ⚠️  Could not delete {}: {}", bucket_name, result.trim());
                    } else {
                        println!("   ✓ Deleted bucket: {}", bucket_name);
                    }
                }
            }
        }

        run_command(
            &session,
            &format!(
                "docker cp {} influxdb:/tmp/influx_restore",
                influx_backup_dir
            ),
            "Copying backup into container",
            config.password.as_ref(),
        )?;

        println!("🔧 Running InfluxDB restore...");
        let mut channel = session.channel_session()?;
        channel.exec(&format!(
            "docker exec influxdb influx restore -t {} /tmp/influx_restore",
            token
        ))?;

        let mut stdout = String::new();
        let mut stderr = String::new();
        channel.read_to_string(&mut stdout).ok();
        channel.wait_close()?;
        channel.stderr().read_to_string(&mut stderr).ok();

        println!("{}", stdout);
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        }

        let exit_status = channel.exit_status()?;
        if exit_status != 0 {
            println!("\n⚠️  InfluxDB restore returned exit code: {}", exit_status);
        } else {
            println!("✅ InfluxDB restore completed");
        }

        run_command(
            &session,
            "docker exec influxdb rm -rf /tmp/influx_restore",
            "Cleaning up",
            config.password.as_ref(),
        )?;
    } else {
        println!("⏭️  No InfluxDB backup found, skipping");
    }

    // 5. Restore Grafana dashboards and datasources
    println!("\n📈 Restoring Grafana data...");
    let grafana_backup_dir = format!("{}/grafana", extract_dir);
    let dashboards_dir = format!("{}/dashboards", grafana_backup_dir);

    let mut channel = session.channel_session()?;
    channel.exec("docker ps --filter name=grafana --format '{{.Names}}'")?;
    let mut grafana_status = String::new();
    channel.read_to_string(&mut grafana_status)?;
    channel.wait_close()?;

    if !grafana_status.trim().contains("grafana") {
        println!("⚠️  Grafana container is not running, skipping Grafana restore");
    } else {
        let mut channel = session.channel_session()?;
        channel.exec(&format!("ls {} 2>/dev/null", dashboards_dir))?;
        let mut grafana_exists = String::new();
        channel.read_to_string(&mut grafana_exists)?;
        channel.wait_close()?;

        if !grafana_exists.trim().is_empty() {
            let mut channel = session.channel_session()?;
            channel.exec(
                "cd ~/monitoring && grep -E 'GRAFANA_ADMIN_USER|GRAFANA_ADMIN_PASSWORD' .env | head -2",
            )?;
            let mut creds = String::new();
            channel.read_to_string(&mut creds)?;
            channel.wait_close()?;

            let mut admin_user = "admin".to_string();
            let mut admin_pass = "admin".to_string();
            for line in creds.lines() {
                if line.starts_with("GRAFANA_ADMIN_USER=") {
                    admin_user = line.split('=').nth(1).unwrap_or("admin").to_string();
                } else if line.starts_with("GRAFANA_ADMIN_PASSWORD=") {
                    admin_pass = line.split('=').nth(1).unwrap_or("admin").to_string();
                }
            }

            let mut channel = session.channel_session()?;
            channel.exec(&format!("ls {}/*.json 2>/dev/null", dashboards_dir))?;
            let mut dashboard_list = String::new();
            channel.read_to_string(&mut dashboard_list)?;
            channel.wait_close()?;

            if !dashboard_list.trim().is_empty() {
                println!("   Restoring dashboards...");
                for dashboard_file in dashboard_list.lines() {
                    let dashboard_file = dashboard_file.trim();
                    if dashboard_file.is_empty() {
                        continue;
                    }
                    let filename = dashboard_file.split('/').last().unwrap_or("unknown");
                    let import_cmd = format!(
                        "curl -s -u {}:'{}' -X POST -H 'Content-Type: application/json' -d @{} 'http://localhost:3000/api/dashboards/db'",
                        admin_user, admin_pass, dashboard_file
                    );
                    let mut channel = session.channel_session()?;
                    channel.exec(&import_cmd)?;
                    let mut result = String::new();
                    channel.read_to_string(&mut result)?;
                    channel.wait_close()?;
                    if result.contains("\"success\":true") || result.contains("\"id\"") {
                        println!("   ✓ Restored dashboard: {}", filename);
                    } else {
                        println!("   ⚠️  Failed to restore {}: {}", filename, result.trim());
                    }
                }
            }

            let datasources_file = format!("{}/datasources.json", grafana_backup_dir);
            let mut channel = session.channel_session()?;
            channel.exec(&format!("ls {} 2>/dev/null", datasources_file))?;
            let mut ds_exists = String::new();
            channel.read_to_string(&mut ds_exists)?;
            channel.wait_close()?;

            if !ds_exists.trim().is_empty() {
                println!("   Restoring datasources...");
                let mut channel = session.channel_session()?;
                channel.exec(&format!("cat {}", datasources_file))?;
                let mut ds_content = String::new();
                channel.read_to_string(&mut ds_content)?;
                channel.wait_close()?;

                for line in ds_content.lines() {
                    let line = line.trim();
                    if !line.starts_with('{') {
                        continue;
                    }
                    let name = if let Some(start) = line.find("\"name\":\"") {
                        let start = start + 8;
                        if let Some(end) = line[start..].find('"') {
                            &line[start..start + end]
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    };

                    let ds_file = "/tmp/ds_single.json";
                    let clean_line = line.trim_end_matches(',');
                    let echo_cmd =
                        format!("echo '{}' > {}", clean_line.replace("'", "'\\''"), ds_file);
                    run_command(
                        &session,
                        &echo_cmd,
                        &format!("Preparing datasource: {}", name),
                        config.password.as_ref(),
                    )?;

                    if force {
                        let delete_cmd = format!(
                            "curl -s -u {}:'{}' 'http://localhost:3000/api/datasources/name/{}' | grep -o '\"id\":[0-9]*' | head -1 | cut -d: -f2",
                            admin_user, admin_pass, name
                        );
                        let mut channel = session.channel_session()?;
                        channel.exec(&delete_cmd)?;
                        let mut id_result = String::new();
                        channel.read_to_string(&mut id_result)?;
                        channel.wait_close()?;
                        let ds_id = id_result.trim();
                        if !ds_id.is_empty() && ds_id.chars().all(|c| c.is_numeric()) {
                            let del_cmd = format!(
                                "curl -s -u {}:'{}' -X DELETE 'http://localhost:3000/api/datasources/{}'",
                                admin_user, admin_pass, ds_id
                            );
                            run_command(
                                &session,
                                &del_cmd,
                                &format!("Deleting datasource: {}", name),
                                config.password.as_ref(),
                            )?;
                        }
                    }

                    let import_cmd = format!(
                        "curl -s -u {}:'{}' -X POST -H 'Content-Type: application/json' -d @{} 'http://localhost:3000/api/datasources'",
                        admin_user, admin_pass, ds_file
                    );
                    let mut channel = session.channel_session()?;
                    channel.exec(&import_cmd)?;
                    let mut result = String::new();
                    channel.read_to_string(&mut result)?;
                    channel.wait_close()?;
                    if result.contains("\"success\":true") || result.contains("\"id\"") {
                        println!("   ✓ Restored datasource: {}", name);
                    } else if result.contains("already exists") {
                        println!("   ⚠️  Datasource '{}' already exists, skipped", name);
                    } else {
                        println!(
                            "   ⚠️  Failed to restore datasource {}: {}",
                            name,
                            result.trim()
                        );
                    }
                }
            }
            println!("✅ Grafana restore completed");
        } else {
            println!("⏭️  No Grafana backup found, skipping");
        }
    }

    // 6. Cleanup
    println!("\n🧹 Cleaning up...");
    run_command(
        &session,
        &format!("rm -rf {} {}", extract_dir, remote_path),
        "Removing temporary files",
        config.password.as_ref(),
    )?;

    println!("\n✅ Restore process completed");
    Ok(())
}
