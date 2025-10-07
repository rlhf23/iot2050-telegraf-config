#!/bin/bash
# Integration tests for the monitoring stack

set -e

echo "🧪 Running integration tests..."

# Wait for services to be ready
echo "⏳ Waiting for services to start..."
sleep 10

# Test 1: Health endpoint
echo ""
echo "Test 1: Health endpoint"
if curl -f -s http://localhost/health > /dev/null; then
  echo "✅ Health endpoint is responding"
else
  echo "❌ Health endpoint failed"
  exit 1
fi

# Test 2: System info endpoint
echo ""
echo "Test 2: System info endpoint"
RESPONSE=$(curl -f -s http://localhost/api/system-info)
if [ $? -eq 0 ]; then
  echo "✅ System info endpoint is responding"
  
  # Validate JSON structure
  echo "$RESPONSE" | jq -e '.hostname, .ip_address, .system_time' > /dev/null
  if [ $? -eq 0 ]; then
    echo "✅ System info JSON has required fields"
  else
    echo "❌ System info JSON is missing required fields"
    exit 1
  fi
else
  echo "❌ System info endpoint failed"
  exit 1
fi

# Test 3: Grafana
echo ""
echo "Test 3: Grafana"
if curl -f -s http://localhost/grafana/api/health > /dev/null; then
  echo "✅ Grafana is responding"
else
  echo "❌ Grafana failed"
  exit 1
fi

# Test 4: InfluxDB
echo ""
echo "Test 4: InfluxDB"
if curl -f -s http://localhost/influxdb/health > /dev/null; then
  echo "✅ InfluxDB is responding"
else
  echo "❌ InfluxDB failed"
  exit 1
fi

# Test 5: Prometheus
echo ""
echo "Test 5: Prometheus"
if curl -f -s http://localhost/prometheus/-/healthy > /dev/null; then
  echo "✅ Prometheus is responding"
else
  echo "❌ Prometheus failed"
  exit 1
fi

# Test 6: Check nginx container is healthy
echo ""
echo "Test 6: Nginx container health"
NGINX_HEALTH=$(docker inspect --format='{{.State.Health.Status}}' nginx-dashboard 2>/dev/null || echo "unknown")
if [ "$NGINX_HEALTH" = "healthy" ]; then
  echo "✅ Nginx container is healthy"
else
  echo "⚠️  Nginx container health: $NGINX_HEALTH"
fi

# Test 7: Verify system-info.json exists
echo ""
echo "Test 7: System info JSON file"
if docker exec nginx-dashboard test -f /usr/share/nginx/html/system-info.json; then
  echo "✅ system-info.json file exists"
else
  echo "❌ system-info.json file does not exist"
  exit 1
fi

echo ""
echo "✅ All integration tests passed!"
