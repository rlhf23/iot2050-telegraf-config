# Monitoring Stack for IoT2050

A portable Docker-based monitoring stack for local development and ARM64 devices like the Siemens IOT2050.

## Features

- **Multi-architecture**: Works on x86_64 (development) and ARM64 (production)
- **Self-contained**: Versioned configurations and automated setup
- **Production-ready**: Secure defaults and persistent storage
- **Automated provisioning**: Scripts for device setup and deployment

## Components

- **InfluxDB 2.7**: Time-series database
- **Telegraf**: Metrics collection (system, Docker, custom)
- **Grafana**: Visualization dashboard

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
   - Grafana: http://localhost:3000
   - InfluxDB: http://localhost:8086
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
- **Grafana**: http://[device-ip]:3000
- **InfluxDB**: http://[device-ip]:8086

Credentials will be displayed after successful deployment.

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

## Data Management

- **Persistent data** is stored in Docker volumes:
  - `influxdb_data`: Time-series data
  - `grafana_data`: Dashboards and settings

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

## Configuration

- Edit `.env` to change default credentials
- Customize `config/telegraf/telegraf.conf` for metrics collection
