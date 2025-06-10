#!/usr/bin/env bash
set -e

# Wait for InfluxDB to be ready with timeout (5 minutes max)
MAX_RETRIES=30
RETRY_INTERVAL=10

# Set up Influx CLI configuration
setup_influx_cli() {
  echo "Setting up InfluxDB CLI configuration..."
  influx config create --config-name default \
    --host-url http://localhost:8086 \
    --org ${INFLUXDB_ORG} \
    --token ${INFLUXDB_TOKEN} \
    --active
}

# Wait for InfluxDB to be ready
wait_for_influxdb() {
  echo "Waiting for InfluxDB to be ready..."
  for ((i = 1; i <= MAX_RETRIES; i++)); do
    if curl -s -o /dev/null http://localhost:8086/health; then
      echo "InfluxDB is ready!"
      # Give it a moment to fully initialize
      sleep 5
      return 0
    fi
    echo "Waiting for InfluxDB to be ready... (Attempt $i/$MAX_RETRIES)"
    sleep $RETRY_INTERVAL
  done
  echo "Error: Timed out waiting for InfluxDB to be ready" >&2
  return 1
}

# Main execution
if wait_for_influxdb; then
  # Setup CLI configuration
  if ! influx config list --json | grep -q '"name":"default"'; then
    setup_influx_cli
  fi

  # Create bucket if it doesn't exist
  if ! influx bucket list --name ${INFLUXDB_BUCKET} &>/dev/null; then
    echo "Creating bucket ${INFLUXDB_BUCKET}..."
    influx bucket create -n ${INFLUXDB_BUCKET} -r 0
  fi

  # Create Telegraf token if it doesn't exist
  if ! influx auth list --user ${INFLUXDB_USER} --json | grep -q '"description":"telegraf"'; then
    echo "Creating Telegraf token..."
    BUCKET_ID=$(influx bucket list -n ${INFLUXDB_BUCKET} --json | jq -r '.[0].id')
    TOKEN_JSON=$(influx auth create \
      --read-bucket ${BUCKET_ID} \
      --write-bucket ${BUCKET_ID} \
      --description "telegraf" \
      --json)
    
    TELEGRAF_TOKEN=$(echo "${TOKEN_JSON}" | jq -r '.token')
    
    if [ -n "${TELEGRAF_TOKEN}" ] && [ "${TELEGRAF_TOKEN}" != "null" ]; then
      echo "Telegraf token created successfully"
      # Update .env with the new token if the file exists and is writable
      if [ -w /docker-entrypoint-initdb.d/../../.env ]; then
        sed -i "s/^TELEGRAF_TOKEN=.*/TELEGRAF_TOKEN=${TELEGRAF_TOKEN}/" /docker-entrypoint-initdb.d/../../.env
      fi
    else
      echo "Warning: Failed to create Telegraf token" >&2
    fi
  else
    echo "Telegraf token already exists"
  fi
  
  echo "InfluxDB initialization complete!"
  exit 0
else
  exit 1
fi
