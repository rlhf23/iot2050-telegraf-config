#!/bin/bash
set -e

cd "$(dirname "$0")/../.."

echo "🚀 Starting monitoring stack..."

# Check if .env exists
if [ ! -f docker/.env ]; then
    echo "❌ Error: docker/.env not found. Run setup.sh first."
    exit 1
fi

# Load environment variables
set -a
source docker/.env
set +a

# Start the stack
docker-compose -f docker/docker-compose.yml up -d

echo "✅ Stack started successfully!"
echo ""
echo "📊 Grafana: http://localhost:3000"
echo "   - User: $GRAFANA_ADMIN_USER"
[ -n "$GRAFANA_ADMIN_PASSWORD" ] && echo "   - Password: $GRAFANA_ADMIN_PASSWORD"
echo ""
echo "📈 InfluxDB: http://localhost:8086"
echo "   - User: $INFLUXDB_USER"
[ -n "$INFLUXDB_PASSWORD" ] && echo "   - Password: $INFLUXDB_PASSWORD"
echo "   - Token: $INFLUXDB_TOKEN"

echo "\n🔄 To view logs: docker-compose -f docker/docker-compose.yml logs -f"
