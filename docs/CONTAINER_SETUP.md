# Containerized Monitoring Stack for IoT Devices

## Overview
This document outlines the approach for deploying a containerized monitoring stack that can be easily set up on IoT devices. The stack includes:
- InfluxDB (time-series database)
- Telegraf (metrics collection)
- Grafana (visualization)
- (Optional) Prometheus (alternative metrics collection)

## Testing Without Hardware

### 1. Quick Emulation with Docker

Test ARM64 containers using Docker's built-in emulation:

```bash
# Enable QEMU for ARM emulation (Linux/macOS)
docker run --privileged --rm tonistiigi/binfmt --install all

# Test an ARM64 container
docker run --rm -it --platform linux/arm64 arm64v8/ubuntu uname -m
# Should output: aarch64
```

### 2. Test the Full Stack

1. First, enable ARM emulation as above
2. Create a test directory and add the docker-compose.yml
3. Start the stack:
   ```bash
   docker-compose up -d
   ```
4. Verify:
   ```bash
   # Check containers are running
   docker ps
   
   # Check logs
   docker-compose logs -f
   
   # Access services
   # Grafana: http://localhost:3000
   # InfluxDB: http://localhost:8086
   ```

### 3. Testing OPC UA (Optional)

To test OPC UA connectivity:

```bash
# Run an OPC UA test server
docker run -d --name opcua-server -p 4840:4840 msiminnalucca/opcua-server

# Test connection from an ARM64 container
docker run --rm -it --network host \
  ghcr.io/opcua/opcua-client opcua-client \
  --endpoint opc.tcp://localhost:4840
```

## Deployment Strategies

### Option 1: Local Build and Transfer (Easier)
- **Process**:
  1. Build container images on the development machine
  2. Save images as tar archives
  3. Transfer archives to the IoT device
  4. Load and run containers on the device

- **Pros**:
  - Works offline after initial setup
  - Single transfer operation
  - More reliable for air-gapped environments

- **Cons**:
  - Larger initial transfer size
  - Manual version updates require rebuilding

### Option 2: On-Device Build with Docker Compose
- **Process**:
  1. Transfer only the docker-compose.yml and config files
  2. Let the device pull/built images as needed

- **Pros**:
  - Smaller initial transfer
  - Easier updates
  - More maintainable

- **Cons**:
  - Requires internet access on the device
  - Initial setup might be slower
  - More complex error handling

## Recommended Approach: Hybrid Solution

### Initial Setup Phase
1. **Development Machine**:
   - Create `docker-compose.yml` and configuration files
   - Build images locally for testing
   - Generate a deployment package

2. **Deployment Package**:
   ```
   deployment/
   ├── docker-compose.yml
   ├── .env.example
   ├── config/
   │   ├── telegraf/
   │   ├── grafana/
   │   └── influxdb/
   └── scripts/
       ├── deploy.sh
       └── update.sh
   ```

3. **Deployment Scripts**:
   - `deploy.sh`: Handles initial setup and offline deployment
   - `update.sh`: Handles future updates (can use online resources)

## Implementation Details

### 1. Docker Compose Configuration
```yaml
version: '3.8'

services:
  influxdb:
    image: influxdb:2.7
    container_name: influxdb
    ports:
      - "8086:8086"
    volumes:
      - influxdb_data:/var/lib/influxdb2
    environment:
      - DOCKER_INFLUXDB_INIT_MODE=setup
      - DOCKER_INFLUXDB_INIT_USERNAME=${INFLUXDB_USER}
      - DOCKER_INFLUXDB_INIT_PASSWORD=${INFLUXDB_PASSWORD}
      - DOCKER_INFLUXDB_INIT_ORG=${INFLUXDB_ORG}
      - DOCKER_INFLUXDB_INIT_BUCKET=${INFLUXDB_BUCKET}
      - DOCKER_INFLUXDB_INIT_ADMIN_TOKEN=${INFLUXDB_TOKEN}
    networks:
      - monitoring

  telegraf:
    image: telegraf:latest
    container_name: telegraf
    volumes:
      - ./config/telegraf/telegraf.conf:/etc/telegraf/telegraf.conf:ro
      - /:/hostfs:ro
      - /var/run/docker.sock:/var/run/docker.sock:ro
    environment:
      - HOST_PROC=/hostfs/proc
      - HOST_SYS=/hostfs/sys
      - HOST_ETC=/hostfs/etc
      - HOST_MOUNT_PREFIX=/hostfs
    depends_on:
      - influxdb
    networks:
      - monitoring

  grafana:
    image: grafana/grafana:latest
    container_name: grafana
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana
      - ./config/grafana/provisioning:/etc/grafana/provisioning
    environment:
      - GF_SECURITY_ADMIN_USER=${GRAFANA_ADMIN_USER}
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_ADMIN_PASSWORD}
    depends_on:
      - influxdb
    networks:
      - monitoring

networks:
  monitoring:
    driver: bridge

volumes:
  influxdb_data:
  grafana_data:
```

