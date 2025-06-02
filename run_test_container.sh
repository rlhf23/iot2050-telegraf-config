#!/bin/sh
set -e

# Stop and remove any existing container
echo "Cleaning up existing container..."
docker rm -f iot2050-test-container 2>/dev/null || true

# Build the minimal test image
echo "Building minimal test container image..."
docker build -t iot2050-test-env -f tests/Dockerfile.test .

# Run the container with exposed SSH port
echo "Starting test container..."
docker run -d \
  --name iot2050-test-container \
  -p 2222:22 \
  iot2050-test-env

echo "Container started. You can now test SSH with:"
echo "ssh -p 2222 testuser@localhost"
echo "Password: testpass"
echo ""
echo "Or run the test with:"
echo "cargo test --test smoke_test -- --nocapture"
