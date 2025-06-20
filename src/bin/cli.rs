use clap::{Command, ArgAction};
use sie_generate_config::{
    backend::{deployment::{DeploymentConfig, IoTDeployer}, opcua_poller::OpcUaPoller, ConfigGenerator, ServiceType},
    TelegrafConfig,
};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::Path;





fn wrap_up(exit_code: i32) -> ! {
    if cfg!(target_os = "windows") {
        println!("Press enter to exit");
        io::stdout().flush().unwrap();
        let _ = io::stdin().read(&mut [0]).unwrap();
    }
    std::process::exit(exit_code)
}

fn exit_with_error(error: impl std::fmt::Display) -> ! {
    eprintln!("Error: {}", error);
    wrap_up(1)
}

fn handle_deploy_command(matches: &clap::ArgMatches) {
    match matches.subcommand() {
        Some(("provision", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);
            
            if let Err(e) = deployer.test_connection() {
                exit_with_error(format!("Connection failed: {}", e));
            }
            
            if let Err(e) = deployer.provision() {
                exit_with_error(format!("Provisioning failed: {}", e));
            }
            
            wrap_up(0);
        }
        Some(("setup", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);
            let build_local = sub_matches.get_flag("build_local");
            
            if let Err(e) = deployer.test_connection() {
                exit_with_error(format!("Connection failed: {}", e));
            }
            
            if let Err(e) = deployer.deploy(build_local) {
                exit_with_error(format!("Deployment failed: {}", e));
            }
            
            wrap_up(0);
        }
        Some(("status", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);
            
            if let Err(e) = deployer.status() {
                exit_with_error(format!("Status check failed: {}", e));
            }
            
            wrap_up(0);
        }
        Some(("start", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);
            
            if let Err(e) = deployer.start() {
                exit_with_error(format!("Start failed: {}", e));
            }
            
            wrap_up(0);
        }
        Some(("stop", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);
            
            if let Err(e) = deployer.stop() {
                exit_with_error(format!("Stop failed: {}", e));
            }
            
            wrap_up(0);
        }
        _ => {
            eprintln!("No deployment action specified");
            wrap_up(1);
        }
    }
}

