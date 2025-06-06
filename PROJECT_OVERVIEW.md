# Telegraf Configuration Generator - Project Overview

This document provides a technical overview of the Telegraf Configuration Generator project architecture and organization, intended for developers and LLMs to quickly understand the codebase.

## Project Structure

```
iot2050-telegraf-config/
├── src/                      # Source code
│   ├── backend/              # Core functionality modules
│   │   ├── mod.rs            # Main backend module definition
│   │   ├── format.rs         # Configuration format handling
│   │   ├── opcua_poller.rs   # OPC UA server connection/polling
│   │   ├── ssh_utils.rs      # SSH communication with IoT devices
│   │   └── *_test.rs         # Unit tests for backend modules
│   ├── bin/                  # Executable entry points and test tools
│   │   ├── cli.rs            # Main CLI interface (sie_generate_config)
│   │   ├── gui.rs            # Graphical user interface (sie_generate_config_gui)
│   │   ├── opcua_client_test.rs  # OPC UA client test tool (opcua_client_test)
│   │   └── opcua_test_server.rs  # OPC UA test server (opcua_test_server)
│   ├── lib.rs                # Core library functionality and TelegrafConfig
│   ├── error.rs              # Error types and handling
│   ├── worker.rs             # Background task processing
│   └── *_test.rs             # Library unit tests
├── pki/                      # PKI certificates for testing
├── pki-server/               # Server certificates for testing
├── .github/                  # GitHub Actions workflows and templates
├── tests/                    # Integration tests and test data
│   └── *.xml                 # Test XML files
├── build.rs                  # Build script for environment variables
├── Cargo.toml                # Project dependencies and configuration
├── Cargo.lock                # Dependency lock file
├── flake.nix                 # Nix flake configuration
├── flake.lock                # Nix dependency lock
├── .env.example              # Example environment variables
├── .codecov.yml              # Code coverage configuration
├── .tarpaulin.toml           # Test coverage settings
├── run_coverage.sh           # Test coverage script (bash)
├── run_coverage.fish         # Test coverage script (fish)
└── README.md                 # User documentation
```

## Code Architecture

### Core Components

1. **TelegrafConfig (lib.rs)**
   - Central configuration struct holding application settings
   - Validation methods for configuration parameters
   - XML file discovery functionality

2. **ConfigGenerator (backend/mod.rs)**
   - Orchestrates the configuration generation process
   - Manages file configurations and output format settings
   - Handles SSH interactions with remote IoT devices

3. **Format Module (backend/format.rs)**
   - Parses XML configuration files
   - Generates Telegraf configuration in required formats
   - Handles template substitution and formatting

4. **OpcUaPoller (backend/opcua_poller.rs)**
   - Connects to OPC UA servers
   - Polls servers for available nodes and data
   - Validates connectivity and configuration

5. **SSH Utils (backend/ssh_utils.rs)**
   - Manages secure connections to IoT devices
   - Transfers configuration files
   - Handles remote command execution and service management

6. **Worker Module (worker.rs)**
   - Background task execution framework
   - Asynchronous command processing
   - Thread-safe communication channels
   - Task queuing and state management

7. **Error Handling (error.rs)**
   - Comprehensive error type hierarchy
   - User-friendly error message formatting
   - Error categorization and context-aware messaging

### User Interfaces

1. **Command Line Interface (bin/cli.rs)**
   - Executable: `sie_generate_config`
   - Argument parsing and validation
   - Command execution flow
   - Service checks and configuration deployment

2. **Graphical User Interface (bin/gui.rs)**
   - Executable: `sie_generate_config_gui`
   - Event-driven UI with eframe
   - Form validation and status feedback
   - Visual configuration management

## Key Data Flows

### Configuration Generation Flow

1. User provides XML files with OPC UA node definitions
2. Application validates XML content and configuration parameters
3. XML files are parsed to extract node information
4. Telegraf configuration is generated according to selected output format
5. Configuration file is written to disk locally
6. Optionally, configuration is deployed to IoT device via SSH

