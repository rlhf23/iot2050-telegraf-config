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
    pub fn new(config: DeploymentConfig) -> Self {
        let branch = config.git_branch.clone().unwrap_or_else(|| "master".to_string());
        let repo_url = format!("https://github.com/rlhf23/iot2050-telegraf-config/archive/refs/heads/{}.tar.gz", branch);
        
        Self { 
            config,
            repo_url,
            branch,
        }
    }

    /// Test SSH connectivity to the device
    pub fn test_connection(&self) -> Result<(), TelegrafError> {
        println!("🔧 Testing SSH connectivity to {}:{}...", self.config.host, self.config.port);
        
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
            Err(TelegrafError::SshOperationError("SSH connection test failed".to_string()))
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
        
        // Update package lists
        self.run_command(&session, "sudo apt-get update", "Updating package lists")?;
        
        // Install required packages
        self.run_command(
            &session,
            "sudo apt-get install -y apt-transport-https ca-certificates curl gnupg lsb-release git openssl",
            "Installing required packages"
        )?;
        
        // Check if Docker is already installed
        let docker_installed = self.check_command(&session, "docker --version")?;
        
        if !docker_installed {
            println!("🐳 Installing Docker...");
            self.run_command(
                &session,
                "sudo apt-get install -y docker.io",
                "Installing Docker"
            )?;
        } else {
            println!("✅ Docker is already installed");
        }
        
        // Check if Docker Compose is installed
        let compose_installed = self.check_command(&session, "docker-compose --version")?;
        
        if !compose_installed {
            println!("📦 Installing Docker Compose...");
            self.run_command(
                &session,
                "sudo apt-get install -y docker-compose",
                "Installing Docker Compose"
            )?;
        } else {
            println!("✅ Docker Compose is already installed");
        }
        
        // Add user to docker group
        self.run_command(
            &session,
            &format!("sudo usermod -aG docker {}", self.config.user),
            "Adding user to docker group"
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
            "Cleaning up existing monitoring directory"
        )?;
        
        // Create monitoring directory
        self.run_command(&session, "mkdir -p ~/monitoring", "Creating monitoring directory")?;
        
        // Download and extract docker folder from the repository
        println!("📥 Downloading docker configuration from branch '{}'...", self.branch);
        
        let download_cmd = format!(
            "cd ~/monitoring && curl -L {} | tar -xz --strip-components=2 iot2050-telegraf-config-{}/docker",
            self.repo_url, self.branch
        );
        
        self.run_command(
            &session,
            &download_cmd,
            "Downloading docker configuration"
        )?;
        
        // Make scripts executable
        self.run_command(
            &session,
            "cd ~/monitoring && chmod +x scripts/*.sh config/nginx/scripts/*.sh docker/scripts/*.sh",
            "Making scripts executable"
        )?;
        
        println!("✅ Device provisioning completed successfully!");
        Ok(())
    }

    /// Deploy the monitoring stack to the device
    pub fn deploy(&self, build_locally: bool) -> Result<(), TelegrafError> {
        println!("🚀 Starting setup...");
        
        let session = self.create_ssh_session()?;
        
        if build_locally {
            println!("🏗️  Building images locally and transferring...");
            // This would involve building locally and transferring
            // For now, we'll implement the remote build approach
            self.deploy_remote_build(&session)?;
        } else {
            println!("🏗️  Building on remote device...");
            self.deploy_remote_build(&session)?;
        }
        
        println!("✅ Setup completed successfully!");
        Ok(())
    }

    /// Deploy using remote build approach (assumes provisioning is already done)
    fn deploy_remote_build(&self, session: &Session) -> Result<(), TelegrafError> {
        // Verify monitoring directory exists
        let dir_exists = self.check_command(session, "test -d ~/monitoring")?;
        if !dir_exists {
            return Err(TelegrafError::ConfigError(
                "Monitoring directory not found. Please run 'provision' first.".to_string()
            ));
        }
        
        // Run setup script
        println!("⚙️  Running setup script...");
        self.run_command(
            session,
            "cd ~/monitoring && ./scripts/setup.sh",
            "Running setup"
        )?;
        
        //TODO:skipped auto-start
        // Start the monitoring stack
        // println!("🚀 Starting monitoring stack...");
        // self.run_command(
        //     session,
        //     "cd ~/monitoring && ./scripts/start.sh",
        //     "Starting monitoring stack"
        // )?;
        
        // Get service status
        // println!("📊 Checking service status...");
        // let mut channel = session.channel_session()?;
        // channel.exec("cd ~/monitoring && docker-compose ps")?;
        
        // let mut output = String::new();
        // channel.read_to_string(&mut output)?;
        // channel.wait_close()?;
        
        // println!("Service Status:");
        // println!("{}", output);
        
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
            "cd ~/monitoring/docker && docker compose down -v"
        } else {
            "cd ~/monitoring && ./scripts/stop.sh"
        };
        
        let description = if remove_volumes {
            "Stopping monitoring stack and removing volumes"
        } else {
            "Stopping monitoring stack"
        };
        
        self.run_command(
            &session,
            stop_command,
            description
        )?;
        
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
            "Starting monitoring stack"
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
                return Err(TelegrafError::SshOperationError(format!("SSH key file not found: {}", key_file)));
            }
        } else if let Some(password) = &self.config.password {
            // Password authentication
            session.userauth_password(&self.config.user, password)?;
        } else {
            return Err(TelegrafError::SshOperationError("No authentication method provided".to_string()));
        }
        
        if !session.authenticated() {
            return Err(TelegrafError::SshOperationError("SSH authentication failed".to_string()));
        }
        
        Ok(session)
    }

    /// Run a command on the remote device
    fn run_command(&self, session: &Session, command: &str, description: &str) -> Result<(), TelegrafError> {
        println!("🔧 {}", description);
        
        // Check if this is a sudo command and we have a password
        let command = if command.contains("sudo ") && self.config.password.is_some() {
            // Use the -S flag to read password from stdin
            let password = self.config.password.as_ref().unwrap();
            format!("echo '{}' | sudo -S {}", password, command.replace("sudo ", ""))
        } else {
            command.to_string()
        };
        
        let mut channel = session.channel_session()?;
        channel.exec(&command)?;
        
        use std::io::{Read, BufReader, Write};
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
                return Err(TelegrafError::SshOperationError(format!("Command timed out: {}", command)));
            }
            // Read stdout
            match stdout_reader.read(&mut stdout_buf) {
                Ok(n) if n > 0 => {
                    let s = String::from_utf8_lossy(&stdout_buf[..n]);
                    print!("{}", s);
                    std::io::stdout().flush().ok();
                },
                _ => {}
            }
            // Read stderr
            match stderr_reader.read(&mut stderr_buf) {
                Ok(n) if n > 0 => {
                    let s = String::from_utf8_lossy(&stderr_buf[..n]);
                    eprint!("{}", s);
                    std::io::stderr().flush().ok();
                },
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
}
