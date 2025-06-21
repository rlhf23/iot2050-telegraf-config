#!/usr/bin/env bash
set -e

cd "$(dirname "$0")/.."

echo "🛑 Stopping monitoring stack..."

# Load environment variables if .env exists
if [ -f .env ]; then
    set -a
    source .env
    set +a
fi

echo "Stopping containers..."
docker-compose -f docker-compose.base.yml -f docker-compose.linux.yml down

echo "✅ Stack stopped successfully!"
