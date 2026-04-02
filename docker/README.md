# Monitoring Stack for IoT2050

A portable Docker-based monitoring stack for local development and ARM64 devices like the Siemens IOT2050.

## Features

- **Multi-architecture**: Works on x86_64 (development) and ARM64 (production)
- **Self-contained**: Versioned configurations and automated setup
- **Production-ready**: Secure defaults and persistent storage
- **Automated provisioning**: Scripts for device setup and deployment

## Components

- **Nginx**: Landing page dashboard with service status and links (port 80)
- **InfluxDB 2.7**: Time-series database (port 8086)
- **Telegraf**: Metrics collection (system, Docker, custom) (port 9273)
- **Grafana**: Visualization dashboard (port 3000)
- **Prometheus**: Monitoring system and time-series database (port 9090)
- **Control Service**: PLC control panel for Siemens S7 (port 8002, accessible via /control)

## Prerequisites

- Docker and Docker Compose
- For ARM64 emulation (on x86_64):
  ```bash
  docker run --privileged --rm tonistiigi/binfmt --install all
  ```

## Quick Start (Local Development)

1. **Initialize the environment**:
   ```bash
   chmod +x scripts/*.sh
   ./scripts/setup.sh  # Creates .env with random credentials
   ```

2. **Start the stack**:
   ```bash
   ./scripts/start.sh
   ```

3. **Access the services**:
   - **Landing Page**: http://localhost (port 80) - Main dashboard with service status and links
   - Grafana: http://localhost:3000 (or via landing page at http://localhost/grafana)
   - InfluxDB: http://localhost:8086 (or via landing page at http://localhost/influxdb)
   - Prometheus: http://localhost:9090 (or via landing page at http://localhost/prometheus)
   - Telegraf Metrics: http://localhost:9273 (or via landing page at http://localhost/telegraf)
   - **PLC Control**: http://localhost/control (requires auth, default: admin/admin)
   - Default credentials are in `.env`

4. **Stop the stack**:
   ```bash
   ./scripts/stop.sh
   ```

## IoT Device Deployment (Recommended)

### Step 1: Provision the Device

Use the `init.sh` script to provision a new IoT device with Docker and required dependencies:

```bash
# Basic usage
./scripts/init.sh 192.168.1.100

# With custom SSH settings
./scripts/init.sh -u iot2050 -p 2222 192.168.1.100

# With SSH key
./scripts/init.sh --key ~/.ssh/iot_key 192.168.1.100

# See all options
./scripts/init.sh --help
```

**What it does:**
- Installs Docker and Docker Compose
- Sets up user permissions
- Configures the system for monitoring stack deployment
- Works with Debian/Armbian based systems

### Step 2: Deploy the Stack

Use the `deploy.sh` script to build locally and deploy to the provisioned device:

```bash
# Build and deploy in one command
./scripts/deploy.sh 192.168.1.100

# With custom SSH settings
./scripts/deploy.sh -u iot2050 -p 2222 192.168.1.100

# Build only (for later deployment)
./scripts/deploy.sh --build-only

# Deploy without rebuilding
./scripts/deploy.sh --no-build 192.168.1.100

# See all options
./scripts/deploy.sh --help
```

**What it does:**
- Builds Docker images locally (with ARM64 support)
- Transfers images and configuration to the device
- Loads images on the remote device
- Sets up and starts the monitoring stack
- Reports service status and credentials

### Step 3: Access Your Services

After deployment, access your services at:
- **Landing Page**: http://[device-ip] (port 80) - Main dashboard with service status and quick links
- **Grafana**: http://[device-ip]:3000 (or http://[device-ip]/grafana)
- **InfluxDB**: http://[device-ip]:8086 (or http://[device-ip]/influxdb)
- **Prometheus**: http://[device-ip]:9090 (or http://[device-ip]/prometheus)
- **PLC Control**: http://[device-ip]/control (default: admin/admin)

Credentials will be displayed after successful deployment.

> **💡 Tip**: The landing page provides a convenient overview of all services with real-time health status checks. Simply navigate to your device's IP address in a browser (e.g., http://192.168.0.1) to access it.

## Manual Deployment Methods

### Option 1: Build on Device (Requires Internet)

1. **SSH into your IOT2050**:
   ```bash
   ssh ${DEFAULT_IOT_USERNAME}@${DEFAULT_IOT_IP}
   ```

2. **Clone and run**:
   ```bash
   git clone <your-repo-url>
   cd iot2050-telegraf-config/docker
   chmod +x scripts/*.sh
   ./scripts/setup.sh
   ./scripts/start.sh
   ```

   **⚠️ Note**: 
   - Requires internet access during build
   - Slower than pre-built images
   - Needs sufficient disk space for build cache

