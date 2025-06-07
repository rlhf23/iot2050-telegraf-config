#!/bin/bash
set -e

# Wait for InfluxDB to be ready
until curl -s http://localhost:8086/health; do
  echo "Waiting for InfluxDB to be ready..."
  sleep 5
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
