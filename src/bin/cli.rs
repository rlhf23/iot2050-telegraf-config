use clap::{ArgAction, Command};
use sie_generate_config::{
    backend::{
        deployment::{DeploymentConfig, IoTDeployer},
        opcua_poller::OpcUaPoller,
        ConfigGenerator, ServiceType,
    },
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

fn confirm_device_name(display_name: &str, _hostname: &str) -> bool {
    println!();
    println!("Device name: {}", display_name);
    println!("This will be used as the device identity, hostname, and InfluxDB bucket name.");
    print!("Press Enter to confirm, or 'n' to abort: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or(0);
    !input.trim().eq_ignore_ascii_case("n") && !input.trim().eq_ignore_ascii_case("no")
}

fn exit_with_error(error: impl std::fmt::Display) -> ! {
    eprintln!("Error: {}", error);
    wrap_up(1)
}

fn handle_device_command(matches: &clap::ArgMatches) {
    match matches.subcommand() {
        Some(("provision", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            if let (Some(display_name), Some(hostname)) = (&config.ship_display_name, &config.ship_hostname) {
                if !confirm_device_name(display_name, hostname) {
                    println!("Aborted.");
                    wrap_up(1);
                }
            }
            let deployer = IoTDeployer::new(config);
            let local_transfer = sub_matches.get_flag("local_transfer");

            if let Err(e) = deployer.test_connection() {
                exit_with_error(format!("Connection failed: {}", e));
            }

            if let Err(e) = deployer.provision_with_transfer_mode(local_transfer) {
                exit_with_error(format!("Provisioning failed: {}", e));
            }

            wrap_up(0);
        }
        Some(("update", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);
            let use_local = sub_matches.get_flag("local");

            if let Err(e) = deployer.test_connection() {
                exit_with_error(format!("Connection failed: {}", e));
            }

            if let Err(e) = deployer.update(use_local) {
                exit_with_error(format!("Update failed: {}", e));
            }

            wrap_up(0);
        }
        Some(("setup", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            if let (Some(display_name), Some(hostname)) = (&config.ship_display_name, &config.ship_hostname) {
                if !confirm_device_name(display_name, hostname) {
                    println!("Aborted.");
                    wrap_up(1);
                }
            }
            let minimal = sub_matches.get_flag("minimal");
            let deployer = IoTDeployer::new(config).with_minimal(minimal);

            if let Err(e) = deployer.test_connection() {
                exit_with_error(format!("Connection failed: {}", e));
            }

            if let Err(e) = deployer.setup() {
                exit_with_error(format!("Setup failed: {}", e));
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
        Some(("time", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);

            if let Err(e) = deployer.test_connection() {
                exit_with_error(format!("Connection failed: {}", e));
            }

            if let Err(e) = deployer.sync_time() {
                exit_with_error(format!("Time sync failed: {}", e));
            }

            wrap_up(0);
        }
        Some(("push-images", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let minimal = sub_matches.get_flag("minimal");
            let arch_arg = sub_matches.get_one::<String>("architecture").unwrap();
            let architecture = if arch_arg == "auto" { None } else { Some(arch_arg.clone()) };
            let deployer = IoTDeployer::new(config).with_minimal(minimal);

            if let Err(e) = deployer.push_images(architecture, minimal) {
                exit_with_error(format!("Push images failed: {}", e));
            }

            wrap_up(0);
        }
        Some(("stop", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);
            let remove_volumes = sub_matches.get_flag("volumes");

            if remove_volumes {
                println!("⚠️  WARNING: This will remove all Docker volumes and DELETE ALL DATA!");
                println!("   This includes:");
                println!("   - InfluxDB data (all metrics)");
                println!("   - Grafana dashboards and settings");
                println!("   - Prometheus data");
                println!();
                print!("Are you sure you want to continue? (yes/no): ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();

                if input.trim().to_lowercase() != "yes" {
                    println!("Aborted.");
                    wrap_up(1);
                }
            }

            if let Err(e) = deployer.stop_with_volumes(remove_volumes) {
                exit_with_error(format!("Stop failed: {}", e));
            }

            wrap_up(0);
        }
        Some(("backup", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);
            let output_dir = sub_matches.get_one::<String>("output").cloned();

            if let Err(e) = deployer.backup(output_dir) {
                exit_with_error(format!("Backup failed: {}", e));
            }

            wrap_up(0);
        }
        Some(("restore", sub_matches)) => {
            let config = create_deployment_config(sub_matches);
            let deployer = IoTDeployer::new(config);
            let archive = sub_matches.get_one::<String>("archive").unwrap().clone();
            let force = sub_matches.get_flag("force");

            if let Err(e) = deployer.restore(archive, force) {
                exit_with_error(format!("Restore failed: {}", e));
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
    let anonymous = matches.get_flag("anonymous");
    let output_format = matches.get_one::<String>("output_format").unwrap();
    let test_inputs = matches.get_flag("test_inputs");
    let opcua_diagnostics = matches.get_flag("opcua_diagnostics");
    let default_interval = matches
        .get_one::<String>("default_interval")
        .unwrap()
        .parse::<u32>()
        .unwrap_or(500);
    let listener_interval = matches
        .get_one::<String>("listener_interval")
        .unwrap()
        .parse::<u32>()
        .unwrap_or(1000);
    let send = matches.get_flag("send");

    println!("Generating Telegraf configuration with sane defaults...");
    println!("Folder: {}", folder);
    println!("OPC UA Server: {}", ip);
    println!(
        "Authentication: {}",
        if anonymous {
            "Anonymous"
        } else {
            "Username/Password"
        }
    );
    println!("Output format: {}", output_format);
    println!("Default interval: {}ms", default_interval);
    println!("Listener interval: {}ms", listener_interval);

    // Create config
    let config = TelegrafConfig {
        folder: folder.into(),
        ip: ip.clone(),
        username: if anonymous {
            String::new()
        } else {
            username.clone()
        },
        password: if anonymous {
            String::new()
        } else {
            password.clone()
        },
        iot_host: matches.get_one::<String>("iot_host").unwrap().clone(),
        iot_username: matches.get_one::<String>("iot_username").unwrap().clone(),
        iot_password: matches.get_one::<String>("iot_password").unwrap().clone(),
        listener_files: Vec::new(),
        output_format: Some(output_format.clone()),
        include_test_inputs: test_inputs,
        include_opcua_diagnostics: opcua_diagnostics,
        selected_opcua_nodes: Vec::new(),
        use_source_timestamp: false,
        ship_display_name: None,
        ship_hostname: None,
    };

    // Discover XML files
    let xml_files = ConfigGenerator::discover_xml_files(&config.folder);

    if xml_files.is_empty() && !test_inputs {
        println!(
            "No XML files found in folder '{}' and test inputs not enabled.",
            folder
        );
        println!("Use --test-inputs to generate config with test inputs only.");
        return Ok(());
    }

    if !xml_files.is_empty() {
        println!("\nFound {} XML file(s):", xml_files.len());
        for file in &xml_files {
            let file_name = Path::new(file)
                .file_name()
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

        let poller = OpcUaPoller::new(config.connection_config())?;
        match poller.get_namespace_info(&xml_files) {
            Ok(map) => {
                if !map.is_empty() {
                    println!(
                        "Successfully retrieved namespace information for {} file(s):",
                        map.len()
                    );
                    for (file_name, namespace) in &map {
                        println!("  {} -> namespace {}", file_name, namespace);
                    }
                } else {
                    println!(
                        "No matching namespaces found. Using default namespace 2 for all files."
                    );
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
        let file_name = Path::new(file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let namespace = namespace_map.get(file_name).copied().unwrap_or(2); // Default namespace

        // For simplicity, assume no files are listeners by default
        // Users can modify this behavior with additional CLI flags if needed
        let interval = default_interval;

        generator.set_file_config(file.clone(), namespace.to_string(), interval.into(), None);
    }

    // Generate config
    let _result = generator.generate_config(&xml_files, &Vec::new())?;
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
            let timeout = sub_matches
                .get_one::<String>("timeout")
                .unwrap()
                .parse::<u64>()
                .unwrap_or(5);

            println!("Checking InfluxDB connectivity at {}...", host);

            // Call SSH utility function directly to avoid ConfigGenerator validation
            // Use default credentials from environment variables
            let username = env!("DEFAULT_IOT_USERNAME");
            let password = env!("DEFAULT_IOT_PASSWORD");

            match sie_generate_config::backend::ssh_utils::check_influxdb_status(
                host, username, password, timeout,
            ) {
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
            let timeout = sub_matches
                .get_one::<String>("timeout")
                .unwrap()
                .parse::<u64>()
                .unwrap_or(5);

            println!("Checking Prometheus connectivity at {}...", host);

            // Call SSH utility function directly to avoid ConfigGenerator validation
            // Use default credentials from environment variables
            let username = env!("DEFAULT_IOT_USERNAME");
            let password = env!("DEFAULT_IOT_PASSWORD");

            match sie_generate_config::backend::ssh_utils::check_service_status(
                host,
                username,
                password,
                host,
                ServiceType::Prometheus,
                timeout,
            ) {
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
        Some(("telegraf-status", sub_matches)) => {
            let host = sub_matches.get_one::<String>("host").unwrap();
            let username = sub_matches.get_one::<String>("username").unwrap();
            let password = sub_matches.get_one::<String>("password").unwrap();

            println!("Getting Telegraf status from {}...", host);

            // Call SSH utility function directly to avoid ConfigGenerator validation
            match sie_generate_config::backend::ssh_utils::get_telegraf_status(
                host, username, password,
            ) {
                Ok(status) => {
                    println!("✅ Telegraf Status:");
                    println!("{}", status);
                }
                Err(e) => {
                    println!("❌ Failed to get Telegraf status: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(("telegraf-logs", sub_matches)) => {
            let host = sub_matches.get_one::<String>("host").unwrap();
            let username = sub_matches.get_one::<String>("username").unwrap();
            let password = sub_matches.get_one::<String>("password").unwrap();
            let lines = sub_matches
                .get_one::<String>("lines")
                .unwrap()
                .parse::<usize>()
                .unwrap_or(30);

            println!(
                "Getting last {} lines of Telegraf logs from {}...",
                lines, host
            );

            // Call SSH utility function directly to avoid ConfigGenerator validation
            match sie_generate_config::backend::ssh_utils::get_telegraf_logs(
                host, username, password, lines,
            ) {
                Ok(logs) => {
                    println!("✅ Telegraf Logs (last {} lines):", lines);
                    println!("{}", logs);
                }
                Err(e) => {
                    println!("❌ Failed to get Telegraf logs: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(("telegraf-restart", sub_matches)) => {
            let host = sub_matches.get_one::<String>("host").unwrap();
            let username = sub_matches.get_one::<String>("username").unwrap();
            let password = sub_matches.get_one::<String>("password").unwrap();

            println!("Restarting Telegraf service on {}...", host);

            // Create a minimal config for the operation
            let config = TelegrafConfig {
                folder: ".".into(),
                ip: "".to_string(),
                username: "".to_string(),
                password: "".to_string(),
                iot_host: host.clone(),
                iot_username: username.clone(),
                iot_password: password.clone(),
                listener_files: Vec::new(),
                output_format: Some("influxdb".to_string()),
                include_test_inputs: false,
                include_opcua_diagnostics: false,
                selected_opcua_nodes: Vec::new(),
                use_source_timestamp: false,
                ship_display_name: None,
                ship_hostname: None,
            };

            let _generator = ConfigGenerator::new(config)?;
            // Use the ssh_utils function directly since ConfigGenerator doesn't expose restart_telegraf
            match sie_generate_config::backend::ssh_utils::restart_telegraf_over_ssh(
                host, username, password,
            ) {
                Ok(result) => {
                    println!("✅ Telegraf Restart Result:");
                    println!("{}", result);
                }
                Err(e) => {
                    println!("❌ Failed to restart Telegraf: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(("grafana-backup", sub_matches)) => {
            let host = sub_matches.get_one::<String>("host").unwrap();
            let username = sub_matches.get_one::<String>("username").unwrap();
            let password = sub_matches.get_one::<String>("password").unwrap();
            let output_dir = sub_matches.get_one::<String>("output").unwrap();

            println!(
                "Backing up Grafana dashboards from {} to {}...",
                host, output_dir
            );

            // Create a minimal config for the operation
            let config = TelegrafConfig {
                folder: ".".into(),
                ip: "".to_string(),
                username: "".to_string(),
                password: "".to_string(),
                iot_host: host.clone(),
                iot_username: username.clone(),
                iot_password: password.clone(),
                listener_files: Vec::new(),
                output_format: Some("influxdb".to_string()),
                include_test_inputs: false,
                include_opcua_diagnostics: false,
                selected_opcua_nodes: Vec::new(),
                use_source_timestamp: false,
                ship_display_name: None,
                ship_hostname: None,
            };

            let generator = ConfigGenerator::new(config)?;
            match generator.backup_grafana() {
                Ok(result) => {
                    println!("✅ Grafana Backup Result:");
                    println!("Backup saved to: {}", output_dir);
                    println!("{}", result);
                }
                Err(e) => {
                    println!("❌ Failed to backup Grafana: {}", e);
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

    if let Ok(Some(git_branch)) = matches.try_get_one::<String>("git_branch") {
        config = config.with_git_branch(git_branch.clone());
    }

    let device_name_input = matches.try_get_one::<String>("device_name").ok().flatten();
    let (display_name, hostname_slug) = match device_name_input {
        Some(name) if name == "random" => {
            let ship = sie_generate_config::backend::ships::random_ship_name();
            (ship.display_name, ship.hostname)
        }
        Some(name) => {
            let hostname_slug = sie_generate_config::backend::ships::derive_hostname(name);
            (name.clone(), hostname_slug)
        }
        None => {
            let ship = sie_generate_config::backend::ships::random_ship_name();
            (ship.display_name, ship.hostname)
        }
    };
    config = config.with_ship_name(display_name, hostname_slug);

    config
}

fn main() {
    let matches = Command::new("IOT2050 config handler")
        .version("0.11")
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
                .arg(clap::Arg::new("anonymous").short('a').long("anonymous").action(clap::ArgAction::SetTrue).help("Use anonymous authentication (ignores username/password)"))
                .arg(clap::Arg::new("output_format").short('o').long("output-format").default_value("influxdb").help("Output format (influxdb or prometheus)"))
                .arg(clap::Arg::new("test_inputs").long("test-inputs").action(clap::ArgAction::SetTrue).help("Include test inputs (CPU, disk, memory)"))
                .arg(clap::Arg::new("opcua_diagnostics").long("opcua-diagnostics").action(clap::ArgAction::SetTrue).help("Include OPC UA server diagnostics monitoring"))
                .arg(clap::Arg::new("default_interval").long("default-interval").default_value("500").help("Default interval in milliseconds for active polling"))
                .arg(clap::Arg::new("listener_interval").long("listener-interval").default_value("1000").help("Default interval in milliseconds for listeners/subscribers"))
                .arg(clap::Arg::new("send").long("send").action(clap::ArgAction::SetTrue).help("Automatically send config to IoT device"))
                .arg(clap::Arg::new("iot_host").long("iot-host").default_value(env!("DEFAULT_IOT_IP")).help("IoT device host for sending config"))
                .arg(clap::Arg::new("iot_username").long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("IoT device username"))
                .arg(clap::Arg::new("iot_password").long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("IoT device password"))
        )
        .subcommand(
            Command::new("device")
                .about("Manage IoT device deployment and configuration")
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
                        .arg(clap::Arg::new("local_transfer").long("local-transfer").action(ArgAction::SetTrue).help("Download to local machine first, then transfer to device (offline-capable)"))
                        .arg(clap::Arg::new("device_name").long("device-name").help("Device name for identity (defaults to random Culture ship name)"))
                )
                .subcommand(
                    Command::new("update")
                        .about("Update monitoring configuration from git (removes and re-downloads)")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                        .arg(clap::Arg::new("git_branch").short('b').long("git-branch").default_value("master").help("Git branch to use"))
                        .arg(clap::Arg::new("local").short('l').long("local").action(clap::ArgAction::SetTrue).help("Download locally and transfer via SCP (no internet needed on device)"))
                )
                .subcommand(
                    Command::new("setup")
                        .about("Run setup on provisioned device (creates .env, installs telegraf config)")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                        .arg(clap::Arg::new("minimal").short('m').long("minimal").action(clap::ArgAction::SetTrue).help("Use minimal profile (InfluxDB + Telegraf + Chronograf only, no Grafana/Prometheus)"))
                        .arg(clap::Arg::new("device_name").long("device-name").help("Device name for identity (defaults to random Culture ship name)"))
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
                        .arg(clap::Arg::new("volumes").short('v').long("volumes").action(clap::ArgAction::SetTrue).help("Remove volumes (WARNING: deletes all data)"))
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
                    Command::new("backup")
                        .about("Backup all monitoring data (InfluxDB, Grafana, Prometheus)")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                        .arg(clap::Arg::new("output").short('o').long("output").help("Output directory for backup (default: ./monitoring_backup_<timestamp>)"))
                )
                .subcommand(
                    Command::new("restore")
                        .about("Restore monitoring data from backup archive")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                        .arg(clap::Arg::new("archive").short('a').long("archive").required(true).help("Path to backup archive (.tar.gz file)"))
                        .arg(clap::Arg::new("force").short('f').long("force").action(clap::ArgAction::SetTrue).help("Delete existing buckets before restore"))
                )
                .subcommand(
                    Command::new("time")
                        .about("Sync system time from your machine to the device")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                )
                .subcommand(
                    Command::new("push-images")
                        .about("Push Docker images to device for offline deployment")
                        .arg(clap::Arg::new("host").help("Device IP address").default_value(env!("DEFAULT_IOT_IP")))
                        .arg(clap::Arg::new("iot_username").short('u').long("iot-username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("iot_password").short('p').long("iot-password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("key_file").short('k').long("key-file").help("SSH private key file path"))
                        .arg(clap::Arg::new("architecture").short('a').long("architecture").default_value("auto").help("Target architecture (auto, arm64, amd64). Auto-detects from device if not specified."))
                        .arg(clap::Arg::new("minimal").short('m').long("minimal").action(clap::ArgAction::SetTrue).help("Push minimal images only (skip Grafana and Prometheus)"))
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
                .subcommand(
                    Command::new("telegraf-status")
                        .about("Get Telegraf service status")
                        .arg(clap::Arg::new("host").help("IoT device host").required(true))
                        .arg(clap::Arg::new("username").short('u').long("username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("password").short('p').long("password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                )
                .subcommand(
                    Command::new("telegraf-logs")
                        .about("Get Telegraf service logs")
                        .arg(clap::Arg::new("host").help("IoT device host").required(true))
                        .arg(clap::Arg::new("username").short('u').long("username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("password").short('p').long("password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("lines").short('n').long("lines").default_value("30").help("Number of log lines to retrieve"))
                )
                .subcommand(
                    Command::new("telegraf-restart")
                        .about("Restart Telegraf service")
                        .arg(clap::Arg::new("host").help("IoT device host").required(true))
                        .arg(clap::Arg::new("username").short('u').long("username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("password").short('p').long("password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                )
                .subcommand(
                    Command::new("grafana-backup")
                        .about("Backup Grafana dashboards")
                        .arg(clap::Arg::new("host").help("IoT device host").required(true))
                        .arg(clap::Arg::new("username").short('u').long("username").default_value(env!("DEFAULT_IOT_USERNAME")).help("SSH username"))
                        .arg(clap::Arg::new("password").short('p').long("password").default_value(env!("DEFAULT_IOT_PASSWORD")).help("SSH password"))
                        .arg(clap::Arg::new("output").short('o').long("output").default_value("./grafana_backup").help("Output directory for backup"))
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("config", sub_matches)) => {
            if let Err(e) = handle_config_command(sub_matches) {
                exit_with_error(e);
            }
        }
        Some(("device", sub_matches)) => {
            handle_device_command(sub_matches);
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
