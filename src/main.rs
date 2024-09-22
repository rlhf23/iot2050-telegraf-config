use clap::{Arg, ArgAction, Command};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::{env, path::Path, path::PathBuf};

mod format;
mod ssh_utils;

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

fn get_default_path() -> PathBuf {
    // Returns the default path by getting the current executable's directory
    let mut path = env::current_exe().unwrap();
    path.pop();
    path
}

fn wrap_up(exit_code: i32) {
    if cfg!(target_os = "windows") {
        println!("Press enter to exit");
        io::stdout().flush().unwrap();
        let _ = io::stdin().read(&mut [0]).unwrap();
    }
    std::process::exit(exit_code);
}

fn main() {
    // Main function: Parses command-line arguments and either sends a config file or generates one based on XML files
    let matches = Command::new("IOT2050 config handler")
        .version("0.4")
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

    let folder = matches.get_one::<String>("folder").unwrap();
    let ip = matches.get_one::<String>("ip").unwrap();
    let username = matches.get_one::<String>("username").unwrap();
    let password = matches.get_one::<String>("password").unwrap();
    let iot_password = matches.get_one::<String>("iot_password").unwrap();
    let iot_host = matches.get_one::<String>("iot_host").unwrap();
    let token_folder = matches.get_one::<String>("token").unwrap();

    // Check if IP address is valid IPv4 format
    let ip_valid = ip
        .split('.')
        .filter(|part| part.parse::<u8>().is_ok())
        .count()
        == 4;

    if !ip_valid {
        eprintln!(
            "Error: Invalid IP address format for '{}', expecting something like: 192.168.0.1",
            ip
        );
        wrap_up(1);
    }

    // Check if IOT host IP address is valid
    let iot_host_valid = {
        let iot_host_parts: Vec<&str> = iot_host.split(':').collect();
        iot_host_parts.len() == 2
            && iot_host_parts[1]
                .parse::<u16>()
                .map_or(false, |port| port > 0)
    };

    if !iot_host_valid {
        eprintln!(
            "Error: Invalid IOT host format for '{}', expecting something like: 192.168.0.1:22",
            iot_host
        );
        wrap_up(1);
    }

    let remote_path = "/etc/telegraf/telegraf.conf";
    let iot_username = "root";

    // If the --send flag is set, attempt to only send the telegraf.conf file over SSH and restart Telegraf
    if matches.get_flag("send") {
        let config_path = Path::new(folder).join("telegraf.conf");
        if !config_path.exists() {
            eprintln!("Error: telegraf.conf file does not exist in the specified folder.");
            wrap_up(1);
        }
        if let Err(e) = ssh_utils::send_and_restart_telegraf(
            &config_path,
            remote_path,
            iot_host,
            iot_username,
            iot_password,
        ) {
            eprintln!(
                "Failed to send telegraf.conf file and restart Telegraf: {}",
                e
            );
            wrap_up(1);
        }
        wrap_up(0);
    }

    // Check if the backup flag is set and perform backup if true
    if matches.get_flag("backup_influx") {
        if let Err(e) = ssh_utils::backup_influxdb(iot_host, iot_username, iot_password) {
            eprintln!("Failed to backup InfluxDB: {}", e);
        }
        wrap_up(0);
    }

    //check if the -g flag is set and perform backup if true
    if matches.get_flag("backup_grafana") {
        let iot_host = matches.get_one::<String>("iot_host").unwrap();
        let iot_password = matches.get_one::<String>("iot_password").unwrap();
        match ssh_utils::backup_grafana_config(iot_host, "root", iot_password) {
            Ok(_) => println!("Grafana configuration backup completed successfully."),
            Err(e) => eprintln!("Failed to backup Grafana configuration: {}", e),
        }
        wrap_up(0);
    }

    let xml_files: Vec<String> = fs::read_dir(folder)
        // Collect all XML files from the specified folder for processing
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "xml") {
                Some(path.to_str().unwrap().to_string())
            } else {
                None
            }
        })
        .collect();

    if !xml_files.is_empty() {
        // Notify the user about the found XML files and ask for confirmation to proceed
        println!("{}", "Found the following XML files in the folder:");
        for (index, file) in xml_files.iter().enumerate() {
            println!("{}. {}", index + 1, file);
        }
    } else {
        println!("{}", "No XML files found in the folder.");
        println!("{}", "This is clearly your fault, not mine..");

        if cfg!(target_os = "windows") {
            println!("Press enter to exit");
            io::stdout().flush().unwrap();
            let _ = io::stdin().read(&mut [0]).unwrap();
        }

        println!("{}", "Aborting.");
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
    //println!("");
    println!("{}","Enter the indexes of the files that should be listeners (subscribers), \nseparated by commas (e.g., 1,3). If none, just press enter:");
    let mut listener_numbers = String::new();
    std::io::stdin().read_line(&mut listener_numbers).unwrap();
    let listener_indices: Vec<usize> = listener_numbers
        .trim()
        .split(',')
        .filter_map(|num| num.trim().parse::<usize>().ok())
        .filter(|&num| num > 0 && num <= xml_files.len())
        .map(|num| num - 1) // Convert to 0-based index
        .collect();
    let listener_files: Vec<String> = listener_indices
        .iter()
        .map(|&index| xml_files[index].clone())
        .collect();

    let mut influx_token = String::new();
    // Attempt to read the InfluxDB token from a file, or ask the user to input it
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

    let mut config_strings = Vec::new();
    // Generate configuration strings for each XML file, checking whether it's a listener
    for file in &xml_files {
        let is_listener = listener_files.contains(file);
        let config_string = format::parse_xml(file, ip, username, password, is_listener);
        config_strings.push(config_string);
    }

    // Combine all configuration strings into the final config file content
    let config_content = format::generate_config_content(&influx_token, &config_strings);

    // Write the config file to the folder
    let config_path = Path::new(folder).join("telegraf.conf");
    let mut config_file = File::create(&config_path).unwrap();
    config_file.write_all(config_content.as_bytes()).unwrap();

    println!("{}", "Config file generated successfully!");

    // Ask the user if they want to automatically send the generated config file to the IOT box
    println!(
        "{}",
        "Do you want to send the config file to the IOT box? (y/N)"
    );

    let mut user_input = String::new();
    std::io::stdin().read_line(&mut user_input).unwrap();
    if user_input.trim().eq_ignore_ascii_case("y") {
        let config_path = Path::new(folder).join("telegraf.conf");
        if !config_path.exists() {
            eprintln!("Error: telegraf.conf file does not exist in the specified folder.");
            wrap_up(1);
        }
        if let Err(e) = ssh_utils::send_and_restart_telegraf(
            &config_path,
            remote_path,
            iot_host,
            iot_username,
            iot_password,
        ) {
            eprintln!(
                "Failed to send telegraf.conf file and restart Telegraf: {}",
                e
            );
        }
        wrap_up(1);
    } else {
        println!(
            "{}",
            "Config file generated. Please copy it and run telegraf manually."
        );
        wrap_up(0);
    }
}
