# Backup & Restore Guide

Complete guide for backing up and restoring monitoring data (InfluxDB, Grafana, Prometheus) from IoT devices.

## Overview

The backup system creates portable archives containing:
- **InfluxDB data** - All time-series metrics and measurements
- **Grafana dashboards** - Dashboard configurations, datasources, and settings
- **Prometheus data** - Prometheus metrics and time-series data
- **Configuration files** - Environment variables and settings

These archives can be:
- Stored for long-term archival
- Restored to the same device after wiping
- Restored to a different IoT device
- Restored to your laptop for local analysis

## Quick Start

### Backup Data

```bash
# Backup from device (creates timestamped archive)
cargo run --bin sie_generate_config deploy backup --host 192.168.1.100

# Backup to specific directory
cargo run --bin sie_generate_config deploy backup --host 192.168.1.100 -o ./my_backup
```

**Output:**
- `monitoring_backup_YYYYMMDD_HHMMSS.tar.gz` - Compressed archive
- `monitoring_backup_YYYYMMDD_HHMMSS/` - Extracted directory (for inspection)

### Restore Data

```bash
# Restore to same or different device
cargo run --bin sie_generate_config deploy restore \
  --host 192.168.1.100 \
  --archive monitoring_backup_20241117_120000.tar.gz
```

## Use Cases

### 1. Archive Before Wiping Device

**Scenario:** You've collected weeks of data and need to wipe the device, but want to keep the data for future reference.

```bash
# 1. Backup everything
cargo run --bin sie_generate_config deploy backup --host 192.168.1.100

# 2. Verify backup was created
ls -lh monitoring_backup_*.tar.gz

# 3. Store backup safely (external drive, cloud storage, etc.)
mv monitoring_backup_*.tar.gz /path/to/safe/storage/

# 4. Wipe device
cargo run --bin sie_generate_config deploy stop --host 192.168.1.100 -v

# 5. Later: Restore if needed
cargo run --bin sie_generate_config deploy restore \
  --host 192.168.1.100 \
  --archive /path/to/safe/storage/monitoring_backup_20241117_120000.tar.gz
```

### 2. View Old Data on Your Laptop

**Scenario:** You want to analyze old data without needing the IoT device.

```bash
# 1. Backup from device
cargo run --bin sie_generate_config deploy backup --host 192.168.1.100

# 2. Set up local monitoring stack on laptop
cd docker
./scripts/setup.sh
./scripts/start.sh

# 3. Restore backup to local stack
cargo run --bin sie_generate_config deploy restore \
  --host localhost \
  --archive monitoring_backup_20241117_120000.tar.gz

# 4. Access data locally
# - Grafana: http://localhost:3000
# - InfluxDB: http://localhost:8086
```

### 3. Migrate to New Device

**Scenario:** Moving monitoring stack from old device to new device.

```bash
# 1. Backup from old device
cargo run --bin sie_generate_config deploy backup --host 192.168.1.100

# 2. Provision new device
cargo run --bin sie_generate_config deploy provision --host 192.168.1.200

# 3. Setup monitoring stack on new device
cargo run --bin sie_generate_config deploy setup --host 192.168.1.200

# 4. Restore backup to new device
cargo run --bin sie_generate_config deploy restore \
  --host 192.168.1.200 \
  --archive monitoring_backup_20241117_120000.tar.gz
```

### 4. Regular Scheduled Backups

**Scenario:** Automated daily backups for disaster recovery.

Create a script `backup_cron.sh`:

```bash
#!/bin/bash
DEVICE_IP="192.168.1.100"
BACKUP_DIR="/mnt/backups/monitoring"
DATE=$(date +%Y%m%d)

# Create backup
cd /path/to/iot2050-telegraf-config
cargo run --bin sie_generate_config deploy backup \
  --host $DEVICE_IP \
  -o "$BACKUP_DIR/backup_$DATE"

# Keep only last 30 days
find $BACKUP_DIR -name "backup_*.tar.gz" -mtime +30 -delete
```

