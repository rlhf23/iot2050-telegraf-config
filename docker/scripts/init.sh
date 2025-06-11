#!/usr/bin/env bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}🔧 $1${NC}"
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

Provision a new IoT device with Docker and monitoring stack requirements.

OPTIONS:
    -u, --user USERNAME     SSH username (default: admin)
    -p, --port PORT         SSH port (default: 22)
    -k, --key PATH          SSH private key path (optional)
    --no-docker-compose     Skip Docker Compose installation
    --no-user-setup         Skip user group setup
    -h, --help              Show this help message

EXAMPLES:
    $0 192.168.1.100
    $0 -u iot2050 -p 2222 192.168.1.100
    $0 --key ~/.ssh/iot_key 192.168.1.100

REQUIREMENTS:
    - Target device running Debian or Armbian
    - SSH access to the device
    - sudo privileges on the device
EOF
}

# Default values
SSH_USER="admin"
SSH_PORT="22"
SSH_KEY=""
INSTALL_DOCKER_COMPOSE=true
SETUP_USER_GROUPS=true

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
        --no-docker-compose)
            INSTALL_DOCKER_COMPOSE=false
            shift
            ;;
        --no-user-setup)
            SETUP_USER_GROUPS=false
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

# Validate required arguments
if [ -z "$DEVICE_IP" ]; then
    print_error "Device IP address is required"
    show_usage
    exit 1
fi

# Build SSH command
SSH_CMD="ssh"
if [ -n "$SSH_KEY" ]; then
    SSH_CMD="$SSH_CMD -i $SSH_KEY"
fi
SSH_CMD="$SSH_CMD -p $SSH_PORT $SSH_USER@$DEVICE_IP"

print_status "Starting provisioning of IoT device at $DEVICE_IP"
print_status "SSH: $SSH_USER@$DEVICE_IP:$SSH_PORT"

# Test SSH connectivity
print_status "Testing SSH connectivity..."
if ! $SSH_CMD "echo 'SSH connection successful'" > /dev/null 2>&1; then
    print_error "Failed to connect via SSH. Please check:"
    echo "  - Device IP: $DEVICE_IP"
    echo "  - SSH user: $SSH_USER"
    echo "  - SSH port: $SSH_PORT"
    echo "  - SSH key: ${SSH_KEY:-"(using default)"}"
    echo "  - Device is powered on and accessible"
    exit 1
fi
print_success "SSH connectivity verified"

# Check if device is Debian/Armbian based
print_status "Checking operating system..."
OS_INFO=$($SSH_CMD "cat /etc/os-release 2>/dev/null || echo 'UNKNOWN'")
if echo "$OS_INFO" | grep -qi "debian\|ubuntu\|armbian"; then
    print_success "Detected compatible OS (Debian-based)"
else
    print_warning "OS detection unclear. Proceeding with Debian/Armbian assumptions."
    echo "Detected OS info:"
    echo "$OS_INFO"
fi

# Update package lists
print_status "Updating package lists..."
$SSH_CMD "sudo apt-get update" || {
    print_error "Failed to update package lists"
    exit 1
}
print_success "Package lists updated"

# Install required packages
print_status "Installing required packages..."
$SSH_CMD "sudo apt-get install -y \
    apt-transport-https \
    ca-certificates \
    curl \
    gnupg \
    lsb-release \
    git \
    openssl" || {
    print_error "Failed to install required packages"
    exit 1
}
print_success "Required packages installed"

# Install Docker
print_status "Checking Docker installation..."
if $SSH_CMD "docker --version" > /dev/null 2>&1; then
    print_success "Docker is already installed"
    $SSH_CMD "docker --version"
else
    print_status "Installing Docker..."
    
    # Add Docker's official GPG key
    $SSH_CMD "curl -fsSL https://download.docker.com/linux/debian/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg"
    
    # Set up the stable repository
    $SSH_CMD "echo \"deb [arch=\$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/debian \$(lsb_release -cs) stable\" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null"
    
    # Update package lists and install Docker
    $SSH_CMD "sudo apt-get update"
    $SSH_CMD "sudo apt-get install -y docker-ce docker-ce-cli containerd.io" || {
        print_error "Failed to install Docker"
        exit 1
    }
    
    print_success "Docker installed successfully"
fi

# Install Docker Compose
if [ "$INSTALL_DOCKER_COMPOSE" = true ]; then
    print_status "Checking Docker Compose installation..."
    if $SSH_CMD "docker-compose --version" > /dev/null 2>&1; then
        print_success "Docker Compose is already installed"
        $SSH_CMD "docker-compose --version"
    else
        print_status "Installing Docker Compose..."
        
        # Get the latest version (or use a stable version)
        COMPOSE_VERSION="v2.24.1"
        ARCH=$($SSH_CMD "uname -m")
        
        $SSH_CMD "sudo curl -L \"https://github.com/docker/compose/releases/download/${COMPOSE_VERSION}/docker-compose-\$(uname -s)-${ARCH}\" -o /usr/local/bin/docker-compose" || {
            print_error "Failed to download Docker Compose"
            exit 1
        }
        
        $SSH_CMD "sudo chmod +x /usr/local/bin/docker-compose" || {
            print_error "Failed to make Docker Compose executable"
            exit 1
        }
        
        print_success "Docker Compose installed successfully"
    fi
fi

# Set up user permissions
if [ "$SETUP_USER_GROUPS" = true ]; then
    print_status "Setting up user permissions..."
    
    # Add user to docker group
    $SSH_CMD "sudo usermod -aG docker $SSH_USER" || {
        print_warning "Failed to add user to docker group (user might already be in group)"
    }
    
    print_success "User permissions configured"
fi

# Start and enable Docker service
print_status "Starting Docker service..."
$SSH_CMD "sudo systemctl start docker"
$SSH_CMD "sudo systemctl enable docker"
print_success "Docker service started and enabled"

# Create monitoring directory
print_status "Creating monitoring directory..."
$SSH_CMD "mkdir -p ~/monitoring"
print_success "Monitoring directory created"

# Verify installation
print_status "Verifying installation..."
DOCKER_VERSION=$($SSH_CMD "docker --version")
print_success "Docker: $DOCKER_VERSION"

if [ "$INSTALL_DOCKER_COMPOSE" = true ]; then
    COMPOSE_VERSION=$($SSH_CMD "docker-compose --version")
    print_success "Docker Compose: $COMPOSE_VERSION"
fi

# Test Docker functionality (requires new shell session for group membership)
print_status "Testing Docker functionality..."
if $SSH_CMD "newgrp docker << EOF
docker run --rm hello-world > /dev/null 2>&1
EOF"; then
    print_success "Docker is working correctly"
else
    print_warning "Docker test failed. You may need to:"
    echo "  1. Restart the SSH session"
    echo "  2. Reboot the device"
    echo "  3. Manually test with: docker run --rm hello-world"
fi

print_success "🎉 Device provisioning completed successfully!"
echo
echo -e "${BLUE}📋 Next Steps:${NC}"
echo "  1. Use deploy.sh to build and deploy the monitoring stack"
echo "  2. Or manually deploy with:"
echo "     scp -r docker/ $SSH_USER@$DEVICE_IP:~/monitoring/"
echo "     ssh $SSH_USER@$DEVICE_IP 'cd ~/monitoring && ./scripts/setup.sh && ./scripts/start.sh'"
echo
echo -e "${BLUE}🔗 Access URLs (after deployment):${NC}"
echo "  - Grafana: http://$DEVICE_IP:3000"
echo "  - InfluxDB: http://$DEVICE_IP:8086"
