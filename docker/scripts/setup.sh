#!/usr/bin/env bash
set -e

# Change to the script's directory
cd "$(dirname "$0")/.."

echo "🚀 Setting up monitoring stack..."

# Create necessary directories
echo "Creating required directories..."
mkdir -p config/grafana/provisioning/datasources
mkdir -p config/grafana/provisioning/dashboards
mkdir -p config/telegraf

# Copy example config if it doesn't exist
if [ ! -f config/telegraf/telegraf.conf ] && [ -f config/telegraf/telegraf.conf.example ]; then
    echo "Creating telegraf.conf from example..."
    cp config/telegraf/telegraf.conf.example config/telegraf/telegraf.conf
fi

# Get Docker GID
DOCKER_GID=$(stat -c '%g' /var/run/docker.sock 2>/dev/null || echo "")

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
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
