#!/bin/sh
# Simple system information script that outputs JSON
# Designed to be fast and efficient for IoT devices

# Get hostname
HOSTNAME=$(hostname)

# Get IP address (first non-loopback IPv4)
IP_ADDRESS=$(ip -4 addr show | grep -oP '(?<=inet\s)\d+(\.\d+){3}' | grep -v '127.0.0.1' | head -n 1)

# Get uptime in a readable format
UPTIME=$(uptime -p 2>/dev/null || uptime | awk '{print $3 " " $4}')

# Get architecture
ARCH=$(uname -m)

# Get load average
LOAD_AVG=$(uptime | awk -F'load average:' '{print $2}' | xargs)

# Get memory info (in MB)
MEM_TOTAL=$(free -m | awk '/^Mem:/ {print $2}')
MEM_USED=$(free -m | awk '/^Mem:/ {print $3}')
MEM_PERCENT=$(awk "BEGIN {printf \"%.1f\", ($MEM_USED/$MEM_TOTAL)*100}")

# Get disk usage for root partition
DISK_TOTAL=$(df -h / | awk 'NR==2 {print $2}')
DISK_USED=$(df -h / | awk 'NR==2 {print $3}')
DISK_PERCENT=$(df -h / | awk 'NR==2 {print $5}')

# Get running container count (if docker is available)
if command -v docker >/dev/null 2>&1; then
    CONTAINER_COUNT=$(docker ps -q 2>/dev/null | wc -l)
else
    CONTAINER_COUNT="N/A"
fi

# Output JSON
cat <<EOF
{
  "hostname": "$HOSTNAME",
  "ip_address": "$IP_ADDRESS",
  "uptime": "$UPTIME",
  "architecture": "$ARCH",
  "load_average": "$LOAD_AVG",
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
  "containers_running": $CONTAINER_COUNT,
  "timestamp": "$(date -Iseconds)"
}
EOF
