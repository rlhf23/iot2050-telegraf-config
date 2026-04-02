# Open-Source Industrial SCADA Stack

**Get PLC data into Grafana with minimal clicking.**

A comprehensive FOSS toolkit for industrial monitoring that transforms XML exports from TIA Portal into live Grafana dashboards. Built with Rust, deployable anywhere Docker runs—from Raspberry Pis to industrial edge devices.

![GUI screenshot](./telegraf-config-gui.png)

## The Problem We Solve

Traditional industrial monitoring requires expensive proprietary SCADA systems and tedious manual configuration. This project provides a complete open-source alternative: drop your TIA Portal XML exports, and watch your PLC data appear in Grafana—automatically.

**Core workflow:** TIA Portal XML → Our Tools → Telegraf Config → InfluxDB → Grafana Dashboard

## Four Ways to Work

Choose your interface based on your environment and workflow:

### 🖥️ **GUI** - Most Feature-Rich (Native Desktop)
The full-featured desktop application with OPC-UA browser, real-time node exploration, and visual configuration management.
```bash
./sie_generate_config_gui
```
**Best for:** Initial setup, browsing OPC-UA servers, detailed configuration

### 💻 **TUI** - Terminal Interface (SSH/Headless)
Full-featured terminal UI with arrow key navigation. Perfect for SSH sessions or when GUI isn't available.
```bash
./sie_generate_config_tui
```
**Best for:** Remote servers, Windows VMs, headless environments

### ⌨️ **CLI** - Automation & Scripting
Command-line interface for deployment automation and basic config generation.
```bash
./sie_generate_config deploy setup 192.168.1.100
```
**Best for:** Device provisioning, CI/CD, scripting

### 🌐 **Web UI** - The Future (Browser-Based)
Drop XML files in your browser, generate configs without installing anything. Just navigate to your device's IP.
```
http://192.168.1.100/
```
**Best for:** Zero-install workflow, remote access, mobile devices

## What's Inside

### Core Rust Application
- **OPC-UA client** with hierarchical browsing and lazy loading
- **XML parser** for TIA Portal exports
- **Telegraf config generator** supporting multiple output formats
- **SSH deployment automation** for remote device management
- **Multi-interface architecture** (GUI/TUI/CLI/Web)

### Docker Monitoring Stack
- **InfluxDB 2.x** - Time-series database
- **Telegraf** - Metrics collection with OPC-UA support
- **Grafana** - Visualization with pre-configured dashboards
- **Prometheus** - Additional metrics and alerting
- **Nginx** - Reverse proxy and web dashboard landing page
- **API Service** - Lightweight Rust HTTP API for container management
- **Control Service** - Web-based PLC control panel for Siemens S7

### Deployment Options
- **Linux-native collectors** - Direct Telegraf deployment
- **Dockerized stack** - Complete monitoring infrastructure
- **Multi-architecture** - x86_64, ARM64 (Raspberry Pi, industrial edge)
- **One-command provisioning** - Automated Docker installation and setup

## Quick Start

### Deploy the Complete Stack (2 Commands)

```bash
# 1. Provision device with Docker
cd docker && ./scripts/init.sh 192.168.1.100

# 2. Deploy monitoring stack
./scripts/deploy.sh 192.168.1.100
```

Access your services:
- **Dashboard**: http://192.168.1.100 (Nginx landing page)
- **Grafana**: http://192.168.1.100:3000
- **InfluxDB**: http://192.168.1.100:8086
- **Prometheus**: http://192.168.1.100/prometheus
- **PLC Control**: http://192.168.1.100/control (requires auth)

### Configure OPC-UA Data Collection

**Option 1: GUI (Recommended)**
```bash
./sie_generate_config_gui
```
Browse OPC-UA servers, select tags, generate config.

**Option 2: Drop XML Files**
Export XML from TIA Portal, drop into GUI or Web UI, done.

**Option 3: Manual OPC-UA Browsing**
Use the built-in OPC-UA browser to click individual fields.

## Technology Stack

### Backend (Rust)
- **`opcua`** - OPC-UA client implementation
- **`eframe/egui`** - Native GUI framework
- **`ratatui`** - Terminal UI framework
- **`clap`** - CLI argument parsing
- **`roxmltree`** - XML parsing for TIA exports
- **`ssh2`** - Remote deployment automation
- **`tokio`** - Async runtime
- **`axum`** - Web framework (API service)

### Monitoring Stack (Docker)
- **InfluxDB 2.x** - Time-series database
- **Telegraf** - Metrics collection (OPC-UA, system, Docker)
- **Grafana** - Visualization and dashboards
- **Prometheus** - Metrics and alerting
- **Nginx** - Reverse proxy and web dashboard
- **Rust API Service** - Container management HTTP API
- **Rust Control Service** - PLC control panel (S7comm)

### Infrastructure
- **Docker Compose** - Multi-service orchestration
- **GitHub Actions** - CI/CD pipelines
- **Multi-arch builds** - x86_64 and ARM64 support
- **Nix** - Reproducible development environment

## CLI Reference

```bash
# Deployment
./sie_generate_config deploy provision <ip>    # Install Docker on device
./sie_generate_config deploy setup <ip>        # Deploy monitoring stack
./sie_generate_config deploy status <ip>       # Check service status
./sie_generate_config deploy start|stop <ip>   # Control services

# Configuration Generation
./sie_generate_config config -f <xml-folder>   # Generate from XML files
./sie_generate_config_gui                      # Launch GUI
./sie_generate_config_tui                      # Launch TUI

# Testing
./opcua_test_server                            # Start test OPC-UA server
./opcua_client_test                            # Test OPC-UA connection
```

## Building from Source

```bash
# Standard build
cargo build --release

# With Nix dev environment
nix develop  # Provides Rust toolchain and dependencies

# Test locally
cd docker && ./scripts/setup.sh && ./scripts/start.sh
```

## Project Vision

This project is building toward a complete **open-source SCADA alternative** using modern FOSS tools:

- **No proprietary software** - Everything from OPC-UA client to visualization is open source
- **Minimal manual work** - XML exports become live dashboards automatically
- **Multiple interfaces** - Work however you want: GUI, TUI, CLI, or Web
- **Deploy anywhere** - Raspberry Pi, industrial PCs, cloud VMs, edge devices
- **Production-ready** - Multi-architecture, automated testing, comprehensive monitoring

The goal: Make industrial monitoring as fluid and accessible as modern web development.

## Documentation

- **`PROJECT_OVERVIEW.md`** - Architecture and code organization
- **`docs/CONTAINER_SETUP.md`** - Docker deployment guide
- **`docs/WEBUI_CONFIG_GENERATOR.md`** - Web UI documentation
- **`docker/README.md`** - Docker stack details
- **`docker/TESTING.md`** - Testing procedures

## License

See LICENSE file for details.
