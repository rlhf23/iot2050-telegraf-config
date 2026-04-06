# Direct OPC-UA Discovery Implementation Plan

## Status: COMPLETED✅

## Overview

Replace the XML-based workflow with direct OPC-UA namespace discovery. Instead of requiring users to export XML files from TIA Portal, browse the PLC's ServerInterfaces directly and generate telegraf.conf from discovered variables.

## Completed Tasks

### Phase 1: Backend Discovery Functions ✅
- [x] Add `DiscoveredNamespace` and `DiscoveredData` structs to `src/lib.rs`
- [x] Implement `discover_all_namespaces()` in `src/backend/opcua_poller.rs`
- [x] Implement `browse_namespace_variables()` and `collect_variables_recursive()` helpers

### Phase 2: API Endpoints ✅
- [x] Extend session storage in `docker/api-service/src/lib.rs` with `SessionStore` and `SessionDiscoveredData`
- [x] Implement `POST /api/opcua/discover` endpoint in `docker/api-service/src/opcua.rs`
- [x] Implement `POST /api/config/generate-from-discovery` endpoint in `docker/api-service/src/config.rs`

### Phase 3: WebUI ✅
- [x] Create `opcua-discover.html` - New discovery page with Grafana dashboard buttons
- [x] Update `index.html` with link to new page
- [x] Add dashboard generation/deployment functions

## Files Changed

| File | Changes |
|------|---------|
| `src/lib.rs` | Added `DiscoveredNamespace`, `DiscoveredData` structs; `SelectedOpcUaNode` derives Serialize/Deserialize |
| `src/backend/opcua_poller.rs` | Added `discover_all_namespaces()`, `browse_namespace_variables()`, `collect_variables_recursive()` |
| `docker/api-service/Cargo.toml` | Added `dashmap` dependency |
| `docker/api-service/src/lib.rs` | Added `SessionStore`, `SessionDiscoveredData`, new routes |
| `docker/api-service/src/opcua.rs` | Added `discover_namespaces()` endpoint |
| `docker/api-service/src/config.rs` | Added `generate_from_discovery()` endpoint |
| `docker/config/nginx/html/opcua-discover.html` | **NEW** - WebUI page |
| `docker/config/nginx/html/index.html` | Added card for OPC-UA Direct Discovery |

## Workflow

```
WebUI (opcua-discover.html)
    │
    ├── Step 1: Enter OPC-UA credentials → POST /api/opcua/discover
    │
    ├── Step 2: View discovered namespaces (names + variable counts)
    │
    └── Step 3: Configure options → POST /api/config/generate-from-discovery
                    │
                    ├── Returns telegraf.conf
                    └── Enables Grafana dashboard generation
```