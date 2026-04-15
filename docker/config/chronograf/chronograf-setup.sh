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
until wget -q -O /dev/null "${CHRONOGRAF_URL}/health" 2>/dev/null; do
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
EXISTING_SOURCES=$(wget -q -O - "${CHRONOGRAF_URL}/chronograf/v1/sources" 2>/dev/null || echo '{"sources":[]}')

if echo "$EXISTING_SOURCES" | grep -q "\"id\""; then
    SOURCE_ID=$(echo "$EXISTING_SOURCES" | grep -o '"id":"[^"]*"' | head -1 | sed 's/"id":"\([^"]*\)"/\1/')
    echo "Found existing source with id: ${SOURCE_ID}"
else
    echo "Creating InfluxDB v2 source..."
    SOURCE_RESPONSE=$(wget -q -O - --post-data="{
        \"name\": \"InfluxDB v2\",
        \"type\": \"influx-v2\",
        \"url\": \"${INFLUXDB_URL}\",
        \"username\": \"${INFLUXDB_USER}\",
        \"password\": \"${INFLUXDB_PASSWORD}\",
        \"token\": \"${INFLUXDB_TOKEN}\",
        \"organization\": \"${INFLUXDB_ORG}\",
        \"default\": true
    }" --header="Content-Type: application/json" "${CHRONOGRAF_URL}/chronograf/v1/sources" 2>/dev/null || echo '{}')

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
echo "Importing dashboards..."

for dashboard_file in "${DASHBOARDS_DIR}"/*.json; do
    if [ ! -f "$dashboard_file" ]; then
        echo "No dashboard files found in ${DASHBOARDS_DIR}"
        continue
    fi

    DASHBOARD_NAME=$(grep -o '"name"[[:space:]]*:[[:space:]]*"[^"]*"' "$dashboard_file" | head -1 | sed 's/"name"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')

    echo "Importing: ${DASHBOARD_NAME} ($(basename "$dashboard_file"))"

    DASHBOARD_JSON=$(cat "$dashboard_file")

    if [ -n "$SOURCE_URL" ]; then
        DASHBOARD_JSON=$(echo "$DASHBOARD_JSON" | sed "s|\"source\":\"\"|\"source\":\"${SOURCE_URL}\"|g")
    fi

    RESPONSE=$(echo "$DASHBOARD_JSON" | wget -q -O - --post-data=@- \
        --header="Content-Type: application/json" \
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