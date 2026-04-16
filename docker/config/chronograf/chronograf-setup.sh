#!/bin/sh
set -e

CHRONOGRAF_URL="http://chronograf:8888"
INFLUXDB_URL="http://influxdb:8086"
DASHBOARDS_DIR="/dashboards"
SOURCE_ID=""

echo "=== Chronograf Setup ==="

echo "Waiting for Chronograf to be ready..."
MAX_RETRIES=60
RETRY=0
until curl -sf -o /dev/null "${CHRONOGRAF_URL}/health" 2>/dev/null; do
    RETRY=$((RETRY + 1))
    if [ $RETRY -ge $MAX_RETRIES ]; then
        echo "ERROR: Chronograf did not become ready within ${MAX_RETRIES} seconds"
        exit 1
    fi
    echo "  Chronograf not ready yet... (attempt ${RETRY}/${MAX_RETRIES})"
    sleep 1
done
echo "Chronograf is healthy."

echo "Checking for existing InfluxDB source..."
EXISTING_SOURCES=$(curl -sf "${CHRONOGRAF_URL}/chronograf/v1/sources" 2>/dev/null || echo '{"sources":[]}')

if echo "$EXISTING_SOURCES" | grep -q "\"id\""; then
    SOURCE_ID=$(echo "$EXISTING_SOURCES" | grep -o '"id":"[^"]*"' | head -1 | sed 's/"id":"\([^"]*\)"/\1/')
    echo "Found existing source with id: ${SOURCE_ID}"
