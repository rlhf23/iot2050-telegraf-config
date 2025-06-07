#!/bin/bash
set -e

# Start SSH server in the background
/usr/sbin/sshd -D &

# Execute the original Telegraf entrypoint
exec /entrypoint.sh "$@"
