#!/bin/sh
# Collect Docker logs for all monitoring stack containers
# Runs periodically to update log files

LOGS_DIR="/usr/share/nginx/html/logs"
TAIL_LINES=200

# Create logs directory if it doesn't exist
mkdir -p "$LOGS_DIR"

# List of containers to collect logs from
CONTAINERS="influxdb telegraf grafana prometheus nginx-dashboard"

for container in $CONTAINERS; do
    if docker ps --format '{{.Names}}' | grep -q "^${container}$"; then
        echo "Collecting logs for $container..."
        docker logs --tail $TAIL_LINES "$container" > "$LOGS_DIR/${container}.log" 2>&1
    else
        echo "Container $container not found or not running" > "$LOGS_DIR/${container}.log"
    fi
done

echo "Log collection completed at $(date)"
