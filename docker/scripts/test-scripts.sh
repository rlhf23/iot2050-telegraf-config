#!/bin/bash
# Test scripts in their actual container environments

set -e

echo "🧪 Testing scripts in container environments..."

# Test system-info.sh in nginx:alpine container
echo ""
echo "Testing system-info.sh in nginx:alpine..."
docker run --rm \
  -v $(pwd)/config/nginx/scripts/system-info.sh:/test/system-info.sh:ro \
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
  -v $(pwd)/config/nginx/scripts/system-info.sh:/test/system-info.sh:ro \
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
