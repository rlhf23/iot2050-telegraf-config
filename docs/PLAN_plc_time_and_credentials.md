# OPC UA Credential Storage & PLC Time Endpoint

## Goal
1. Persist OPC UA credentials so they can be reused (autofill in web UI, PLC time endpoint)
2. Add a PLC time endpoint to the API service that reads ServerStatus.CurrentTime (i=2257) from the PLC
3. Show PLC time + offset on the dashboard

## Credential Store

**File**: `/telegraf/opcua-credentials.json` on the device (mounted at `/telegraf/` in api-service)

```json
{
  "endpoint": "192.168.1.100:4840",
  "username": "admin",
  "password": "secret",
  "anonymous": false
}
```

- Written during config deployment (same time as telegraf.conf)
- Read by API service for PLC time endpoint
- Read by web UI for autofill (new endpoint)
- Plaintext, same security posture as telegraf.conf which already has credentials

## Changes

### 1. Write credentials during deployment
- `src/backend/deployment.rs` — After writing telegraf.conf, also write `opcua-credentials.json` to `/telegraf/` on the device
- Thread OPC UA credentials through `DeploymentConfig` or access them from the config struct

### 2. Read credentials endpoint (API service)
- `docker/api-service/src/opcua.rs` — Add `GET /api/opcua/credentials` that reads `/telegraf/opcua-credentials.json`
- `docker/api-service/src/lib.rs` — Add route

### 3. PLC time endpoint (API service)
- `src/backend/opcua_poller.rs` — Add `read_current_time(config)` method that reads node i=2257
- `docker/api-service/src/opcua.rs` — Add `GET /api/opcua/plc-time` that reads credentials, connects, reads PLC time, returns offset from host clock
- `docker/api-service/src/lib.rs` — Add route

### 4. Dashboard integration
- `docker/config/nginx/scripts/system-info.sh` — Curl `http://api-service:8000/api/opcua/plc-time`, add `plc_time` and `plc_time_offset_ms` to JSON
- `docker/config/nginx/html/index.html` — Add PLC Time row in System Information panel

### 5. Docker-compose
- Add `/telegraf/opcua-credentials.json` volume mount or just ensure `/telegraf/` is readable by api-service (already mounted)

### 6. Web UI autofill (follow-up)
- Frontend can call `GET /api/opcua/credentials` on page load to pre-fill the OPC UA connection fields