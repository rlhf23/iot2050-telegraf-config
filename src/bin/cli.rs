use clap::{Arg, ArgAction, Command};
use sie_generate_config::{backend::ConfigGenerator, TelegrafConfig};
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

fn read_influx_token(token_folder: &str) -> String {
    let mut influx_token = String::new();
    let token_file_path = Path::new(token_folder).join("token.txt");
    if token_file_path.exists() {
        match std::fs::read_to_string(&token_file_path) {
            Ok(content) => {
                influx_token = content.trim().to_string();
                println!(
                    "InfluxDB token read from {}",
                    token_file_path.to_string_lossy()
                );
            }
            Err(e) => {
                eprintln!(
                    "Failed to read InfluxDB token from {}: {}",
                    token_file_path.to_string_lossy(),
                    e
                );
                wrap_up(1);
            }
        }
    } else {
        println!(
            "{}",
            "No 'token.txt' found, enter the InfluxDB token manually:"
        );
        match std::io::stdin().read_line(&mut influx_token) {
            Ok(_) => {
                influx_token = influx_token.trim().to_string();
            }
            Err(e) => {
                eprintln!("Failed to read InfluxDB token from stdin: {}", e);
                wrap_up(1);
            }
        }
    }
    influx_token
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
        influx_token: None,
        listener_files: Vec::new(),
    };

    // For operations that don't need full config setup
    if matches.get_flag("send")
        || matches.get_flag("backup_influx")
        || matches.get_flag("backup_grafana")
    {
        // Create a clone of config for early operations
        let mut early_config = config.clone();

        // Set influx token if needed for backup
        if matches.get_flag("backup_influx") {
            early_config.influx_token = Some(read_influx_token(
                &early_config.token_folder.to_string_lossy(),
            ));
        }

        let generator = match ConfigGenerator::new(early_config) {
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
    }

    // Get XML files
    let xml_files = match fs::read_dir(&config.folder) {
        Ok(entries) => entries
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "xml") {
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

    if !xml_files.is_empty() {
        println!("{}", "Found the following XML files in the folder:");
        for (index, file) in xml_files.iter().enumerate() {
            println!("{}. {}", index + 1, file);
        }
    } else {
        println!("{}", "No XML files found in the folder.");
        println!("{}", "This is clearly your fault, not mine..");
        wrap_up(1);
    }

    println!("");
    println!("{}", "Do you want to use these files? (y/N)");
    let mut confirm = String::new();
    std::io::stdin().read_line(&mut confirm).unwrap();

    if confirm.trim().to_lowercase() != "y" {
        println!("Aborting.");
        wrap_up(1);
    }

    println!("{}","OPC clients can be active (standard), pulling data every interval, or \npassive (subscribers), listening for changes.");
    println!("{}","Enter the indexes of the files that should be listeners (subscribers), \nseparated by commas (e.g., 1,3). If none, just press enter:");

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

    // Update config with influx token and bucket name
    config.influx_token = Some(read_influx_token(&config.token_folder.to_string_lossy()));

    println!("Enter the bucket name (press Enter for default 'line'):");
    let mut bucket_name = String::new();
    std::io::stdin().read_line(&mut bucket_name).unwrap();
    config.bucket_name = if bucket_name.trim().is_empty() {
        "line".to_string()
    } else {
        bucket_name.trim().to_string()
    };

    // Create generator with complete config
    let generator = match ConfigGenerator::new(config) {
        Ok(gen) => gen,
        Err(e) => exit_with_error(format!("Configuration error: {}", e)),
    };

    // Generate config
    let _config_content = match generator.generate_config(&xml_files, &listener_files) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to generate config: {}", e);
            wrap_up(1);
        }
    };

    println!("{}", "Config file generated successfully!");

    println!(
        "{}",
        "Do you want to send the config file to the IOT box? (y/N)"
    );
    let mut user_input = String::new();
    std::io::stdin().read_line(&mut user_input).unwrap();

    if user_input.trim().eq_ignore_ascii_case("y") {
        if let Err(e) = generator.send_config() {
            eprintln!("Failed to send config: {}", e);
            wrap_up(1);
        }
        println!("Config sent successfully!");
    } else {
        println!(
            "{}",
            "Config file generated. Please copy it and run telegraf manually."
        );
    }

    wrap_up(0);
}
