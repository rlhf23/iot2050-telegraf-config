#!/bin/sh
# Simple system information script that outputs JSON
# Designed to be fast and efficient for IoT devices running in Alpine container

# Get hostname
HOSTNAME=$(hostname)

# Get IP address (first non-loopback IPv4) - Alpine-compatible
IP_ADDRESS=$(ip -4 addr show 2>/dev/null | grep 'inet ' | grep -v '127.0.0.1' | head -n 1 | awk '{print $2}' | cut -d'/' -f1)

# Get uptime in a readable format
UPTIME=$(uptime 2>/dev/null | awk '{print $3 " " $4}' | sed 's/,//')

# Get architecture
ARCH=$(uname -m)

# Get load average
LOAD_AVG=$(uptime 2>/dev/null | awk -F'load average:' '{print $2}' | sed 's/^[[:space:]]*//')

# Get memory info from /proc/meminfo (Alpine-compatible)
if [ -f /host/proc/meminfo ]; then
    MEM_TOTAL=$(grep MemTotal /host/proc/meminfo | awk '{printf "%.0f", $2/1024}')
    MEM_AVAILABLE=$(grep MemAvailable /host/proc/meminfo | awk '{printf "%.0f", $2/1024}')
    MEM_USED=$(awk "BEGIN {printf \"%.0f\", $MEM_TOTAL - $MEM_AVAILABLE}")
    MEM_PERCENT=$(awk "BEGIN {printf \"%.1f\", ($MEM_USED/$MEM_TOTAL)*100}")
else
    MEM_TOTAL="N/A"
    MEM_USED="N/A"
    MEM_PERCENT="0"
fi

# Get disk usage for root partition
DISK_TOTAL=$(df -h / 2>/dev/null | awk 'NR==2 {print $2}')
DISK_USED=$(df -h / 2>/dev/null | awk 'NR==2 {print $3}')
DISK_PERCENT=$(df -h / 2>/dev/null | awk 'NR==2 {print $5}')

# Container count - not available from inside nginx container
CONTAINER_COUNT="N/A"

# Get current system time (human readable)
SYSTEM_TIME=$(date '+%Y-%m-%d %H:%M:%S')

# Output JSON
cat <<EOF
{
  "hostname": "$HOSTNAME",
  "ip_address": "$IP_ADDRESS",
  "uptime": "$UPTIME",
  "architecture": "$ARCH",
  "load_average": "$LOAD_AVG",
  "system_time": "$SYSTEM_TIME",
  "memory": {
    "total_mb": $MEM_TOTAL,
    "used_mb": $MEM_USED,
    "percent": $MEM_PERCENT
  },
  "disk": {
    "total": "$DISK_TOTAL",
    "used": "$DISK_USED",
    "percent": "$DISK_PERCENT"
  },
  "containers_running": "$CONTAINER_COUNT",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%S%z)"
}
EOF
