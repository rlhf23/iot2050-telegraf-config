# Docker API Service

A lightweight Rust-based API service for managing Docker containers via HTTP requests.

## Features

- **Container Management**: Restart, start, and stop containers
- **Security**: Whitelist-based access control
- **Lightweight**: ~15-20MB container size
- **Fast**: Minimal memory footprint (~5-10MB)
- **CORS Enabled**: Works with web dashboards

## API Endpoints

### Health Check
```
GET /health
```
Returns service health status.

### List Containers
```
GET /api/containers
```
Returns list of allowed containers with their status.

### Restart Container
```
POST /api/containers/{name}/restart
```
Restarts the specified container.

### Start Container
```
POST /api/containers/{name}/start
```
Starts the specified container.

### Stop Container
```
POST /api/containers/{name}/stop
```
Stops the specified container.

## Allowed Containers

Only the following containers can be managed:
- `grafana`
- `influxdb`
- `prometheus`
- `telegraf`

## Building

```bash
docker build -t api-service .
```

## Running

The service is automatically started via docker-compose and listens on port 8000.

## Security

- Whitelist validation prevents unauthorized container access
- Read-only Docker socket access
- No arbitrary command execution
- Proper error handling and logging
