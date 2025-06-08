#!/usr/bin/env bash
set -e

echo "🚀 Setting up monitoring stack..."

# Create necessary directories
mkdir -p ../docker/config/grafana/provisioning/datasources
mkdir -p ../docker/config/grafana/provisioning/dashboards

# Copy example config if it doesn't exist
if [ ! -f ../docker/config/telegraf/telegraf.conf ]; then
    echo "ℹ️ Creating telegraf.conf from example..."
    cp ../docker/config/telegraf/telegraf.conf.example ../docker/config/telegraf/telegraf.conf
fi

# Create .env file if it doesn't exist
if [ ! -f ../docker/.env ]; then
    echo "ℹ️ Creating .env file with default values..."
    cat > ../docker/.env <<EOL
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
TELEGRAF_SSH_PASSWORD=$(openssl rand -base64 16 | tr -dc 'a-zA-Z0-9' | head -c 16)
EOL
fi

# Load environment variables
echo "📋 Loading environment variables..."
set -a
source ../docker/.env
set +a

# Enable ARM emulation if not on ARM64
if [ "$(uname -m)" != "aarch64" ]; then
    echo "🔧 Enabling ARM emulation..."
    docker run --privileged --rm tonistiigi/binfmt --install all
fi

echo "✅ Setup complete!"
echo "🔑 Generated credentials are in docker/.env"
echo "🔄 Start the stack with: docker-compose up -d"