### Option 2: Manual Pre-build and Transfer

1. **Build and save images** (on your dev machine):
   ```bash
   cd docker
   docker-compose build
   docker save $(docker-compose config --images) -o monitoring-stack.tar
   ```

2. **Transfer to device**:
   ```bash
   scp -r . ${DEFAULT_IOT_USERNAME}@${DEFAULT_IOT_IP}:~/monitoring
   scp monitoring-stack.tar ${DEFAULT_IOT_USERNAME}@${DEFAULT_IOT_IP}:~/
   ```

3. **On the IOT2050**:
   ```bash
   # Load Docker images
   docker load -i ~/monitoring-stack.tar
   
   # Start the stack
   cd ~/monitoring
   chmod +x scripts/*.sh
   ./scripts/setup.sh
   ./scripts/start.sh
   ```

## Landing Page Dashboard

The monitoring stack includes a beautiful landing page accessible on port 80 that provides:

### Features
- **Real-time Service Status**: Automatic health checks every 10 seconds
- **Quick Access Links**: Direct links to all web interfaces (Grafana, InfluxDB, Prometheus, Telegraf)
- **Reverse Proxy**: Access all services through a single port with path-based routing
- **Modern UI**: Responsive design that works on desktop and mobile devices
- **Service Information**: Port numbers and service descriptions at a glance

### Access Methods

1. **Direct access to landing page**: http://[device-ip] or http://[device-ip]:80
2. **Access services through landing page**:
   - Grafana: http://[device-ip]/grafana
   - InfluxDB: http://[device-ip]/influxdb
   - Prometheus: http://[device-ip]/prometheus
   - Telegraf: http://[device-ip]/telegraf

3. **Direct access to services** (still available):
   - Grafana: http://[device-ip]:3000
   - InfluxDB: http://[device-ip]:8086
   - Prometheus: http://[device-ip]:9090
   - Telegraf: http://[device-ip]:9273

### Customization

The landing page can be customized by editing:
- **HTML/CSS/JS**: `config/nginx/html/index.html`
- **Nginx configuration**: `config/nginx/nginx.conf`

After making changes, restart the nginx service:
```bash
docker-compose restart nginx
```

## Data Management

- **Persistent data** is stored in Docker volumes:
  - `influxdb_data`: Time-series data
  - `grafana_data`: Dashboards and settings
  - `prometheus_data`: Prometheus metrics and data

- **Backup InfluxDB data**:
  ```bash
  docker run --rm -v influxdb_data:/source -v $(pwd):/backup alpine tar czf /backup/influxdb_backup.tar.gz -C /source .
  ```

## Troubleshooting

- **View logs**: `docker-compose logs -f`
- **Check containers**: `docker ps`
- **Access shell**: `docker exec -it <container_name> sh`
- **Reset everything**:
  ```bash
  ./scripts/stop.sh
  docker-compose down -v
  ```

## Testing

### Automated Testing

The monitoring stack includes comprehensive automated tests that run in GitHub Actions:

- **Docker Compose validation**: Ensures the configuration is syntactically correct
- **Service startup testing**: Verifies all services start correctly
- **Health check testing**: Tests that services become healthy within expected timeframes
- **Service integration**: Ensures services can communicate properly
- **Configuration validation**: Tests that configurations are valid
- **Multi-architecture support**: Verifies ARM64 compatibility

### Local Testing

Run the complete test suite locally:

```bash
# Run all tests
./scripts/test.sh
```

**What it tests:**
- Docker Compose syntax validation
- Service startup and health checks
- Endpoint accessibility
- Basic authentication
- Configuration validation
- Integration between services

**Test output:**
- Colored output showing pass/fail status
- Service logs for debugging
- Access URLs and credentials
- Cleanup instructions

### Manual Testing

For development and debugging:

```bash
# Start the stack
./scripts/setup.sh
./scripts/start.sh

# Check service status
docker-compose ps
docker-compose logs -f

# Test individual services
curl http://localhost:8086/health      # InfluxDB
curl http://localhost:3000/api/health  # Grafana
curl http://localhost:9090/-/healthy   # Prometheus

# Stop when done
./scripts/stop.sh
```

### CI/CD Integration

The monitoring stack is automatically tested in GitHub Actions on:
- Push to `master` or `poller` branches (when docker files change)
- Pull requests affecting the docker directory
- Manual workflow dispatch

Tests include:
- Ubuntu latest environment
- Docker Compose validation
- Multi-service orchestration
- Health checks with timeout
- Multi-architecture build capability
- Deployment script validation

## Configuration

- Edit `.env` to change default credentials
- Customize `config/telegraf/telegraf.conf` for metrics collection
