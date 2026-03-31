# Control Service Implementation Plan

## Overview

Rename plc-service to control-service, add HTTP Basic Auth, create separate UI page, and restructure for independence from monitoring stack.

## Decisions Made

- **Auth scope:** Entire control-service (UI + API) - prevents bypassing auth via direct API calls
- **Default username:** `admin` with password `change_me`
- **Port:** 8002
- **DB default:** DB2036 (unlikely to be in use)
- **Route:** `/control/` for UI, `/api/control/` for API

## Implementation Steps

### Phase 1: Rename and Restructure

#### 1.1 Rename directory and files
- [ ] Move `docker/plc-service/` → `docker/control-service/`
- [ ] Update `Cargo.toml` name to `control-service`
- [ ] Rename binary output in Dockerfile

#### 1.2 Move config
- [ ] Create `docker/config/control/` directory
- [ ] Create `config.toml` with combined auth + buttons config
- [ ] Create `config.toml.example` with safe defaults (DB2036)
- [ ] Remove `docker/config/plc/` directory

#### 1.3 Update docker-compose.yml
- [ ] Rename `plc-service` → `control-service`
- [ ] Change port from 8001 → 8002
- [ ] Update volume mounts for new config path
- [ ] Update depends_on for nginx

### Phase 2: Add Authentication

#### 2.1 Create auth module
- [ ] Create `docker/control-service/src/auth.rs`
- [ ] Implement HTTP Basic Auth middleware for Axum
- [ ] Add auth config to config parsing

#### 2.2 Update routes
- [ ] Wrap all routes with auth middleware
- [ ] Add auth to AppState

#### 2.3 Update config parsing
- [ ] Add `[auth]` section to config
- [ ] Update `buttons.rs` → `config.rs` (broader scope)

### Phase 3: Separate UI

#### 3.1 Create control.html
- [ ] Create `docker/config/nginx/html/control.html`
- [ ] Extract PLC control panel from index.html
- [ ] Update API paths to `/api/control/`
- [ ] Make mobile-friendly (already mostly responsive)

#### 3.2 Remove from index.html
- [ ] Remove PLC control panel section
- [ ] Remove PLC CSS styles
- [ ] Remove PLC JavaScript functions

### Phase 4: Update Routing

#### 4.1 Update nginx.conf
- [ ] Change `/plc-api/` → `/api/control/`
- [ ] Add `/control/` route to serve control.html
- [ ] Remove old plc references

#### 4.2 Update control-service routes
- [ ] Change `/api/plc/*` → `/api/control/*`
- [ ] Add endpoint to serve control.html (optional, or serve static)

### Phase 5: Update CI/CD

#### 5.1 GitHub workflows
- [ ] Rename `plc-service-build.yml` → `control-service-build.yml`
- [ ] Update paths and names in workflow
- [ ] Update binary names

#### 5.2 Dockerfiles
- [ ] Update binary names in Dockerfile
- [ ] Update binary names in Dockerfile.prebuilt

### Phase 6: Documentation and Cleanup

#### 6.1 Update docs
- [ ] Update `docs/plc-control-service.md` → `docs/control-service.md`
- [ ] Update README if needed

#### 6.2 Cleanup
- [ ] Remove old `docker/config/plc/` directory
- [ ] Update .gitignore for new paths
- [ ] Remove old references in code

## Config File Format

```toml
# docker/config/control/config.toml

[auth]
username = "admin"
password = "change_me"

[plc]
ip = "192.168.1.10"
rack = 0
slot = 1
connection_timeout_ms = 2000
read_timeout_ms = 2000
write_timeout_ms = 2000

[[buttons]]
name = "Pump Start"
address = "DB2036.DBX0.0"
mode = "toggle"

[[buttons]]
name = "Valve Open"
address = "DB2036.DBX0.1"
mode = "momentary"
momentary_duration_ms = 500
```

## File Changes Summary

### New Files
- `docker/control-service/` (renamed from plc-service)
- `docker/control-service/src/auth.rs`
- `docker/config/control/config.toml.example`
- `docker/config/nginx/html/control.html`

### Modified Files
- `docker/docker-compose.yml`
- `docker/config/nginx/nginx.conf`
- `docker/config/nginx/html/index.html`
- `.github/workflows/control-service-build.yml`
- `.gitignore`

### Deleted Files
- `docker/plc-service/` (entire directory)
- `docker/config/plc/` (entire directory)
- `.github/workflows/plc-service-build.yml`

## Testing Checklist

- [ ] Build control-service locally
- [ ] Run with valid config
- [ ] Test auth: unauthorized request returns 401
- [ ] Test auth: correct credentials allow access
- [ ] Test API routes under `/api/control/`
- [ ] Test UI at `/control/`
- [ ] Verify mobile responsiveness
- [ ] Test standalone (without monitoring stack)
- [ ] Test with monitoring stack running