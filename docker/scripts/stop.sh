#!/usr/bin/env bash
set -e

cd "$(dirname "$0")/.."

echo "🛑 Stopping monitoring stack with graceful shutdown..."

# Stop containers with 30 second timeout for graceful shutdown
# This allows OPC UA connections and other resources to close properly
docker-compose -f docker-compose.yml down --timeout 30

echo "✅ Stack stopped successfully!"
