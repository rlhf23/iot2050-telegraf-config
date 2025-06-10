#!/usr/bin/env bash
set -e

# Wait for InfluxDB to be ready (faster check using ping endpoint)
MAX_RETRIES=12  # 1 minute max (12 * 5s)
RETRY_INTERVAL=5

for ((i=1; i<=MAX_RETRIES; i++)); do
  # Use ping endpoint which is faster than health check
  # Using service name 'influxdb' instead of localhost for Docker's internal DNS
  if curl -s -o /dev/null -f http://influxdb:8086/ping; then
    # Give it one more second to fully initialize
    sleep 1
    echo "InfluxDB is ready!"
    break
  fi
  echo "Waiting for InfluxDB to be ready... (Attempt $i/$MAX_RETRIES)"
  if [ $i -eq $MAX_RETRIES ]; then
    echo "Error: Timed out waiting for InfluxDB to be ready" >&2
    exit 1
  fi
  sleep $RETRY_INTERVAL
done

# Create Telegraf bucket if it doesn't exist
if ! influx bucket list --name $INFLUXDB_BUCKET &> /dev/null; then
  echo "Creating bucket $INFLUXDB_BUCKET..."
  influx bucket create -n $INFLUXDB_BUCKET
fi

# Create Telegraf token if it doesn't exist
if ! influx auth list --user $INFLUXDB_USER --json | grep -q '"description":"telegraf"'; then
  echo "Creating Telegraf token..."
  TELEGRAF_TOKEN=$(influx auth create \
    --read-bucket $(influx bucket list -n $INFLUXDB_BUCKET --json | jq -r '.[0].id') \
    --description "telegraf" \
    --json | jq -r '.token')
  
  echo "Telegraf token: $TELEGRAF_TOKEN"
  # Update .env with the new token
  sed -i "s/^TELEGRAF_TOKEN=.*/TELEGRAF_TOKEN=$TELEGRAF_TOKEN/" /docker-entrypoint-initdb.d/../../.env
fi

echo "InfluxDB initialization complete!"
