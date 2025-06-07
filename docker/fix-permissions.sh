#!/usr/bin/env bash
# Fix permissions in the Telegraf container
docker exec -it telegraf bash -c "chown -R root:root /etc/telegraf && chmod -R 755 /etc/telegraf"