Add to crontab:
```bash
# Daily backup at 2 AM
0 2 * * * /path/to/backup_cron.sh
```

## Backup Contents

### Archive Structure

```
monitoring_backup_20241117_120000.tar.gz
└── monitoring_backup_20241117_120000/
    ├── backup_info.txt          # Metadata (timestamp, host, user)
    ├── influxdb/                # InfluxDB backup files
    │   ├── 20241117T120000Z.bolt
    │   ├── 20241117T120000Z.manifest
    │   └── ...
    ├── grafana/                 # Grafana volume backup
    │   └── grafana_data.tar.gz
    ├── prometheus/              # Prometheus volume backup
    │   └── prometheus_data.tar.gz
    └── config/                  # Configuration files
        └── env_backup           # .env file backup
```

### What's Included

| Component | What's Backed Up | What's NOT Backed Up |
|-----------|------------------|---------------------|
| **InfluxDB** | All databases, buckets, measurements, retention policies | Running queries, temporary data |
| **Grafana** | Dashboards, datasources, users, settings, plugins | Active sessions, temporary files |
| **Prometheus** | All metrics, time-series data, TSDB blocks | Temporary WAL files |
| **Config** | .env file with tokens and credentials | Docker images, application code |

## Advanced Usage

### Inspect Backup Without Restoring

```bash
# Extract archive
tar -xzf monitoring_backup_20241117_120000.tar.gz

# View metadata
cat monitoring_backup_20241117_120000/backup_info.txt

# Check InfluxDB backup size
du -sh monitoring_backup_20241117_120000/influxdb/

# Extract Grafana data to inspect
cd monitoring_backup_20241117_120000/grafana
tar -xzf grafana_data.tar.gz
```

### Partial Restore

If you only want to restore specific components, you can manually extract and restore:

```bash
# Example: Restore only Grafana dashboards
tar -xzf monitoring_backup_20241117_120000.tar.gz
cd monitoring_backup_20241117_120000/grafana
tar -xzf grafana_data.tar.gz

# Copy to device
scp -r grafana/ user@192.168.1.100:/tmp/

# On device: Restore Grafana volume
ssh user@192.168.1.100
docker run --rm -v grafana_data:/data -v /tmp:/backup alpine \
  sh -c 'cd /data && tar xzf /backup/grafana/grafana_data.tar.gz'
```

### Export InfluxDB Data to CSV

For long-term archival or analysis in other tools:

```bash
# SSH into device
ssh user@192.168.1.100

# Export specific measurement to CSV
docker exec influxdb influx query \
  -t <token> \
  'from(bucket:"telegraf") |> range(start: -30d)' \
  --raw > /tmp/export.csv

# Download to laptop
scp user@192.168.1.100:/tmp/export.csv ./
```

## Troubleshooting

### Backup Fails: "Permission Denied"

**Problem:** Cannot access Docker volumes or containers.

**Solution:**
```bash
# Ensure user is in docker group
ssh user@device
sudo usermod -aG docker $USER
# Log out and back in
```

### Restore Fails: "Container Not Found"

**Problem:** Monitoring stack not running.

**Solution:**
```bash
# Start the stack first
cargo run --bin sie_generate_config deploy start --host 192.168.1.100

# Then restore
cargo run --bin sie_generate_config deploy restore --host 192.168.1.100 --archive backup.tar.gz
```

### Large Backup Size

**Problem:** Backup archive is very large (>5GB).

**Solutions:**
1. **Reduce retention period** - Configure InfluxDB/Prometheus to keep less historical data
2. **Selective backup** - Manually backup only needed components
3. **Compress better** - Use `tar -czf` with higher compression (slower but smaller)

### InfluxDB Restore Fails

**Problem:** InfluxDB restore command fails with "bucket already exists".

