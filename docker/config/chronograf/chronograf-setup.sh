#!/bin/sh
set -e

command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is required but not found"; exit 1; }

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

EXISTING_NAMES=$(echo "$EXISTING_DASHBOARDS" | jq -r '.dashboards[]?.name // empty' 2>/dev/null || echo "$EXISTING_DASHBOARDS" | grep -o '"name":"[^"]*"' | sed 's/"name":"\([^"]*\)"/\1/')

echo "Existing dashboards: $(echo "$EXISTING_NAMES" | tr '\n' ', ' | sed 's/,$//')"

echo ""
echo "Importing dashboards..."

for dashboard_file in "${DASHBOARDS_DIR}"/*.json; do
    if [ ! -f "$dashboard_file" ]; then
        echo "No dashboard files found in ${DASHBOARDS_DIR}"
        continue
    fi

    # Extract dashboard name from file
    DASHBOARD_NAME=$(jq -r 'if has("dashboard") then .dashboard.name else .name end // empty' "$dashboard_file" 2>/dev/null || echo "")

    if [ -z "$DASHBOARD_NAME" ]; then
        DASHBOARD_NAME=$(grep -o '"name"[[:space:]]*:[[:space:]]*"[^"]*"' "$dashboard_file" | head -1 | sed 's/"name"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')
    fi

    echo "Processing: ${DASHBOARD_NAME} ($(basename "$dashboard_file"))"

    # Substitute environment variables in the dashboard JSON
    PROCESSED_FILE=$(mktemp)
    sed -e "s/\${INFLUXDB_DIAGNOSTICS_BUCKET}/$INFLUXDB_DIAGNOSTICS_BUCKET/g" \
         -e "s/\${INFLUXDB_BUCKET}/$INFLUXDB_BUCKET/g" \
         < "$dashboard_file" > "$PROCESSED_FILE"
    trap "rm -f $PROCESSED_FILE" EXIT

    # Check if dashboard already exists by name
    if echo "$EXISTING_NAMES" | grep -qF "$DASHBOARD_NAME"; then
        echo "  Skipping: dashboard '${DASHBOARD_NAME}' already exists"
        continue
    fi

    # The Chronograf REST API POST /chronograf/v1/dashboards expects just
    # the flat dashboard object, not the {meta, dashboard} export format.
    # Extract the dashboard portion and populate query source fields.
    DASHBOARD_JSON=$(jq --arg source "$SOURCE_URL" '
      if has("dashboard") and has("meta") then .dashboard else . end
      | del(.id, .links, (.cells[].links))
      | .cells[].queries[].source = $source
    ' "$PROCESSED_FILE" 2>/dev/null || echo "")

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