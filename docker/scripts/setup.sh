#!/usr/bin/env bash
set -e

# Change to the script's directory
cd "$(dirname "$0")/.."

echo "🚀 Setting up monitoring stack..."

# Install Telegraf configuration from example (only if not already present)
if [ -f config/telegraf/telegraf.conf.example ]; then
    mkdir -p ~/telegraf
    if [ -f ~/telegraf/telegraf.conf ]; then
        echo "ℹ️  Telegraf config already exists at ~/telegraf/telegraf.conf - preserving existing configuration."
    else
        echo "Found config/telegraf/telegraf.conf.example. Installing to ~/telegraf/telegraf.conf..."
        cp config/telegraf/telegraf.conf.example ~/telegraf/telegraf.conf
        chmod 644 ~/telegraf/telegraf.conf
        echo "✅ Telegraf configuration installed to ~/telegraf/telegraf.conf from example."
    fi
else
    echo "⚠️  WARNING: config/telegraf/telegraf.conf.example not found!"
    echo "Telegraf will likely use a default or no configuration."
fi

# Get Docker GID
DOCKER_GID=$(stat -c '%g' /var/run/docker.sock 2>/dev/null || echo "")

# Check if .env file exists
if [ ! -f .env ]; then
    echo "ℹ️ .env file not found. New credentials will be generated."
    
    # Detect non-interactive mode (no TTY)
    if ! [ -t 0 ]; then
        AUTO_DENY="yes"
    fi
    # Only ask about removing volumes if they exist
    if docker volume ls | grep -q 'influxdb_data\|grafana_data\|prometheus_data\|monitoring_influxdb_data\|monitoring_grafana_data\|monitoring_prometheus_data'; then
        echo "To ensure new credentials (especially for InfluxDB, Grafana, and Prometheus) take effect,"
        echo "it's recommended to remove existing data volumes."
        if [ -n "$AUTO_DENY" ]; then
            confirmation="no"
            echo "⚠️  Non-interactive mode: Skipping volume removal to prevent data loss."
        else
            read -r -p "Do you want to remove influxdb_data, grafana_data, prometheus_data (with or without 'monitoring_' prefix) volumes? (yes/NO): " confirmation
        fi
        if [[ "$confirmation" =~ ^[Yy][Ee][Ss]$ ]]; then
            for v in influxdb_data grafana_data prometheus_data monitoring_influxdb_data monitoring_grafana_data monitoring_prometheus_data; do
                echo "Attempting to remove $v..."
                docker volume rm "$v" 2>/dev/null || echo "$v not found or could not be removed."
            done
            echo "✅ Volumes removal process finished."
        else
            echo "Skipping volume removal. Existing data will be preserved."
            echo "If you experience issues with old credentials, manually remove the volumes and re-run setup."
        fi
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

# Detect and export architecture for api-service binary selection
detect_architecture() {
    local machine_arch
    machine_arch=$(uname -m)
    
    case "$machine_arch" in
        x86_64|amd64)
            export TARGETARCH="amd64"
            ;;
        aarch64|arm64)
            export TARGETARCH="arm64"
            ;;
        *)
            echo "⚠️  WARNING: Unknown architecture '$machine_arch', defaulting to arm64"
            export TARGETARCH="arm64"
            ;;
    esac
}

detect_architecture
echo "🔧 Detected architecture: $TARGETARCH"

# Write architecture to .env file for docker-compose
if [ -f .env ]; then
    # Remove existing TARGETARCH line if present
    grep -v "^TARGETARCH=" .env > .env.tmp || true
    mv .env.tmp .env
fi
# Add TARGETARCH to .env
echo "TARGETARCH=${TARGETARCH}" >> .env

echo "✅ Setup complete!"
echo "   Target architecture: ${TARGETARCH}"
