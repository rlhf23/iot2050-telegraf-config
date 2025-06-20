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
    exit 1
}

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Set up local environment for the monitoring stack.

OPTIONS:
    --no-docker-compose     Skip Docker Compose installation
    --no-user-setup         Skip user group setup
    -h, --help              Show this help message

EXAMPLES:
    $0
    $0 --no-docker-compose

REQUIREMENTS:
    - Running Debian-based system
    - Sudo privileges
EOF
}

# Default values
INSTALL_DOCKER_COMPOSE=true
SETUP_USER_GROUPS=true

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
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
            print_error "Unexpected argument: $1"
            show_usage
            exit 1
            ;;
    esac
done

print_status "Starting local environment setup"

# Check if running as root
if [ "$(id -u)" -eq 0 ]; then
    print_warning "Running as root is not recommended. Please run as a regular user with sudo access."
    read -r -p "Continue anyway? [y/N] " response
    if [[ ! "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
        exit 1
    fi
fi

# Check if device is Debian/Armbian based
print_status "Checking operating system..."
if [ -f /etc/os-release ]; then
    OS_INFO=$(cat /etc/os-release)
    if echo "$OS_INFO" | grep -qi "debian\|ubuntu\|armbian"; then
        print_success "Detected compatible OS (Debian-based)"
    else
        print_warning "OS detection unclear. Proceeding with Debian/Armbian assumptions."
        echo "Detected OS info:"
        echo "$OS_INFO"
    fi
else
    print_warning "Could not detect OS. Proceeding with Debian/Armbian assumptions."
fi

# Update package lists
print_status "Updating package lists..."
sudo apt-get update || {
    print_error "Failed to update package lists"
}
print_success "Package lists updated"

# Install required packages
print_status "Installing required packages..."
sudo apt-get install -y \
    apt-transport-https \
    ca-certificates \
    curl \
    gnupg \
    lsb-release \
    git \
    openssl || {
    print_error "Failed to install required packages"
}
print_success "Required packages installed"

# Install Docker
print_status "Checking Docker installation..."
if command -v docker &> /dev/null; then
    print_success "Docker is already installed"
    docker --version
else
    print_status "Installing Docker..."
    
    # Add Docker's official GPG key
    sudo install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/debian/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    sudo chmod a+r /etc/apt/keyrings/docker.gpg
    
    # Set up the stable repository
    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian \
      $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
      sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
    
    # Update package lists and install Docker
    sudo apt-get update
    sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin || {
        print_error "Failed to install Docker"
    }
    
    print_success "Docker installed successfully"
    docker --version
fi

# Install Docker Compose
if [ "$INSTALL_DOCKER_COMPOSE" = true ]; then
    print_status "Checking Docker Compose installation..."
    if command -v docker-compose &> /dev/null; then
        print_success "Docker Compose is already installed"
        docker-compose --version
    else
        print_status "Installing Docker Compose..."
        
        # Install Docker Compose from GitHub releases
        COMPOSE_VERSION=$(curl -s https://api.github.com/repos/docker/compose/releases/latest | grep 'tag_name' | cut -d '"' -f 4)
        sudo curl -L "https://github.com/docker/compose/releases/download/${COMPOSE_VERSION}/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
        sudo chmod +x /usr/local/bin/docker-compose
        
        # Verify installation
        if ! command -v docker-compose &> /dev/null; then
            print_error "Failed to install Docker Compose"
        fi
        
        print_success "Docker Compose installed successfully"
        docker-compose --version
    fi
fi

# Add user to docker group if needed
if [ "$SETUP_USER_GROUPS" = true ]; then
    print_status "Setting up user groups..."
    if ! groups | grep -q "\bdocker\b"; then
        print_status "Adding current user to docker group..."
        sudo usermod -aG docker "$USER" || {
            print_warning "Failed to add user to docker group"
        }
        print_success "User added to docker group. You may need to log out and back in for this to take effect."
    else
        print_success "User is already in the docker group"
    fi
fi

print_success "Local environment setup complete!"
echo ""
echo "Next steps:"
echo "1. Log out and back in if you were added to the docker group"
echo "2. Run './deploy_local.sh' to deploy the monitoring stack"
echo ""
