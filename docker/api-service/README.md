# Docker API Service

Lightweight Rust HTTP API for managing Docker containers and generating Telegraf configurations from TIA Portal XML exports.

## Features

- **Container Management**: Control stack containers (start, stop, restart, logs)
- **Configuration**: Generate Telegraf configs from Siemens TIA Portal XML files
- **OPC-UA Discovery**: Poll PLC namespaces for metadata
- **Security**: Whitelist-based access, read-only Docker socket
- **Multi-Architecture**: Supports ARM64 (IoT devices) and AMD64 (VMs/servers)

## API Endpoints

### Container Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Service health check |
| `GET` | `/api/containers` | List allowed containers with status |
| `GET` | `/api/containers/{name}/logs` | Retrieve last 200 lines of logs |
| `POST` | `/api/containers/{name}/restart` | Restart container |
| `POST` | `/api/containers/{name}/start` | Start container |
| `POST` | `/api/containers/{name}/stop` | Stop container |

**Allowed Containers**: `grafana`, `influxdb`, `prometheus`, `telegraf`

### Configuration

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/session/create` | Create upload session |
| `POST` | `/api/config/upload` | Upload TIA Portal XML files |
| `GET` | `/api/config/files/{session_id}` | List uploaded files |
| `DELETE` | `/api/config/files/{session_id}/{filename}` | Delete uploaded file |
| `POST` | `/api/config/generate` | Generate telegraf.conf |
| `POST` | `/api/config/deploy` | Deploy config and restart Telegraf |

**Config Generation Workflow**:
1. Create session → Upload XML files → Generate config → Deploy

### OPC-UA

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/opcua/poll-namespaces` | Poll PLC for namespace information |

## Building

```bash
# Local development
cd docker/api-service
cargo build --release

# Build for specific architecture
make build-amd64   # For x86_64 VMs/servers
make build-arm64   # For ARM IoT devices
make all-archs     # Build both
```

## Running

Started automatically via docker-compose:

```yaml
# docker-compose.yml
api-service:
  build:
    context: .
    dockerfile: api-service/Dockerfile.prebuilt
    args:
      TARGETARCH: ${TARGETARCH:-arm64}
  ports:
    - "8000:8000"
```

Architecture is auto-detected by `setup.sh` or specify manually:

```bash
TARGETARCH=amd64 docker-compose up -d api-service
```

## Security

- Whitelist validation prevents unauthorized container access
- Only 4 containers allowed: `grafana`, `influxdb`, `prometheus`, `telegraf`
- Read-only Docker socket access
- No arbitrary command execution
- CORS enabled for web dashboard integration

## Deployment

Via CLI tool from this repository:

```bash
# Automatically detects architecture and deploys correct binary
./sie_generate_config deploy provision --host <device-ip>
./sie_generate_config deploy setup --host <device-ip>
./sie_generate_config deploy start --host <device-ip>
```

## Architecture

| Architecture | Binary | Target Devices |
|--------------|--------|----------------|
| `arm64` | `docker-api-service-arm64` | IoT2050, Raspberry Pi, ARM64 devices |
| `amd64` | `docker-api-service-amd64` | x86_64 servers, VMs, desktops |
