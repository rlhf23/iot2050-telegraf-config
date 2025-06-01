#!/bin/bash
set -e

# Configure SSH
mkdir -p /run/sshd
chmod 755 /run/sshd

# Function to start a service with logging
start_service() {
    local name=$1
    local cmd=$2
    echo "Starting $name..."
    $cmd > /var/log/${name}.log 2>&1 &
    local pid=$!
    echo $pid > /var/run/${name}.pid
    echo "$name started with PID $pid"
}

# Start services
start_services() {
    # Start SSH server
    mkdir -p /run/sshd
    /usr/sbin/sshd -D &
    echo $! > /var/run/sshd.pid
    
    # Start InfluxDB
    if [ -f /usr/lib/influxdb/scripts/influxd-systemd-start.sh ]; then
        /usr/lib/influxdb/scripts/influxd-systemd-start.sh
    else
        influxd --http-bind-address :8086 --reporting-disabled > /var/log/influxdb.log 2>&1 &
    fi
    echo $! > /var/run/influxdb.pid
    
    # Start Prometheus
    mkdir -p /var/lib/prometheus
    prometheus --config.file=/etc/prometheus/prometheus.yml --storage.tsdb.path=/var/lib/prometheus > /var/log/prometheus.log 2>&1 &
    echo $! > /var/run/prometheus.pid
    
    # Start Node Exporter
    node_exporter > /var/log/node_exporter.log 2>&1 &
    echo $! > /var/run/node_exporter.pid
    
    # Start Telegraf
    telegraf --config /etc/telegraf/telegraf.conf > /var/log/telegraf.log 2>&1 &
    echo $! > /var/run/telegraf.pid
}

# Function to stop all services
stop_services() {
    echo "Shutting down services..."
    for service in telegraf node_exporter prometheus influxdb sshd; do
        if [ -f /var/run/${service}.pid ]; then
            local pid=$(cat /var/run/${service}.pid)
            if ps -p $pid > /dev/null; then
                echo "Stopping $service (PID: $pid)..."
                kill $pid
            fi
        fi
    done
    exit 0
}

# Set up trap to catch signals
trap 'stop_services' SIGTERM SIGINT

# Start all services
start_services

# Wait for all services to start
echo "Waiting for services to initialize..."
sleep 5

# Check if services are running
for service in sshd influxdb prometheus node_exporter telegraf; do
    if ! pgrep -f $service > /dev/null; then
        echo "Warning: $service is not running. Check /var/log/${service}.log for details."
    fi
    
    # Additional check for InfluxDB HTTP endpoint
    if [ "$service" = "influxdb" ]; then
        if ! curl -s -o /dev/null -w "%{http_code}" http://localhost:8086/health | grep -q "200\|204"; then
            echo "Warning: InfluxDB HTTP endpoint is not responding"
        fi
    fi
    
    # Additional check for Prometheus HTTP endpoint
    if [ "$service" = "prometheus" ]; then
        if ! curl -s -o /dev/null -w "%{http_code}" http://localhost:9090/-/healthy | grep -q "200"; then
            echo "Warning: Prometheus HTTP endpoint is not responding"
        fi
    fi
    
    # Additional check for Node Exporter HTTP endpoint
    if [ "$service" = "node_exporter" ]; then
        if ! curl -s -o /dev/null -w "%{http_code}" http://localhost:9100/metrics | grep -q "200"; then
            echo "Warning: Node Exporter HTTP endpoint is not responding"
        fi
    fi
done

echo "All services are running. Press Ctrl+C to stop."

# Keep container running
while true; do
    sleep 3600 &
    wait $!
done