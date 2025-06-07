# Monitoring Stack for IoT2050

A portable Docker-based monitoring stack for local development and ARM64 devices like the Siemens IOT2050.

## Features

- **Multi-architecture**: Works on x86_64 (development) and ARM64 (production)
- **Self-contained**: Versioned configurations and automated setup
- **Production-ready**: Secure defaults and persistent storage

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

## Quick Start

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

## Deployment to IOT2050

### Option 1: Build on Device (Simpler, requires internet)

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

### Option 2: Pre-build and Transfer (Offline-friendly)

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

### Access Remotely
- Grafana: http://[device-ip]:3000
- InfluxDB: http://[device-ip]:8086

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
