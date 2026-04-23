#!/usr/bin/env bash
set -e

# Usage: setup.sh [--minimal]
#   --minimal  Use minimal profile (InfluxDB + Telegraf + Chronograf only, no Grafana/Prometheus)
#              Also applies InfluxDB memory tuning for constrained devices (~320MB total)

cd "$(dirname "$0")/.."

MINIMAL=false
if [ "$1" = "--minimal" ] || [ "$1" = "-m" ]; then
    MINIMAL=true
fi

echo "🚀 Setting up monitoring stack..."

# --- Telegraf config ---
if [ -f config/telegraf/telegraf.conf.example ]; then
    mkdir -p ~/telegraf
    if [ ! -f ~/telegraf/telegraf.conf ]; then
        cp config/telegraf/telegraf.conf.example ~/telegraf/telegraf.conf
        chmod 644 ~/telegraf/telegraf.conf
        echo "✅ Telegraf configuration installed"
    else
        echo "ℹ️  Telegraf config already exists - preserving"
    fi
else
    echo "⚠️  WARNING: config/telegraf/telegraf.conf.example not found!"
fi

# --- Architecture ---
if [ -z "$TARGETARCH" ]; then
    case "$(uname -m)" in
        x86_64|amd64) TARGETARCH="amd64" ;;
        aarch64|arm64) TARGETARCH="arm64" ;;
        *) TARGETARCH="arm64"; echo "⚠️  Unknown architecture, defaulting to arm64" ;;
    esac
    echo "🔧 Detected architecture: $TARGETARCH"
else
    echo "🔧 Using architecture: $TARGETARCH"
fi

# --- .env file ---
if [ -f .env ]; then
    echo "ℹ️  .env file already exists"
    # Update device identity if provided
    if [ -n "$DEVICE_DISPLAY_NAME" ] || [ -n "$DEVICE_HOSTNAME" ]; then
        if [ -n "$DEVICE_DISPLAY_NAME" ]; then
            sed -i "s|^DEVICE_DISPLAY_NAME=.*|DEVICE_DISPLAY_NAME=\"${DEVICE_DISPLAY_NAME}\"|" .env
        fi
        if [ -n "$DEVICE_HOSTNAME" ]; then
            sed -i "s|^DEVICE_HOSTNAME=.*|DEVICE_HOSTNAME=${DEVICE_HOSTNAME}|" .env
        fi
        echo "✅ Updated device identity in .env"
    fi
else
    echo "ℹ️  Creating .env file with generated credentials..."

    DOCKER_GID=$(stat -c '%g' /var/run/docker.sock 2>/dev/null || echo "")

    if [ "$MINIMAL" = true ]; then
        PROFILE="minimal"
    else
        PROFILE="full"
    fi

    # Device name: use provided values, or generate from CLI, or fall back to defaults
    DEVICE_DISPLAY_NAME=${DEVICE_DISPLAY_NAME:-}
    DEVICE_HOSTNAME=${DEVICE_HOSTNAME:-}

    # Derive bucket names from device hostname, or fall back to defaults
    INFLUXDB_BUCKET=${DEVICE_HOSTNAME:-telegraf}
    INFLUXDB_DIAGNOSTICS_BUCKET=${INFLUXDB_BUCKET}-diag

    cat > .env << EOL
# Device identity
DEVICE_DISPLAY_NAME="${DEVICE_DISPLAY_NAME}"
DEVICE_HOSTNAME=${DEVICE_HOSTNAME}

# InfluxDB
INFLUXDB_USER=admin
INFLUXDB_PASSWORD=$(openssl rand -base64 16 | tr -dc 'a-zA-Z0-9' | head -c 16)
INFLUXDB_ORG=iot2050
INFLUXDB_BUCKET=${INFLUXDB_BUCKET}
INFLUXDB_DIAGNOSTICS_BUCKET=${INFLUXDB_DIAGNOSTICS_BUCKET}
INFLUXDB_TOKEN=$(openssl rand -base64 32 | tr -dc 'a-zA-Z0-9' | head -c 32)

# Grafana
GRAFANA_ADMIN_USER=admin
GRAFANA_ADMIN_PASSWORD=$(openssl rand -base64 16 | tr -dc 'a-zA-Z0-9' | head -c 16)

# Telegraf
TELEGRAF_TOKEN=$(openssl rand -base64 32 | tr -dc 'a-zA-Z0-9' | head -c 32)

# Docker
HOST_DOCKER_GID=${DOCKER_GID}

# Deployment profile: full (all services) or minimal (TICK stack only)
COMPOSE_PROFILE=${PROFILE}

TARGETARCH=${TARGETARCH}
EOL

    if [ "$MINIMAL" = true ]; then
        cat >> .env << 'EOF'

# InfluxDB memory tuning (minimal profile - constrained devices)
INFLUXD_STORAGE_CACHE_MAX_MEMORY_SIZE=134217728
INFLUXD_STORAGE_CACHE_SNAPSHOT_MEMORY_SIZE=67108864
INFLUXD_STORAGE_MAX_CONCURRENT_COMPACTIONS=2
INFLUXD_NO_TASKS=true
INFLUXD_REPORTING_DISABLED=true
EOF
    fi

    echo "✅ Created .env file"
fi

PROFILE_DISPLAY="${PROFILE:-full}"
DEVICE_DISPLAY="${DEVICE_DISPLAY_NAME:-unnamed}"
echo "✅ Setup complete!"
echo "   Architecture: $TARGETARCH"
echo "   Profile: $PROFILE_DISPLAY"
echo "   Device: $DEVICE_DISPLAY"
echo ""
echo "   Start the stack: ./scripts/start.sh"