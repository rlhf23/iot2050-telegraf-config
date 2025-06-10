#!/usr/bin/env bash
set -e

# Simple wait for InfluxDB to be ready (local socket check)
MAX_RETRIES=10
RETRY_INTERVAL=2

for ((i=1; i<=MAX_RETRIES; i++)); do
  if [ -S /var/run/influxd.sock ]; then
    echo "InfluxDB is ready!"
    break
  fi
  
  echo "Waiting for InfluxDB socket... (Attempt $i/$MAX_RETRIES)"
  if [ $i -eq $MAX_RETRIES ]; then
    echo "Continuing anyway - InfluxDB might be starting up" >&2
  fi
  sleep $RETRY_INTERVAL
done

# Small delay to ensure InfluxDB is fully up
sleep 2

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
