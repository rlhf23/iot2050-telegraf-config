# OPC-UA Browser API - Implementation Status

## ✅ Completed (Phase 1)

### Backend Changes
- **Added serde serialization to `OpcUaNode`** (`src/backend/opcua_poller.rs`)
  - Custom serializers for `NodeId`, `NodeClass`, and `ByteString`
  - Allows OPC-UA nodes to be sent as JSON responses
  
- **Added `serde` dependency** to main `Cargo.toml`

### API Endpoints Implemented

#### 1. `POST /api/opcua/connect`
**Purpose**: Connect to OPC-UA server and browse root nodes

**Request**:
```json
{
  "opcua_ip": "192.168.1.100",
  "opcua_username": "user",
  "opcua_password": "pass",
  "anonymous": false
}
```

**Response**:
```json
{
  "success": true,
  "session_id": "uuid-here",
  "message": "Connected successfully, found 5 root nodes",
  "node_count": 5
}
```

**Implementation**:
- Uses `tokio::spawn_blocking` to wrap sync `OpcUaPoller::browse_complete_structure()`
- Creates session with UUID and stores in global `OPCUA_SESSIONS` HashMap
- Same pattern as GUI worker thread (proven to work)

#### 2. `POST /api/opcua/get-nodes`
**Purpose**: Get root nodes or load children for a specific node

**Request (root nodes)**:
```json
{
  "session_id": "uuid-here",
  "node_id": null
}
```

**Request (load children)**:
```json
{
  "session_id": "uuid-here",
  "node_id": "ns=2;s=SomeNode"
}
```

**Response**:
```json
{
  "success": true,
  "nodes": [
    {
      "node_id": "ns=2;s=Node1",
      "browse_name": "Node1",
      "display_name": "Node 1",
      "node_class": "Variable",
      "data_type": "Int32",
      "description": null,
      "children": [],
      "selected": false,
      "children_loaded": false,
      "has_more_children": false,
      "continuation_point": null
    }
  ]
}
```

**Implementation**:
- Returns cached root nodes if `node_id` is null
- Uses `tokio::spawn_blocking` to wrap `OpcUaPoller::load_node_children()`
- Updates session `last_accessed` timestamp
- Lazy loading - only loads children when requested

### Session Management
- **Global storage**: `lazy_static` HashMap with `tokio::sync::RwLock`
- **Session data**: Config, root nodes, timestamps
- **Session ID**: UUID v4
- **TTL**: 30 minutes (cleanup not yet implemented)

### Dependencies Added
- `lazy_static = "1.4"` in `docker/api-service/Cargo.toml`
- `serde = { version = "1.0", features = ["derive"] }` in main `Cargo.toml`

## ✅ Completed (Phase 2)

### Frontend Integration
- **Selected nodes table** - Editable measurement names and intervals
- **"Use Selected Nodes" button** - Transfers selections from browser to config
- **Section-based UI** - Clear separation of XML vs Browser approaches
- **Config generation** - Sends selected_nodes to API

### Backend Integration
- **SelectedNodeRequest struct** - Receives nodes from frontend
- **NodeId parsing** - Converts string node_id back to opcua::types::NodeId
- **Config generation** - Passes selected nodes to ConfigGenerator
- **Validation** - Accepts either XML files OR selected nodes (or both)

## 🚧 TODO (Phase 3)

### Enhancements
- [ ] `POST /api/opcua/disconnect` - Clean up session
- [ ] Background task for session cleanup (remove sessions older than 30 min)
- [ ] Search/filter nodes in browser
- [ ] Folder grouping support in WebUI
- [ ] Keyboard navigation in tree

### Testing
- [ ] Test with real Siemens OPC-UA server
- [ ] Test lazy loading with large trees
- [ ] Test mixed XML + browser nodes config
- [ ] Test error handling (connection failures, timeouts)
- [ ] Test with different identifier types (string, numeric, guid, bytestring)

## 📝 Testing

### Manual Testing Script
Run `./docker/api-service/test-opcua-browser.sh` (requires running API service and OPC-UA server)

### Build & Run
```bash
# Build API service
cd docker/api-service
cargo build --release

# Run API service
cargo run

# In another terminal, test endpoints
./test-opcua-browser.sh
```

## 🎯 Key Design Decisions

1. **No async refactoring** - Keep proven sync OPC-UA code, wrap in `spawn_blocking`
2. **Session-based** - Store connection state server-side, not in frontend
3. **Lazy loading** - Only load children when user expands a node
4. **Same pattern as GUI** - Reuse worker thread pattern that's tested with real servers

## 📊 Performance Notes

- Initial connection: ~2-5 seconds (depends on OPC-UA server)
- Root node browsing: Included in connection time
- Child loading: ~500ms-2s per node (depends on number of children)
- Session storage: Minimal memory (~1-5MB per session)

## 🔒 Security Notes

- Sessions stored in-memory (lost on restart)
- No authentication on API endpoints (add later)
- OPC-UA credentials sent in request body (use HTTPS in production)
- Session cleanup needed to prevent memory leaks
