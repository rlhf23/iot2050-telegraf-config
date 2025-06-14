# CLI Refactor Summary

## Overview
The `cli.rs` file has been successfully refactored to focus on deployment functionality and provide quick config generation with sane defaults, removing all interactive prompts.

## Key Changes

### 1. New Command Structure
The CLI now uses a clear subcommand structure:
- `config` - Generate Telegraf configuration with sane defaults
- `deploy` - Deploy monitoring stack to IoT devices (unchanged functionality)
- `check` - Check service connectivity (moved from flags to subcommands)

### 2. Config Generation (`config` subcommand)
**New Features:**
- ✅ Non-interactive config generation with sane defaults
- ✅ Automatic namespace detection using `get_namespace_info()` method
- ✅ Fallback to default namespace (2) when OPC UA server is unreachable
- ✅ Default intervals (500ms for active polling, 1000ms for listeners)
- ✅ No user prompts - everything is configurable via CLI arguments

**Usage:**
```bash
# Basic config generation with test inputs
sie_generate_config config --ip 192.168.1.100 --username user --password pass --test-inputs

# Config generation with XML files
sie_generate_config config --ip 192.168.1.100 --username user --password pass --folder /path/to/xml

# Config generation with custom intervals and auto-send
sie_generate_config config --ip 192.168.1.100 --username user --password pass \
  --folder /path/to/xml --default-interval 1000 --listener-interval 2000 \
  --send --iot-host 192.168.1.50 --iot-username admin --iot-password admin
```

**Arguments:**
- `--ip` (required): OPC UA server IP address
- `--username` (required): OPC UA username
- `--password` (required): OPC UA password
- `--folder`: Folder containing XML files (default: current directory)
- `--output-format`: Output format - influxdb or prometheus (default: influxdb)
- `--test-inputs`: Include test inputs (CPU, disk, memory)
- `--default-interval`: Default interval in ms for active polling (default: 500)
- `--listener-interval`: Default interval in ms for listeners (default: 1000)
- `--send`: Automatically send config to IoT device
- `--iot-host`: IoT device host for sending config
- `--iot-username`: IoT device username
- `--iot-password`: IoT device password

### 3. Deployment Commands (unchanged)
All deployment functionality remains the same:
```bash
sie_generate_config deploy provision <host> [options]
sie_generate_config deploy setup <host> [options]
sie_generate_config deploy status <host> [options]
sie_generate_config deploy start <host> [options]
sie_generate_config deploy stop <host> [options]
```

### 4. Connectivity Checks (improved)
Moved from global flags to dedicated subcommands:
```bash
sie_generate_config check influxdb <host> [--timeout 5]
sie_generate_config check prometheus <host> [--timeout 5]
```

## Removed Features
- ❌ All interactive prompts
- ❌ Legacy global flags for config generation
- ❌ Manual namespace entry prompts
- ❌ Manual interval entry prompts
- ❌ File selection confirmations
- ❌ Listener file selection prompts

## Benefits
1. **Non-interactive**: Perfect for automation and CI/CD pipelines
2. **Sane defaults**: Works out of the box with minimal configuration
3. **Automatic namespace detection**: Uses the existing `get_namespace_info()` method
4. **Clear command structure**: Easier to understand and use
5. **Backward compatibility**: All deployment and connectivity features preserved
6. **Faster workflow**: No waiting for user input

## Migration Guide
**Old usage (interactive):**
```bash
sie_generate_config
# Then answer multiple prompts...
```

**New usage (non-interactive):**
```bash
sie_generate_config config --ip 192.168.1.100 --username user --password pass --folder /path/to/xml
```

The refactored CLI maintains all core functionality while providing a much more streamlined and automation-friendly experience.