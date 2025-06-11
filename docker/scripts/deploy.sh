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
Usage: $0 [OPTIONS] <device_ip>

Build the monitoring stack locally and deploy it to a provisioned IoT device.

OPTIONS:
    -u, --user USERNAME     SSH username (default: admin)
    -p, --port PORT         SSH port (default: 22)
    -k, --key PATH          SSH private key path (optional)
    --no-build              Skip building images locally (use existing)
    --no-transfer           Skip transferring files (use existing on device)
    --build-only            Only build images, don't deploy
    --force-rebuild         Force rebuild even if images exist
    -h, --help              Show this help message

EXAMPLES:
    $0 192.168.1.100
    $0 -u iot2050 -p 2222 192.168.1.100
    $0 --key ~/.ssh/iot_key --no-build 192.168.1.100
    $0 --build-only

REQUIREMENTS:
    - Docker and Docker Compose installed locally
    - Target device provisioned with init.sh
    - SSH access to the device
EOF
}

# Default values
SSH_USER="admin"
SSH_PORT="22"
SSH_KEY=""
BUILD_IMAGES=true
TRANSFER_FILES=true
BUILD_ONLY=false
FORCE_REBUILD=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -u|--user)
            SSH_USER="$2"
            shift 2
            ;;
        -p|--port)
            SSH_PORT="$2"
            shift 2
            ;;
        -k|--key)
            SSH_KEY="$2"
            shift 2
            ;;
        --no-build)
            BUILD_IMAGES=false
            shift
            ;;
        --no-transfer)
            TRANSFER_FILES=false
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
            if [ -z "$DEVICE_IP" ]; then
                DEVICE_IP="$1"
            else
                print_error "Multiple device IPs specified. Only one is allowed."
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate required arguments (unless build-only)
if [ "$BUILD_ONLY" = false ] && [ -z "$DEVICE_IP" ]; then
    print_error "Device IP address is required (unless using --build-only)"
    show_usage
    exit 1
fi

# Change to the script's directory (docker/)
cd "$(dirname "$0")/.."

# Verify we're in the right directory
if [ ! -f "docker-compose.yml" ]; then
    print_error "docker-compose.yml not found. Are you in the correct directory?"
    exit 1
fi

# Build SSH command
if [ "$BUILD_ONLY" = false ]; then
    SSH_CMD="ssh"
    if [ -n "$SSH_KEY" ]; then
        SSH_CMD="$SSH_CMD -i $SSH_KEY"
    fi
    SSH_CMD="$SSH_CMD -p $SSH_PORT $SSH_USER@$DEVICE_IP"
    
    SCP_CMD="scp"
    if [ -n "$SSH_KEY" ]; then
        SCP_CMD="$SCP_CMD -i $SSH_KEY"
    fi
    SCP_CMD="$SCP_CMD -P $SSH_PORT"
fi

print_status "Starting deployment process"
if [ "$BUILD_ONLY" = false ]; then
    print_status "Target: $SSH_USER@$DEVICE_IP:$SSH_PORT"
fi

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
    
    # Save images to tar file
    print_status "Saving images to tar file..."
    IMAGE_LIST=$(docker-compose config --images | tr '\n' ' ')
    if [ -z "$IMAGE_LIST" ]; then
        print_error "No images found in docker-compose.yml"
        exit 1
    fi
    
    echo "Images to save: $IMAGE_LIST"
    docker save $IMAGE_LIST -o monitoring-stack.tar || {
        print_error "Failed to save Docker images"
        exit 1
    }
    
    IMAGE_SIZE=$(du -h monitoring-stack.tar | cut -f1)
    print_success "Images saved to monitoring-stack.tar ($IMAGE_SIZE)"
fi

# Exit early if build-only
if [ "$BUILD_ONLY" = true ]; then
    print_success "Build completed! Use without --build-only to deploy."
    exit 0
fi

# Test SSH connectivity
print_status "Testing SSH connectivity..."
if ! $SSH_CMD "echo 'SSH connection successful'" > /dev/null 2>&1; then
    print_error "Failed to connect via SSH. Please check:"
    echo "  - Device IP: $DEVICE_IP"
    echo "  - SSH user: $SSH_USER"
    echo "  - SSH port: $SSH_PORT"
    echo "  - SSH key: ${SSH_KEY:-"(using default)"}"
    echo "  - Device is accessible and provisioned"
    exit 1