else
    echo "Creating InfluxDB v2 source..."
    SOURCE_RESPONSE=$(curl -sf -X POST -d "{
        \"name\": \"InfluxDB v2\",
        \"type\": \"influx-v2\",
        \"url\": \"${INFLUXDB_URL}\",
        \"username\": \"${INFLUXDB_USER}\",
        \"password\": \"${INFLUXDB_PASSWORD}\",
        \"token\": \"${INFLUXDB_TOKEN}\",
        \"organization\": \"${INFLUXDB_ORG}\",
        \"default\": true
    }" -H "Content-Type: application/json" "${CHRONOGRAF_URL}/chronograf/v1/sources" 2>/dev/null || echo '{}')

    SOURCE_ID=$(echo "$SOURCE_RESPONSE" | grep -o '"id":"[^"]*"' | head -1 | sed 's/"id":"\([^"]*\)"/\1/')

    if [ -z "$SOURCE_ID" ]; then
        echo "WARN: Could not parse source ID from response. Attempting alternate extraction..."
        SOURCE_ID=$(echo "$SOURCE_RESPONSE" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p' | head -1)
    fi

    if [ -z "$SOURCE_ID" ]; then
        echo "ERROR: Failed to create InfluxDB source. Response:"
        echo "$SOURCE_RESPONSE"
        echo "Will attempt to create dashboards with empty source reference."
    else
        echo "Created InfluxDB source with id: ${SOURCE_ID}"
    fi
fi

SOURCE_URL="${INFLUXDB_URL}"
if [ -n "$SOURCE_ID" ]; then
    SOURCE_URL="${CHRONOGRAF_URL}/chronograf/v1/sources/${SOURCE_ID}"
fi

echo ""
echo "Fetching existing dashboards to avoid duplicates..."
EXISTING_DASHBOARDS=$(curl -sf "${CHRONOGRAF_URL}/chronograf/v1/dashboards" 2>/dev/null || echo '{"dashboards":[]}')

EXISTING_NAMES=$(echo "$EXISTING_DASHBOARDS" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    for d in data.get('dashboards', []):
        print(d.get('name', ''))
except: pass
" 2>/dev/null)

if [ -z "$EXISTING_NAMES" ]; then
    EXISTING_NAMES=$(echo "$EXISTING_DASHBOARDS" | grep -o '"name":"[^"]*"' | sed 's/"name":"\([^"]*\)"/\1/')
fi

echo "Existing dashboards: $(echo "$EXISTING_NAMES" | tr '\n' ', ' | sed 's/,$//')"

echo ""
echo "Importing dashboards..."

for dashboard_file in "${DASHBOARDS_DIR}"/*.json; do
    if [ ! -f "$dashboard_file" ]; then
        echo "No dashboard files found in ${DASHBOARDS_DIR}"
        continue
    fi

    # Extract dashboard name from file
    DASHBOARD_NAME=$(python3 -c "
import json
with open('$dashboard_file') as f:
    data = json.load(f)
if 'dashboard' in data and 'meta' in data:
    print(data['dashboard'].get('name', ''))
else:
    print(data.get('name', ''))
" 2>/dev/null || echo "")

    if [ -z "$DASHBOARD_NAME" ]; then
        DASHBOARD_NAME=$(grep -o '"name"[[:space:]]*:[[:space:]]*"[^"]*"' "$dashboard_file" | head -1 | sed 's/"name"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')
    fi

    echo "Processing: ${DASHBOARD_NAME} ($(basename "$dashboard_file"))"

    # Check if dashboard already exists by name
    if echo "$EXISTING_NAMES" | grep -qF "$DASHBOARD_NAME"; then
        echo "  Skipping: dashboard '${DASHBOARD_NAME}' already exists"
        continue
    fi

    # The Chronograf REST API POST /chronograf/v1/dashboards expects just
    # the flat dashboard object, not the {meta, dashboard} export format.
    # Extract the dashboard portion and populate query source fields.
    DASHBOARD_JSON=$(python3 -c "
import json, sys
with open('$dashboard_file') as f:
    data = json.load(f)
if 'dashboard' in data and 'meta' in data:
    dashboard = data['dashboard']
else:
    dashboard = data
source_url = '${SOURCE_URL}'
if source_url:
    for cell in dashboard.get('cells', []):
        for query in cell.get('queries', []):
            query['source'] = source_url
# Remove server-assigned fields that should not be sent on creation
dashboard.pop('id', None)
dashboard.pop('links', None)
for cell in dashboard.get('cells', []):
    cell.pop('links', None)
print(json.dumps(dashboard))
" 2>/dev/null || echo "")

    # Fallback: if python3 is not available, try with jq
    if [ -z "$DASHBOARD_JSON" ]; then
        echo "  python3 not available, trying jq..."
        HAS_META=$(cat "$dashboard_file" | jq 'has("meta")' 2>/dev/null || echo "false")
        if [ "$HAS_META" = "true" ]; then
            DASHBOARD_JSON=$(cat "$dashboard_file" | jq '.dashboard' 2>/dev/null || echo "")
        else
            DASHBOARD_JSON=$(cat "$dashboard_file")
        fi
        if [ -n "$DASHBOARD_JSON" ] && [ -n "$SOURCE_URL" ]; then
            DASHBOARD_JSON=$(echo "$DASHBOARD_JSON" | sed "s|\"source\":\"\"|\"source\":\"${SOURCE_URL}\"|g")
        fi
    fi

    # Last fallback: send the raw file (may not work for {meta,dashboard} format)
    if [ -z "$DASHBOARD_JSON" ]; then
        echo "  WARN: python3 and jq not available, sending raw JSON (may not import correctly)"
        DASHBOARD_JSON=$(cat "$dashboard_file")
    fi

    RESPONSE=$(echo "$DASHBOARD_JSON" | curl -sf -X POST -d @- \
        -H "Content-Type: application/json" \
        "${CHRONOGRAF_URL}/chronograf/v1/dashboards" 2>/dev/null || echo '{}')

    if echo "$RESPONSE" | grep -q '"id"'; then
        echo "  Dashboard imported successfully."
    else
        echo "  WARN: Dashboard import may have failed. Response:"
        echo "  $RESPONSE" | head -c 200
        echo ""
    fi
done

echo ""
echo "=== Chronograf setup complete ==="