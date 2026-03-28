# Direct OPC-UA Discovery Implementation Plan

## Overview

Replace the XML-based workflow with direct OPC-UA namespace discovery. Instead of requiring users to export XML files from TIA Portal, browse the PLC's ServerInterfaces directly and generate telegraf.conf from discovered variables.

## Current vs New Workflow

### Current (XML-based)
```
TIA Portal → Export XML → Upload XML → Poll namespaces (match XML) → Generate config
```

### New (Direct Discovery)
```
Connect to PLC → Discover ServerInterfaces → Collect all variables → Generate config
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  WebUI (opcua-discover.html)                                    │
│  Step 1: OPC-UA Connection (IP, credentials)                    │
│  Step 2: Auto-discover all namespaces/variables                  │
│  Step 3: Generate & Deploy                                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  API Endpoints                                                   │
│  POST /api/opcua/discover        → Discover namespaces/vars     │
│  POST /api/config/generate-from-discovery → Generate telegraf   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Backend (opcua_poller.rs)                                      │
│  discover_all_namespaces() → Browse ServerInterfaces             │
│  browse_namespace_variables() → Collect all variables           │
└─────────────────────────────────────────────────────────────────┘
```

## Data Structures

### New Structs (src/lib.rs)

```rust
#[derive(Serialize, Clone)]
pub struct DiscoveredNamespace {
    pub index: u16,
    pub name: String,
    pub variable_count: usize,
}

#[derive(Serialize, Clone)]
pub struct DiscoveredData {
    pub namespaces: Vec<DiscoveredNamespace>,
    pub variables: Vec<SelectedOpcUaNode>,
    pub namespace_names: HashMap<u16, String>,
    pub plc_name: Option<String>,
}
```

### Existing Structs (Reused)

```rust
// Already exists - used by browser flow
pub struct SelectedOpcUaNode {
    pub node_id: NodeId,
    pub namespace: u16,
    pub browse_name: String,
    pub display_name: String,
    pub measurement_name: String,
    pub interval_ms: u32,
    pub folder_name: Option<String>,
}
```

## Implementation Tasks

### Phase 1: Backend Discovery Functions

#### Task 1.1: Add new structs to src/lib.rs
- [ ] Add `DiscoveredNamespace` struct
- [ ] Add `DiscoveredData` struct
- [ ] Add serialization derives

#### Task 1.2: Implement discover_all_namespaces() in src/backend/opcua_poller.rs
- [ ] Connect to OPC-UA server
- [ ] Browse `ServerInterfaces` node (ns=3)
- [ ] For each child namespace:
  - [ ] Extract namespace index and name
  - [ ] Browse all variables under that namespace
  - [ ] Create `SelectedOpcUaNode` for each variable
- [ ] Return `DiscoveredData`

#### Task 1.3: Implement helper browse_namespace_variables()
- [ ] Recursive variable collection
- [ ] Handle nested structures (DataBlocks)
- [ ] Extract display_name, node_id, namespace

### Phase 2: API Endpoints

#### Task 2.1: Add session storage for discovered data
- [ ] Extend `SessionData` in docker/api-service/src/lib.rs
- [ ] Add `discovered_data: Option<DiscoveredData>` field

#### Task 2.2: Implement /api/opcua/discover endpoint
- [ ] Create `DiscoverRequest` struct
- [ ] Create `DiscoverResponse` struct
- [ ] Call `discover_all_namespaces()`
- [ ] Store result in session
- [ ] Return namespace summary (names + variable counts)

#### Task 2.3: Implement /api/config/generate-from-discovery endpoint
- [ ] Create request struct
- [ ] Retrieve discovered data from session
- [ ] Call existing `generate_config()` with `selected_opcua_nodes`
- [ ] Return generated config

### Phase 3: WebUI

#### Task 3.1: Create opcua-discover.html
- [ ] Create new HTML file in docker/config/nginx/html/
- [ ] Step 1: Connection form (IP, username, password, anonymous checkbox)
- [ ] Step 2: Discover button + results display
- [ ] Step 3: Generate & Deploy buttons
- [ ] Output format selection (InfluxDB/Prometheus)
- [ ] Options checkboxes (source timestamp, test inputs, diagnostics)

#### Task 3.2: Update index.html
- [ ] Add link to new discovery page
- [ ] Card for "Direct OPC-UA Discovery"

#### Task 3.3: Update nginx.conf (if needed)
- [ ] Ensure routing for new HTML file

### Phase 4: Testing

