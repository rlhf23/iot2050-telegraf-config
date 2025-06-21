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

# Start the stack
echo "Starting containers..."
docker-compose -f docker-compose.base.yml -f docker-compose.linux.yml up -d

# Wait for all running containers to be healthy or exited (timeout after 60 seconds)
timeout=60
interval=2
elapsed=0

# Get list of services that should be running
services=($(docker-compose -f docker-compose.base.yml -f docker-compose.linux.yml config --services))

if [ ${#services[@]} -eq 0 ]; then
  echo "⚠️  No services found in docker-compose files"
  exit 1
fi

echo "⏳ Waiting for all containers to be healthy..."

while [ $elapsed -lt $timeout ]; do
  all_healthy=true
  
  for service in "${services[@]}"; do
    container_id=$(docker-compose -f docker-compose.base.yml -f docker-compose.linux.yml ps -q "$service")
    
    if [ -z "$container_id" ]; then
      echo "⚠️  Container for service $service not found"
      all_healthy=false
      continue
    fi
    
    health_status=$(docker inspect --format='{{.State.Health.Status}}' "$container_id" 2>/dev/null || echo "no healthcheck")
    
    if [ "$health_status" = "unhealthy" ] || [ "$health_status" = "starting" ]; then
      all_healthy=false
      echo "  $service: $health_status"
      break
    fi
  done
  
  if $all_healthy; then
    break
  fi
  
  sleep $interval
  elapsed=$((elapsed + interval))
  
  # Show progress every 10 seconds
  if [ $((elapsed % 10)) -eq 0 ]; then
    echo "  Still waiting... (${elapsed}s / ${timeout}s)"
  fi
done

if [ $elapsed -ge $timeout ]; then
  echo "⚠️  Timeout waiting for containers to become healthy. Some services may still be starting."
  docker-compose -f docker-compose.base.yml -f docker-compose.linux.yml ps
fi

echo ""
echo "✅ Stack started successfully!"
echo ""
echo "📊 Grafana: http://localhost:3000"
echo "   - User: ${GRAFANA_ADMIN_USER:-admin}"
[ -n "$GRAFANA_ADMIN_PASSWORD" ] && echo "   - Password: $GRAFANA_ADMIN_PASSWORD"
echo ""
echo "📈 InfluxDB: http://localhost:8086"
echo "   - User: ${INFLUXDB_USER:-admin}"
[ -n "$INFLUXDB_PASSWORD" ] && echo "   - Password: $INFLUXDB_PASSWORD"
[ -n "$INFLUXDB_TOKEN" ] && echo "   - Token: $INFLUXDB_TOKEN"
echo ""
echo "🔍 Prometheus: http://localhost:9090"
echo ""
echo "📊 Telegraf metrics: http://localhost:9273/metrics"
echo ""
echo "To stop the stack, run: docker-compose -f docker-compose.base.yml -f docker-compose.linux.yml down"
echo "🔄 To view logs: docker-compose -f docker-compose.base.yml -f docker-compose.linux.yml logs -f"
