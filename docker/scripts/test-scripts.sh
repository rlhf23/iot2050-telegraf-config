#!/bin/bash
# Test scripts in their actual container environments

set -e

# Check for required dependencies
if ! command -v jq &> /dev/null; then
    echo "❌ Error: jq is required but not installed"
    echo "Install with: sudo apt-get install jq (Ubuntu/Debian) or brew install jq (macOS)"
    exit 1
fi

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}").." && pwd)"

echo "🧪 Testing scripts in container environments..."

# Test system-info.sh in nginx:alpine container
echo ""
echo "Testing system-info.sh in nginx:alpine..."
docker run --rm \
  -v "$SCRIPT_DIR/config/nginx/scripts/system-info.sh:/test/system-info.sh:ro" \
  -v /proc:/host/proc:ro \
  -v /sys:/host/sys:ro \
  nginx:alpine \
  sh -c "chmod +x /test/system-info.sh && /test/system-info.sh"

if [ $? -eq 0 ]; then
  echo "✅ system-info.sh works in nginx:alpine"
else
  echo "❌ system-info.sh failed in nginx:alpine"
  exit 1
fi

# Validate JSON output
echo ""
echo "Validating JSON output..."
docker run --rm \
  -v "$SCRIPT_DIR/config/nginx/scripts/system-info.sh:/test/system-info.sh:ro" \
  -v /proc:/host/proc:ro \
  -v /sys:/host/sys:ro \
  nginx:alpine \
  sh -c "chmod +x /test/system-info.sh && /test/system-info.sh | jq ."

if [ $? -eq 0 ]; then
  echo "✅ JSON output is valid"
else
  echo "❌ JSON output is invalid"
  exit 1
fi

echo ""
echo "✅ All script tests passed!"