fn handle_config_command(matches: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let folder = matches.get_one::<String>("folder").unwrap();
    let ip = matches.get_one::<String>("ip").unwrap();
    let username = matches.get_one::<String>("username").unwrap();
    let password = matches.get_one::<String>("password").unwrap();
    let output_format = matches.get_one::<String>("output_format").unwrap();
    let test_inputs = matches.get_flag("test_inputs");
    let default_interval = matches.get_one::<String>("default_interval").unwrap().parse::<u32>().unwrap_or(500);
    let listener_interval = matches.get_one::<String>("listener_interval").unwrap().parse::<u32>().unwrap_or(1000);
    let send = matches.get_flag("send");

    println!("Generating Telegraf configuration with sane defaults...");
    println!("Folder: {}", folder);
    println!("OPC UA Server: {}", ip);
    println!("Output format: {}", output_format);
    println!("Default interval: {}ms", default_interval);
    println!("Listener interval: {}ms", listener_interval);

    // Create config
    let config = TelegrafConfig {
        folder: folder.into(),
        ip: ip.clone(),
        username: username.clone(),
        password: password.clone(),
        iot_host: matches.get_one::<String>("iot_host").unwrap().clone(),
        iot_username: matches.get_one::<String>("iot_username").unwrap().clone(),
        iot_password: matches.get_one::<String>("iot_password").unwrap().clone(),
        listener_files: Vec::new(),
        output_format: Some(output_format.clone()),
        include_test_inputs: test_inputs,
        include_diagnostics: false, // Default to false for CLI
        selected_opcua_nodes: Vec::new(),
    };

    // Discover XML files
    let xml_files = ConfigGenerator::discover_xml_files(&config.folder);
    
    if xml_files.is_empty() && !test_inputs {
        println!("No XML files found in folder '{}' and test inputs not enabled.", folder);
        println!("Use --test-inputs to generate config with test inputs only.");
        return Ok(());
    }

    if !xml_files.is_empty() {
        println!("\nFound {} XML file(s):", xml_files.len());
        for file in &xml_files {
            let file_name = Path::new(file).file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file);
            println!("  - {}", file_name);
        }
    }

    // Create generator
    let mut generator = ConfigGenerator::new(config.clone())?;

    // Get namespace information automatically
    let namespace_map = if !xml_files.is_empty() {
        println!("\nConnecting to OPC UA server to retrieve namespace information...");
        
        let poller = OpcUaPoller::new(config.clone())?;
        match poller.get_namespace_info(&xml_files) {
            Ok(map) => {
                if !map.is_empty() {
                    println!("Successfully retrieved namespace information for {} file(s):", map.len());
                    for (file_name, namespace) in &map {
                        println!("  {} -> namespace {}", file_name, namespace);
                    }
                } else {
                    println!("No matching namespaces found. Using default namespace 2 for all files.");
                }
                map
            }
            Err(e) => {
                println!("Warning: Failed to get namespace information: {}", e);
                println!("Using default namespace 2 for all files.");
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };

    // Configure each XML file with defaults
    for file in &xml_files {
        let file_name = Path::new(file).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        let namespace = namespace_map.get(file_name)
            .copied()
            .unwrap_or(2); // Default namespace
        
        // For simplicity, assume no files are listeners by default
        // Users can modify this behavior with additional CLI flags if needed
        let interval = default_interval;
        
        generator.set_file_config(file.clone(), namespace.to_string(), interval.into(), None);
    }

    // Generate config
    let _config_content = generator.generate_config(&xml_files, &Vec::new())?;
    println!("\n✅ Telegraf configuration generated successfully!");

    // Send config if requested
    if send {
        if config.iot_host.is_empty() {
            println!("❌ Cannot send config: --iot-host not specified");
            return Ok(());
        }
        
        println!("Sending configuration to IoT device...");
        generator.send_config()?;
        println!("✅ Configuration sent successfully!");
    } else {
        println!("Configuration saved locally. Use --send to automatically deploy to IoT device.");
    }

    Ok(())
}

fn handle_check_command(matches: &clap::ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        Some(("influxdb", sub_matches)) => {
            let host = sub_matches.get_one::<String>("host").unwrap();
            let timeout = sub_matches.get_one::<String>("timeout").unwrap().parse::<u64>().unwrap_or(5);
            
            println!("Checking InfluxDB connectivity at {}...", host);
            
            // Create a minimal config for the check
            let config = TelegrafConfig {
                folder: ".".into(),
                ip: "".to_string(),
                username: "".to_string(),
                password: "".to_string(),
                iot_host: host.clone(),
                iot_username: "".to_string(),
                iot_password: "".to_string(),
                listener_files: Vec::new(),
                output_format: Some("influxdb".to_string()),
                include_test_inputs: false,
                include_diagnostics: false, // Not needed for service check
                selected_opcua_nodes: Vec::new(),
            };
            
            let generator = ConfigGenerator::new(config)?;
            match generator.check_service_status(host, ServiceType::InfluxDB, timeout) {
                Ok((true, message)) => {
                    println!("✅ {}", message);
                }
                Ok((false, message)) => {
                    println!("❌ {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    println!("❌ Failed to check InfluxDB: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(("prometheus", sub_matches)) => {
            let host = sub_matches.get_one::<String>("host").unwrap();
            let timeout = sub_matches.get_one::<String>("timeout").unwrap().parse::<u64>().unwrap_or(5);
            
            println!("Checking Prometheus connectivity at {}...", host);
            
            // Create a minimal config for the check
            let config = TelegrafConfig {
                folder: ".".into(),
                ip: "".to_string(),
                username: "".to_string(),
                password: "".to_string(),
                iot_host: host.clone(),
                iot_username: "".to_string(),
                iot_password: "".to_string(),
                listener_files: Vec::new(),
                output_format: Some("prometheus".to_string()),
                include_test_inputs: false,
                include_diagnostics: false, // Not needed for service check
                selected_opcua_nodes: Vec::new(),
            };
            
            let generator = ConfigGenerator::new(config)?;
            match generator.check_service_status(host, ServiceType::Prometheus, timeout) {
                Ok((true, message)) => {
                    println!("✅ {}", message);
                }
                Ok((false, message)) => {
                    println!("❌ {}", message);
                    std::process::exit(1);
                }
                Err(e) => {
                    println!("❌ Failed to check Prometheus: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            println!("No check service specified");
            std::process::exit(1);
        }
    }
    
    Ok(())
}

fn create_deployment_config(matches: &clap::ArgMatches) -> DeploymentConfig {
    let host = matches.get_one::<String>("host").unwrap().clone();
    let user = matches.get_one::<String>("iot_username").unwrap().clone();
    
    // Parse host:port - use default port 22 if not specified
    let (hostname, port) = if host.contains(':') {
        let parts: Vec<&str> = host.splitn(2, ':').collect();
        let port = parts[1].parse().unwrap_or(22);
        (parts[0].to_string(), port)
    } else {
        (host, 22)
    };
    
    let mut config = DeploymentConfig::new(hostname, user).with_port(port);
    
    if let Some(password) = matches.get_one::<String>("iot_password") {
        config = config.with_password(password.clone());
    }
    
    if let Some(key_file) = matches.get_one::<String>("key_file") {
        config = config.with_key_file(key_file.clone());
    }
    
    // Add git branch if specified (for provision command)
    if let Some(git_branch) = matches.get_one::<String>("git_branch") {
        config = config.with_git_branch(git_branch.clone());
    }
    
    config
}

fn main() {
    let matches = Command::new("IOT2050 config handler")
        .version("0.8")
        .about("Generates Telegraf configs and deploys monitoring stack to IoT devices")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("config")
                .about("Generate Telegraf configuration with sane defaults")
                .arg(clap::Arg::new("folder").short('f').long("folder").default_value(".").help("Folder containing XML files"))
                .arg(clap::Arg::new("ip").short('i').long("ip").default_value(env!("DEFAULT_IP")).help("OPC UA server IP address"))
                .arg(clap::Arg::new("username").short('u').long("username").default_value(env!("DEFAULT_USERNAME")).help("OPC UA username"))
                .arg(clap::Arg::new("password").short('p').long("password").default_value(env!("DEFAULT_PASSWORD")).help("OPC UA password"))
                .arg(clap::Arg::new("output_format").short('o').long("output-format").default_value("influxdb").help("Output format (influxdb or prometheus)"))
                .arg(clap::Arg::new("test_inputs").long("test-inputs").action(clap::ArgAction::SetTrue).help("Include test inputs (CPU, disk, memory)"))
                .arg(clap::Arg::new("default_interval").long("default-interval").default_value("500").help("Default interval in milliseconds for active polling"))
                .arg(clap::Arg::new("listener_interval").long("listener-interval").default_value("1000").help("Default interval in milliseconds for listeners/subscribers"))
                .arg(clap::Arg::new("send").long("send").action(clap::ArgAction::SetTrue).help("Automatically send config to IoT device"))
                .arg(clap::Arg::new("iot_host").long("iot-host").default_value(env!("DEFAULT_IOT_IP")).help("IoT device host for sending config"))
                .arg(clap::Arg::new("iot_username").long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("IoT device username"))
                .arg(clap::Arg::new("iot_password").long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("IoT device password"))
        )
        .subcommand(
            Command::new("deploy")
                .about("Deploy monitoring stack to IoT devices")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("provision")
                        .about("Provision a new IoT device with Docker and requirements")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                        .arg(clap::Arg::new("git_branch").short('b').long("git-branch").default_value("master").help("Git branch to use for deployment"))
                )
                .subcommand(
                    Command::new("setup")
                        .about("Deploy monitoring stack to provisioned device")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                        .arg(clap::Arg::new("build_local").long("build-local").action(ArgAction::SetTrue).help("Build images locally instead of on device"))
                )
                .subcommand(
                    Command::new("status")
                        .about("Check deployment status")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                )
                .subcommand(
                    Command::new("start")
                        .about("Start monitoring stack")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                )
                .subcommand(
                    Command::new("stop")
                        .about("Stop monitoring stack")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                )
        )
        .subcommand(
            Command::new("check")
                .about("Check service connectivity")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("influxdb")
                        .about("Check if InfluxDB is responding")
                        .arg(clap::Arg::new("host").help("IoT device host").required(true))
                        .arg(clap::Arg::new("timeout").long("timeout").default_value("5").help("Timeout in seconds"))
                )
                .subcommand(
                    Command::new("prometheus")
                        .about("Check if Prometheus is responding")
                        .arg(clap::Arg::new("host").help("IoT device host").required(true))
                        .arg(clap::Arg::new("timeout").long("timeout").default_value("5").help("Timeout in seconds"))
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("config", sub_matches)) => {
            if let Err(e) = handle_config_command(sub_matches) {
                exit_with_error(e);
            }
        }
        Some(("deploy", sub_matches)) => {
            handle_deploy_command(sub_matches);
        }
        Some(("check", sub_matches)) => {
            if let Err(e) = handle_check_command(sub_matches) {
                exit_with_error(e);
            }
        }
        _ => {
            eprintln!("No command specified");
            wrap_up(1);
        }
    }

    wrap_up(0);
}
