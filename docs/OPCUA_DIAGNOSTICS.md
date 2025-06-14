# OPC UA Diagnostics and Monitoring Guide

This guide explains how to use the new OPC UA diagnostics and monitoring features in the IoT2050 Monitoring Stack.

## Overview

The monitoring stack now supports comprehensive OPC UA connection health monitoring including:

- **Telegraf Internal Metrics**: Monitor Telegraf performance and resource usage
- **OPC UA Server Diagnostics**: Track server health, session counts, and connection status
- **Dual Bucket Strategy**: Separate process data from diagnostics data
- **Enhanced Alerting**: Better visibility into connection issues and performance problems

## Quick Start

### 1. Enable Diagnostics in CLI

Generate a Telegraf configuration with diagnostics enabled:

```bash
# Generate config with OPC UA diagnostics and system metrics
./sie_generate_config --diagnostics --test-inputs -f /path/to/xml/files

# Generate config with only diagnostics (no XML files needed)
./sie_generate_config --diagnostics --test-inputs
```

### 2. Deploy to IoT Device

Deploy the enhanced monitoring stack:

```bash
cd docker
./scripts/deploy.sh 192.168.1.100
```

The stack now automatically creates two InfluxDB buckets:
- `telegraf` - Process data (OPC UA measurements)
- `telegraf_diagnostics` - Diagnostics data (system metrics, OPC UA server health)

### 3. Access Monitoring Services

After deployment:
- **Grafana**: http://192.168.1.100:3000
- **InfluxDB**: http://192.168.1.100:8086
- **Prometheus**: http://192.168.1.100:9090

## Generated Configuration

With `--diagnostics` enabled, the configuration includes:

### Dual Output Configuration

```toml
# Process data goes to main bucket
[[outputs.influxdb_v2]]
  urls = ["http://influxdb:8086"]
  token = "${INFLUXDB_TOKEN}"
  organization = "${INFLUXDB_ORG}"
  bucket = "${INFLUXDB_BUCKET}"
  namepass = ["opcua*"]  # Only OPC UA process data

# Diagnostics data goes to separate bucket
[[outputs.influxdb_v2]]
  urls = ["http://influxdb:8086"]
  token = "${INFLUXDB_TOKEN}"
  organization = "${INFLUXDB_ORG}"
  bucket = "${INFLUXDB_DIAGNOSTICS_BUCKET}"
  namedrop = ["opcua*"]  # Everything except OPC UA process data
```

### Telegraf Internal Monitoring

```toml
# Monitor Telegraf's performance
[[inputs.internal]]
  collect_memstats = true
```

### OPC UA Server Diagnostics

```toml
# Monitor OPC UA server health
[[inputs.opcua]]
  name = "opcua_diagnostics"
  endpoint = "opc.tcp://your-server:4840"
  connect_timeout = "300s"
  request_timeout = "10s"
  security_policy = "Basic256Sha256"
  security_mode = "SignAndEncrypt"
  username = "your-username"
  password = "your-password"
  interval = "10s"

  [[inputs.opcua.group]]
    name = "server_diagnostics"
    namespace = "0"  # Standard OPC UA namespace
    identifier_type = "i"
    nodes = [
      {name="server_state", identifier="2259"},              # ServerState
      {name="current_sessions", identifier="2277"},          # CurrentSessionCount  
      {name="current_subscriptions", identifier="2285"},     # CurrentSubscriptionCount  
      {name="cumulated_sessions", identifier="2278"},        # CumulatedSessionCount
      {name="cumulated_subscriptions", identifier="2286"},   # CumulatedSubscriptionCount
      {name="publishing_interval_count", identifier="2284"}, # PublishingIntervalCount
      {name="rejected_requests", identifier="2288"},         # RejectedRequestsCount
      {name="rejected_sessions", identifier="3705"},         # RejectedSessionCount
      {name="session_abort_count", identifier="2282"},       # SessionAbortCount
      {name="session_timeout_count", identifier="2281"}      # SessionTimeoutCount
    ]
```

### System Metrics

```toml
# System resource monitoring
[[inputs.cpu]]
  percpu = true
  totalcpu = true

[[inputs.mem]]
  # no configuration

[[inputs.disk]]
  ignore_fs = ["tmpfs", "devtmpfs", "devfs", "iso9660", "overlay", "aufs", "squashfs"]

[[inputs.net]]
  # no configuration

[[inputs.system]]
  # no configuration
```

