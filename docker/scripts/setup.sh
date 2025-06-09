#!/usr/bin/env bash
set -e

# Change to the script's directory
cd "$(dirname "$0")/.."

echo "🚀 Setting up monitoring stack..."

# Create necessary directories for Grafana
echo "Creating required Grafana directories..."
mkdir -p config/grafana/provisioning/datasources
mkdir -p config/grafana/provisioning/dashboards

# Install Telegraf configuration from example
if [ -f config/telegraf/telegraf.conf.example ]; then
    echo "Found config/telegraf/telegraf.conf.example. Installing to /etc/telegraf/telegraf.conf..."
    if [ ! -d /etc/telegraf ]; then
        echo "Creating /etc/telegraf directory..."
        sudo mkdir -p /etc/telegraf
    fi
    sudo cp config/telegraf/telegraf.conf.example /etc/telegraf/telegraf.conf
    sudo chown root:root /etc/telegraf/telegraf.conf # Ensure correct ownership
    sudo chmod 644 /etc/telegraf/telegraf.conf      # Ensure correct permissions
    echo "✅ Telegraf configuration installed to /etc/telegraf/telegraf.conf from example."
else
    echo "⚠️  WARNING: config/telegraf/telegraf.conf.example not found!"
    echo "Telegraf will likely use a default or no configuration."
fi

# Get Docker GID
DOCKER_GID=$(stat -c '%g' /var/run/docker.sock 2>/dev/null || echo "")

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    echo "ℹ️ .env file not found. New credentials will be generated."
    echo "To ensure new credentials (especially for InfluxDB and Grafana) take effect,"
    echo "it's recommended to remove existing data volumes."
    read -r -p "Do you want to remove 'docker_influxdb_data' and 'docker_grafana_data' volumes? (yes/NO): " confirmation
    if [[ "$confirmation" =~ ^[Yy][Ee][Ss]$ ]]; then
        echo "Attempting to remove InfluxDB data volume..."
        docker volume rm docker_influxdb_data 2>/dev/null || echo "InfluxDB data volume not found or could not be removed."
        echo "Attempting to remove Grafana data volume..."
        docker volume rm docker_grafana_data 2>/dev/null || echo "Grafana data volume not found or could not be removed."
        echo "✅ Volumes removal process finished."
    else
        echo "Skipping volume removal. Existing data will be preserved."
        echo "If you experience issues with old credentials, manually remove the volumes and re-run setup."
    fi
    echo "Creating .env file with default values..."
    cat > .env <<EOL
# InfluxDB
INFLUXDB_USER=admin
INFLUXDB_PASSWORD=$(openssl rand -base64 16 | tr -dc 'a-zA-Z0-9' | head -c 16)
INFLUXDB_ORG=iot2050
INFLUXDB_BUCKET=telegraf
INFLUXDB_TOKEN=$(openssl rand -base64 32 | tr -dc 'a-zA-Z0-9' | head -c 32)

# Grafana
GRAFANA_ADMIN_USER=admin
GRAFANA_ADMIN_PASSWORD=$(openssl rand -base64 16 | tr -dc 'a-zA-Z0-9' | head -c 16)

# Telegraf
TELEGRAF_TOKEN=$(openssl rand -base64 32 | tr -dc 'a-zA-Z0-9' | head -c 32)

# Docker
HOST_DOCKER_GID=${DOCKER_GID}
EOL
    echo "✅ Created .env file"
else
    echo "ℹ️  .env file already exists"
fi

# Enable ARM emulation if not on ARM64
if [ "$(uname -m)" != "aarch64" ]; then
    echo "Enabling ARM emulation..."
    if ! docker run --privileged --rm tonistiigi/binfmt --install all; then
        echo "⚠️  Failed to enable ARM emulation (this might be expected in some environments)" >&2
    fi
fi

echo "✅ Setup complete!"