### Remote Interaction Flow

1. SSH connection established with IoT device using provided credentials
2. Commands executed to validate connection and services
3. Configuration files transferred to appropriate locations
4. Telegraf service restarted to apply new configuration
5. Status feedback returned to user

## Environment Variables

The following environment variables are used during build and runtime:

- `DEFAULT_IP`: Default OPC UA server IP (192.168.1.1)
- `DEFAULT_USERNAME`: Default OPC UA server username (user)
- `DEFAULT_PASSWORD`: Default OPC UA server password (pass)
- `DEFAULT_IOT_USERNAME`: Default IoT device username (iotuser)
- `DEFAULT_IOT_PASSWORD`: Default IoT device password (iotpass)
- `DEFAULT_IOT_IP`: Default IoT device address and port (192.168.1.2:22)

These can be customized using a `.env` file and are integrated via the `build.rs` script.

## Common Modification Patterns

### Adding a New Configuration Option

1. Add the field to `TelegrafConfig` struct in `lib.rs`
2. Add validation logic if necessary
3. Update CLI argument handling in `cli.rs`
4. Add UI elements in `gui.rs` if applicable
5. Modify `ConfigGenerator` to utilize the new option

### Supporting a New Output Format

1. Add a new variant to `OutputFormat` enum in `backend/format.rs`
2. Implement formatting logic in `format_config_header` function
3. Update CLI and GUI to support the new format option
4. Add validation and specific output handling as needed

### Adding New Error Types

1. Add a new variant to `TelegrafError` enum in `error.rs`
2. Implement user-friendly error message in `user_friendly_message` method
3. Add appropriate `From` implementations for error conversion

### XML Template Modifications

1. XML files should follow the established structure with namespace definitions
2. Each node requires a unique identifier within its namespace
3. File validation ensures no duplicate namespaces across files for the same IP

## Testing Approach

1. **Unit Tests**: Co-located with source files using `_test.rs` suffix
   - Backend modules have corresponding test files in `src/backend/`
   - Library tests are in `lib_test.rs`
   - Error handling tests are in `error_test.rs`

2. **Test Tools**:
   - `opcua_test_server.rs`: Standalone OPC UA test server
   - `opcua_client_test.rs`: Tool for testing OPC UA client functionality

3. **Test Coverage**:
   - Uses `cargo-tarpaulin` for code coverage
   - Coverage configuration in `.tarpaulin.toml`
   - Run coverage with `run_coverage.sh` or `run_coverage.fish`
   - Integrated with Codecov (`.codecov.yml`)

4. **Test Data**:
   - XML sample files in `tests/` directory
   - PKI certificates in `pki/` and `pki-server/` for secure testing

## Build and Deployment

The project supports multiple build and deployment methods:

### Cargo Build System
- Development: `cargo build`
- Release: `cargo build --release` (with optimizations)
- Test: `cargo test`
- Windows builds include OpenSSL vendoring

### Nix Support
- Development shell: `nix develop`
- Build with Nix: `nix build`
- Run tests: `nix flake check`

### CI/CD
- GitHub Actions workflows in `.github/workflows/`
- Automated testing on push/pull requests
- Code coverage reporting to Codecov

## Dependencies

### Main Dependencies
- `clap`: Command-line argument parsing
- `eframe`: GUI framework
- `roxmltree`: XML parsing
- `ssh2`: SSH client functionality
- `opcua`: OPC UA client implementation
- `tokio`: Async runtime
- `thiserror`: Error handling utilities
- `dotenv`: Environment variable management
- `serde`: Serialization/deserialization
- `anyhow`: Flexible error handling
- `log`/`env_logger`: Logging infrastructure

### Development Dependencies
- `assert_cmd`: Command assertion for testing
- `predicates`: Test assertions
- `tempfile`: Temporary file handling in tests
- `mockall`: Mocking for unit tests
- `cargo-tarpaulin`: Code coverage
- `nix`: Nix package manager integration
