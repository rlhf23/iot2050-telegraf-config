# Monitoring Stack for IoT2050

This directory contains the Docker-based monitoring stack for the IoT2050 device.

## Components

- **InfluxDB 2.7**: Time-series database
- **Telegraf**: Metrics collection
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
   ./scripts/setup.sh
   ```

2. **Start the stack**:
   ```bash
   ./scripts/start.sh
   ```

3. **Access the services**:
   - Grafana: http://localhost:3000
   - InfluxDB: http://localhost:8086

4. **Stop the stack**:
   ```bash
   ./scripts/stop.sh
   ```

## Configuration

- Edit `.env` to change default credentials
- Edit `config/telegraf/telegraf.conf` to modify metrics collection

## Deployment to IoT2050

1. Build and save the images:
   ```bash
   docker-compose build
   docker save $(docker-compose config --images) -o monitoring-stack.tar
   ```

2. Transfer to device:
   ```bash
   scp -r . ${DEFAULT_IOT_USERNAME}@${DEFAULT_IOT_IP}:~/monitoring
   scp monitoring-stack.tar ${DEFAULT_IOT_USERNAME}@${DEFAULT_IOT_IP}:~/
   ```

3. On the device:
   ```bash
   docker load -i monitoring-stack.tar
   cd monitoring
   ./scripts/start.sh
   ```

## Troubleshooting

- View logs: `docker-compose logs -f`
- Check container status: `docker ps`
- Access container shell: `docker exec -it <container_name> sh`
