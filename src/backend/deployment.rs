use crate::backend::ssh_utils;
use crate::error::TelegrafError;
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::time::Duration;

pub const GITHUB_REPO_OWNER: &str = "rlhf23";
pub const GITHUB_REPO_NAME: &str = "iot2050-telegraf-config";
pub const DEFAULT_BRANCH: &str = "master";

pub const DOCKER_IMAGES: &[(&str, &str)] = &[
    ("influxdb", "influxdb:2"),
    ("telegraf", "telegraf:1.30"),
    ("chronograf", "chronograf:1.10"),
    ("nginx", "nginx:alpine"),
    ("alpine", "alpine:3.19"),
    ("grafana", "grafana/grafana:latest"),
    ("prometheus", "prom/prometheus:latest"),
];

pub const MINIMAL_IMAGE_NAMES: &[&str] = &["influxdb", "telegraf", "chronograf", "nginx", "alpine"];

pub const CUSTOM_SERVICES: &[(&str, &str)] = &[
    ("api-service", "api-service/Dockerfile.prebuilt"),
    ("control-service", "control-service/Dockerfile.prebuilt"),
];

pub fn filter_pull_images(minimal: bool, image_filter: &Option<Vec<String>>) -> Vec<&'static str> {
    match image_filter {
        Some(filter) => {
            let filter_lower: Vec<String> = filter.iter().map(|s| s.to_lowercase()).collect();
            DOCKER_IMAGES
                .iter()
                .filter(|(name, _)| filter_lower.iter().any(|f| f == &name.to_lowercase()))
                .map(|(_, tag)| *tag)
                .collect()
        }
        None => {
            if minimal {
                DOCKER_IMAGES
                    .iter()
                    .filter(|(name, _)| MINIMAL_IMAGE_NAMES.contains(name))
                    .map(|(_, tag)| *tag)
                    .collect()
            } else {
                DOCKER_IMAGES.iter().map(|(_, tag)| *tag).collect()
            }
        }
    }
}

pub fn filter_custom_services(
    skip_custom: bool,
    image_filter: &Option<Vec<String>>,
) -> Vec<(&'static str, &'static str)> {
    if skip_custom {
        return vec![];
    }
    match image_filter {
        Some(filter) => {
            let filter_lower: Vec<String> = filter.iter().map(|s| s.to_lowercase()).collect();
            CUSTOM_SERVICES
                .iter()
                .filter(|(name, _)| filter_lower.iter().any(|f| f == &name.to_lowercase()))
                .cloned()
                .collect()
        }
        None => CUSTOM_SERVICES.to_vec(),
    }
}

pub fn image_tag_to_tar_filename(tag: &str) -> String {
    format!("{}.tar", tag.replace('/', "_").replace(':', "-"))
}

pub fn validate_push_flags(
    architecture: &Option<String>,
    save_dir: &Option<std::path::PathBuf>,
    load_dir: &Option<std::path::PathBuf>,
) -> Result<(), String> {
    if save_dir.is_some() && load_dir.is_some() {
        return Err("Cannot use --save-dir and --load-dir together.".to_string());
    }
    if save_dir.is_some() && architecture.is_none() {
        return Err(
            "--architecture is required when using --save-dir (no device to auto-detect from)."
                .to_string(),
        );
    }
    Ok(())
}

/// Sanitize a branch name for use in GitHub tarball URLs
/// GitHub replaces '/' with '-' in archive names
pub fn sanitize_branch_name(branch: &str) -> String {
    branch.replace('/', "-")
}

/// Generate the GitHub tarball URL for a given branch
pub fn generate_repo_url(branch: &str) -> String {
    format!(
        "https://github.com/{}/{}/archive/refs/heads/{}.tar.gz",
        GITHUB_REPO_OWNER, GITHUB_REPO_NAME, branch
    )
}

/// Get the expected directory name after extracting a tarball for a branch
pub fn get_extracted_dir_name(branch: &str) -> String {
    format!("{}-{}", GITHUB_REPO_NAME, sanitize_branch_name(branch))
}

#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub host: String,
    pub user: String,
    pub password: Option<String>,
    pub key_file: Option<String>,
    pub port: u16,
    pub git_branch: Option<String>,
    pub ship_display_name: Option<String>,
    pub ship_hostname: Option<String>,
    pub theme: Option<String>,
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
            ship_display_name: None,
            ship_hostname: None,
            theme: None,
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

    /// Set the ship name (both display name and derived hostname)
    pub fn with_ship_name(mut self, display_name: String, hostname: String) -> Self {
        self.ship_display_name = Some(display_name);
        self.ship_hostname = Some(hostname);
        self
    }

    pub fn with_theme(mut self, theme: String) -> Self {
        self.theme = Some(theme);
        self
    }
}

pub struct IoTDeployer {
    config: DeploymentConfig,
    repo_url: String,
    branch: String,
    minimal: bool,
    progress_sender: Option<Sender<String>>,
}

