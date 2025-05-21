use clap::{Arg, ArgAction, Command};
use sie_generate_config::{
    backend::{opcua_poller::OpcUaPoller, ConfigGenerator},
    error::TelegrafError,
    TelegrafConfig,
};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

fn get_default_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path
}

fn print_config(matches: &clap::ArgMatches) {
    println!("Current configuration:");
    println!("=====================");
    println!("Folder: {}", matches.get_one::<String>("folder").unwrap());
    println!("IP: {}", matches.get_one::<String>("ip").unwrap());
    println!(
        "Username: {}",
        matches.get_one::<String>("username").unwrap()
    );
    println!(
        "IOT Host: {}",
        matches.get_one::<String>("iot_host").unwrap()
    );
    println!(
        "Token Folder: {}",
        matches.get_one::<String>("token").unwrap()
    );
    println!("Send config: {}", matches.get_flag("send"));
    println!("Backup InfluxDB: {}", matches.get_flag("backup_influx"));
    println!("Backup Grafana: {}", matches.get_flag("backup_grafana"));
    println!(
        "Output format: {}",
        matches.get_one::<String>("output_format").unwrap()
    );
    println!("Include test inputs: {}", matches.get_flag("test_inputs"));
    println!("=====================\n");
}

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

fn main() {
    let matches = Command::new("IOT2050 config handler")
        .version("0.6")
        .about("Generates a config file for Telegraf from XML files in the folder")
        .arg(
            Arg::new("folder")
            .short('f')
            .long("folder")
            .value_name("FOLDER")
            .help("Sets the folder containing the XML files")
            .default_value(get_default_path().into_os_string()),
        )
        .arg(
            Arg::new("ip")
            .short('i')
            .long("ip")
            .value_name("IP")
            .help("Sets the OPC IP address")
            .default_value(env!("DEFAULT_IP")),
        )
        .arg(
            Arg::new("username")
            .short('u')
            .long("username")
            .value_name("USERNAME")
            .help("Sets the OPC username")
            .default_value(env!("DEFAULT_USERNAME")),
        )
        .arg(
            Arg::new("password")
            .short('p')
            .long("password")
            .value_name("PASSWORD")
            .help("Sets the OPC password")
            .default_value(env!("DEFAULT_PASSWORD")),
        )
        .arg(
            Arg::new("iot_username")
            .short('e')
            .long("iot-username")
            .value_name("IOT_USERNAME")
            .help("Sets the IOT-2050 user name")
            .default_value(env!("DEFAULT_IOT_USERNAME")),
        )
        .arg(
            Arg::new("iot_password")
            .short('w')
            .long("iot-password")
            .value_name("IOT_PASSWORD")
            .help("Sets the IOT-2050 password")
            .default_value(env!("DEFAULT_IOT_PASSWORD")),
        )
        .arg(
            Arg::new("iot_host")
            .short('a')
            .long("iot-host")
            .value_name("IOT_HOST")
            .help("Sets the IOT-2050 host address and port")
            .default_value(env!("DEFAULT_IOT_IP")),
        )
        .arg(
            Arg::new("token")
            .short('t')
            .long("token")
            .value_name("TOKEN_FOLDER")
            .help("Sets the location of the InfluxDB token.txt")
            .default_value(get_default_path().into_os_string()),
        )
        .arg(
            Arg::new("send")
            .short('s')
            .long("send")
            .action(ArgAction::SetTrue)
            .help("Sends the existing telegraf.conf file to the IOT-2050 and quits"),
        )
        .arg(
            Arg::new("backup_influx")
            .short('b')
            .long("backup-influx")
            .action(ArgAction::SetTrue)
            .help("Backs up the InfluxDB v2 database from the IOT-2050 and copies it to the current working directory"),
        )
        .arg(
            Arg::new("backup_grafana")
            .short('g')
            .long("backup-grafana")
            .action(ArgAction::SetTrue)
            .help("Backs up the Grafana configuration from the IOT-2050 and copies it to the current working directory"),
        )
        .arg(
            Arg::new("output_format")
            .short('o')
            .long("output-format")
            .value_name("FORMAT")
            .help("Sets the output format (influxdb or prometheus)")
            .default_value("influxdb"),
        )
        .arg(
            Arg::new("test_inputs")
            .short('x')
            .long("test-inputs")
            .action(ArgAction::SetTrue)
            .help("Include test inputs (CPU, disk, memory) in the configuration"),
        )
        .arg(
            Arg::new("get_namespaces")
            .short('n')
            .long("get-namespaces")
            .action(ArgAction::SetTrue)
            .help("Connect to OPC UA server and retrieve namespace information for XML files"),
        )
        .arg(
            Arg::new("check_influxdb")
            .long("check-influxdb")
            .value_name("INFLUXDB_URL")
            .help("Check if InfluxDB is responding at the specified URL"),
        )
        .arg(
            Arg::new("check_prometheus")
            .long("check-prometheus")
            .value_name("PROMETHEUS_URL")
            .help("Check if Prometheus is responding at the specified URL"),
        )
        .arg(
            Arg::new("service_timeout")
            .long("service-timeout")
            .value_name("SECONDS")
            .help("Timeout in seconds for service health checks")
            .default_value("5"),
        )
        .get_matches();

    // print the current config
    print_config(&matches);

    let mut config = TelegrafConfig {
        folder: matches.get_one::<String>("folder").unwrap().into(),
        ip: matches.get_one::<String>("ip").unwrap().to_string(),
        username: matches.get_one::<String>("username").unwrap().to_string(),
        password: matches.get_one::<String>("password").unwrap().to_string(),
        iot_host: matches.get_one::<String>("iot_host").unwrap().to_string(),
        iot_username: matches
            .get_one::<String>("iot_username")
            .unwrap()
            .to_string(),
        iot_password: matches
            .get_one::<String>("iot_password")
            .unwrap()
            .to_string(),
        token_folder: matches.get_one::<String>("token").unwrap().into(),
        bucket_name: String::from("line"),
        influx_token: Some("${INFLUX_TOKEN}".to_string()), // moved to telegraf env var
        listener_files: Vec::new(),
        output_format: Some(
            matches
                .get_one::<String>("output_format")
                .unwrap()
                .to_string(),
        ),
        include_test_inputs: matches.get_flag("test_inputs"),
    };

    // For operations that don't need full config setup
    if matches.get_flag("send")
        || matches.get_flag("backup_influx")
        || matches.get_flag("backup_grafana")
        || matches.get_flag("get_namespaces")
        || matches.contains_id("check_influxdb")
        || matches.contains_id("check_prometheus")
    {
        // Create a clone of config for early operations
        let early_config = config.clone();

        let generator = match ConfigGenerator::new(early_config.clone()) {
            Ok(gen) => gen,
            Err(e) => exit_with_error(format!("Configuration error: {}", e)),
        };

        if matches.get_flag("send") {
            if let Err(e) = generator.send_config() {
                eprintln!("Failed to send config: {}", e);
                wrap_up(1);
            }
            wrap_up(0);
        }

        if matches.get_flag("backup_influx") {
            if let Err(e) = generator.backup_influx() {
                eprintln!("Failed to backup InfluxDB: {}", e);
                wrap_up(1);
            }
            wrap_up(0);
        }

        if matches.get_flag("backup_grafana") {
            if let Err(e) = generator.backup_grafana() {
                eprintln!("Failed to backup Grafana: {}", e);
                wrap_up(1);
            }
            wrap_up(0);
        }

        // Handle InfluxDB status check
        if let Some(influx_url) = matches.get_one::<String>("check_influxdb") {
            println!("Checking if InfluxDB is responding at {}...", influx_url);

            // Get timeout value
            let timeout_seconds = matches
                .get_one::<String>("service_timeout")
                .unwrap_or(&"5".to_string())
                .parse::<u64>()
                .unwrap_or(5);

            // Create the generator with current config
            let generator = match ConfigGenerator::new(early_config.clone()) {
                Ok(gen) => gen,
                Err(e) => exit_with_error(format!("Configuration error: {}", e)),
            };

            // Check if InfluxDB is responding
            match generator.check_influxdb_status(influx_url, timeout_seconds) {
                Ok(true) => {
                    println!("✅ InfluxDB is responding normally");
                    wrap_up(0);
                }
                Ok(false) => {
                    println!("❌ InfluxDB is not responding");
                    wrap_up(1);
                }
                Err(e) => {
                    eprintln!("Failed to check InfluxDB status: {}", e);
                    wrap_up(1);
                }
            }
        }

        // Handle Prometheus status check
        if let Some(prometheus_url) = matches.get_one::<String>("check_prometheus") {
            println!(
                "Checking if Prometheus is responding at {}...",
                prometheus_url
            );

            // Get timeout value
            let timeout_seconds = matches
                .get_one::<String>("service_timeout")
                .unwrap_or(&"5".to_string())
                .parse::<u64>()
                .unwrap_or(5);

            // Create the generator with current config
            let generator = match ConfigGenerator::new(early_config.clone()) {
                Ok(gen) => gen,
                Err(e) => exit_with_error(format!("Configuration error: {}", e)),
            };

            // Check if Prometheus is responding
            match generator.check_prometheus_status(prometheus_url, timeout_seconds) {
                Ok(true) => {
                    println!("✅ Prometheus is responding normally");
                    wrap_up(0);
                }
                Ok(false) => {
                    println!("❌ Prometheus is not responding");
                    wrap_up(1);
                }
                Err(e) => {
                    eprintln!("Failed to check Prometheus status: {}", e);
                    wrap_up(1);
                }
            }
        }
    }

    // Get XML files
    let xml_files = match fs::read_dir(&config.folder) {
        Ok(entries) => entries
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "xml") {
                    Some(path.to_str()?.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>(),
        Err(e) => {
            eprintln!("Failed to read XML files: {}", e);
            wrap_up(1);
        }
    };

    // Check if we're generating a config with only test inputs
    let test_inputs_only = matches.get_flag("test_inputs") && xml_files.is_empty();

    if !xml_files.is_empty() {
        println!("Found the following XML files in the folder:");
        for (index, file) in xml_files.iter().enumerate() {
            println!("{}. {}", index + 1, file);
        }
    } else if !test_inputs_only {
        println!("No XML files found in the folder.");
        println!("This is clearly your fault, not mine..");
        wrap_up(1);
    } else {
        println!("No XML files found, but continuing with test inputs only.");
    }

    // Only ask for confirmation if there are XML files or we're not in test-only mode
    if !test_inputs_only {
        println!(" ");
        println!("Do you want to use these files? (y/N)");
        let mut confirm = String::new();
        std::io::stdin().read_line(&mut confirm).unwrap();

        if confirm.trim().to_lowercase() != "y" {
            println!("Aborting.");
            wrap_up(1);
        }
    }

    println!("OPC clients can be active (standard), pulling data every interval, or \npassive (subscribers), listening for changes.");
    println!("Enter the indexes of the files that should be listeners (subscribers), \nseparated by commas (e.g., 1,3). If none, just press enter:");

    let mut listener_numbers = String::new();
    std::io::stdin().read_line(&mut listener_numbers).unwrap();
    let listener_indices: Vec<usize> = listener_numbers
        .trim()
        .split(',')
        .filter_map(|num| num.trim().parse::<usize>().ok())
        .filter(|&num| num > 0 && num <= xml_files.len())
        .map(|num| num - 1)
        .collect();

    let listener_files: Vec<String> = listener_indices
        .iter()
        .map(|&index| xml_files[index].clone())
        .collect();

    // Check if we're using InfluxDB or Prometheus
    let using_influxdb = config.output_format.as_deref() != Some("prometheus");

    if using_influxdb {
        // Only need influx token and bucket name for InfluxDB output

        println!("Enter the bucket name (press Enter for default 'line'):");
        let mut bucket_name = String::new();
        std::io::stdin().read_line(&mut bucket_name).unwrap();
        config.bucket_name = if bucket_name.trim().is_empty() {
            "line".to_string()
        } else {
            bucket_name.trim().to_string()
        };
    }

    // Create generator with complete config
    let mut generator = match ConfigGenerator::new(config.clone()) {
        Ok(gen) => gen,
        Err(e) => exit_with_error(format!("Configuration error: {}", e)),
    };

    let namespace_confirm = if !matches.get_flag("get_namespaces") {
        println!("Check namespaces from server? (y/N)");
        let mut confirm = String::new();
        std::io::stdin().read_line(&mut confirm).unwrap();

        if confirm.trim().to_lowercase() != "y" {
            false
        } else {
            true
        }
    } else {
        true
    };

    // Initialize namespace map
    let namespace_map = if namespace_confirm {
        // Create an OpcUaPoller with the current configuration
        let poller = match OpcUaPoller::new(config) {
            Ok(p) => p,
            Err(e) => exit_with_error(format!("Failed to create OPC UA poller: {}", e)),
        };

        println!("Connecting to OPC UA server to retrieve namespace information...");

        // Get namespace information from OPC UA server
        match poller.get_namespace_info(&xml_files) {
            Ok(map) => {
                let found_count = map.len();
                if found_count > 0 {
                    println!("\nNamespaces found for {} XML files:\n", found_count);
                    println!("{:<40} {:<10}", "File", "Namespace");
                    println!("{}", "-".repeat(51));

                    for (file_name, namespace) in &map {
                        // Find the full path for reporting
                        if let Some(full_path) =
                            xml_files.iter().find(|path| path.ends_with(file_name))
                        {
                            let display_path = Path::new(full_path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(file_name);
                            println!("{:<40} {:<10}", display_path, namespace);
                        }
                    }
                } else {
                    println!("No matching namespaces found. Check if XML filenames match OPC UA namespace names.");
                }
                map
            }
            Err(e) => {
                eprintln!("Failed to get namespace information: {}", e);
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };

    // Get namespace and interval for each XML file
    for file in &xml_files {
        println!("\nConfiguration for file: {}", file);

        // Get namespace - try to find it in the namespace_map first
        let file_name = Path::new(file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let namespace = if let Some(ns) = namespace_map.get(file_name) {
            println!("Using automatically detected namespace: {}", ns);
            ns.clone()
        } else {
            // If not found in the map, ask user
            println!("Enter the namespace number:");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            input.trim().parse::<u16>().unwrap()
        };

        // Get interval
        let is_listener = listener_files.contains(file);
        let default_interval = if is_listener { 1000 } else { 500 };
        println!(
            "Enter the interval in milliseconds (default {}ms):",
            default_interval
        );
        let mut interval = String::new();
        std::io::stdin().read_line(&mut interval).unwrap();
        let interval_ms = if interval.trim().is_empty() {
            default_interval
        } else {
            interval.trim().parse().unwrap_or(default_interval)
        };

        // We don't prompt for custom IP in CLI mode, so pass None
        generator.set_file_config(file.clone(), namespace.to_string(), interval_ms, None);
    }

    // Generate config
    let _config_content = match generator.generate_config(&xml_files, &listener_files) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to generate config: {}", e);
            wrap_up(1);
        }
    };

    println!("Config file generated successfully!");

    println!("Do you want to send the config file to the IOT box? (y/N)");
    let mut user_input = String::new();
    std::io::stdin().read_line(&mut user_input).unwrap();

    if user_input.trim().eq_ignore_ascii_case("y") {
        if let Err(e) = generator.send_config() {
            eprintln!("Failed to send config: {}", e);
            wrap_up(1);
        }
        println!("Config sent successfully!");
    } else {
        println!("Config file generated. Please copy it and run telegraf manually.");
    }

    wrap_up(0);
}
