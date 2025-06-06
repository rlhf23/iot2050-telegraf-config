#!/usr/bin/env bash
set -e

CONTAINER_NAME="iot2050-test"
CONFIG_FILE="./container.nix"

case "${1:-start}" in
  start)
    echo "Cleaning up existing container..."
    sudo nixos-container stop $CONTAINER_NAME 2>/dev/null || true
    sudo nixos-container destroy $CONTAINER_NAME 2>/dev/null || true
    
    echo "Creating NixOS container..."
    sudo nixos-container create $CONTAINER_NAME --config-file $CONFIG_FILE
    
    echo "Starting container..."
    sudo nixos-container start $CONTAINER_NAME
    
    # Wait for container to be ready
    echo "Waiting for container to start..."
    sleep 3
    
    # Get container IP
    IP=$(nixos-container show-ip $CONTAINER_NAME)
    echo "Container started at IP: $IP"
    echo ""
    echo "Services should be running automatically (telegraf, influxdb2, ssh)"
    echo "You can now test SSH with:"
    echo "ssh testuser@$IP"
    echo "Password: testpass"
    ;;
    
  stop)
    echo "Stopping and removing container..."
    sudo nixos-container stop $CONTAINER_NAME
    sudo nixos-container destroy $CONTAINER_NAME
    echo "Container removed."
    ;;
    
  restart)
    $0 stop
    $0 start
    ;;
    
  ip)
    nixos-container show-ip $CONTAINER_NAME
    ;;
    
  ssh)
    IP=$(nixos-container show-ip $CONTAINER_NAME)
    ssh testuser@$IP
    ;;
    
  status)
    sudo nixos-container status $CONTAINER_NAME
    ;;
    
  shell)
    # Root shell into container for debugging
    sudo nixos-container root-login $CONTAINER_NAME
    ;;
    
  *)
    echo "Usage: $0 {start|stop|restart|ip|ssh|status|shell}"
    echo ""
    echo "Commands:"
    echo "  start   - Create and start the test container"
    echo "  stop    - Stop and destroy the container"  
    echo "  restart - Stop and start the container"
    echo "  ip      - Show container IP address"
    echo "  ssh     - SSH into container as testuser"
    echo "  status  - Show container status"
    echo "  shell   - Root shell into container"
    ;;
esac