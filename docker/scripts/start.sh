#!/usr/bin/env bash
set -e

echo "🚀 Starting monitoring stack..."

# Change to the script's directory
cd "$(dirname "$0")/.."

# Check if .env exists
if [ ! -f .env ]; then
  echo "❌ Error: .env not found. Run setup.sh first."
  exit 1
fi

# Load environment variables
set -a
source .env
set +a

# Build compose profile flags from COMPOSE_PROFILE
PROFILE_ARGS=""
if [ -n "$COMPOSE_PROFILE" ]; then
  for profile in $COMPOSE_PROFILE; do
    PROFILE_ARGS="$PROFILE_ARGS --profile $profile"
  done
  echo "🔧 Using profile(s): $COMPOSE_PROFILE"
fi

# Start the stack
echo "🔧 Starting containers..."
docker-compose -f docker-compose.yml $PROFILE_ARGS up -d

# Rebuild prebuilt containers to pick up binary updates
echo "🔧 Rebuilding prebuilt containers..."
docker-compose build api-service control-service 2>/dev/null && \
  docker-compose up -d --force-recreate --no-deps api-service control-service

# Wait for all running containers to be healthy or exited (timeout after 60 seconds)
timeout=10
interval=2
elapsed=0

echo "⏳ Waiting for all containers to be healthy..."

while true; do
  unhealthy=$(docker ps --filter "health=unhealthy" --format '{{.Names}}')
  starting=$(docker ps --filter "health=starting" --format '{{.Names}}')
  if [ -z "$unhealthy" ] && [ -z "$starting" ]; then
    break
  fi
  sleep $interval
  elapsed=$((elapsed + interval))
  if [ $elapsed -ge $timeout ]; then
    echo "⚠️  Timeout waiting for containers to become healthy."
    break
  fi
done

echo "✅ Stack started successfully!"
echo ""
echo "📈 InfluxDB: http://localhost:8086"
echo "   - User: $INFLUXDB_USER"
[ -n "$INFLUXDB_PASSWORD" ] && echo "   - Password: $INFLUXDB_PASSWORD"
[ -n "$INFLUXDB_TOKEN" ] && echo "   - Token: $INFLUXDB_TOKEN"
echo ""
echo "📊 Chronograf: http://localhost:8888"

if echo "$COMPOSE_PROFILE" | grep -qw "full"; then
  echo ""
  echo "📊 Grafana: http://localhost:3000"
  echo "   - User: $GRAFANA_ADMIN_USER"
  [ -n "$GRAFANA_ADMIN_PASSWORD" ] && echo "   - Password: $GRAFANA_ADMIN_PASSWORD"
  echo ""
  echo "📊 Prometheus: http://localhost:9090"
fi

echo ""
echo "🔄 To view logs: docker-compose -f docker-compose.yml logs -f"