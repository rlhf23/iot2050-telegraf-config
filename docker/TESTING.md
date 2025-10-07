# Testing Guide

This document describes how to test the monitoring stack components.

## Quick Start

```bash
# Test scripts in their container environments (before deploying)
cd docker
chmod +x scripts/test-scripts.sh
./scripts/test-scripts.sh

# Test the full stack (after starting with docker-compose)
chmod +x scripts/test-integration.sh
./scripts/test-integration.sh
```

## Script Testing

### Testing Shell Scripts in Alpine Containers

Before deploying scripts that run in Alpine-based containers (like nginx:alpine), test them locally:

```bash
# Test system-info.sh
docker run --rm \
  -v $(pwd)/config/nginx/scripts/system-info.sh:/test/system-info.sh:ro \
  -v /proc:/host/proc:ro \
  -v /sys:/host/sys:ro \
  nginx:alpine \
  sh -c "chmod +x /test/system-info.sh && /test/system-info.sh"
```

### Common Alpine Gotchas

Alpine Linux uses BusyBox, which has limited versions of common commands:

- ❌ `grep -P` (Perl regex) - Use basic regex or awk instead
- ❌ `free -m` - Not available, read from `/proc/meminfo` instead
- ❌ `date -Iseconds` - Use `date -u +%Y-%m-%dT%H:%M:%S%z` instead
- ✅ `awk`, `sed`, `cut` - Available and work well

## Integration Testing

### Manual Testing Checklist

After starting the stack, verify:

1. **Health endpoint**: `curl http://localhost/health`
2. **System info**: `curl http://localhost/api/system-info | jq`
3. **Grafana**: `curl http://localhost/grafana/api/health`
4. **InfluxDB**: `curl http://localhost/influxdb/health`
5. **Prometheus**: `curl http://localhost/prometheus/-/healthy`

### Container Health Checks

Check container status:

```bash
# View all containers
docker-compose ps

# Check specific container logs
docker logs nginx-dashboard --tail 50

# Check if system-info.json is being generated
docker exec nginx-dashboard ls -la /usr/share/nginx/html/system-info.json
docker exec nginx-dashboard cat /usr/share/nginx/html/system-info.json | jq
```

## Troubleshooting

### Nginx Container Issues

If nginx container is unhealthy or restarting:

```bash
# Check logs
docker logs nginx-dashboard

# Common issues:
# - "chmod: read-only file system" - Remove chmod from read-only mounted files
# - "Permission denied" - Script needs execute permissions or run with sh
# - "No such file" - Check volume mounts and file paths
```

### System Info Not Updating

```bash
# Check if background process is running
docker exec nginx-dashboard ps aux | grep system-info

# Manually test the script
docker exec nginx-dashboard /usr/local/bin/system-info.sh

# Check if file can be written
docker exec nginx-dashboard touch /usr/share/nginx/html/test.txt
```

### 404 Errors on API Endpoints

```bash
# Check nginx config is correct
docker exec nginx-dashboard cat /etc/nginx/nginx.conf | grep system-info

# Reload nginx config
docker exec nginx-dashboard nginx -s reload

# Check file exists at expected path
docker exec nginx-dashboard ls -la /usr/share/nginx/html/
```

## CI/CD Integration

Add to your CI pipeline:

```yaml
# Example GitHub Actions
- name: Test Scripts
  run: |
    cd docker
    ./scripts/test-scripts.sh

- name: Start Stack
  run: |
    cd docker
    docker-compose up -d

- name: Run Integration Tests
  run: |
    cd docker
    ./scripts/test-integration.sh
```

## Best Practices

1. **Always test scripts in Alpine containers before deploying**
2. **Use the automated test scripts before committing changes**
3. **Check container logs when things don't work**
4. **Verify file permissions and volume mounts**
5. **Test locally with docker-compose before deploying to devices**
