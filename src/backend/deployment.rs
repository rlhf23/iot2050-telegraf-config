use crate::error::TelegrafError;
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub host: String,
    pub user: String,
    pub password: Option<String>,
    pub key_file: Option<String>,
    pub port: u16,
    pub git_branch: Option<String>,
}

impl DeploymentConfig {
    pub fn new(host: String, user: String) -> Self {
        Self {
            host,
            user,
            password: None,
            key_file: None,
            port: 22,
            git_branch: None,
        }
    }

    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    pub fn with_key_file(mut self, key_file: String) -> Self {
        self.key_file = Some(key_file);
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the git branch to use for deployment
    pub fn with_git_branch(mut self, branch: String) -> Self {
        self.git_branch = Some(branch);
        self
    }
}

pub struct IoTDeployer {
    config: DeploymentConfig,
    repo_url: String,
    branch: String,
}

impl IoTDeployer {
    /// Get the host for this deployer
    pub fn host(&self) -> &str {
        &self.config.host
    }
    /// Get the user for this deployer
    pub fn user(&self) -> &str {
        &self.config.user
    }

    /// Detect the architecture of the remote device
    /// Returns "arm64" for aarch64/arm64 devices, "amd64" for x86_64/amd64
    pub fn detect_architecture(&self) -> Result<String, TelegrafError> {
        let session = self.create_ssh_session()?;

        println!("🔍 Detecting device architecture...");

        let mut channel = session.channel_session()?;
        channel.exec("uname -m")?;

        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;

        let arch = output.trim();

        let targetarch = match arch {
            "x86_64" | "amd64" => {
                println!("   Detected x86_64 architecture -> using amd64 binaries");
                "amd64".to_string()
            }
            "aarch64" | "arm64" => {
                println!("   Detected ARM64 architecture -> using arm64 binaries");
                "arm64".to_string()
            }
            _ => {
                println!(
                    "   ⚠️  Unknown architecture '{}', defaulting to arm64",
                    arch
                );
                "arm64".to_string()
            }
        };

        Ok(targetarch)
    }

    pub fn new(config: DeploymentConfig) -> Self {
        let branch = config
            .git_branch
            .clone()
            .unwrap_or_else(|| "master".to_string());
        let repo_url = format!(
            "https://github.com/rlhf23/iot2050-telegraf-config/archive/refs/heads/{}.tar.gz",
            branch
        );

        Self {
            config,
            repo_url,
            branch,
        }
    }

    /// Test SSH connectivity to the device
    pub fn test_connection(&self) -> Result<(), TelegrafError> {
        println!(
            "🔧 Testing SSH connectivity to {}:{}...",
            self.config.host, self.config.port
        );

        let session = self.create_ssh_session()?;
        let mut channel = session.channel_session()?;
        channel.exec("echo 'SSH connection successful'")?;

        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;

        if channel.exit_status()? == 0 {
            println!("✅ SSH connectivity verified");
            Ok(())
        } else {
            Err(TelegrafError::SshOperationError(
                "SSH connection test failed".to_string(),
            ))
        }
    }

    /// Provision the IoT device with Docker and requirements
    pub fn provision(&self) -> Result<(), TelegrafError> {
        self.provision_with_transfer_mode(false)
    }

    /// Provision with option to download locally first (offline-capable)
    pub fn provision_with_transfer_mode(&self, local_transfer: bool) -> Result<(), TelegrafError> {
        println!("🚀 Starting device provisioning...");

        let session = self.create_ssh_session()?;

        // Check internet connectivity
        println!("🌐 Checking internet connectivity...");
        self.run_command(
            &session,
            "ping -c 1 www.google.com > /dev/null 2>&1 || { echo 'Error: No internet connectivity'; exit 1; }",
            "Verifying internet connectivity"
        )?;

        // Check repository accessibility
        println!("🔍 Verifying repository accessibility...");
        self.run_command(
            &session,
            &format!("curl -s -o /dev/null -I -w '%{{http_code}}' {} | grep -q '200\\|302' || {{ echo 'Error: Cannot access repository branch: {}'; exit 1; }}", self.repo_url, self.branch),
            &format!("Checking if repository branch '{}' is accessible", self.branch)
        )?;

        // Check if all required packages are already installed
        println!("🔍 Checking for required packages...");
        let packages_status = self.check_required_packages(&session)?;

        // Only proceed with apt operations if packages are missing
        if packages_status.needs_installation() {
            // Warn about potential time issues before running apt
            self.check_system_time(&session)?;

            println!("📦 Installing missing packages...");

            // Update package lists
            self.run_command(&session, "sudo apt-get update", "Updating package lists")?;

            // Install base packages if needed
            if packages_status.needs_base_packages {
                self.run_command(
                    &session,
                    "sudo apt-get install -y apt-transport-https ca-certificates curl gnupg lsb-release git openssl",
                    "Installing required packages"
                )?;
            }

            // Install Docker if needed
            if packages_status.needs_docker {
                println!("🐳 Installing Docker...");
                self.run_command(
                    &session,
                    "sudo apt-get install -y docker.io",
                    "Installing Docker",
                )?;
            }

            // Install Docker Compose if needed
            if packages_status.needs_compose {
                println!("📦 Installing Docker Compose...");
                self.run_command(
                    &session,
                    "sudo apt-get install -y docker-compose",
                    "Installing Docker Compose",
                )?;
            }
        } else {
            println!("✅ All required packages are already installed");
        }

        // Add user to docker group
        self.run_command(
            &session,
            &format!("sudo usermod -aG docker {}", self.config.user),
            "Adding user to docker group",
        )?;

        // Start and enable Docker
        self.run_command(&session, "sudo systemctl start docker", "Starting Docker")?;
        self.run_command(&session, "sudo systemctl enable docker", "Enabling Docker")?;

        // Setup monitoring directory structure
        println!("📂 Setting up monitoring directory...");

        // Remove existing monitoring directory if it exists
        self.run_command(
            &session,
            "rm -rf ~/monitoring",
            "Cleaning up existing monitoring directory",
        )?;

        // Create monitoring directory
        self.run_command(
            &session,
            "mkdir -p ~/monitoring",
            "Creating monitoring directory",
        )?;

        // Download docker folder - either directly on device or via local transfer
        if local_transfer {
            println!(
                "📥 Downloading docker configuration locally from branch '{}'...",
                self.branch
            );
            self.download_and_transfer_monitoring_folder(&session)?;
        } else {
            println!(
                "📥 Downloading docker configuration directly on device from branch '{}'...",
                self.branch
            );

            // GitHub replaces '/' with '-' in branch names when creating tarballs
            let sanitized_branch = self.branch.replace('/', "-");
            let download_cmd = format!(
                "cd ~/monitoring && curl -L {} | tar -xz --strip-components=2 iot2050-telegraf-config-{}/docker",
                self.repo_url, sanitized_branch
            );

            self.run_command(&session, &download_cmd, "Downloading docker configuration")?;
        }
        // Make scripts executable
        self.run_command(
            &session,
            "cd ~/monitoring && chmod +x scripts/*.sh config/nginx/scripts/*.sh",
            "Making scripts executable",
        )?;

        println!("✅ Device provisioning completed successfully!");
        Ok(())
    }