#### Task 4.1: Unit tests
- [ ] Test `discover_all_namespaces()` with mock OPC-UA server
- [ ] Test `browse_namespace_variables()`
- [ ] Test API endpoints

#### Task 4.2: Integration tests
- [ ] Test full discovery flow with test PLC
- [ ] Compare output with XML-based workflow

## Key Implementation Details

### discover_all_namespaces() Logic

```rust
pub fn discover_all_namespaces(&self) -> Result<DiscoveredData, TelegrafError> {
    // 1. Connect to OPC-UA server
    let session = self.connect()?;
    
    // 2. Browse ServerInterfaces (ns=3;s=ServerInterfaces)
    let server_interfaces_id = NodeId::new(3, "ServerInterfaces");
    let namespaces = self.browse_node(&session, &server_interfaces_id)?;
    
    // 3. For each namespace (child of ServerInterfaces)
    let mut discovered_namespaces = Vec::new();
    let mut all_variables = Vec::new();
    let mut namespace_names = HashMap::new();
    
    for ns_node in namespaces {
        let namespace_index = ns_node.node_id.namespace;
        let namespace_name = ns_node.display_name.clone();
        
        // 4. Browse all variables in this namespace
        let variables = self.browse_namespace_variables(
            &session,
            &ns_node.node_id,
            &namespace_name,
            namespace_index
        )?;
        
        let variable_count = variables.len();
        namespace_names.insert(namespace_index, namespace_name.clone());
        
        discovered_namespaces.push(DiscoveredNamespace {
            index: namespace_index,
            name: namespace_name,
            variable_count,
        });
        
        all_variables.extend(variables);
    }
    
    Ok(DiscoveredData {
        namespaces: discovered_namespaces,
        variables: all_variables,
        namespace_names,
        plc_name: None,
    })
}
```

### browse_namespace_variables() Logic

```rust
fn browse_namespace_variables(
    &self,
    session: &Arc<RwLock<Session>>,
    namespace_root: &NodeId,
    namespace_name: &str,
    namespace_index: u16,
) -> Result<Vec<SelectedOpcUaNode>, TelegrafError> {
    let mut variables = Vec::new();
    
    // Recursively browse all nodes under namespace_root
    let nodes = self.browse_recursive(session, namespace_root)?;
    
    for node in nodes {
        if node.node_class == NodeClass::Variable {
            variables.push(SelectedOpcUaNode {
                node_id: node.node_id.clone(),
                namespace: namespace_index,
                browse_name: node.browse_name.clone(),
                display_name: node.display_name.clone(),
                measurement_name: namespace_name.to_string(),
                interval_ms: 1000,
                folder_name: Some(namespace_name.to_string()),
            });
        }
    }
    
    Ok(variables)
}
```

### API Endpoint Flow

```
POST /api/opcua/discover
    │
    ├── Create OpcUaPoller with credentials
    ├── Call discover_all_namespaces()
    ├── Store DiscoveredData in session
    └── Return summary (namespace names + counts)
    │
    ▼
POST /api/config/generate-from-discovery
    │
    ├── Retrieve DiscoveredData from session
    ├── Create ConfigGenerator with selected_opcua_nodes
    ├── Call generate_config(&[], &[]) // No XML files
    └── Return telegraf.conf content
```

## File Changes Summary

| File | Changes |
|------|---------|
| `src/lib.rs` | Add DiscoveredNamespace, DiscoveredData structs |
| `src/backend/opcua_poller.rs` | Add discover_all_namespaces(), browse_namespace_variables() |
| `docker/api-service/src/lib.rs` | Extend session data for discovered_data |
| `docker/api-service/src/opcua.rs` | Add discover endpoint, generate-from-discovery endpoint |
| `docker/api-service/src/config.rs` | Add generate-from-discovery handler |
| `docker/config/nginx/html/opcua-discover.html` | NEW - Discovery page |
| `docker/config/nginx/html/index.html` | Add link to new page |

## Estimated Effort

| Phase | Estimated Time |
|-------|----------------|
| Phase 1: Backend | 2 days |
| Phase 2: API | 1 day |
| Phase 3: WebUI | 1-2 days |
| Phase 4: Testing | 1 day |
| **Total** | **5-6 days** |

## Compatibility Notes

- Generated telegraf.conf should be identical to XML-based output
- Namespace names from discovery used for Grafana dashboard panel titles
- Existing `generate_config()` for `selected_opcua_nodes` is reused unchanged
- XML-based workflow remains available as parallel option