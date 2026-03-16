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

        // Download and extract docker folder from the repository
        println!(
            "📥 Downloading docker configuration from branch '{}'...",
            self.branch
        );

        let download_cmd = format!(
            "cd ~/monitoring && curl -L {} | tar -xz --strip-components=2 iot2050-telegraf-config-{}/docker",
            self.repo_url, self.branch
        );

        self.run_command(&session, &download_cmd, "Downloading docker configuration")?;

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

            // Find the docker folder
            let docker_path = extract_dir
                .join(format!("iot2050-telegraf-config-{}", self.branch))
                .join("docker");

            println!("📤 Transferring files to device via SFTP...");

            // Use SFTP through the existing SSH session
            let sftp = session.sftp().map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to create SFTP session: {}", e))
            })?;

            // Recursively upload the docker folder
            self.upload_directory(
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

            let download_cmd = format!(
                "cd ~/monitoring && curl -L {} | tar -xz --strip-components=2 iot2050-telegraf-config-{}/docker",
                self.repo_url, self.branch
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
    fn upload_directory(
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
                self.upload_directory(sftp, &local_file_path, remote_file_path)?;
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
