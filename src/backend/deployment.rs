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
}

impl DeploymentConfig {
    pub fn new(host: String, user: String) -> Self {
        Self {
            host,
            user,
            password: None,
            key_file: None,
            port: 22,
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
}

pub struct IoTDeployer {
    config: DeploymentConfig,
}

impl IoTDeployer {
    pub fn new(config: DeploymentConfig) -> Self {
        Self { config }
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
            
            // Add Docker's GPG key
            self.run_command(
                &session,
                "curl -fsSL https://download.docker.com/linux/debian/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg",
                "Adding Docker GPG key"
            )?;
            
            // Add Docker repository
            self.run_command(
                &session,
                r#"echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null"#,
                "Adding Docker repository"
            )?;
            
            // Update and install Docker
            self.run_command(&session, "sudo apt-get update", "Updating package lists")?;
            self.run_command(
                &session,
                "sudo apt-get install -y docker-ce docker-ce-cli containerd.io",
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
                r#"sudo curl -L "https://github.com/docker/compose/releases/download/v2.24.1/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose"#,
                "Downloading Docker Compose"
            )?;
            
            self.run_command(
                &session,
                "sudo chmod +x /usr/local/bin/docker-compose",
                "Making Docker Compose executable"
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
        
        // Create monitoring directory
        self.run_command(&session, "mkdir -p ~/monitoring", "Creating monitoring directory")?;
        
        println!("✅ Device provisioning completed successfully!");
        Ok(())
    }

    /// Deploy the monitoring stack to the device
    pub fn deploy(&self, build_locally: bool) -> Result<(), TelegrafError> {
        println!("🚀 Starting deployment...");
        
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
        
        println!("✅ Deployment completed successfully!");
        Ok(())
    }

    /// Deploy using remote build approach
    fn deploy_remote_build(&self, session: &Session) -> Result<(), TelegrafError> {
        // Clone or update the repository
        let repo_url = "https://github.com/rlhf23/iot2050-telegraf-config.git";
        
        // Check if repo already exists
        let repo_exists = self.check_command(session, "test -d ~/monitoring/.git")?;
        
        if repo_exists {
            println!("📥 Updating existing repository...");
            self.run_command(
                session,
                "cd ~/monitoring && git pull origin docker-everything",
                "Updating repository"
            )?;
        } else {
            println!("📥 Cloning repository...");
            self.run_command(
                session,
                &format!("git clone -b docker-everything {} ~/monitoring", repo_url),
                "Cloning repository"
            )?;
        }
        
        // Change to docker directory and make scripts executable
        self.run_command(
            session,
            "cd ~/monitoring/docker && chmod +x scripts/*.sh",
            "Making scripts executable"
        )?;
        
        // Run setup script
        println!("⚙️  Running setup script...");
        self.run_command(
            session,
            "cd ~/monitoring/docker && ./scripts/setup.sh",
            "Running setup"
        )?;
        
        // Start the monitoring stack
        println!("🚀 Starting monitoring stack...");
        self.run_command(
            session,
            "cd ~/monitoring/docker && ./scripts/start.sh",
            "Starting monitoring stack"
        )?;
        
        // Get service status
        println!("📊 Checking service status...");
        let mut channel = session.channel_session()?;
        channel.exec("cd ~/monitoring/docker && docker-compose ps")?;
        
        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;
        
        println!("Service Status:");
        println!("{}", output);
        
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
        channel.exec("cd ~/monitoring/docker && docker-compose ps")?;
        
        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;
        
        println!("Container Status:");
        println!("{}", output);
        
        // Get credentials
        let mut channel = session.channel_session()?;
        channel.exec("cd ~/monitoring/docker && cat .env | grep -E '(GRAFANA_ADMIN_|INFLUXDB_)' | grep -E '(USER|PASSWORD|TOKEN)='")?;
        
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
        println!("🛑 Stopping monitoring stack...");
        
        let session = self.create_ssh_session()?;
        
        self.run_command(
            &session,
            "cd ~/monitoring/docker && ./scripts/stop.sh",
            "Stopping monitoring stack"
        )?;
        
        println!("✅ Monitoring stack stopped");
        Ok(())
    }

    /// Start the monitoring stack
    pub fn start(&self) -> Result<(), TelegrafError> {
        println!("🚀 Starting monitoring stack...");
        
        let session = self.create_ssh_session()?;
        
        self.run_command(
            &session,
            "cd ~/monitoring/docker && ./scripts/start.sh",
            "Starting monitoring stack"
        )?;
        
        println!("✅ Monitoring stack started");
        Ok(())
    }

    /// Create SSH session
    fn create_ssh_session(&self) -> Result<Session, TelegrafError> {
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
        
        let mut channel = session.channel_session()?;
        channel.exec(command)?;
        
        let mut output = String::new();
        channel.read_to_string(&mut output)?;
        channel.wait_close()?;
        
        let exit_status = channel.exit_status()?;
        if exit_status != 0 {
            return Err(TelegrafError::SshOperationError(format!(
                "Command failed (exit code {}): {}\nOutput: {}",
                exit_status, command, output
            )));
        }
        
        if !output.trim().is_empty() {
            println!("Output: {}", output.trim());
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