    /// Update monitoring configuration on the device (download fresh copy from git)
    pub fn update(&self, use_local: bool) -> Result<(), TelegrafError> {
        println!("🔄 Updating monitoring configuration...");

        let session = self.create_ssh_session()?;

        // Remove existing monitoring directory
        self.run_command(
            &session,
            "rm -rf ~/monitoring",
            "Removing old monitoring directory",
        )?;

        // Create monitoring directory
        self.run_command(
            &session,
            "mkdir -p ~/monitoring",
            "Creating monitoring directory",
        )?;

        if use_local {
            // Download locally and transfer via SFTP
            println!(
                "📥 Downloading docker configuration locally from branch '{}'...",
                self.branch
            );

            // Create temp directory
            let temp_dir = std::env::temp_dir().join(format!("monitoring-{}", self.branch));
            std::fs::create_dir_all(&temp_dir)?;

            // Download tarball locally
            let tarball_path = temp_dir.join("docker.tar.gz");
            let output = std::process::Command::new("curl")
                .arg("-L")
                .arg(&self.repo_url)
                .arg("-o")
                .arg(&tarball_path)
                .output()
                .map_err(|e| TelegrafError::ConfigError(format!("Failed to download: {}", e)))?;

            if !output.status.success() {
                return Err(TelegrafError::ConfigError(
                    "Failed to download tarball".to_string(),
                ));
            }

            // Extract locally
            let extract_dir = temp_dir.join("extracted");
            std::fs::create_dir_all(&extract_dir)?;
            let extract_output = std::process::Command::new("tar")
                .arg("-xzf")
                .arg(&tarball_path)
                .arg("-C")
                .arg(&extract_dir)
                .output()
                .map_err(|e| TelegrafError::ConfigError(format!("Failed to extract: {}", e)))?;

            if !extract_output.status.success() {
                return Err(TelegrafError::ConfigError(
                    "Failed to extract tarball".to_string(),
                ));
            }

            // GitHub replaces '/' with '-' in branch names when creating tarballs
            let sanitized_branch = self.branch.replace('/', "-");
            let docker_path = extract_dir
                .join(format!("iot2050-telegraf-config-{}", sanitized_branch))
                .join("docker");

            println!("📤 Transferring files to device via SFTP...");

            // Use SFTP through the existing SSH session
            let sftp = session.sftp().map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to create SFTP session: {}", e))
            })?;

            // Recursively upload the docker folder
            self.upload_directory_sftp(
                &sftp,
                &docker_path,
                Path::new("/home").join(self.user()).join("monitoring"),
            )?;

            // Cleanup temp directory
            let _ = std::fs::remove_dir_all(&temp_dir);
        } else {
            // Download directly on the device (requires internet)
            println!(
                "📥 Downloading docker configuration on device from branch '{}'...",
                self.branch
            );

            // Check if device has internet connectivity
            println!("🌐 Checking device internet connectivity...");
            let has_internet =
                self.check_command(&session, "ping -c 1 www.google.com > /dev/null 2>&1")?;

            if !has_internet {
                return Err(TelegrafError::ConfigError(
                    "Device has no internet connectivity. Use --local flag to download and transfer from your machine instead.".to_string()
                ));
            }

            // GitHub replaces '/' with '-' in branch names when creating tarballs
            let sanitized_branch = self.branch.replace('/', "-");
            let download_cmd = format!(
                "cd ~/monitoring && curl -L {} | tar -xz --strip-components=2 iot2050-telegraf-config-{}/docker",
                self.repo_url, sanitized_branch
            );

            self.run_command(&session, &download_cmd, "Downloading docker configuration")?;
        }

        // Make scripts executable
        self.run_command(
            &session,
            "cd ~/monitoring && chmod +x scripts/*.sh config/nginx/scripts/*.sh",
            "Making scripts executable",
        )?;

