# IoT2050 Monitoring Stack - Project Overview

This document provides a technical overview of the IoT2050 Monitoring Stack project architecture and organization, intended for developers and LLMs to quickly understand the codebase.

## Project Structure

```
iot2050-telegraf-config/
├── src/                      # Source code
│   ├── backend/              # Core functionality modules
│   │   ├── mod.rs            # Main backend module definition
│   │   ├── format.rs         # Configuration format handling
│   │   ├── opcua_poller.rs   # OPC UA server connection/polling
│   │   ├── deployment.rs     # Docker deployment management
│   │   ├── ssh_utils.rs      # SSH communication with IoT devices
│   │   └── *_test.rs         # Unit tests for backend modules
│   ├── bin/                  # Executable entry points and test tools
│   │   ├── cli.rs            # Main CLI interface with deployment commands
│   │   ├── gui.rs            # GUI with OPC UA browser and config generation
│   │   ├── tui.rs            # Terminal UI with arrow key navigation
│   │   ├── opcua_client_test.rs  # OPC UA client test tool
│   │   └── opcua_test_server.rs  # OPC UA test server
│   ├── lib.rs                # Core library functionality and TelegrafConfig
│   ├── error.rs              # Error types and handling
│   ├── worker.rs             # Background task processing
│   └── *_test.rs             # Library unit tests
├── docker/                   # Docker monitoring stack
│   ├── docker-compose.yml    # Multi-service monitoring stack definition
│   ├── docker-compose.ci.yml # CI-specific Docker Compose configuration
│   ├── api-service/          # HTTP API for container management
│   │   ├── src/              # API service source code
│   │   ├── Dockerfile        # Standard Docker build
│   │   ├── Dockerfile.build  # Cross-compilation build
│   │   ├── Dockerfile.prebuilt  # Prebuilt binary deployment
│   │   ├── Cargo.toml        # API service dependencies
│   │   ├── Makefile          # Build automation
│   │   └── README.md         # API service documentation
│   ├── config/               # Service configurations
│   │   ├── telegraf/         # Telegraf configuration templates
│   │   ├── grafana/          # Grafana dashboards and datasources
│   │   ├── prometheus/       # Prometheus configuration
│   │   └── nginx/            # Nginx reverse proxy and dashboard
│   │       ├── nginx.conf    # Nginx configuration
│   │       ├── html/         # Dashboard landing page
│   │       └── scripts/      # System info and startup scripts
│   ├── scripts/              # Deployment and management scripts
│   │   ├── init.sh           # Device provisioning script (local)
│   │   ├── init_remote.sh    # Device provisioning script (remote)
│   │   ├── deploy.sh         # Stack deployment script (local)
│   │   ├── deploy_remote.sh  # Stack deployment script (remote)
│   │   ├── setup.sh          # Environment setup script
│   │   ├── start.sh          # Start monitoring stack
│   │   ├── stop.sh           # Stop monitoring stack
│   │   ├── test.sh           # Monitoring stack test runner
│   │   ├── test-integration.sh  # Integration tests
│   │   └── test-scripts.sh   # Script validation tests
│   ├── README.md             # Docker stack documentation
│   └── TESTING.md            # Testing documentation
├── docs/                     # Documentation
│   ├── CONTAINER_SETUP.md    # Container deployment guide
│   ├── CLI_REFACTOR_SUMMARY.md  # CLI refactoring notes
│   └── WEBUI_CONFIG_GENERATOR.md  # Web UI configuration guide
├── .github/                  # GitHub Actions CI/CD
│   ├── workflows/            # GitHub Actions workflows
│   │   ├── linux-ci.yml      # Linux CI pipeline
│   │   ├── windows-build.yml # Windows build pipeline
│   │   ├── coverage.yml      # Code coverage reporting
│   │   ├── monitoring-stack-test.yml  # Docker stack testing
│   │   ├── api-service-build.yml  # API service build
│   │   ├── binary-release.yml  # Binary release automation
│   │   └── manual-build.yml  # Manual build trigger
│   └── pull_request_template.md  # PR template
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
├── run_coverage.sh           # Test coverage script
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
   - Integrates with Docker deployment workflow

3. **Format Module (backend/format.rs)**
   - Parses XML configuration files
   - Generates Telegraf configuration in required formats
   - Handles template substitution and formatting

4. **OpcUaPoller (backend/opcua_poller.rs)**
   - Connects to OPC UA servers
   - Provides node browsing and selection capabilities
   - Hierarchical node tree navigation with lazy loading
   - Tag selection and validation

5. **Deployment Module (backend/deployment.rs)**
   - Manages Docker-based monitoring stack deployment
   - Device provisioning with Docker and Docker Compose
   - SSH-based deployment automation
   - Service lifecycle management

6. **SSH Utils (backend/ssh_utils.rs)**
   - Manages secure connections to IoT devices
   - Executes remote commands for deployment
   - Handles file transfers and service management

7. **Worker Module (worker.rs)**
   - Background task execution framework
   - Asynchronous command processing
   - Thread-safe communication channels
   - Task queuing and state management

8. **Error Handling (error.rs)**
   - Comprehensive error type hierarchy
   - User-friendly error message formatting
   - Error categorization and context-aware messaging

### User Interfaces

1. **Command Line Interface (bin/cli.rs)**
   - Configuration generation commands
   - Docker deployment management
   - Device provisioning and setup commands
   - Stack lifecycle operations (start/stop/status)

2. **Graphical User Interface (bin/gui.rs)**
   - Interactive OPC UA browser with node selection
   - Real-time node tree exploration
   - Visual configuration management
   - Integration with Docker deployment workflow

3. **Terminal User Interface (bin/tui.rs)**
   - Cross-platform terminal-based interface for headless environments
   - Arrow key navigation with visual selection highlighting
   - Four-tab interface: Files, OPC-UA Config, IoT Config, Actions
   - Per-file configuration editing (namespace, IP, interval)
   - OPC-UA namespace polling with timeout and error feedback
   - Anonymous authentication support
   - Backward compatible with legacy hotkeys

### Docker Monitoring Stack

1. **Multi-Service Architecture**
   - **InfluxDB**: Time-series database for metrics storage
   - **Telegraf**: Metrics collection agent with OPC UA support
   - **Grafana**: Visualization and dashboarding
   - **Prometheus**: Alternative metrics collection and storage
   - **API Service**: HTTP API for container management (restart, start, stop)
   - **Nginx**: Reverse proxy and dashboard landing page with system info

2. **API Service (docker/api-service/)**
   - Lightweight Rust-based HTTP API (~15-20MB container)
   - Container management endpoints (list, restart, start, stop)
   - Whitelist-based security for allowed containers
   - CORS-enabled for web dashboard integration
   - Health check endpoint for monitoring
   - Multiple build options: standard, cross-compilation, prebuilt binary

3. **Nginx Dashboard**
   - Reverse proxy for all services (Grafana, Prometheus, InfluxDB, API)
   - Landing page with system information and service links
   - Dynamic system info generation (CPU, memory, uptime)
   - Unified access point on port 80

4. **Deployment Scripts**
   - **init.sh/init_remote.sh**: Provisions IoT devices with Docker requirements
   - **deploy.sh/deploy_remote.sh**: Builds and deploys the complete stack
   - **setup.sh**: Configures environment and credentials
   - **start.sh/stop.sh**: Service lifecycle management
   - **test.sh**: Comprehensive stack testing with colored output
   - **test-integration.sh**: Integration testing suite
   - **test-scripts.sh**: Script validation

5. **Configuration Management**
   - Environment-based configuration with .env files
   - Provisioned dashboards and datasources
   - Persistent storage with Docker volumes
   - Multi-architecture support (x86_64/ARM64)
   - Nginx reverse proxy configuration with sub-paths

## Key Data Flows

### Configuration Generation Flow (Legacy)

1. User provides XML files with OPC UA node definitions or uses OPC UA browser
2. Application validates XML content and configuration parameters
3. XML files are parsed to extract node information
4. Telegraf configuration is generated according to selected output format
5. Configuration file is written to disk locally

### Docker Stack Deployment Flow (Primary)

1. **Device Provisioning**:
   - `init.sh` provisions target device with Docker and dependencies
   - System requirements validation and installation
   - User permissions and service setup

2. **Stack Deployment**:
   - `deploy.sh` builds Docker images locally (multi-architecture)
   - Images and configuration transferred to target device
   - Stack deployed using Docker Compose
   - Services started and health-checked

3. **Configuration Integration**:
   - Generated Telegraf configurations integrated into Docker stack
   - Environment variables and secrets managed securely
   - Persistent storage configured for data retention

### OPC UA Browser Flow

1. User connects to OPC UA server using GUI
2. Node tree is loaded hierarchically with lazy loading
3. User browses and selects relevant nodes/tags
4. Selected nodes are validated and configured
5. Configuration is integrated into monitoring stack

## Environment Variables

### Build-time Variables (build.rs)

- `DEFAULT_IP`: Default OPC UA server IP (192.168.1.1)
- `DEFAULT_USERNAME`: Default OPC UA server username (user)
- `DEFAULT_PASSWORD`: Default OPC UA server password (pass)
- `DEFAULT_IOT_USERNAME`: Default IoT device username (iotuser)
- `DEFAULT_IOT_PASSWORD`: Default IoT device password (iotpass)
- `DEFAULT_IOT_IP`: Default IoT device address and port (192.168.1.2:22)

### Docker Stack Runtime Variables (.env)

- `INFLUXDB_USER`: InfluxDB admin username
- `INFLUXDB_PASSWORD`: InfluxDB admin password
- `INFLUXDB_ORG`: InfluxDB organization name
- `INFLUXDB_BUCKET`: InfluxDB default bucket
- `INFLUXDB_TOKEN`: InfluxDB admin token
- `TELEGRAF_TOKEN`: Telegraf write token
- `GRAFANA_ADMIN_USER`: Grafana admin username
- `GRAFANA_ADMIN_PASSWORD`: Grafana admin password
- `HOST_DOCKER_GID`: Host Docker group ID for container permissions

Build-time variables can be customized using a `.env` file and are integrated via the `build.rs` script. Docker stack variables are managed by the deployment scripts and stored in `docker/.env`.

## Common Modification Patterns

### Adding a New Configuration Option

1. Add the field to `TelegrafConfig` struct in `lib.rs`
2. Add validation logic if necessary
3. Update CLI argument handling in `cli.rs`
4. Add UI elements in `gui.rs` if applicable
5. Modify `ConfigGenerator` to utilize the new option
6. Update Docker configuration templates if needed

### Adding a New Docker Service

1. Add service definition to `docker/docker-compose.yml`
2. Create configuration directory under `docker/config/`
3. Update deployment scripts to handle new service
4. Add service-specific environment variables to `.env` template
5. Update health checks and startup dependencies

### Supporting a New Output Format

1. Add a new variant to `OutputFormat` enum in `backend/format.rs`
2. Implement formatting logic in `format_config_header` function
3. Update CLI and GUI to support the new format option
4. Add Docker service configuration if required
5. Update deployment templates and scripts

### Adding New Deployment Features

1. Extend `DeploymentConfig` struct in `backend/deployment.rs`
2. Add corresponding CLI commands in `bin/cli.rs`
3. Update deployment scripts in `docker/scripts/`
4. Add error handling and validation
5. Update documentation and examples

### Extending the API Service

1. Add new endpoints in `docker/api-service/src/main.rs`
2. Update container whitelist if needed
3. Add corresponding API documentation in README
4. Update health checks and error handling
5. Rebuild and test with `make` in `docker/api-service/`
6. Update Nginx configuration if new routes are needed

### Modifying Nginx Dashboard

1. Update `docker/config/nginx/nginx.conf` for routing changes
2. Modify HTML/CSS in `docker/config/nginx/html/` for UI changes
3. Update system info script in `docker/config/nginx/scripts/system-info.sh`
4. Test reverse proxy configuration with all services
5. Update health check endpoints if needed

### Extending OPC UA Browser

1. Modify `OpcUaPoller` in `backend/opcua_poller.rs`
2. Update GUI components in `bin/gui.rs`
3. Add new node types or selection criteria
4. Update worker commands for background operations
5. Add appropriate error handling

### Adding New Error Types

1. Add a new variant to `TelegrafError` enum in `error.rs`
2. Implement user-friendly error message in `user_friendly_message` method
3. Add appropriate `From` implementations for error conversion
4. Update deployment error handling if applicable

## Testing Approach

1. **Unit Tests**: Co-located with source files using `_test.rs` suffix
   - Backend modules have corresponding test files in `src/backend/`
   - Library tests are in `lib_test.rs`
   - Error handling tests are in `error_test.rs`

2. **Integration Tests**: Full application testing in `tests/` directory
   - Command-line interface testing
   - Configuration generation end-to-end tests
   - Snapshot testing for generated configurations

3. **Monitoring Stack Tests**: Comprehensive Docker stack validation
   - **Automated CI Testing**: GitHub Actions workflow (`.github/workflows/monitoring-stack-test.yml`)
     - Docker Compose syntax validation
     - Multi-service orchestration testing
     - Health check validation with timeouts
     - Service integration testing (InfluxDB ↔ Telegraf ↔ Grafana)
     - Multi-architecture build testing (x86_64/ARM64)
     - Deployment script validation
   - **Local Testing**: Interactive test script (`docker/scripts/test.sh`)
     - Complete test suite with colored output
     - Service health monitoring
     - Configuration validation
     - Integration testing
     - Debugging information and logs

4. **Test Tools**:
   - `opcua_test_server.rs`: Standalone OPC UA test server
   - `opcua_client_test.rs`: Tool for testing OPC UA client functionality
   - `docker/scripts/test.sh`: Monitoring stack test runner

5. **Test Coverage**:
   - Uses `cargo-tarpaulin` for code coverage
   - Coverage configuration in `.tarpaulin.toml`
   - Run coverage with `run_coverage.sh`
   - Integrated with Codecov (`.codecov.yml`)

6. **Test Data**:
   - XML sample files in `tests/` directory
   - PKI certificates in `pki/` and `pki-server/` for secure testing
   - Docker test configurations with mock credentials

## Build and Deployment

The project supports multiple build and deployment methods:

### Development Build System
- Development: `cargo build`
- Release: `cargo build --release` (with optimizations)
- Test: `cargo test`
- Windows builds include OpenSSL vendoring

### Docker Stack Deployment
- **Device Provisioning**: `./docker/scripts/init.sh <device_ip>`
- **Stack Deployment**: `./docker/scripts/deploy.sh <device_ip>`
- **Local Testing**: `./docker/scripts/setup.sh && ./docker/scripts/start.sh`
- **Multi-architecture**: Supports x86_64 (development) and ARM64 (production)

### CLI Deployment Commands
- **Provision Device**: `./sie_generate_config deploy provision <host>`
- **Deploy Stack**: `./sie_generate_config deploy setup <host>`
- **Check Status**: `./sie_generate_config deploy status <host>`
- **Start/Stop**: `./sie_generate_config deploy start|stop <host>`

### Nix Support
- Development shell: `nix develop`
- Build with Nix: `nix build`
- Run tests: `nix flake check`

### CI/CD
- **GitHub Actions workflows** in `.github/workflows/`:
  - **linux-ci.yml**: Linux build and test pipeline
  - **windows-build.yml**: Windows cross-compilation
  - **coverage.yml**: Code coverage with Codecov integration
  - **monitoring-stack-test.yml**: Comprehensive Docker stack testing
  - **api-service-build.yml**: API service build and test
  - **binary-release.yml**: Automated binary releases
  - **manual-build.yml**: Manual build triggers
- Automated testing on push/pull requests
- Multi-architecture Docker builds (x86_64/ARM64)
- Pull request templates for consistent contributions

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