## Key Metrics to Monitor

### OPC UA Connection Health

- **server_state**: OPC UA server operational status
- **current_sessions**: Number of active client sessions
- **rejected_sessions**: Failed connection attempts
- **session_timeout_count**: Sessions lost due to timeouts

### Performance Metrics

- **current_subscriptions**: Active data subscriptions
- **publishing_interval_count**: Data publishing frequency
- **rejected_requests**: Failed OPC UA requests

### Telegraf Health

- **internal_memstats**: Telegraf memory usage
- **internal_agent**: Telegraf performance metrics

### System Resources

- **cpu**: CPU utilization per core
- **mem**: Memory usage and availability
- **disk**: Disk space and I/O
- **net**: Network interface statistics

## Grafana Dashboard Setup

### 1. Process Data Dashboard

Create dashboards using the main bucket (`telegraf`):
- Machine operational data
- Process variables
- Production metrics

### 2. Diagnostics Dashboard

Create monitoring dashboards using the diagnostics bucket (`telegraf_diagnostics`):

```sql
-- OPC UA Connection Status
SELECT last("server_state") FROM "opcua_diagnostics" WHERE time >= now() - 1h

-- Session Health Over Time
SELECT mean("current_sessions") FROM "opcua_diagnostics" WHERE time >= now() - 24h GROUP BY time(1h)

-- Connection Failures
SELECT sum("rejected_sessions") FROM "opcua_diagnostics" WHERE time >= now() - 24h GROUP BY time(1h)

-- Telegraf Memory Usage
SELECT last("heap_inuse") FROM "internal_memstats" WHERE time >= now() - 1h

-- System Resource Usage
SELECT mean("usage_percent") FROM "cpu" WHERE time >= now() - 1h GROUP BY time(5m), "cpu"
```

## Alerting Rules

### Critical Alerts

- **OPC UA Server Down**: `server_state != 0`
- **No Active Sessions**: `current_sessions == 0`
- **High Session Rejections**: `rejected_sessions > 10/hour`
- **Memory Issues**: `heap_inuse > 100MB`

### Warning Alerts

- **Session Timeouts**: `session_timeout_count > 5/hour`
- **Request Rejections**: `rejected_requests > 50/hour`
- **High CPU**: `cpu.usage_percent > 80%`
- **Low Disk Space**: `disk.free < 1GB`

## Troubleshooting

### High Memory Usage

Check Telegraf internal metrics:
```sql
SELECT * FROM "internal_memstats" WHERE time >= now() - 1h
```

### Connection Issues

Monitor OPC UA diagnostics:
```sql
SELECT * FROM "opcua_diagnostics" WHERE time >= now() - 1h
```

### Performance Problems

Check system metrics:
```sql
SELECT * FROM "cpu", "mem", "disk" WHERE time >= now() - 1h
```

## Environment Variables

The monitoring stack uses these additional environment variables:

```bash
# Main data bucket
INFLUXDB_BUCKET=telegraf

# Diagnostics data bucket  
INFLUXDB_DIAGNOSTICS_BUCKET=telegraf_diagnostics

# Standard InfluxDB configuration
INFLUXDB_ORG=iot2050
INFLUXDB_TOKEN=your-token
INFLUXDB_USER=admin
INFLUXDB_PASSWORD=your-password
```

## Best Practices

1. **Separate Concerns**: Keep process data and diagnostics in separate buckets
2. **Reasonable Intervals**: Use 10s for diagnostics, match your process data needs
3. **Monitor Trends**: Look for patterns in connection failures and performance
4. **Set Alerts**: Configure proactive alerting for critical metrics
5. **Regular Cleanup**: Set retention policies appropriate for your needs

## Advanced Configuration

### Custom Diagnostic Intervals

Modify the generated configuration to adjust monitoring frequency:

```toml
# More frequent monitoring for critical systems
interval = "5s"

# Less frequent for stable systems  
interval = "30s"
```

### Additional OPC UA Metrics

Add more server diagnostic nodes by extending the nodes list:

```toml
{name="redundancy_support", identifier="2296"},        # RedundancySupport
{name="server_profile_array", identifier="2269"},      # ServerProfileArray  
{name="locale_id_array", identifier="2271"},           # LocaleIdArray
```

For a complete list of available diagnostic nodes, consult the OPC UA specification or your server documentation.
