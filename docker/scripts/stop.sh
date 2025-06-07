#!/bin/bash
set -e

cd "$(dirname "$0")/../.."

echo "🛑 Stopping monitoring stack..."

docker-compose -f docker/docker-compose.yml down

echo "✅ Stack stopped successfully!"
