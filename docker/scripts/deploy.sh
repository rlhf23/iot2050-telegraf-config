#!/usr/bin/env bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}🚀 $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Build and deploy the monitoring stack locally.

OPTIONS:
    --no-build      Skip building images (use existing)
    --build-only    Only build images, don't start the stack
    --force-rebuild Force rebuild even if images exist
    -h, --help      Show this help message

EXAMPLES:
    $0
    $0 --no-build
    $0 --build-only

REQUIREMENTS:
    - Docker and Docker Compose installed
    - Local environment initialized with init.sh
EOF
}

# Default values
BUILD_IMAGES=true
BUILD_ONLY=false
FORCE_REBUILD=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-build)
            BUILD_IMAGES=false
            shift
            ;;
        --build-only)
            BUILD_ONLY=true
            shift
            ;;
        --force-rebuild)
            FORCE_REBUILD=true
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        -*)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
        *)
            print_error "Unexpected argument: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Change to the script's directory (docker/)
cd "$(dirname "$0")/.."

# Verify we're in the right directory
if [ ! -f "docker-compose.yml" ]; then
    print_error "docker-compose.yml not found. Are you in the correct directory?"
    exit 1
fi

print_status "Starting local deployment process"

# Build images locally
if [ "$BUILD_IMAGES" = true ]; then
    print_status "Building Docker images locally..."
    
    # Check if images already exist and force rebuild is not set
    if [ "$FORCE_REBUILD" = false ]; then
        existing_images=$(docker images --filter "reference=*influxdb*" --filter "reference=*telegraf*" --filter "reference=*grafana*" -q | wc -l)
        if [ "$existing_images" -gt 0 ]; then
            print_warning "Some images already exist. Use --force-rebuild to rebuild them."
            echo "Existing images:"
            docker images --filter "reference=*influxdb*" --filter "reference=*telegraf*" --filter "reference=*grafana*" --format "table {{.Repository}}\t{{.Tag}}\t{{.CreatedAt}}"
            read -r -p "Continue with existing images? (Y/n): " continue_existing
            if [[ "$continue_existing" =~ ^[Nn]$ ]]; then
                print_error "Deployment cancelled"
                exit 1
            fi
            BUILD_IMAGES=false
        fi
    fi
    
    if [ "$BUILD_IMAGES" = true ]; then
        # Enable ARM emulation if not on ARM64
        if [ "$(uname -m)" != "aarch64" ]; then
            print_status "Enabling ARM emulation for cross-platform builds..."
            if ! docker run --privileged --rm tonistiigi/binfmt --install all; then
                print_warning "Failed to enable ARM emulation (this might be expected in some environments)"
            fi
        fi
        
        # Build images with docker-compose
        docker-compose build || {
            print_error "Failed to build Docker images"
            exit 1
        }
        
        print_success "Docker images built successfully"
    fi
fi

# Exit if build-only mode
if [ "$BUILD_ONLY" = true ]; then
    print_success "Build complete. Exiting due to --build-only flag."
    exit 0
fi

# Start the stack (with profile if set)
print_status "Starting the monitoring stack..."

if [ -f .env ]; then
    # Load COMPOSE_PROFILE if defined
    COMPOSE_PROFILE=$(grep -E '^COMPOSE_PROFILE=' .env 2>/dev/null | cut -d'=' -f2 || echo "")
fi

PROFILE_ARGS=""
if [ -n "$COMPOSE_PROFILE" ]; then
    for profile in $COMPOSE_PROFILE; do
        PROFILE_ARGS="$PROFILE_ARGS --profile $profile"
    done
    print_status "Using profile(s): $COMPOSE_PROFILE"
fi

docker-compose $PROFILE_ARGS up -d || {
    print_error "Failed to start the monitoring stack"
    exit 1
}

# Wait for all running containers to be healthy or exited (timeout after 60 seconds)
timeout=60
interval=2
elapsed=0

print_status "Waiting for all containers to be healthy..."

while true; do
    unhealthy=$(docker ps --filter "health=unhealthy" --format '{{.Names}}')
    starting=$(docker ps --filter "health=starting" --format '{{.Names}}')
    if [ -z "$unhealthy" ] && [ -z "$starting" ]; then
        break
    fi
    sleep $interval
    elapsed=$((elapsed + interval))
    if [ $elapsed -ge $timeout ]; then
        print_warning "Timeout waiting for containers to become healthy."
        break
    fi
done

print_success "Deployment completed successfully!"
echo ""
echo "📊 InfluxDB: http://localhost:8086"
echo "📊 Chronograf: http://localhost:8888"
if echo "$COMPOSE_PROFILE" | grep -qw "full"; then
    echo "📊 Grafana: http://localhost:3000"
    echo "📊 Prometheus: http://localhost:9090"
fi
echo ""
echo "To stop the stack, run: ./stop.sh"
echo "To view logs, run: docker-compose logs -f"
echo ""
