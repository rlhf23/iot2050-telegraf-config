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
docker-compose -f docker-compose.yml up -d

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
echo ""
echo "🔄 To view logs: docker-compose -f docker-compose.yml logs -f"
