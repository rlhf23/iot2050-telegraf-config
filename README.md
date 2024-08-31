# Config Generator for Telegraf

## Introduction
This tool is designed to simplify the process of generating and deploying Telegraf configuration files, especially for users managing IoT devices. It allows for the automatic generation of configuration files based on XML templates and provides functionalities for sending these configurations to remote IoT devices via SSH.

## Features
- **Generate Configurations:** Automatically generate Telegraf configuration files from XML templates.
- **Send Configurations:** Directly send the generated configuration files to IoT devices using SSH.
- **Backup InfluxDB:** Facilitate the backup of InfluxDB databases from remote IoT devices.

## Basic Usage
By default it will use files in the current working directory. Normally you can just run the .exe and follow the prompts to create a new config and send it to the IOT box, if all passwords and IP addresses are the defaults.

Here are some basic commands for other use cases:

### Generating a Config File
To generate a Telegraf configuration file from XML templates in a specified folder:
```
./config_generator -f <path_to_folder>
```
### Sending Configuration to an IoT Device
To send a generated `telegraf.conf` file to an IoT device and restart Telegraf:
```
./config_generator -s -f <path_to_folder> -a <iot_host> -w <iot_password>
```

### Backing Up InfluxDB
To backup an InfluxDB database from an IoT device:
```
./config_generator -b -a <iot_host> -w <iot_password>
```

## Advanced Usage
For more advanced usage and options, run the help command:
```
./config_generator --help
```

This will display all the available commands and their descriptions, helping you to make full use of the program's capabilities.

## Building from source
Before you can build this tool, ensure you have Rust installed on your system. Follow these steps to install Rust: https://www.rust-lang.org/tools/install

Once Rust is installed, you can compile the program by navigating to the program's directory and running:
```
cargo build --release
```
This command compiles the program in release mode, optimizing for performance. The compiled binary will be located in `target/release/`.

## Support
If you encounter any issues or have questions, please refer to the project's documentation or submit an issue on the project's GitHub page.

Thank you for using Config Generator for Telegraf!