        println!("✅ Monitoring configuration updated successfully!");
        Ok(())
    }

    /// Run setup on the device (assumes provisioning is already done)
    pub fn setup(&self) -> Result<(), TelegrafError> {
        println!("🚀 Starting setup...");

        let session = self.create_ssh_session()?;

        // Verify monitoring directory exists
        let dir_exists = self.check_command(&session, "test -d ~/monitoring")?;
        if !dir_exists {
            return Err(TelegrafError::ConfigError(
                "Monitoring directory not found. Please run 'provision' or 'update' first."
                    .to_string(),
            ));
        }

        // Detect device architecture
        let targetarch = self.detect_architecture()?;

        // Run setup script (this will also export TARGETARCH via .env)
        println!("⚙️  Running setup script...");
        self.run_command(
            &session,
            "cd ~/monitoring && ./scripts/setup.sh",
            "Running setup",
        )?;

        println!(
            "✅ Setup completed successfully! (Architecture: {})",
            targetarch
        );
        Ok(())
    }

    /// Sync system time from local machine to device
    pub fn sync_time(&self) -> Result<(), TelegrafError> {
        println!("🕐 Syncing time to device...");

        let session = self.create_ssh_session()?;

        // Get local time in format suitable for `date` command
        let local_time = chrono::Local::now();
        let time_str = local_time.format("%Y-%m-%d %H:%M:%S").to_string();

        println!("📅 Local time: {}", time_str);

        // Set time on device (requires sudo)
        let set_time_cmd = format!("sudo date -s '{}'", time_str);

        self.run_command(&session, &set_time_cmd, "Setting device time")?;

        // Verify the time was set
        let mut channel = session.channel_session()?;
        channel.exec("date")?;

        let mut device_time = String::new();
        channel.read_to_string(&mut device_time)?;
        channel.wait_close()?;

        println!("✅ Device time updated: {}", device_time.trim());

        Ok(())
    }

    /// Get deployment status
    pub fn status(&self) -> Result<(), TelegrafError> {
        println!("📊 Checking deployment status...");

        let session = self.create_ssh_session()?;

        // Check if monitoring directory exists
        if !self.check_command(&session, "test -d ~/monitoring")? {
            println!("❌ Monitoring stack not deployed");
            return Ok(());
        }

        // Check Docker status
        self.run_command(&session, "docker --version", "Docker version")?;

        // Check container status
        let mut channel = session.channel_session()?;
        channel.exec("cd ~/monitoring && docker-compose ps")?;

        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;

        println!("Container Status:");
        println!("{}", output);

        // Get credentials
        let mut channel = session.channel_session()?;
        channel.exec("cd ~/monitoring && cat .env | grep -E '(GRAFANA_ADMIN_|INFLUXDB_)' | grep -E '(USER|PASSWORD|TOKEN)='")?;

        let mut credentials = String::new();
        channel.read_to_string(&mut credentials)?;
        channel.wait_close()?;

        if !credentials.trim().is_empty() {
            println!("\n🔑 Credentials:");
            println!("{}", credentials);
        }

        println!("\n🔗 Access URLs:");
        println!("  - Grafana: http://{}:3000", self.config.host);
        println!("  - InfluxDB: http://{}:8086", self.config.host);

        Ok(())
    }

    /// Stop the monitoring stack
    pub fn stop(&self) -> Result<(), TelegrafError> {
        self.stop_with_volumes(false)
    }

    /// Stop the monitoring stack with optional volume removal
    pub fn stop_with_volumes(&self, remove_volumes: bool) -> Result<(), TelegrafError> {
        println!("🛑 Stopping monitoring stack...");

        let session = self.create_ssh_session()?;

        let stop_command = if remove_volumes {
            "cd ~/monitoring && docker compose down -v"
        } else {
            "cd ~/monitoring && ./scripts/stop.sh"
        };

        let description = if remove_volumes {
            "Stopping monitoring stack and removing volumes"
        } else {
            "Stopping monitoring stack"
        };

        self.run_command(&session, stop_command, description)?;

        if remove_volumes {
            println!("✅ Monitoring stack stopped and volumes removed");
            println!("⚠️  All data has been deleted. You will need to reconfigure services on next start.");
        } else {
            println!("✅ Monitoring stack stopped");
        }
        Ok(())
    }

    /// Start the monitoring stack
    pub fn start(&self) -> Result<(), TelegrafError> {
        println!("🚀 Starting monitoring stack...");

        let session = self.create_ssh_session()?;

        self.run_command(
            &session,
            "cd ~/monitoring && ./scripts/start.sh",
            "Starting monitoring stack",
        )?;

        println!("✅ Monitoring stack started");
        Ok(())
    }

    /// Backup all monitoring data (InfluxDB, Grafana, Prometheus) to local directory
    pub fn backup(&self, output_dir: Option<String>) -> Result<String, TelegrafError> {
        use chrono::Utc;
        use std::fs;
        use std::process::Command;

        println!("💾 Starting comprehensive backup...");

        let session = self.create_ssh_session()?;

        // Create timestamped backup directory
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_name = format!("monitoring_backup_{}", timestamp);
        let local_backup_dir = output_dir.unwrap_or_else(|| format!("./{}", backup_name));

        fs::create_dir_all(&local_backup_dir).map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to create backup directory: {}", e))
        })?;

        println!("📂 Backup directory: {}", local_backup_dir);

        // 1. Backup InfluxDB data
        println!("\n📊 Backing up InfluxDB data...");
        let influx_backup_remote = format!("/tmp/influx_backup_{}", timestamp);

        // Get InfluxDB token from .env file
        let token_cmd = "cd ~/monitoring && grep INFLUXDB_TOKEN= .env | cut -d'=' -f2";
        let mut channel = session.channel_session()?;
        channel.exec(token_cmd)?;
        let mut token = String::new();
        channel.read_to_string(&mut token)?;
        channel.wait_close()?;
        let token = token.trim();

        // Run influx backup inside container
        let backup_cmd = format!(
            "docker exec influxdb influx backup -t {} {}",
            token, influx_backup_remote
        );
        self.run_command(&session, &backup_cmd, "Creating InfluxDB backup")?;

        // Copy backup from container to host
        let copy_cmd = format!("docker cp influxdb:{} /tmp/", influx_backup_remote);
        self.run_command(&session, &copy_cmd, "Copying backup from container")?;

        // Download backup to local machine
        let influx_local = format!("{}/influxdb", local_backup_dir);
        fs::create_dir_all(&influx_local)?;
        self.download_directory(&session, &influx_backup_remote, &influx_local)?;

        // Cleanup remote backup
        self.run_command(
            &session,
            &format!("rm -rf {}", influx_backup_remote),
            "Cleaning up remote backup",
        )?;

        // 2. Backup Grafana dashboards via API
        println!("\n📈 Backing up Grafana dashboards...");
        let grafana_local = format!("{}/grafana", local_backup_dir);
        fs::create_dir_all(&grafana_local)?;

        // Get Grafana admin credentials from .env or use defaults
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

        // List all dashboards via Grafana API
        let list_cmd = format!(
            "curl -s -u {}:'{}' 'http://localhost:3000/api/search?type=dash-db'",
            admin_user, admin_pass
        );
        let mut channel = session.channel_session()?;
        channel.exec(&list_cmd)?;
        let mut dashboards_json = String::new();
        channel.read_to_string(&mut dashboards_json)?;
        channel.wait_close()?;

        // Parse dashboard UIDs and export each one
        let dashboards_dir = format!("{}/dashboards", grafana_local);
        fs::create_dir_all(&dashboards_dir)?;

        // Extract UIDs using grep/sed (simple JSON parsing on remote)
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

            // Export dashboard
            let export_cmd = format!(
                "curl -s -u {}:'{}' 'http://localhost:3000/api/dashboards/uid/{}'",
                admin_user, admin_pass, uid
            );
            let mut channel = session.channel_session()?;
            channel.exec(&export_cmd)?;
            let mut dashboard_json = String::new();
            channel.read_to_string(&mut dashboard_json)?;
            channel.wait_close()?;

            // Save dashboard locally
            let dashboard_file = format!("{}/{}.json", dashboards_dir, uid);
            fs::write(&dashboard_file, &dashboard_json)?;
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
            "docker run --rm -v prometheus_data:/data -v /tmp:/backup alpine tar czf /backup/prometheus_data_{}.tar.gz -C /data .",
            timestamp
        );
        self.run_command(&session, &export_cmd, "Exporting Prometheus volume")?;

        // Download Prometheus backup
        self.download_file(
            &session,
            &prometheus_tar,
            &format!("{}/prometheus_data.tar.gz", prometheus_local),
        )?;

        // Cleanup
        self.run_command(
            &session,
            &format!("rm {}", prometheus_tar),
            "Cleaning up Prometheus backup",
        )?;

        // 4. Backup configuration files
        println!("\n⚙️  Backing up configuration files...");
        let config_local = format!("{}/config", local_backup_dir);
        fs::create_dir_all(&config_local)?;

        // Copy .env file
        self.download_file(
            &session,
            "~/monitoring/.env",
            &format!("{}/env_backup", config_local),
        )?;

        // 5. Create backup metadata
        let metadata = format!(
            "Backup created: {}\nHost: {}\nUser: {}\n",
            Utc::now().to_rfc3339(),
            self.config.host,
            self.config.user
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
        println!("\n💡 To restore this backup:");
        println!(
            "   cargo run --bin sie_generate_config deploy restore --archive {}",
            archive_name
        );

        Ok(format!("Backup saved to: {}", archive_name))
    }

    /// Restore monitoring data from backup archive
    pub fn restore(&self, archive_path: String) -> Result<(), TelegrafError> {
        use std::fs;
        use std::process::Command;

        println!("♻️  Starting restore from backup...");
        println!("📦 Archive: {}", archive_path);

        // Verify archive exists
        if !std::path::Path::new(&archive_path).exists() {
            return Err(TelegrafError::ConfigError(format!(
                "Backup archive not found: {}",
                archive_path
            )));
        }

        let session = self.create_ssh_session()?;

        // Extract archive locally
        println!("📦 Extracting archive...");
        let output = Command::new("tar")
            .args(&["-xzf", &archive_path])
            .output()
            .map_err(|e| TelegrafError::ConfigError(format!("Failed to extract archive: {}", e)))?;

        if !output.status.success() {
            return Err(TelegrafError::ConfigError(format!(
                "Failed to extract archive: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Find extracted directory
        let backup_dir = archive_path.trim_end_matches(".tar.gz");

        // Stop monitoring stack if running
        println!("\n🛑 Stopping monitoring stack...");
        let _ = self.run_command(
            &session,
            "cd ~/monitoring && docker-compose down",
            "Stopping stack",
        );

        // 1. Restore InfluxDB data
        println!("\n📊 Restoring InfluxDB data...");
        let influx_backup = format!("{}/influxdb", backup_dir);

        if std::path::Path::new(&influx_backup).exists() {
            // Upload backup to device
            let remote_backup = "/tmp/influx_restore";
            self.upload_directory(&influx_backup, remote_backup)?;

            // Copy into container and restore
            let restore_cmd = format!("docker exec influxdb influx restore {}", remote_backup);
            self.run_command(&session, &restore_cmd, "Restoring InfluxDB data")?;

            // Cleanup
            self.run_command(
                &session,
                &format!("rm -rf {}", remote_backup),
                "Cleaning up",
            )?;
        }

        // 2. Restore Grafana dashboards via API
        println!("\n📈 Restoring Grafana dashboards...");
        let dashboards_dir = format!("{}/grafana/dashboards", backup_dir);

        if std::path::Path::new(&dashboards_dir).exists() {
            // Get Grafana admin credentials
            let creds_cmd = "cd ~/monitoring && grep -E 'GF_SECURITY_ADMIN_USER|GF_SECURITY_ADMIN_PASSWORD' .env | head -2";
            let mut channel = session.channel_session()?;
            channel.exec(creds_cmd)?;
            let mut creds = String::new();
            channel.read_to_string(&mut creds)?;
            channel.wait_close()?;

            let mut admin_user = "admin".to_string();
            let mut admin_pass = "admin".to_string();
            for line in creds.lines() {
                if line.starts_with("GF_SECURITY_ADMIN_USER=") {
                    admin_user = line.split('=').nth(1).unwrap_or("admin").to_string();
                } else if line.starts_with("GF_SECURITY_ADMIN_PASSWORD=") {
                    admin_pass = line.split('=').nth(1).unwrap_or("admin").to_string();
                }
            }

            // Upload and import each dashboard
            for entry in fs::read_dir(&dashboards_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    let filename = path.file_name().unwrap().to_string_lossy();
                    let remote_path = format!("/tmp/{}", filename);

                    println!("   Restoring dashboard: {}", filename);

                    // Upload dashboard JSON
                    self.upload_file(&path.to_string_lossy(), &remote_path)?;

                    // Import dashboard via API
                    let import_cmd = format!(
                        "curl -s -u {}:'{}' -X POST -H 'Content-Type: application/json' -d @{} 'http://localhost:3000/api/dashboards/db'",
                        admin_user, admin_pass, remote_path
                    );
                    self.run_command(&session, &import_cmd, &format!("Importing {}", filename))?;

                    // Cleanup
                    self.run_command(&session, &format!("rm {}", remote_path), "Cleaning up")?;
                }
            }
        }

        // Restore datasources
        let datasources_file = format!("{}/grafana/datasources.json", backup_dir);
        if std::path::Path::new(&datasources_file).exists() {
            println!("   Restoring datasources...");

            // Get credentials again (in case not set above)
            let creds_cmd = "cd ~/monitoring && grep -E 'GF_SECURITY_ADMIN_USER|GF_SECURITY_ADMIN_PASSWORD' .env | head -2";
            let mut channel = session.channel_session()?;
            channel.exec(creds_cmd)?;
            let mut creds = String::new();
            channel.read_to_string(&mut creds)?;
            channel.wait_close()?;

            let mut admin_user = "admin".to_string();
            let mut admin_pass = "admin".to_string();
            for line in creds.lines() {
                if line.starts_with("GF_SECURITY_ADMIN_USER=") {
                    admin_user = line.split('=').nth(1).unwrap_or("admin").to_string();
                } else if line.starts_with("GF_SECURITY_ADMIN_PASSWORD=") {
                    admin_pass = line.split('=').nth(1).unwrap_or("admin").to_string();
                }
            }

            let datasources_remote = "/tmp/datasources.json";
            self.upload_file(&datasources_file, datasources_remote)?;

            let import_ds_cmd = format!(
                "curl -s -u {}:'{}' -X POST -H 'Content-Type: application/json' -d @{} 'http://localhost:3000/api/datasources'",
                admin_user, admin_pass, datasources_remote
            );
            self.run_command(&session, &import_ds_cmd, "Importing datasources")?;

            // Cleanup
            self.run_command(
                &session,
                &format!("rm {}", datasources_remote),
                "Cleaning up",
            )?;
        }

        // 3. Restore Prometheus data
        println!("\n📉 Restoring Prometheus data...");
        let prometheus_tar = format!("{}/prometheus/prometheus_data.tar.gz", backup_dir);

        if std::path::Path::new(&prometheus_tar).exists() {
            // Upload tar to device
            self.upload_file(&prometheus_tar, "/tmp/prometheus_restore.tar.gz")?;

            // Restore volume
            let restore_cmd = "docker run --rm -v prometheus_data:/data -v /tmp:/backup alpine sh -c 'cd /data && tar xzf /backup/prometheus_restore.tar.gz'";
            self.run_command(&session, restore_cmd, "Restoring Prometheus volume")?;

            // Cleanup
            self.run_command(&session, "rm /tmp/prometheus_restore.tar.gz", "Cleaning up")?;
        }

        // 4. Restore configuration
        println!("\n⚙️  Restoring configuration...");
        let env_backup = format!("{}/config/env_backup", backup_dir);

        if std::path::Path::new(&env_backup).exists() {
            self.upload_file(&env_backup, "~/monitoring/.env")?;
        }

        // Start monitoring stack
        println!("\n🚀 Starting monitoring stack...");
        self.start()?;

        println!("\n✅ Restore completed successfully!");
        println!("🔗 Access your services at:");
        println!("   - Grafana: http://{}:3000", self.config.host);
        println!("   - InfluxDB: http://{}:8086", self.config.host);

        Ok(())
    }

    /// Download monitoring folder locally and transfer to device
    fn download_and_transfer_monitoring_folder(
        &self,
        _session: &Session,
    ) -> Result<(), TelegrafError> {
        use std::fs;
        use std::process::Command;

        let temp_dir = std::env::temp_dir().join("iot2050-monitoring-transfer");
        let tar_file = temp_dir.join("monitoring.tar.gz");
        let extract_dir = temp_dir.join("extracted");

        // Clean up any existing temp directory
        if temp_dir.exists() {
            println!("🧹 Cleaning up temporary directory...");
            fs::remove_dir_all(&temp_dir).map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to clean temp directory: {}", e))
            })?;
        }

        // Create temp directories
        fs::create_dir_all(&extract_dir).map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to create temp directory: {}", e))
        })?;

        // Download tarball locally
        println!("📥 Downloading tarball from {}...", self.repo_url);
        let output = Command::new("curl")
            .args(&["-L", &self.repo_url, "-o", tar_file.to_str().unwrap()])
            .output()
            .map_err(|e| TelegrafError::ConfigError(format!("Failed to run curl: {}", e)))?;

        if !output.status.success() {
            return Err(TelegrafError::ConfigError(format!(
                "Failed to download tarball: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Extract tarball locally
        println!("📦 Extracting tarball locally...");
        // GitHub replaces '/' with '-' in branch names when creating tarballs
        let sanitized_branch = self.branch.replace('/', "-");
        let output = Command::new("tar")
            .args(&[
                "-xzf",
                tar_file.to_str().unwrap(),
                "-C",
                extract_dir.to_str().unwrap(),
                "--strip-components=2",
                &format!("iot2050-telegraf-config-{}/docker", sanitized_branch),
            ])
            .output()
            .map_err(|e| TelegrafError::ConfigError(format!("Failed to run tar: {}", e)))?;

        if !output.status.success() {
            return Err(TelegrafError::ConfigError(format!(
                "Failed to extract tarball: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Transfer extracted folder to device using SCP
        println!("📤 Transferring files to device...");
        self.transfer_directory(&extract_dir, "~/monitoring")?;

        // Clean up temp directory
        println!("🧹 Cleaning up local temporary files...");
        fs::remove_dir_all(&temp_dir).map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to clean up temp directory: {}", e))
        })?;

        println!("✅ Files transferred successfully");
        Ok(())
    }

    /// Transfer a local directory to remote device using SCP
    fn transfer_directory(
        &self,
        local_path: &std::path::Path,
        remote_path: &str,
    ) -> Result<(), TelegrafError> {
        use std::process::Command;

        // Build SCP command
        let mut scp_cmd = Command::new("scp");
        scp_cmd.args(&["-r", "-o", "StrictHostKeyChecking=no"]);

        // Add port if not default
        if self.config.port != 22 {
            scp_cmd.args(&["-P", &self.config.port.to_string()]);
        }

        // Add source (local directory contents)
        let source = format!("{}/*", local_path.display());
        scp_cmd.arg(&source);

        // Add destination
        let destination = format!("{}@{}:{}", self.config.user, self.config.host, remote_path);
        scp_cmd.arg(&destination);

        // Set password via environment if using password auth
        if let Some(password) = &self.config.password {
            // Use sshpass if available for password authentication
            let output = Command::new("which").arg("sshpass").output().map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to check for sshpass: {}", e))
            })?;

            if output.status.success() {
                // Use sshpass for password authentication
                let mut sshpass_cmd = Command::new("sshpass");
                sshpass_cmd.args(&["-p", password]);
                sshpass_cmd.arg("scp");
                sshpass_cmd.args(&["-r", "-o", "StrictHostKeyChecking=no"]);

                if self.config.port != 22 {
                    sshpass_cmd.args(&["-P", &self.config.port.to_string()]);
                }

                sshpass_cmd.arg(&source);
                sshpass_cmd.arg(&destination);

                let output = sshpass_cmd.output().map_err(|e| {
                    TelegrafError::ConfigError(format!("Failed to run sshpass: {}", e))
                })?;

                if !output.status.success() {
                    return Err(TelegrafError::ConfigError(format!(
                        "SCP transfer failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )));
                }
            } else {
                return Err(TelegrafError::ConfigError(
                    "Password authentication requires 'sshpass' to be installed. Install it with: sudo apt-get install sshpass".to_string()
                ));
            }
        } else if self.config.key_file.is_some() {
            // Key-based authentication
            if let Some(key_file) = &self.config.key_file {
                scp_cmd.args(&["-i", key_file]);
            }

            let output = scp_cmd
                .output()
                .map_err(|e| TelegrafError::ConfigError(format!("Failed to run scp: {}", e)))?;

            if !output.status.success() {
                return Err(TelegrafError::ConfigError(format!(
                    "SCP transfer failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        } else {
            return Err(TelegrafError::ConfigError(
                "No authentication method available for SCP".to_string(),
            ));
        }

        Ok(())
    }

    /// Create SSH session
    pub(crate) fn create_ssh_session(&self) -> Result<Session, TelegrafError> {
        let tcp = TcpStream::connect((self.config.host.as_str(), self.config.port))
            .map_err(|e| TelegrafError::SshOperationError(format!("Failed to connect: {}", e)))?;

        tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
        tcp.set_write_timeout(Some(Duration::from_secs(30)))?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // Authenticate
        if let Some(key_file) = &self.config.key_file {
            // Key-based authentication
            let key_path = Path::new(key_file);
            if key_path.exists() {
                session.userauth_pubkey_file(&self.config.user, None, key_path, None)?;
            } else {
                return Err(TelegrafError::SshOperationError(format!(
                    "SSH key file not found: {}",
                    key_file
                )));
            }
        } else if let Some(password) = &self.config.password {
            // Password authentication
            session.userauth_password(&self.config.user, password)?;
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
        &self,
        session: &Session,
        command: &str,
        description: &str,
    ) -> Result<(), TelegrafError> {
        println!("🔧 {}", description);

        // Check if this is a sudo command and we have a password
        let command = if command.contains("sudo ") && self.config.password.is_some() {
            // Use the -S flag to read password from stdin
            let password = self.config.password.as_ref().unwrap();
            format!(
                "echo '{}' | sudo -S {}",
                password,
                command.replace("sudo ", "")
            )
        } else {
            command.to_string()
        };

        let mut channel = session.channel_session()?;
        channel.exec(&command)?;

        use std::io::{BufReader, Read, Write};
        use std::time::{Duration, Instant};
        // ssh2::Channel is not used directly
        use std::thread;

        let mut stdout = channel.stream(0);
        let mut stderr = channel.stderr();
        let mut stdout_reader = BufReader::new(&mut stdout);
        let mut stderr_reader = BufReader::new(&mut stderr);
        let start_time = Instant::now();
        let timeout = Duration::from_secs(300); // 5 minutes

        let mut stdout_buf = [0u8; 1024];
        let mut stderr_buf = [0u8; 1024];
        loop {
            // Timeout check
            if start_time.elapsed() > timeout {
                println!("⏰ Command timed out after 5 minutes");
                channel.close().ok();
                return Err(TelegrafError::SshOperationError(format!(
                    "Command timed out: {}",
                    command
                )));
            }
            // Read stdout
            match stdout_reader.read(&mut stdout_buf) {
                Ok(n) if n > 0 => {
                    let s = String::from_utf8_lossy(&stdout_buf[..n]);
                    print!("{}", s);
                    std::io::stdout().flush().ok();
                }
                _ => {}
            }
            // Read stderr
            match stderr_reader.read(&mut stderr_buf) {
                Ok(n) if n > 0 => {
                    let s = String::from_utf8_lossy(&stderr_buf[..n]);
                    eprint!("{}", s);
                    std::io::stderr().flush().ok();
                }
                _ => {}
            }
            // Check if command is done
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
        &self,
        session: &Session,
        remote_path: &str,
        local_path: &str,
    ) -> Result<(), TelegrafError> {
        use std::fs::File;

        let (mut remote_file, _stat) = session.scp_recv(std::path::Path::new(remote_path))?;
        let mut local_file = File::create(local_path)?;
        std::io::copy(&mut remote_file, &mut local_file)?;

        Ok(())
    }

    /// Download a directory from remote device to local machine
    fn download_directory(
        &self,
        session: &Session,
        remote_path: &str,
        local_path: &str,
    ) -> Result<(), TelegrafError> {
        use std::fs::File;

        // List files in remote directory
        let mut channel = session.channel_session()?;
        channel.exec(&format!("ls {}", remote_path))?;
        let mut file_list = String::new();
        channel.read_to_string(&mut file_list)?;
        channel.wait_close()?;

        // Download each file
        for file_name in file_list.lines() {
            let remote_file = format!("{}/{}", remote_path, file_name);
            let local_file = format!("{}/{}", local_path, file_name);

            let (mut remote, _stat) = session.scp_recv(std::path::Path::new(&remote_file))?;
            let mut local = File::create(local_file)?;
            std::io::copy(&mut remote, &mut local)?;
        }

        Ok(())
    }

    /// Upload a file from local machine to remote device
    fn upload_file(&self, local_path: &str, remote_path: &str) -> Result<(), TelegrafError> {
        use std::process::Command;

        let mut scp_cmd = Command::new("scp");
        scp_cmd.args(&["-o", "StrictHostKeyChecking=no"]);

        if self.config.port != 22 {
            scp_cmd.args(&["-P", &self.config.port.to_string()]);
        }

        if let Some(key_file) = &self.config.key_file {
            scp_cmd.args(&["-i", key_file]);
        }

        scp_cmd.arg(local_path);
        scp_cmd.arg(format!(
            "{}@{}:{}",
            self.config.user, self.config.host, remote_path
        ));

        if let Some(password) = &self.config.password {
            // Use sshpass for password auth
            let mut sshpass_cmd = Command::new("sshpass");
            sshpass_cmd.args(&["-p", password, "scp", "-o", "StrictHostKeyChecking=no"]);

            if self.config.port != 22 {
                sshpass_cmd.args(&["-P", &self.config.port.to_string()]);
            }

            sshpass_cmd.arg(local_path);
            sshpass_cmd.arg(format!(
                "{}@{}:{}",
                self.config.user, self.config.host, remote_path
            ));

            let output = sshpass_cmd.output()?;
            if !output.status.success() {
                return Err(TelegrafError::ConfigError(format!(
                    "SCP upload failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        } else {
            let output = scp_cmd.output()?;
            if !output.status.success() {
                return Err(TelegrafError::ConfigError(format!(
                    "SCP upload failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }

        Ok(())
    }

    /// Upload a directory from local machine to remote device
    fn upload_directory(&self, local_path: &str, remote_path: &str) -> Result<(), TelegrafError> {
        use std::process::Command;

        // Create remote directory first
        let session = self.create_ssh_session()?;
        self.run_command(
            &session,
            &format!("mkdir -p {}", remote_path),
            "Creating remote directory",
        )?;

        let mut scp_cmd = Command::new("scp");
        scp_cmd.args(&["-r", "-o", "StrictHostKeyChecking=no"]);

        if self.config.port != 22 {
            scp_cmd.args(&["-P", &self.config.port.to_string()]);
        }

        if let Some(key_file) = &self.config.key_file {
            scp_cmd.args(&["-i", key_file]);
        }

        scp_cmd.arg(format!("{}/*", local_path));
        scp_cmd.arg(format!(
            "{}@{}:{}",
            self.config.user, self.config.host, remote_path
        ));

        if let Some(password) = &self.config.password {
            // Use sshpass for password auth
            let mut sshpass_cmd = Command::new("sshpass");
            sshpass_cmd.args(&[
                "-p",
                password,
                "scp",
                "-r",
                "-o",
                "StrictHostKeyChecking=no",
            ]);

            if self.config.port != 22 {
                sshpass_cmd.args(&["-P", &self.config.port.to_string()]);
            }

            sshpass_cmd.arg(format!("{}/*", local_path));
            sshpass_cmd.arg(format!(
                "{}@{}:{}",
                self.config.user, self.config.host, remote_path
            ));

            let output = sshpass_cmd.output()?;
            if !output.status.success() {
                return Err(TelegrafError::ConfigError(format!(
                    "SCP upload failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        } else {
            let output = scp_cmd.output()?;
            if !output.status.success() {
                return Err(TelegrafError::ConfigError(format!(
                    "SCP upload failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }

        Ok(())
    }

    /// Check if a command succeeds (returns true) or fails (returns false)
    fn check_command(&self, session: &Session, command: &str) -> Result<bool, TelegrafError> {
        let mut channel = session.channel_session()?;
        channel.exec(command)?;

        let mut _output = String::new();
        channel.read_to_string(&mut _output)?;
        channel.wait_close()?;

        let exit_status = channel.exit_status()?;
        Ok(exit_status == 0)
    }

    /// Check system time and warn if it appears incorrect
    fn check_system_time(&self, session: &Session) -> Result<(), TelegrafError> {
        println!("⏰ Checking system time...");

        // Get remote time with timezone
        let mut channel = session.channel_session()?;
        channel.exec("date '+%Y-%m-%d %H:%M:%S %Z (UTC%z)'")?;

        let mut remote_time_str = String::new();
        channel.read_to_string(&mut remote_time_str)?;
        channel.wait_close()?;

        println!("   Remote time: {}", remote_time_str.trim());

        // Get remote time as Unix timestamp for comparison
        let mut channel = session.channel_session()?;
        channel.exec("date +%s")?;

        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;

        if let Ok(remote_timestamp) = output.trim().parse::<i64>() {
            // Get local time
            let local_timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let diff_seconds = (remote_timestamp - local_timestamp).abs();
            let diff_hours = diff_seconds / 3600;

            // Warn if time difference is more than 3 hours (certificate validation typically fails)
            if diff_seconds > 10800 {
                // 3 hours in seconds
                println!(
                    "⚠️  WARNING: System time differs from local time by {} hours",
                    diff_hours
                );
                println!("⚠️  This will likely cause apt-get to fail due to certificate validation issues.");
                println!(
                    "⚠️  Device may be missing CMOS battery. Set time manually before continuing:"
                );
                println!("⚠️  sudo timedatectl set-time \"2025-10-01 09:11\"");
                println!("⚠️  Continuing anyway...");
            } else if diff_seconds > 300 {
                // More than 5 minutes but less than 3 hours
                println!(
                    "⚠️  WARNING: System time differs by {} seconds ({} minutes)",
                    diff_seconds,
                    diff_seconds / 60
                );
                println!("⚠️  This may cause issues. Consider syncing time if problems occur.");
            } else {
                println!(
                    "✅ System time is within {} seconds of local time",
                    diff_seconds
                );
            }
        }

        Ok(())
    }

    /// Check which packages need to be installed
    fn check_required_packages(&self, session: &Session) -> Result<PackageStatus, TelegrafError> {
        let mut status = PackageStatus::default();

        // Check base packages (just check a few key ones as indicators)
        let has_curl = self.check_command(session, "which curl")?;
        let has_git = self.check_command(session, "which git")?;
        let has_openssl = self.check_command(session, "which openssl")?;
        status.needs_base_packages = !(has_curl && has_git && has_openssl);

        if status.needs_base_packages {
            println!(
                "  ⚙️  Base packages needed (curl: {}, git: {}, openssl: {})",
                has_curl, has_git, has_openssl
            );
        } else {
            println!("  ✅ Base packages present");
        }

        // Check Docker
        status.needs_docker = !self.check_command(session, "docker --version")?;
        if status.needs_docker {
            println!("  🐳 Docker needs installation");
        } else {
            println!("  ✅ Docker present");
        }

        // Check Docker Compose
        status.needs_compose = !self.check_command(session, "docker-compose --version")?;
        if status.needs_compose {
            println!("  📦 Docker Compose needs installation");
        } else {
            println!("  ✅ Docker Compose present");
        }

        Ok(status)
    }
}

#[derive(Debug, Default)]
struct PackageStatus {
    needs_base_packages: bool,
    needs_docker: bool,
    needs_compose: bool,
}

impl PackageStatus {
    fn needs_installation(&self) -> bool {
        self.needs_base_packages || self.needs_docker || self.needs_compose
    }
}

impl IoTDeployer {
    /// Recursively upload a directory via SFTP
    fn upload_directory_sftp(
        &self,
        sftp: &ssh2::Sftp,
        local_path: &Path,
        remote_path: std::path::PathBuf,
    ) -> Result<(), TelegrafError> {
        use std::fs;

        // Try to create directory (ignore error if it exists)
        let _ = sftp.mkdir(&remote_path, 0o755);

        // Iterate through local directory
        for entry in fs::read_dir(local_path)
            .map_err(|e| TelegrafError::ConfigError(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry
                .map_err(|e| TelegrafError::ConfigError(format!("Failed to read entry: {}", e)))?;
            let local_file_path = entry.path();
            let file_name = entry.file_name();
            let remote_file_path = remote_path.join(&file_name);

            if local_file_path.is_dir() {
                // Recursively upload subdirectory
                self.upload_directory_sftp(sftp, &local_file_path, remote_file_path)?;
            } else {
                // Upload file
                let remote_file_str = remote_file_path.to_str().ok_or_else(|| {
                    TelegrafError::ConfigError("Invalid remote file path".to_string())
                })?;

                let mut local_file = fs::File::open(&local_file_path).map_err(|e| {
                    TelegrafError::ConfigError(format!("Failed to open local file: {}", e))
                })?;

                let mut remote_file = sftp.create(&remote_file_path).map_err(|e| {
                    TelegrafError::ConfigError(format!(
                        "Failed to create remote file {}: {}",
                        remote_file_str, e
                    ))
                })?;

                std::io::copy(&mut local_file, &mut remote_file).map_err(|e| {
                    TelegrafError::ConfigError(format!("Failed to upload file: {}", e))
                })?;
            }
        }

        Ok(())
    }
}