### 2. Environment Configuration (`.env`)
```
# InfluxDB
INFLUXDB_USER=admin
INFLUXDB_PASSWORD=your_secure_password
INFLUXDB_ORG=iot2050
INFLUXDB_BUCKET=telegraf
INFLUXDB_TOKEN=your_secure_token

# Grafana
GRAFANA_ADMIN_USER=admin
GRAFANA_ADMIN_PASSWORD=your_secure_password
```

### 3. Deployment Script (`deploy.sh`)
```bash
#!/bin/bash
set -e

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Create necessary directories
mkdir -p ./config/telegraf
mkdir -p ./config/grafana/provisioning/datasources
mkdir -p ./config/grafana/provisioning/dashboards

# Generate Telegraf config if not exists
if [ ! -f ./config/telegraf/telegraf.conf ]; then
    cat > ./config/telegraf/telegraf.conf <<EOL
[agent]
  interval = "10s"
  round_interval = true
  metric_batch_size = 1000
  metric_buffer_limit = 10000
  collection_jitter = "0s"
  flush_interval = "10s"
  flush_jitter = "0s"
  precision = ""
  hostname = ""
  omit_hostname = false

[[outputs.influxdb_v2]]
  urls = ["http://influxdb:8086"]
  token = "${INFLUXDB_TOKEN}"
  organization = "${INFLUXDB_ORG}"
  bucket = "${INFLUXDB_BUCKET}"

[[inputs.cpu]]
  percpu = true
  totalcpu = true
  collect_cpu_time = false
  report_active = false

[[inputs.mem]]
[[inputs.disk]]
  ignore_fs = ["tmpfs", "devtmpfs", "devfs", "iso9660", "overlay", "aufs", "squashfs"]

[[inputs.net]]
[[inputs.system]]
EOL
fi

# Setup Grafana provisioning
cat > ./config/grafana/provisioning/datasources/influxdb.yml <<EOL
apiVersion: 1

datasources:
  - name: InfluxDB
    type: influxdb
    access: proxy
    url: http://influxdb:8086
    jsonData:
      httpMode: POST
      organization: ${INFLUXDB_ORG}
      defaultBucket: ${INFLUXDB_BUCKET}
      tlsSkipVerify: true
    secureJsonData:
      token: ${INFLUXDB_TOKEN}
    version: 1
    editable: true
EOL

# Start services
docker-compose up -d

echo "\nSetup complete!"
echo "- Grafana: http://localhost:3000"
echo "- InfluxDB: http://localhost:8086"
echo "\nDefault credentials:"
echo "- Grafana: ${GRAFANA_ADMIN_USER} / [set in .env]"
echo "- InfluxDB: ${INFLUXDB_USER} / [set in .env]"
```

## Security Considerations

1. **Default Credentials**:
   - Never commit real credentials to version control
   - Use `.env` file (in `.gitignore`)
   - Generate strong random passwords

2. **Network Security**:
   - Expose only necessary ports
   - Consider using a reverse proxy with HTTPS
   - Use Docker's built-in network isolation

3. **Updates**:
   - Regularly update container images
   - Monitor for security vulnerabilities

## Next Steps

1. Test the deployment locally
2. Create a deployment package
3. Document the deployment process
4. Create update scripts for maintenance
5. Add monitoring and alerting for the monitoring stack itself

## Maintenance

To update the stack:
1. Pull the latest images: `docker-compose pull`
2. Recreate containers: `docker-compose up -d`
3. Clean up old images: `docker image prune -f`

## Troubleshooting

Common issues and solutions:

1. **Port conflicts**:
   - Check if ports 3000, 8086 are already in use
   - Modify `docker-compose.yml` if needed

2. **Permission issues**:
   - Ensure the Docker daemon is running
   - Check directory permissions for mounted volumes

3. **Container startup failures**:
   - Check logs: `docker-compose logs [service]`
   - Verify environment variables
   - Check disk space: `df -h`