impl IoTDeployer {
    fn progress(&self, msg: &str) {
        if let Some(sender) = &self.progress_sender {
            let _ = sender.send(msg.to_string());
        } else {
            println!("{}", msg);
        }
    }

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

        self.progress("🔍 Detecting device architecture...");

        let mut channel = session.channel_session()?;
        channel.exec("uname -m")?;

        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;

        let arch = output.trim();

        let targetarch = match arch {
            "x86_64" | "amd64" => {
                self.progress("   Detected x86_64 architecture -> using amd64 binaries");
                "amd64".to_string()
            }
            "aarch64" | "arm64" => {
                self.progress("   Detected ARM64 architecture -> using arm64 binaries");
                "arm64".to_string()
            }
            _ => {
                self.progress(&format!(
                    "   ⚠️  Unknown architecture '{}', defaulting to arm64",
                    arch
                ));
                "arm64".to_string()
            }
        };

        Ok(targetarch)
    }

    pub fn new(config: DeploymentConfig) -> Self {
        let branch = config
            .git_branch
            .clone()
            .unwrap_or_else(|| DEFAULT_BRANCH.to_string());
        let repo_url = generate_repo_url(&branch);

        Self {
            config,
            repo_url,
            branch,
            minimal: false,
            progress_sender: None,
        }
    }

    pub fn with_minimal(mut self, minimal: bool) -> Self {
        self.minimal = minimal;
        self
    }

    pub fn with_progress_sender(mut self, sender: Sender<String>) -> Self {
        self.progress_sender = Some(sender);
        self
    }

    /// Test SSH connectivity to the device
    pub fn test_connection(&self) -> Result<(), TelegrafError> {
        self.progress(&format!(
            "🔧 Testing SSH connectivity to {}:{}...",
            self.config.host, self.config.port
        ));

        let session = self.create_ssh_session()?;
        let mut channel = session.channel_session()?;
        channel.exec("echo 'SSH connection successful'")?;

        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;

        if channel.exit_status()? == 0 {
            self.progress("✅ SSH connectivity verified");
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
        self.provision_with_options(local_transfer, None, None)
    }

    /// Provision with full options including airgap support
    pub fn provision_with_options(
        &self,
        local_transfer: bool,
        save_dir: Option<std::path::PathBuf>,
        load_dir: Option<std::path::PathBuf>,
    ) -> Result<(), TelegrafError> {
        if save_dir.is_some() && load_dir.is_some() {
            return Err(TelegrafError::ConfigError(
                "Cannot use --save-dir and --load-dir together.".to_string(),
            ));
        }

        // Save-only mode: download from GitHub and save locally, no device needed
        if let Some(dir) = &save_dir {
            return self.save_config_to_dir(dir);
        }

        self.progress("🚀 Starting device provisioning...");

        let session = self.create_ssh_session()?;

        // Check internet connectivity
        self.progress("🌐 Checking internet connectivity...");
        self.run_command(
            &session,
            "ping -c 1 www.google.com > /dev/null 2>&1 || { echo 'Error: No internet connectivity'; exit 1; }",
            "Verifying internet connectivity"
        )?;

        // Check repository accessibility
        self.progress("🔍 Verifying repository accessibility...");
        self.run_command(
            &session,
            &format!("curl -s -o /dev/null -I -w '%{{http_code}}' {} | grep -q '200\\|302' || {{ echo 'Error: Cannot access repository branch: {}'; exit 1; }}", self.repo_url, self.branch),
            &format!("Checking if repository branch '{}' is accessible", self.branch)
        )?;

        // Check if all required packages are already installed
        self.progress("🔍 Checking for required packages...");
        let packages_status = self.check_required_packages(&session)?;

        // Only proceed with apt operations if packages are missing
        if packages_status.needs_installation() {
            // Warn about potential time issues before running apt
            self.check_system_time(&session)?;

            self.progress("📦 Installing missing packages...");

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
                self.progress("🐳 Installing Docker...");
                self.run_command(
                    &session,
                    "sudo apt-get install -y docker.io",
                    "Installing Docker",
                )?;
            }

            // Install Docker Compose if needed
            if packages_status.needs_compose {
                self.progress("📦 Installing Docker Compose...");
                self.run_command(
                    &session,
                    "sudo apt-get install -y docker-compose",
                    "Installing Docker Compose",
                )?;
            }
        } else {
            self.progress("✅ All required packages are already installed");
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
        self.progress("📂 Setting up monitoring directory...");

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

        // Download docker folder
        if let Some(dir) = &load_dir {
            self.progress("📥 Loading configuration from local directory...");
            self.transfer_config_to_device(&session, dir)?;
        } else if local_transfer {
            self.progress(&format!(
                "📥 Downloading docker configuration locally from branch '{}'...",
                self.branch
            ));
            self.download_and_transfer_monitoring_folder(&session)?;
        } else {
            self.progress(&format!(
                "📥 Downloading docker configuration directly on device from branch '{}'...",
                self.branch
            ));

            let download_cmd = format!(
                "cd ~/monitoring && curl -L {} | tar -xz --strip-components=2 {}/docker",
                self.repo_url,
                get_extracted_dir_name(&self.branch)
            );

            self.run_command(&session, &download_cmd, "Downloading docker configuration")?;
        }
        // Make scripts executable
        self.run_command(
            &session,
            "cd ~/monitoring && chmod +x scripts/*.sh config/nginx/scripts/*.sh",
            "Making scripts executable",
        )?;

        self.progress("✅ Device provisioning completed successfully!");
        Ok(())
    }

    /// Update monitoring configuration on the device (download fresh copy from git)
    pub fn update(&self, use_local: bool) -> Result<(), TelegrafError> {
        self.update_with_options(use_local, None, None)
    }

    /// Update with full options including airgap support
    pub fn update_with_options(
        &self,
        use_local: bool,
        save_dir: Option<std::path::PathBuf>,
        load_dir: Option<std::path::PathBuf>,
    ) -> Result<(), TelegrafError> {
        if save_dir.is_some() && load_dir.is_some() {
            return Err(TelegrafError::ConfigError(
                "Cannot use --save-dir and --load-dir together.".to_string(),
            ));
        }

        // Save-only mode: download from GitHub and save locally, no device needed
        if let Some(dir) = &save_dir {
            return self.save_config_to_dir(dir);
        }

        self.progress("🔄 Updating monitoring configuration...");

        let session = self.create_ssh_session()?;

        // Backup .env and telegraf.conf before removing monitoring directory
        self.progress("💾 Backing up configuration files...");
        self.run_command(
            &session,
            "cp ~/monitoring/.env ~/monitoring.env.backup 2>/dev/null || true",
            "Backing up .env file",
        )?;
        self.run_command(
            &session,
            "cp ~/telegraf/telegraf.conf ~/telegraf.conf.backup 2>/dev/null || true",
            "Backing up telegraf.conf",
        )?;

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

        if let Some(dir) = &load_dir {
            // Airgap mode: upload from local directory
            self.progress("📥 Loading configuration from local directory...");
            self.transfer_config_to_device(&session, dir)?;
        } else if use_local {
            // Download locally and transfer via SFTP
            self.progress(&format!(
                "📥 Downloading docker configuration locally from branch '{}'...",
                self.branch
            ));

            let temp_dir = std::env::temp_dir().join(format!("monitoring-{}", self.branch));
            std::fs::create_dir_all(&temp_dir)?;

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

            let extract_dir = temp_dir.join("extracted");
            std::fs::create_dir_all(&extract_dir)?;

            // Extract only the docker/ directory, stripping the top-level branch prefix.
            // This avoids extracting .direnv, .git, etc. and works on Windows
            // where symlinks in those directories would cause tar to fail.
            let extract_output = std::process::Command::new("tar")
                .args([
                    "-xzf",
                    tarball_path.to_str().unwrap(),
                    "-C",
                    extract_dir.to_str().unwrap(),
                    "--strip-components=1",
                    &format!("{}/docker", get_extracted_dir_name(&self.branch)),
                ])
                .output()
                .map_err(|e| TelegrafError::ConfigError(format!("Failed to extract: {}", e)))?;

            if !extract_output.status.success() {
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(TelegrafError::ConfigError(
                    "Failed to extract tarball".to_string(),
                ));
            }

            let docker_path = extract_dir.join("docker");

            let sftp = session.sftp().map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to create SFTP session: {}", e))
            })?;

            self.upload_directory_sftp(
                &sftp,
                &docker_path,
                Path::new("/home").join(self.user()).join("monitoring"),
            )?;

            let _ = std::fs::remove_dir_all(&temp_dir);
        } else {
            // Download directly on the device (requires internet)
            self.progress(&format!(
                "📥 Downloading docker configuration on device from branch '{}'...",
                self.branch
            ));

            let has_internet =
                self.check_command(&session, "ping -c 1 www.google.com > /dev/null 2>&1")?;

            if !has_internet {
                return Err(TelegrafError::ConfigError(
                    "Device has no internet connectivity. Use --local flag to download and transfer from your machine instead.".to_string()
                ));
            }

            let download_cmd = format!(
                "cd ~/monitoring && curl -L {} | tar -xz --strip-components=2 {}/docker",
                self.repo_url,
                get_extracted_dir_name(&self.branch)
            );

            self.run_command(&session, &download_cmd, "Downloading docker configuration")?;
        }

        // Make scripts executable
        self.run_command(
            &session,
            "cd ~/monitoring && chmod +x scripts/*.sh config/nginx/scripts/*.sh",
            "Making scripts executable",
        )?;

        // Restore backed up configuration files
        self.progress("💾 Restoring configuration files...");
        let env_backup_exists = self.check_command(&session, "test -f ~/monitoring.env.backup")?;
        if env_backup_exists {
            self.run_command(
                &session,
                "mv ~/monitoring.env.backup ~/monitoring/.env",
                "Restoring .env file",
            )?;
            // Update TARGETARCH in .env for current device architecture
            let targetarch = self.detect_architecture()?;
            let update_arch_cmd = format!(
                "cd ~/monitoring && grep -v '^TARGETARCH=' .env > .env.tmp && mv .env.tmp .env && echo 'TARGETARCH={}' >> .env",
                targetarch
            );
            self.run_command(&session, &update_arch_cmd, "Updating architecture in .env")?;
        } else {
            self.progress("ℹ️  No .env backup found - run 'setup' to create one");
        }

        let telegraf_backup_exists =
            self.check_command(&session, "test -f ~/telegraf.conf.backup")?;
        if telegraf_backup_exists {
            self.run_command(
                &session,
                "mkdir -p ~/telegraf && mv ~/telegraf.conf.backup ~/telegraf/telegraf.conf",
                "Restoring telegraf.conf",
            )?;
        }

        self.progress("✅ Monitoring configuration updated successfully!");
        Ok(())
    }

    /// Run setup on the device (assumes provisioning is already done)
    pub fn setup(&self) -> Result<(), TelegrafError> {
        self.progress("🚀 Starting setup...");

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

        // Build environment variables for setup script
        let mut env_vars = format!("TARGETARCH={}", targetarch);

        if let Some(ref name) = self.config.ship_display_name {
            env_vars.push_str(&format!(" DEVICE_DISPLAY_NAME='{}'", name));
        }
        if let Some(ref host) = self.config.ship_hostname {
            env_vars.push_str(&format!(" DEVICE_HOSTNAME='{}'", host));
        }
        if let Some(ref theme) = self.config.theme {
            env_vars.push_str(&format!(" THEME='{}'", theme));
        }

        let setup_cmd = if self.minimal {
            self.progress("📦 Using minimal profile (TICK stack only)");
            format!(
                "cd ~/monitoring && {} ./scripts/setup.sh --minimal",
                env_vars
            )
        } else {
            format!("cd ~/monitoring && {} ./scripts/setup.sh", env_vars)
        };

        self.run_command(&session, &setup_cmd, "Running setup")?;

        self.progress(&format!(
            "✅ Setup completed successfully! (Architecture: {})",
            targetarch
        ));
        Ok(())
    }

    /// Sync system time from local machine to device
    pub fn sync_time(&self) -> Result<(), TelegrafError> {
        self.progress("🕐 Syncing time to device...");

        let session = self.create_ssh_session()?;

        // Get UTC time in format suitable for `date` command
        let utc_time = chrono::Utc::now();
        let time_str = utc_time.format("%Y-%m-%d %H:%M:%S").to_string();

        self.progress(&format!("📅 UTC time: {}", time_str));

        // Set time on device (requires sudo)
        // Use -u flag to interpret time as UTC, avoiding timezone issues
        let set_time_cmd = format!("sudo date -u -s '{}'", time_str);

        self.run_command(&session, &set_time_cmd, "Setting device time")?;

        // Verify the time was set
        let mut channel = session.channel_session()?;
        channel.exec("date")?;

        let mut device_time = String::new();
        channel.read_to_string(&mut device_time)?;
        channel.wait_close()?;

        self.progress(&format!("✅ Device time updated: {}", device_time.trim()));

        Ok(())
    }

    /// Get deployment status
    pub fn status(&self) -> Result<(), TelegrafError> {
        self.progress("📊 Checking deployment status...");

        let session = self.create_ssh_session()?;

        // Check if monitoring directory exists
        if !self.check_command(&session, "test -d ~/monitoring")? {
            self.progress("❌ Monitoring stack not deployed");
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

        self.progress("Container Status:");
        self.progress(&output.to_string());

        // Get credentials
        let mut channel = session.channel_session()?;
        channel.exec("cd ~/monitoring && cat .env | grep -E '(GRAFANA_ADMIN_|INFLUXDB_)' | grep -E '(USER|PASSWORD|TOKEN)='")?;

        let mut credentials = String::new();
        channel.read_to_string(&mut credentials)?;
        channel.wait_close()?;

        if !credentials.trim().is_empty() {
            self.progress("\n🔑 Credentials:");
            self.progress(&credentials.to_string());
        }

        self.progress("\n🔗 Access URLs:");
        self.progress(&format!("  - InfluxDB: http://{}:8086", self.config.host));
        self.progress(&format!("  - Chronograf: http://{}:8888", self.config.host));

        // Check which profile is active by looking for COMPOSE_PROFILE in .env
        let mut profile_channel = session.channel_session()?;
        profile_channel.exec("cd ~/monitoring && grep -q '^COMPOSE_PROFILE=minimal' .env 2>/dev/null && echo minimal || echo full")?;

        let mut profile_output = String::new();
        profile_channel.read_to_string(&mut profile_output)?;
        profile_channel.wait_close()?;

        let is_minimal = profile_output.trim() == "minimal";

        if !is_minimal {
            self.progress(&format!("  - Grafana: http://{}:3000", self.config.host));
        }

        Ok(())
    }

    /// Stop the monitoring stack
    pub fn stop(&self) -> Result<(), TelegrafError> {
        self.stop_with_volumes(false)
    }

    /// Stop the monitoring stack with optional volume removal
    pub fn stop_with_volumes(&self, remove_volumes: bool) -> Result<(), TelegrafError> {
        self.progress("🛑 Stopping monitoring stack...");

        let session = self.create_ssh_session()?;

        let stop_command = if remove_volumes {
            "cd ~/monitoring && (docker compose down || docker-compose down) && docker volume prune -f"
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
            self.progress("✅ Monitoring stack stopped and volumes removed");
            self.progress("⚠️  All data has been deleted. You will need to reconfigure services on next start.");
        } else {
            self.progress("✅ Monitoring stack stopped");
        }
        Ok(())
    }

    /// Start the monitoring stack
    pub fn start(&self) -> Result<(), TelegrafError> {
        self.progress("🚀 Starting monitoring stack...");

        let session = self.create_ssh_session()?;

        self.run_command(
            &session,
            "cd ~/monitoring && ./scripts/start.sh",
            "Starting monitoring stack",
        )?;

        self.progress("✅ Monitoring stack started");
        Ok(())
    }

    /// Backup all monitoring data (InfluxDB, Grafana, Prometheus) to local directory
    pub fn backup(&self, output_dir: Option<String>) -> Result<String, TelegrafError> {
        crate::backend::backup::backup_with_progress(
            &self.config,
            output_dir,
            self.progress_sender.as_ref(),
        )
    }

    /// Restore monitoring data from backup archive
    pub fn restore(&self, archive_path: String, force: bool) -> Result<(), TelegrafError> {
        crate::backend::backup::restore_with_progress(
            &self.config,
            archive_path,
            force,
            self.progress_sender.as_ref(),
        )
    }

    /// Download configuration from GitHub and save to a local directory (airgap step 1).
    /// No device connection needed.
    pub fn save_config_to_dir(&self, save_dir: &std::path::Path) -> Result<(), TelegrafError> {
        self.progress(&format!(
            "📥 Downloading configuration from branch '{}' to {}...",
            self.branch,
            save_dir.display()
        ));

        let target_dir = save_dir.join("docker");

        if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir).map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to remove existing directory: {}", e))
            })?;
        }
        std::fs::create_dir_all(save_dir)?;

        let temp_dir = std::env::temp_dir().join(format!("iot2050-config-save-{}", self.branch));

        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to clean temp directory: {}", e))
            })?;
        }
        std::fs::create_dir_all(&temp_dir)?;

        let tarball_path = temp_dir.join("docker.tar.gz");

        let output = std::process::Command::new("curl")
            .args(["-L", &self.repo_url, "-o"])
            .arg(&tarball_path)
            .output()
            .map_err(|e| TelegrafError::ConfigError(format!("Failed to run curl: {}", e)))?;

        if !output.status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(TelegrafError::ConfigError(format!(
                "Failed to download tarball: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        self.progress("📦 Extracting docker/ directory...");

        // Extract only the docker/ directory, stripping the top-level branch prefix.
        // This avoids extracting .direnv, .git, etc. and works on Windows
        // where symlinks in those directories would cause tar to fail.
        let extract_output = std::process::Command::new("tar")
            .args([
                "-xzf",
                tarball_path.to_str().unwrap(),
                "-C",
                save_dir.to_str().unwrap(),
                "--strip-components=1",
                &format!("{}/docker", get_extracted_dir_name(&self.branch)),
            ])
            .output()
            .map_err(|e| TelegrafError::ConfigError(format!("Failed to extract: {}", e)))?;

        if !extract_output.status.success() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(TelegrafError::ConfigError(format!(
                "Failed to extract tarball: {}",
                String::from_utf8_lossy(&extract_output.stderr)
            )));
        }

        let _ = std::fs::remove_dir_all(&temp_dir);

        self.progress(&format!("✅ Configuration saved to {}", save_dir.display()));
        Ok(())
    }

    /// Upload a local config directory to the device (airgap step 2 for update).
    /// Expects `load_dir/docker/` to contain the configuration files.
    fn transfer_config_to_device(
        &self,
        session: &Session,
        load_dir: &std::path::Path,
    ) -> Result<(), TelegrafError> {
        let docker_path = load_dir.join("docker");

        if !docker_path.exists() {
            return Err(TelegrafError::ConfigError(format!(
                "Expected directory not found: {}. Run with --save-dir first.",
                docker_path.display()
            )));
        }

        self.progress("📤 Transferring configuration files to device via SFTP...");

        let sftp = session.sftp().map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to create SFTP session: {}", e))
        })?;

        self.upload_directory_sftp(
            &sftp,
            &docker_path,
            Path::new("/home").join(self.user()).join("monitoring"),
        )?;

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
            self.progress("🧹 Cleaning up temporary directory...");
            fs::remove_dir_all(&temp_dir).map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to clean temp directory: {}", e))
            })?;
        }

        // Create temp directories
        fs::create_dir_all(&extract_dir).map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to create temp directory: {}", e))
        })?;

        // Download tarball locally
        self.progress(&format!("📥 Downloading tarball from {}...", self.repo_url));
        let output = Command::new("curl")
            .args(["-L", &self.repo_url, "-o", tar_file.to_str().unwrap()])
            .output()
            .map_err(|e| TelegrafError::ConfigError(format!("Failed to run curl: {}", e)))?;

        if !output.status.success() {
            return Err(TelegrafError::ConfigError(format!(
                "Failed to download tarball: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Extract tarball locally
        self.progress("📦 Extracting tarball locally...");
        let output = Command::new("tar")
            .args([
                "-xzf",
                tar_file.to_str().unwrap(),
                "-C",
                extract_dir.to_str().unwrap(),
                "--strip-components=2",
                &format!("{}/docker", get_extracted_dir_name(&self.branch)),
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
        self.progress("📤 Transferring files to device...");
        self.transfer_directory(&extract_dir, "~/monitoring")?;

        // Clean up temp directory
        self.progress("🧹 Cleaning up local temporary files...");
        fs::remove_dir_all(&temp_dir).map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to clean up temp directory: {}", e))
        })?;

        self.progress("✅ Files transferred successfully");
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
        scp_cmd.args(["-r", "-o", "StrictHostKeyChecking=no"]);

        // Add port if not default
        if self.config.port != 22 {
            scp_cmd.args(["-P", &self.config.port.to_string()]);
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
                sshpass_cmd.args(["-p", password]);
                sshpass_cmd.arg("scp");
                sshpass_cmd.args(["-r", "-o", "StrictHostKeyChecking=no"]);

                if self.config.port != 22 {
                    sshpass_cmd.args(["-P", &self.config.port.to_string()]);
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
                scp_cmd.args(["-i", key_file]);
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
        ssh_utils::run_command_with_progress(
            session,
            command,
            description,
            self.config.password.as_ref(),
            self.progress_sender.as_ref(),
        )
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
        self.progress("⏰ Checking system time...");

        // Get remote time with timezone
        let mut channel = session.channel_session()?;
        channel.exec("date '+%Y-%m-%d %H:%M:%S %Z (UTC%z)'")?;

        let mut remote_time_str = String::new();
        channel.read_to_string(&mut remote_time_str)?;
        channel.wait_close()?;

        self.progress(&format!("   Remote time: {}", remote_time_str.trim()));

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
                self.progress(&format!(
                    "⚠️  WARNING: System time differs from local time by {} hours",
                    diff_hours
                ));
                self.progress("⚠️  This will likely cause apt-get to fail due to certificate validation issues.");
                self.progress(
                    "⚠️  Device may be missing CMOS battery. Set time manually before continuing:",
                );
                self.progress("⚠️  sudo timedatectl set-time \"2025-10-01 09:11\"");
                self.progress("⚠️  Continuing anyway...");
            } else if diff_seconds > 300 {
                // More than 5 minutes but less than 3 hours
                self.progress(&format!(
                    "⚠️  WARNING: System time differs by {} seconds ({} minutes)",
                    diff_seconds,
                    diff_seconds / 60
                ));
                self.progress(
                    "⚠️  This may cause issues. Consider syncing time if problems occur.",
                );
            } else {
                self.progress(&format!(
                    "✅ System time is within {} seconds of local time",
                    diff_seconds
                ));
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
            self.progress(&format!(
                "  ⚙️  Base packages needed (curl: {}, git: {}, openssl: {})",
                has_curl, has_git, has_openssl
            ));
        } else {
            self.progress("  ✅ Base packages present");
        }

        // Check Docker
        status.needs_docker = !self.check_command(session, "docker --version")?;
        if status.needs_docker {
            self.progress("  🐳 Docker needs installation");
        } else {
            self.progress("  ✅ Docker present");
        }

        // Check Docker Compose
        status.needs_compose = !self.check_command(session, "docker-compose --version")?;
        if status.needs_compose {
            self.progress("  📦 Docker Compose needs installation");
        } else {
            self.progress("  ✅ Docker Compose present");
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
    /// Push Docker images to the device for offline deployment.
    ///
    /// Pulls all required images locally (cross-arch if needed), builds custom
    /// service images, saves them to tar files, transfers them to the device
    /// via SFTP, and loads them into the device's Docker daemon.
    ///
    /// If `architecture` is None, it will be auto-detected from the device.
    /// If `minimal` is true, Grafana and Prometheus images are skipped.
    pub fn push_images(
        &self,
        architecture: Option<String>,
        minimal: bool,
        skip_custom: bool,
        image_filter: Option<Vec<String>>,
        save_dir: Option<std::path::PathBuf>,
        load_dir: Option<std::path::PathBuf>,
    ) -> Result<(), TelegrafError> {
        validate_push_flags(&architecture, &save_dir, &load_dir)
            .map_err(TelegrafError::ConfigError)?;

        let (arch, needs_device) = if load_dir.is_some() {
            (architecture.unwrap_or_else(|| "arm64".to_string()), true)
        } else if save_dir.is_some() {
            (architecture.unwrap_or_else(|| unreachable!()), false)
        } else {
            let a = match architecture {
                Some(a) => {
                    self.progress(&format!("   Using specified architecture: {}", a));
                    a
                }
                None => {
                    self.progress("   Detecting device architecture...");
                    self.detect_architecture()?
                }
            };
            (a, true)
        };

        let platform = format!("linux/{}", arch);
        let docker_context = self.find_docker_context()?;

        self.progress(&format!("   Target platform: {}", platform));
        if !needs_device {
            self.progress(
                "   Save-only mode: images will be saved locally (no device connection needed)",
            );
        } else if load_dir.is_some() {
            self.progress(
                "   Load-only mode: using pre-saved image tars (no Docker needed on host)",
            );
        }
        self.progress(&format!("   Docker context: {}", docker_context.display()));

        let images_to_pull = filter_pull_images(minimal, &image_filter);
        let services_to_build =
            filter_custom_services(skip_custom || load_dir.is_some(), &image_filter);

        if images_to_pull.is_empty() && services_to_build.is_empty() {
            self.progress("ℹ️  No images selected. Use --images to specify which images to push, or omit it to push all.");
            return Ok(());
        }

        // --- Phase 1: Pull, build, save ---
        let output_dir = save_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("iot2050-image-push"));
        // Create the output directory if it doesn't exist.
        // We do NOT delete existing .tar files here: the airgap workflow
        // allows incremental saves (e.g. --images influxdb, then --images
        // api-service) into the same directory.  docker save -o overwrites
        // any existing file of the same name, so stale tars from a previous
        // run are replaced naturally.
        std::fs::create_dir_all(&output_dir).map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to create output directory: {}", e))
        })?;

        let mut tar_files: Vec<(String, std::path::PathBuf)> = Vec::new();

        if load_dir.is_none() {
            for image in &images_to_pull {
                self.progress(&format!("📦 Pulling {} (platform: {})...", image, platform));

                let pull_output = std::process::Command::new("docker")
                    .args(["pull", "--platform", &platform, image])
                    .output()
                    .map_err(|e| {
                        TelegrafError::ConfigError(format!(
                            "Failed to run docker pull: {}. Is Docker installed and running?",
                            e
                        ))
                    })?;

                if !pull_output.status.success() {
                    let stderr = String::from_utf8_lossy(&pull_output.stderr);
                    return Err(TelegrafError::ConfigError(format!(
                        "Failed to pull image {}: {}",
                        image, stderr
                    )));
                }

                let tar_path = output_dir.join(image_tag_to_tar_filename(image));

                self.progress(&format!("💾 Saving {} to tar...", image));

                let save_output = std::process::Command::new("docker")
                    .args(["save", "-o", tar_path.to_str().unwrap(), image])
                    .output()
                    .map_err(|e| {
                        TelegrafError::ConfigError(format!("Failed to run docker save: {}", e))
                    })?;

                if !save_output.status.success() {
                    let stderr = String::from_utf8_lossy(&save_output.stderr);
                    return Err(TelegrafError::ConfigError(format!(
                        "Failed to save image {}: {}",
                        image, stderr
                    )));
                }

                tar_files.push((image.to_string(), tar_path));
            }

            if !services_to_build.is_empty() {
                for (service_name, dockerfile) in &services_to_build {
                    let image_tag = format!("{}:local", service_name);

                    self.progress(&format!("🔨 Building custom image {}...", image_tag));

                    let build_output = std::process::Command::new("docker")
                        .args([
                            "buildx",
                            "build",
                            "--platform",
                            &platform,
                            "-f",
                            dockerfile,
                            "--build-arg",
                            &format!("TARGETARCH={}", arch),
                            "-t",
                            &image_tag,
                            "--load",
                            ".",
                        ])
                        .current_dir(&docker_context)
                        .output()
                        .map_err(|e| {
                            TelegrafError::ConfigError(format!(
                                "Failed to run docker buildx: {}",
                                e
                            ))
                        })?;

                    if !build_output.status.success() {
                        let stderr = String::from_utf8_lossy(&build_output.stderr);
                        return Err(TelegrafError::ConfigError(format!(
                            "Failed to build {}: {}",
                            image_tag, stderr
                        )));
                    }

                    let tar_path = output_dir.join(format!("{}.tar", service_name));

                    self.progress(&format!("💾 Saving {} to tar...", image_tag));

                    let save_output = std::process::Command::new("docker")
                        .args(["save", "-o", tar_path.to_str().unwrap(), &image_tag])
                        .output()
                        .map_err(|e| {
                            TelegrafError::ConfigError(format!("Failed to run docker save: {}", e))
                        })?;

                    if !save_output.status.success() {
                        let stderr = String::from_utf8_lossy(&save_output.stderr);
                        return Err(TelegrafError::ConfigError(format!(
                            "Failed to save image {}: {}",
                            image_tag, stderr
                        )));
                    }

                    tar_files.push((image_tag, tar_path));
                }
            } else if skip_custom {
                self.progress("⏭️  Skipping custom service images (--skip-custom)");
            }
        }

        // Save-only mode: stop here
        if save_dir.is_some() {
            let file_count = tar_files.len();
            self.progress(&format!(
                "✅ Saved {} image tar(s) to {}",
                file_count,
                output_dir.display()
            ));
            return Ok(());
        }

        // --- If load_dir, discover tars from that directory ---
        if let Some(dir) = &load_dir {
            self.progress(&format!("📂 Loading tars from {}...", dir.display()));
            for entry in std::fs::read_dir(dir).map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to read load directory: {}", e))
            })? {
                let entry = entry.map_err(|e| {
                    TelegrafError::ConfigError(format!("Failed to read directory entry: {}", e))
                })?;
                let path = entry.path();
                if path.extension().map(|e| e == "tar").unwrap_or(false) {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    tar_files.push((name, path));
                }
            }
            if tar_files.is_empty() {
                return Err(TelegrafError::ConfigError(
                    "No .tar files found in the specified directory.".to_string(),
                ));
            }
        }

        // --- Phase 2: Transfer tar files to device ---
        self.progress("📤 Transferring images to device...");

        let session = self.create_ssh_session()?;

        self.run_command(
            &session,
            "mkdir -p ~/monitoring/images",
            "Creating images directory on device",
        )?;

        let sftp = session.sftp().map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to create SFTP session: {}", e))
        })?;

        let remote_base = Path::new("/home")
            .join(self.user())
            .join("monitoring")
            .join("images");

        for (image_name, tar_path) in &tar_files {
            let file_size = std::fs::metadata(tar_path).map(|m| m.len()).unwrap_or(0);
            let size_mb = file_size as f64 / (1024.0 * 1024.0);
            self.progress(&format!(
                "   Uploading {} ({:.1} MB)...",
                image_name, size_mb
            ));

            let remote_path = remote_base.join(tar_path.file_name().unwrap());

            let mut local_file = std::fs::File::open(tar_path).map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to open tar file: {}", e))
            })?;

            let mut remote_file = sftp.create(&remote_path).map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to create remote file: {}", e))
            })?;

            std::io::copy(&mut local_file, &mut remote_file).map_err(|e| {
                TelegrafError::ConfigError(format!("Failed to upload image tar: {}", e))
            })?;
        }

        // --- Phase 3: Load images on device ---
        self.progress("📥 Loading images on device...");

        for (image_name, tar_path) in &tar_files {
            let tar_filename = tar_path.file_name().unwrap().to_str().unwrap();
            self.progress(&format!("   Loading {}...", image_name));

            let load_cmd = format!("docker load -i ~/monitoring/images/{}", tar_filename);

            self.run_command(
                &session,
                &load_cmd,
                &format!("Loading image {}", image_name),
            )?;
        }

        // Clean up local temp files (only if we created them)
        if load_dir.is_none() {
            self.progress("🧹 Cleaning up local temporary files...");
            let temp_dir = std::env::temp_dir().join("iot2050-image-push");
            let _ = std::fs::remove_dir_all(temp_dir);
        }

        let image_count = tar_files.len();
        self.progress(&format!(
            "✅ Successfully pushed {} images to device!",
            image_count
        ));
        Ok(())
    }

    /// Find the docker context directory by looking for docker-compose.yml
    fn find_docker_context(&self) -> Result<std::path::PathBuf, TelegrafError> {
        let exe_dir = std::env::current_exe()
            .map_err(|e| TelegrafError::ConfigError(format!("Failed to get exe path: {}", e)))?;
        let exe_dir = exe_dir.parent().ok_or_else(|| {
            TelegrafError::ConfigError("Failed to determine exe directory".to_string())
        })?;

        let mut search_dir = exe_dir.to_path_buf();
        loop {
            let candidate = search_dir.join("docker").join("docker-compose.yml");
            if candidate.exists() {
                return Ok(search_dir.join("docker"));
            }
            if !search_dir.pop() {
                break;
            }
        }

        let cwd = std::env::current_dir().map_err(|e| {
            TelegrafError::ConfigError(format!("Failed to get current directory: {}", e))
        })?;

        let mut search_dir = cwd;
        loop {
            let candidate = search_dir.join("docker").join("docker-compose.yml");
            if candidate.exists() {
                return Ok(search_dir.join("docker"));
            }
            if !search_dir.pop() {
                break;
            }
        }

        Err(TelegrafError::ConfigError(
            "Could not find docker/ directory with docker-compose.yml. Run push-images from the project root.".to_string()
        ))
    }

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
