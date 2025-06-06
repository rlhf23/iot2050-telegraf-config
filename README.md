# Telegraf Configuration Generator

## Introduction
Telegraf Configuration Generator is a simple automation tool designed to streamline the process of exporting OPCUA server interface lists from TIA Portal and quickly converting them into configuration files for Telegraf when using small simple repeatable configs. The primary goal is to facilitate data transfer into InfluxDB for later visualization in Grafana or similar platforms. This tool serves as an efficient alternative to Node-RED and similar approaches, focusing on fewer mouse clicks and greater automation when working with the SIEMENS SIMATIC IOT2050 and other IoT devices.

![Telegraf Config Generator GUI](./telegraf-config-gui.png)

## Key Features

### Graphical User Interface
- **User-friendly interface** for all configuration operations
- **Real-time validation** of configuration parameters
- **Detailed status feedback** with formatted error messages

### Configuration Management
- **XML-based templates** for easy definition of data points
- **Namespace configuration** to organize data from multiple sources
- **Custom sampling intervals** for each XML file
- **Support for listener/subscriber configurations** for event-based monitoring
- **Include test inputs** option to monitor CPU, disk, and memory of the IoT device

### Output Options
- **Dual output formats**:
  - **InfluxDB** for time-series data storage with token authentication
  - **Prometheus** for exposing metrics via HTTP endpoint

### Remote Device Operations
- **One-click deployment** of configurations to IoT devices
- **SSH-based communication** with secure credential management
- **Telegraf service management** (restart and status monitoring)
- **Remote log viewing** for troubleshooting

### Backup Capabilities
- **InfluxDB backup** for preserving time-series data
- **Grafana backup** for dashboard configurations
- Convenient storage of backups on your local machine

## Getting Started

### Installation
1. Download the latest release from the Releases page
2. Extract the archive to your preferred location
3. Run the executable file (`telegraf-config-generator.exe` on Windows)

### Basic Configuration
1. **Launch the application** to access the GUI
2. **Configure connection details**:
   - OPC IP address and credentials
   - IoT host address and credentials
3. **Select XML folder** containing your configuration templates
4. **Configure each XML file**:
   - Assign unique namespaces
   - Set appropriate sampling intervals
   - Select listener/subscriber status as needed
5. **Choose output format** (InfluxDB or Prometheus)
6. **Generate configuration** with a single click
7. **Deploy to IoT device** directly from the interface

### Monitoring & Maintenance
Use the "Other Commands" section to:
- **Check Telegraf status** on the IoT device
- **View Telegraf logs** for troubleshooting
- **Backup InfluxDB** or Grafana as needed

## Command Line Interface

While the GUI provides the most user-friendly experience, a command-line interface is also available for automation and scripting:

### Main Commands

```bash
# Generate configuration
./sie_generate_config -f <path_to_folder>

# Send configuration to IoT device
./sie_generate_config -s -f <path_to_folder> -a <iot_host> -w <iot_password>

# Backup InfluxDB
./sie_generate_config -b -a <iot_host> -w <iot_password>


# Start the GUI
./sie_generate_config_gui
```

### Test Tools

```bash
# Start OPC UA test server
./opcua_test_server

# Test OPC UA client connection
./opcua_client_test
```

For more CLI options, run:
```bash
./sie_generate_config --help
```

## Building from Source
1. Install Rust: https://www.rust-lang.org/tools/install
2. Clone this repository
3. Run `cargo build --release`
4. The executable will be available in `target/release/`

## Development & Testing

### NixOS Container Testing Environment
This project includes a NixOS container setup for testing SSH operations and Telegraf configurations in an isolated environment.

#### Prerequisites
- NixOS system
- nixos-container support

#### Container Management

```bash
# Start the test container
./run_test_container.sh start

# Stop and remove the container
./run_test_container.sh stop

# Restart the container
./run_test_container.sh restart

# Get container IP address
./run_test_container.sh ip

# SSH into the container as testuser
./run_test_container.sh ssh

# Check container status
./run_test_container.sh status

# Open root shell in container (for debugging)
./run_test_container.sh shell
```

#### Container Details
- **Container name**: `iot2050-test`
- **Test user**: `testuser` (password: `testpass`)
- **Root user**: `root` (password: `root`)
- **Services**: SSH, Telegraf, InfluxDB2
- **Configuration**: `container.nix`

The container provides a clean, reproducible environment equivalent to the original Docker setup, with all necessary services pre-configured for testing SSH-based deployment and monitoring operations.

## Test Coverage
This project includes comprehensive test coverage capabilities using `cargo-tarpaulin`.

### Running Test Coverage Locally
1. Install `cargo-tarpaulin`:
   ```bash
   cargo install cargo-tarpaulin
   ```

2. Run the coverage script:
   ```bash
   ./run_coverage.sh
   ```

3. View the HTML report in your browser:
   ```bash
   open coverage/tarpaulin-report.html
   ```

### Coverage Configuration
- `.tarpaulin.toml` - Configuration file for tarpaulin with custom settings
- `.codecov.yml` - Configuration for Codecov integration
- GitHub Actions automatically run coverage on each push and PR
- Reports are uploaded to Codecov and available as GitHub Actions artifacts

### Current Coverage
- Minimum required coverage: 60%
- Reports include line, branch, and function coverage
- Detailed per-file metrics are available in the HTML and JSON reports

## Support
If you encounter issues or have questions, please submit an issue on our GitHub page.

## License
This project is licensed under the terms of the included LICENSE file.

Thank you for using Telegraf Configuration Generator!

