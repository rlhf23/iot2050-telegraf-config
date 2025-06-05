#!/bin/sh
set -e

# Stop and remove any existing container
echo "Cleaning up existing container..."
docker rm -f iot2050-test-container 2>/dev/null || true

# Build the minimal test image
echo "Building minimal test container image..."
docker build -t iot2050-test-env -f tests/Dockerfile.minimal .

# Run the container with exposed SSH port
echo "Starting test container..."
docker run -d \
  --name iot2050-test-container \
  -p 2222:22 \
  iot2050-test-env

# ---

# Wait for container to be ready
echo "Waiting for container to start..."
sleep 3

# Start telegraf and influxdb services
echo "Starting telegraf and influxdb services..."
docker exec iot2050-test-container sudo service telegraf start
docker exec iot2050-test-container sudo service influxdb start

# ---

echo "Container started. You can now test SSH with:"
echo "ssh -p 2222 testuser@localhost"
echo "Password: testpass"