**Solution:**
```bash
# Option 1: Wipe volumes before restore
cargo run --bin sie_generate_config deploy stop --host 192.168.1.100 -v
cargo run --bin sie_generate_config deploy setup --host 192.168.1.100
# Then restore

# Option 2: Manually delete conflicting buckets
ssh user@192.168.1.100
docker exec influxdb influx bucket delete -n telegraf
```

## Best Practices

### 1. Regular Backups
- Schedule automated backups (daily/weekly)
- Keep multiple backup versions (7-30 days)
- Test restore process periodically

### 2. Backup Verification
- Always verify backup completed successfully
- Check archive file size is reasonable
- Occasionally test restore to verify integrity

### 3. Storage
- Store backups on separate device/storage
- Use redundant storage (RAID, cloud backup)
- Encrypt sensitive backups

### 4. Retention Policy
```bash
# Example: Keep daily for 7 days, weekly for 4 weeks, monthly for 12 months
# Daily backups
0 2 * * * /path/to/backup_cron.sh daily
find /backups/daily -mtime +7 -delete

# Weekly backups (Sunday)
0 3 * * 0 /path/to/backup_cron.sh weekly
find /backups/weekly -mtime +28 -delete

# Monthly backups (1st of month)
0 4 1 * * /path/to/backup_cron.sh monthly
find /backups/monthly -mtime +365 -delete
```

## Performance Considerations

### Backup Duration

Typical backup times (depends on data volume):

| Data Size | Backup Time | Archive Size |
|-----------|-------------|--------------|
| 1 week data | 1-2 min | 100-500 MB |
| 1 month data | 3-5 min | 500 MB - 2 GB |
| 3 months data | 10-15 min | 2-5 GB |

### Network Impact

- Backup downloads data over SSH/SCP
- Expect ~10-50 MB/s transfer speed on local network
- Use `--output` to specify local storage with enough space

### Device Impact

- Backup runs while stack is running (no downtime)
- Minimal CPU impact
- Temporary disk usage on device (~2x data size during backup)
- Cleanup happens automatically

## Security

### Backup Contains Sensitive Data

Backups include:
- InfluxDB tokens and credentials
- Grafana admin passwords
- API keys and secrets from .env file

**Recommendations:**
1. Encrypt backup archives:
   ```bash
   # Encrypt
   gpg -c monitoring_backup_20241117_120000.tar.gz
   
   # Decrypt
   gpg monitoring_backup_20241117_120000.tar.gz.gpg
   ```

2. Secure storage permissions:
   ```bash
   chmod 600 monitoring_backup_*.tar.gz
   ```

3. Use SSH keys instead of passwords:
   ```bash
   cargo run --bin sie_generate_config deploy backup \
     --host 192.168.1.100 \
     --key-file ~/.ssh/id_rsa
   ```

## Integration with Existing Tools

### Grafana Cloud Backup

Export dashboards to Grafana Cloud for additional redundancy:

```bash
# Export all dashboards
ssh user@device
docker exec grafana grafana-cli admin export-dashboards /tmp/dashboards
scp -r user@device:/tmp/dashboards ./
```

### InfluxDB Cloud Sync

Replicate data to InfluxDB Cloud:

```bash
# Configure replication in telegraf.conf
[[outputs.influxdb_v2]]
  urls = ["https://cloud.influxdata.com"]
  token = "$CLOUD_TOKEN"
  organization = "your-org"
  bucket = "telegraf-backup"
```

## Summary

The backup/restore system provides a complete solution for:
- ✅ **Disaster recovery** - Restore after device failure
- ✅ **Data archival** - Keep historical data indefinitely  
- ✅ **Migration** - Move between devices
- ✅ **Local analysis** - View data on laptop
- ✅ **Compliance** - Meet data retention requirements

**Key Commands:**
```bash
# Backup
cargo run --bin sie_generate_config deploy backup --host <device-ip>

# Restore
cargo run --bin sie_generate_config deploy restore --host <device-ip> --archive <file>

# View help
cargo run --bin sie_generate_config deploy backup --help
cargo run --bin sie_generate_config deploy restore --help
```
