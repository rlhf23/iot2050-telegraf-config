# PLC Control Service

## Overview

A separate Docker service for writing bool values to Siemens S7 PLCs via S7comm protocol. This enables web UI buttons to toggle/momentary control PLC memory addresses.

## Architecture

```
┌──────────────┐     ┌─────────────────┐     ┌──────────────────┐
│  Web UI      │────▶│  PLC Service    │────▶│  PLC (S7comm)    │
│  (Nginx)     │     │  Port 8001      │     │  Port 102        │
└──────────────┘     └─────────────────┘     └──────────────────┘
        │                    │
        │                    ▼
        │            ┌─────────────────┐
        │            │ buttons.toml   │
        │            │ (Config)       │
        │            └─────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  Existing: Telegraf ──▶ InfluxDB ──▶ Grafana (read-only)        │
└─────────────────────────────────────────────────────────────────┘
```

## Components

### 1. PLC Service (`docker/plc-service/`)

Separate Rust binary using `axum` for HTTP API and `s7` crate for S7comm communication.

**Key files:**
- `src/main.rs` - Entry point
- `src/lib.rs` - Module exports
- `src/s7_client.rs` - S7comm connection handling
- `src/buttons.rs` - Button config parsing
- `src/routes.rs` - HTTP routes

### 2. Configuration (`docker/config/plc/buttons.toml`)

Button definitions with PLC addresses:

```toml
[plc]
ip = "192.168.1.10"
rack = 0
slot = 1
connection_timeout_ms = 2000

[[buttons]]
name = "Pump Start"
address = "DB1.DBX0.0"
mode = "toggle"

[[buttons]]
name = "Valve Open"
address = "DB1.DBX0.1"
mode = "momentary"
momentary_duration_ms = 500
```

### 3. Web UI Control Panel

Added to `index.html` with JavaScript for button interactions.

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/plc/buttons` | List available buttons |
| GET | `/api/plc/status` | Connection status |
| GET | `/api/plc/read/:id` | Read current button state |
| POST | `/api/plc/write` | Write bool value |
| POST | `/api/plc/toggle/:id` | Toggle bool value |

## Button Modes

### Toggle
Click to flip the bool value (ON ↔ OFF). State persists until next toggle.

### Momentary
- Press: Sends TRUE
- Release (or timeout): Sends FALSE
- Configurable duration for auto-release

## S7-1200/1500 Setup Requirements

1. **Enable PUT/GET**: TIA Portal → PLC Properties → Protection → "Allow PUT/GET communication"
2. **DB Access**: Data blocks must be "Non-optimized" or have "Full access" setting
3. **Network**: PLC must be reachable on port 102

## Address Format

- `DB1.DBX0.0` - DB 1, Byte 0, Bit 0 (bool)
- `DB1.DBX2.3` - DB 1, Byte 2, Bit 3 (bool)
- `DB1.DBW4` - DB 1, Word 4 (not implemented yet)

## Implementation Steps

1. [x] Create documentation
2. [x] Create `docker/plc-service/` directory structure
3. [x] Implement `Cargo.toml` with dependencies
4. [x] Implement configuration parsing (`buttons.rs`)
5. [x] Implement S7 client (`s7_client.rs`)
6. [x] Implement HTTP routes (`routes.rs`)
7. [x] Create main entry point (`main.rs`)
8. [x] Create sample `buttons.toml`
9. [x] Update `docker-compose.yml`
10. [x] Update nginx config for proxy
11. [x] Add control panel to `index.html`
12. [x] Create Dockerfile and Dockerfile.prebuilt
13. [x] Create GitHub workflow for building binaries
14. [ ] Test and debug

## Dependencies

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
s7 = "0.1"
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tower-http = { version = "0.5", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = "0.3"
thiserror = "2.0"
```

## Error Handling

- Connection timeout: Return cached state (if available) with warning
- Write failure: Return error, attempt reconnect on next request
- Config parse error: Log and return empty button list

## Security Considerations

- PLC service runs on internal Docker network
- Only accessible via nginx proxy
- No authentication required (as per requirements)
- Future: Could add basic auth or API tokens if needed

## Building Binaries

### Via GitHub Actions (Recommended)

1. Push changes to `docker/plc-service/` directory
2. Go to Actions → "Build PLC Service Binary" → Run workflow
3. Binaries are automatically committed to:
   - `docker/plc-service/docker-plc-service-arm64`
   - `docker/plc-service/docker-plc-service-amd64`

### Local Build for Testing

```bash
# Install cross for cross-compilation
cargo install cross --git https://github.com/cross-rs/cross

# Build for AMD64 (x86_64)
cd docker/plc-service
cross build --release --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/release/plc-service docker-plc-service-amd64

# Build for ARM64 (aarch64)
cross build --release --target aarch64-unknown-linux-musl
cp target/aarch64-unknown-linux-musl/release/plc-service docker-plc-service-arm64
```

### Docker Build Options

**Development (from source):**
```bash
cd docker
docker-compose build plc-service
```

**Production (prebuilt binary):**
Update `docker-compose.yml` to use `Dockerfile.prebuilt`:
```yaml
plc-service:
  build:
    context: .
    dockerfile: plc-service/Dockerfile.prebuilt
    args:
      - TARGETARCH=${TARGETARCH:-arm64}
```