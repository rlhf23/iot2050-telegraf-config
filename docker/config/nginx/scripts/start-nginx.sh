#!/bin/sh
# Wrapper script to start nginx and the system-info background process

# Make system-info script executable
chmod +x /usr/local/bin/system-info.sh

# Make html directory writable
chmod 777 /usr/share/nginx/html

# Start the system-info update loop in the background
while true; do
    /usr/local/bin/system-info.sh > /usr/share/nginx/html/system-info.json 2>/dev/null
    sleep 5
done &

# Start nginx in the foreground
exec nginx -g 'daemon off;'
