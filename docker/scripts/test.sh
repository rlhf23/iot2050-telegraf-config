#!/bin/bash
set -e

# Local testing script for the monitoring stack
# This script provides the same tests as the GitHub Actions workflow
# but can be run locally for development and debugging

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$(dirname "$SCRIPT_DIR")"

cd "$DOCKER_DIR"

# Determine docker compose command (v2 or v1)
if docker compose version &>/dev/null; then
    COMPOSE_CMD="docker compose"
elif docker-compose version &>/dev/null; then
    COMPOSE_CMD="docker-compose"
else
    echo "Error: Docker Compose not found"
    exit 1
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_step() {
    echo -e "\n${BLUE}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

cleanup() {
    print_step "Cleaning up"
    $COMPOSE_CMD down -v 2>/dev/null || true
    docker system prune -f 2>/dev/null || true
}

# Trap cleanup on exit
trap cleanup EXIT

print_step "Testing Monitoring Stack Locally"

# Test 1: Validate docker-compose.yml
print_step "Validating docker-compose.yml syntax"
if $COMPOSE_CMD config > /dev/null 2>&1; then
    print_success "docker-compose.yml syntax is valid"
else
    print_error "docker-compose.yml syntax is invalid"
    exit 1
fi

# Test 2: Create test environment
print_step "Setting up test environment"
cat > .env << EOF
# Test environment variables
INFLUXDB_USER=testadmin
INFLUXDB_PASSWORD=testpassword123
INFLUXDB_ORG=testorg
INFLUXDB_BUCKET=testbucket
INFLUXDB_TOKEN=testtoken123456789abcdef
TELEGRAF_TOKEN=telegraf_test_token_123
GRAFANA_ADMIN_USER=testadmin
GRAFANA_ADMIN_PASSWORD=testgrafana123
HOST_DOCKER_GID=$(id -g docker 2>/dev/null || echo "999")
EOF

# Create telegraf config
mkdir -p "$HOME/telegraf"
cp config/telegraf/telegraf.conf.example "$HOME/telegraf/telegraf.conf"

print_success "Test environment created"

# Test 3: Start the stack
print_step "Starting monitoring stack"
$COMPOSE_CMD up -d

print_success "Stack started, waiting for services..."
sleep 30

# Test 4: Check container status
print_step "Checking container status"
$COMPOSE_CMD ps

RUNNING_CONTAINERS=$($COMPOSE_CMD ps --services --filter "status=running" | wc -l)
EXPECTED_CONTAINERS=4  # influxdb, telegraf, grafana, prometheus

if [ "$RUNNING_CONTAINERS" -ne "$EXPECTED_CONTAINERS" ]; then
    print_error "Expected $EXPECTED_CONTAINERS containers running, but found $RUNNING_CONTAINERS"
    $COMPOSE_CMD logs
    exit 1
fi

print_success "All expected containers are running"

# Test 5: Health checks
print_step "Performing health checks"

# Function to check service health
check_service_health() {
    local service_name=$1
    local url=$2
    local max_attempts=${3:-18}
    
    for _ in $(seq 1 "$max_attempts"); do
        if curl -f "$url" > /dev/null 2>&1; then
            print_success "$service_name is healthy"
            return 0
        fi
        sleep 10
    done
    
    print_error "$service_name failed to become healthy"
    return 1
}

# Check each service
check_service_health "InfluxDB" "http://localhost:8086/health" &
INFLUXDB_PID=$!

check_service_health "Grafana" "http://localhost:3000/api/health" &
GRAFANA_PID=$!

check_service_health "Prometheus" "http://localhost:9090/-/healthy" &
PROMETHEUS_PID=$!

# Wait for all health checks
wait $INFLUXDB_PID || exit 1
wait $GRAFANA_PID || exit 1
wait $PROMETHEUS_PID || exit 1

# Check Telegraf (no health endpoint)
if $COMPOSE_CMD ps telegraf | grep -q "Up"; then
    print_success "Telegraf is running"
else
    print_error "Telegraf is not running"
    exit 1
fi

# Test 6: Service endpoints
print_step "Testing service endpoints"

# Test InfluxDB
print_success "InfluxDB health: $(curl -s http://localhost:8086/health)"

# Test Grafana
print_success "Grafana health: $(curl -s http://localhost:3000/api/health)"

# Test Prometheus
PROMETHEUS_STATUS=$(curl -s http://localhost:9090/-/healthy)
print_success "Prometheus health: $PROMETHEUS_STATUS"

# Test Telegraf metrics (optional)
if curl -f http://localhost:9273/metrics > /dev/null 2>&1; then
    print_success "Telegraf metrics endpoint is accessible"
else
    print_warning "Telegraf metrics endpoint not accessible (may be expected)"
fi

# Test 7: Integration tests
print_step "Running integration tests"

# Test Grafana authentication
if curl -f -u testadmin:testgrafana123 http://localhost:3000/api/org > /dev/null 2>&1; then
    print_success "Grafana authentication works"
else
    print_warning "Grafana authentication test failed"
fi

# Test 8: Configuration validation
print_step "Validating configurations"

# Test Telegraf config
if command -v telegraf > /dev/null 2>&1; then
    if telegraf --config "$HOME/telegraf/telegraf.conf" --test > /dev/null 2>&1; then
        print_success "Telegraf configuration is valid"
    else
        print_warning "Telegraf configuration test failed (may need environment variables)"
    fi
else
    print_warning "Telegraf not installed locally, skipping config validation"
fi

# Test 9: Show service logs (for debugging)
print_step "Service logs summary"
echo "Recent logs from each service:"

echo -e "\n${YELLOW}InfluxDB logs:${NC}"
$COMPOSE_CMD logs --tail=5 influxdb

echo -e "\n${YELLOW}Telegraf logs:${NC}"
$COMPOSE_CMD logs --tail=5 telegraf

echo -e "\n${YELLOW}Grafana logs:${NC}"
$COMPOSE_CMD logs --tail=5 grafana

echo -e "\n${YELLOW}Prometheus logs:${NC}"
$COMPOSE_CMD logs --tail=5 prometheus

print_step "Test Summary"
print_success "All monitoring stack tests passed!"
echo
echo "Services are accessible at:"
echo "- Grafana: http://localhost:3000 (testadmin/testgrafana123)"
echo "- InfluxDB: http://localhost:8086"
echo "- Prometheus: http://localhost:9090"
echo
echo "Run '$COMPOSE_CMD logs -f' to view live logs"
echo "Run '$COMPOSE_CMD down -v' to stop and clean up"