fi
print_success "SSH connectivity verified"

# Transfer files to device
if [ "$TRANSFER_FILES" = true ]; then
    print_status "Transferring files to device..."
    
    # Create remote monitoring directory
    $SSH_CMD "mkdir -p ~/monitoring"
    
    # Transfer the entire docker directory (excluding the tar file for now)
    print_status "Transferring docker configuration..."
    rsync -av --progress \
        --exclude="monitoring-stack.tar" \
        --exclude=".env" \
        -e "ssh ${SSH_KEY:+-i $SSH_KEY} -p $SSH_PORT" \
        ./ "$SSH_USER@$DEVICE_IP:~/monitoring/" || {
        print_error "Failed to transfer docker configuration"
        exit 1
    }
    
    # Transfer the images tar file separately (it's large)
    if [ -f "monitoring-stack.tar" ]; then
        print_status "Transferring Docker images archive (this may take a while)..."
        $SCP_CMD monitoring-stack.tar "$SSH_USER@$DEVICE_IP:~/" || {
            print_error "Failed to transfer Docker images"
            exit 1
        }
        print_success "Docker images transferred"
    else
        print_warning "monitoring-stack.tar not found. Skipping image transfer."
    fi
    
    print_success "Files transferred successfully"
fi

# Load images on remote device
if [ -f "monitoring-stack.tar" ]; then
    print_status "Loading Docker images on remote device..."
    $SSH_CMD "docker load -i ~/monitoring-stack.tar" || {
        print_error "Failed to load Docker images on remote device"
        exit 1
    }
    print_success "Docker images loaded on remote device"
fi

# Setup and start the monitoring stack
print_status "Setting up monitoring stack on device..."
$SSH_CMD "cd ~/monitoring && chmod +x scripts/*.sh" || {
    print_error "Failed to make scripts executable"
    exit 1
}

# Run setup
print_status "Running setup.sh on device..."
$SSH_CMD "cd ~/monitoring && ./scripts/setup.sh" || {
    print_error "Setup failed on remote device"
    exit 1
}
print_success "Setup completed on device"

# Start the stack
print_status "Starting monitoring stack on device..."
$SSH_CMD "cd ~/monitoring && ./scripts/start.sh" || {
    print_error "Failed to start monitoring stack"
    exit 1
}
print_success "Monitoring stack started successfully"

# Get service status
print_status "Checking service status..."
CONTAINER_STATUS=$($SSH_CMD "cd ~/monitoring && docker-compose ps --format table")
echo "$CONTAINER_STATUS"

# Get credentials for user
print_status "Retrieving credentials..."
CREDENTIALS=$($SSH_CMD "cd ~/monitoring && cat .env | grep -E '(GRAFANA_ADMIN_|INFLUXDB_)' | grep -E '(USER|PASSWORD|TOKEN)='")

print_success "🎉 Deployment completed successfully!"
echo
echo -e "${BLUE}📋 Service URLs:${NC}"
echo "  - Grafana: http://$DEVICE_IP:3000"
echo "  - InfluxDB: http://$DEVICE_IP:8086"
echo
echo -e "${BLUE}🔑 Credentials:${NC}"
echo "$CREDENTIALS"
echo
echo -e "${BLUE}🔧 Useful Commands:${NC}"
echo "  - View logs: ssh $SSH_USER@$DEVICE_IP 'cd ~/monitoring && docker-compose logs -f'"
echo "  - Stop stack: ssh $SSH_USER@$DEVICE_IP 'cd ~/monitoring && ./scripts/stop.sh'"
echo "  - Restart stack: ssh $SSH_USER@$DEVICE_IP 'cd ~/monitoring && ./scripts/start.sh'"
echo "  - Check status: ssh $SSH_USER@$DEVICE_IP 'cd ~/monitoring && docker-compose ps'"
echo
echo -e "${GREEN}✨ Happy monitoring!${NC}"
