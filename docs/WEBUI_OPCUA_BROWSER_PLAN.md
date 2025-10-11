# WebUI OPC-UA Browser Implementation Plan (Revised)

## Important Context

**The current sync OPC-UA implementation is battle-tested with real Siemens servers.** Previous async attempts failed due to the complexity of the `opcua` crate (continuation points, session management, etc.). 

**Strategy**: Use the proven sync code with `tokio::spawn_blocking`, clean up unused functions, and base the WebUI on what actually works in the GUI.

## Current State Analysis

### What's Working (Phases 1-3 Complete)
✅ **Phase 1**: File upload & management  
✅ **Phase 2**: Configuration generation from XML files  
✅ **Phase 3**: Deployment integration  
✅ **Bonus**: OPC-UA namespace polling (basic)

### What We Have for OPC-UA Browser

#### Backend (`src/backend/opcua_poller.rs`)
The OPC-UA browser backend exists and is **tested in the GUI**, but has some complexity:

**Key Components:**
- `OpcUaPoller` - Main struct for OPC-UA operations
- `OpcUaNode` - Node representation with lazy loading support
- `browse_complete_structure()` - Browse from root (tested in GUI)
- `browse_nodes()` - Recursive browsing with depth limits
- `get_namespace_info()` - Already used in API service

**Features:**
- ✅ Hierarchical browsing with lazy loading
- ✅ Connection management with timeout handling
- ✅ Anonymous and username/password authentication
- ✅ Node metadata (display name, browse name, node class, data type)
- ✅ Continuation point support for large node trees
- ⚠️ **Async/sync mixing** - Uses `opcua::sync` but called from async contexts

**Potential Issues:**
1. **Blocking operations** - OPC-UA operations are synchronous, may block async runtime
2. **Session management** - Sessions need to be kept alive during browsing
3. **Memory usage** - Large node trees can consume significant memory
4. **Timeout handling** - Need proper timeout for long-running browse operations

#### GUI Implementation (`src/gui_controller.rs` + `src/gui_renderer.rs`)
The GUI uses a **worker thread pattern** to avoid blocking:

**Architecture:**
```
GUI Thread (egui)
    ↓ send command
Worker Thread (tokio::spawn_blocking)
    ↓ execute OPC-UA operation
    ↓ send response
GUI Thread (process response)
```

**Key Patterns:**
- `WorkerCommand::BrowseOpcUaNodes` - Sent to worker thread
- `WorkerResponse::OpcUaNodesReady(Vec<OpcUaNode>)` - Response with nodes
- `OpcUaBrowseState` - State machine for browse operations
- Lazy loading - Only load children when user expands a node

#### API Service (`docker/api-service/src/opcua.rs`)
Currently only implements namespace polling:
- `poll_namespaces()` - Gets namespace info for XML files
- Uses `OpcUaPoller::get_namespace_info()` directly
- ⚠️ **Blocking call in async handler** - Works for quick operations, risky for browsing

## What Functions Are Actually Used?

From `opcua_poller.rs`, the **tested and working** public functions:

1. ✅ **`OpcUaPoller::new()`** - Create poller with config
2. ✅ **`get_namespace_info()`** - Already used in API service
3. ✅ **`browse_complete_structure()`** - Used in GUI, returns root nodes
4. ✅ **`load_node_children()`** - Lazy load children when user expands node
5. ✅ **`is_load_more_node()`** - Check if node is a "load more" placeholder
6. ✅ **`OpcUaNode::is_folder_node()`** - Check if node is a folder
7. ✅ **`OpcUaNode::collect_selected_variable_nodes()`** - Get selected variables
8. ✅ **`OpcUaNode::convert_selected_nodes_to_config()`** - Convert to config format

**These are the only functions we need to expose via API.**

## Implementation Approach: Use What Works

**Estimated Time: 5-7 hours**

### Step 1: Clean Up Backend (1 hour)

**Audit `opcua_poller.rs`:**
- [ ] Identify unused functions (grep for usage across codebase)
- [ ] Add `#[allow(dead_code)]` or delete unused functions
- [ ] Add doc comments to public API
- [ ] Ensure all public functions are tested in GUI

**No refactoring to async** - just cleanup and documentation.

### Step 2: Add API Endpoints (2-3 hours)

**Strategy**: Wrap the sync `OpcUaPoller` calls in `tokio::spawn_blocking` - same pattern as GUI's worker thread.

```rust
// docker/api-service/src/opcua.rs

use tokio::task::spawn_blocking;
use std::sync::Arc;
use parking_lot::RwLock;

// Session storage (in-memory)
lazy_static! {
    static ref OPCUA_SESSIONS: Arc<RwLock<HashMap<String, OpcUaBrowserSession>>> = 
        Arc::new(RwLock::new(HashMap::new()));
}

struct OpcUaBrowserSession {
    config: TelegrafConfig,
    root_nodes: Vec<OpcUaNode>,
    selected_nodes: Vec<OpcUaNode>,
    created_at: Instant,
    last_accessed: Instant,
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub opcua_ip: String,
    pub opcua_username: Option<String>,
    pub opcua_password: Option<String>,
    pub anonymous: bool,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub success: bool,
    pub session_id: String,
    pub message: String,
}

// POST /api/opcua/connect - Connect and get root nodes
pub async fn connect_opcua(
    Json(request): Json<ConnectRequest>,
) -> Result<Json<ConnectResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Create config
    let config = TelegrafConfig {
        ip: request.opcua_ip.clone(),
        username: if request.anonymous { String::new() } 
                  else { request.opcua_username.unwrap_or_default() },
        password: if request.anonymous { String::new() } 
                  else { request.opcua_password.unwrap_or_default() },
        // ... other fields
    };
    
    // Browse root nodes in blocking task
    let root_nodes = spawn_blocking(move || {
        let poller = OpcUaPoller::new(config.clone())?;
        poller.browse_complete_structure()
    })
    .await
    .map_err(|e| /* handle join error */)?
    .map_err(|e| /* handle opcua error */)?;
    
    // Create session
    let session_id = uuid::Uuid::new_v4().to_string();
    let session = OpcUaBrowserSession {
        config,
        root_nodes,
        selected_nodes: Vec::new(),
        created_at: Instant::now(),
        last_accessed: Instant::now(),
    };
    
    OPCUA_SESSIONS.write().insert(session_id.clone(), session);
    
    Ok(Json(ConnectResponse {
        success: true,
        session_id,
        message: "Connected successfully".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct GetNodesRequest {
    pub session_id: String,
    pub node_id: Option<String>, // None = get root nodes
}

// POST /api/opcua/get-nodes - Get root or child nodes
pub async fn get_nodes(
    Json(request): Json<GetNodesRequest>,
) -> Result<Json<Vec<OpcUaNode>>, (StatusCode, Json<ErrorResponse>)> {
    let sessions = OPCUA_SESSIONS.read();
    let session = sessions.get(&request.session_id)
        .ok_or_else(|| /* session not found error */)?;
    
    if request.node_id.is_none() {
        // Return root nodes
        return Ok(Json(session.root_nodes.clone()));
    }
    
    // Load children for specific node
    let node_id_str = request.node_id.unwrap();
    let config = session.config.clone();
    
    // Find the parent node
    let parent_node = find_node_in_tree(&session.root_nodes, &node_id_str)
        .ok_or_else(|| /* node not found error */)?;
    
    // Load children in blocking task
    let children = spawn_blocking(move || {
        let poller = OpcUaPoller::new(config)?;
        poller.load_node_children(&parent_node, 1)
    })
    .await??;
    
    Ok(Json(children))
}

#[derive(Deserialize)]
pub struct SelectNodesRequest {
    pub session_id: String,
    pub node_ids: Vec<String>,
    pub selected: bool, // true = select, false = deselect
}

// POST /api/opcua/select-nodes - Mark nodes as selected
pub async fn select_nodes(
    Json(request): Json<SelectNodesRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut sessions = OPCUA_SESSIONS.write();
    let session = sessions.get_mut(&request.session_id)
        .ok_or_else(|| /* session not found */)?;
    
    // Update selection state
    for node_id in &request.node_ids {
        update_node_selection(&mut session.root_nodes, node_id, request.selected);
    }
    
    // Collect selected nodes
    session.selected_nodes.clear();
    OpcUaNode::collect_selected_variable_nodes(&session.root_nodes, &mut session.selected_nodes);
    
    session.last_accessed = Instant::now();
    
    Ok(Json(SuccessResponse {
        success: true,
        message: format!("{} nodes selected", session.selected_nodes.len()),
    }))
}

// Helper to find node in tree recursively
fn find_node_in_tree(nodes: &[OpcUaNode], node_id: &str) -> Option<OpcUaNode> {
    for node in nodes {
        if node.node_id.to_string() == node_id {
            return Some(node.clone());
        }
        if let Some(found) = find_node_in_tree(&node.children, node_id) {
            return Some(found);
        }
    }
    None
}
```

**Tasks:**
- [ ] Add session storage with `lazy_static` and `parking_lot::RwLock`
- [ ] Implement `connect_opcua` endpoint (browse root)
- [ ] Implement `get_nodes` endpoint (get root or load children)
- [ ] Implement `select_nodes` endpoint
- [ ] Add session cleanup background task (remove sessions older than 30 min)
- [ ] Add proper error responses
- [ ] Use `spawn_blocking` for all OPC-UA operations

#### Step 3: Build Frontend Tree Component (3-4 hours)

**HTML Structure:**
```html
<div id="opcua-browser-modal" class="modal">
  <div class="modal-content">
    <h2>OPC-UA Browser</h2>
    
    <!-- Connection Form -->
    <div id="connection-form">
      <input type="text" id="opcua-ip" placeholder="192.168.1.100">
      <input type="text" id="opcua-username" placeholder="Username">
      <input type="password" id="opcua-password" placeholder="Password">
      <label><input type="checkbox" id="anonymous"> Anonymous</label>
      <button onclick="connectOpcUa()">Connect</button>
    </div>
    
    <!-- Tree View -->
    <div id="tree-container" class="tree-view">
      <!-- Dynamically generated tree -->
    </div>
    
    <!-- Selected Nodes -->
    <div id="selected-nodes">
      <h3>Selected Nodes</h3>
      <ul id="selected-list"></ul>
    </div>
    
    <button onclick="confirmSelection()">Use Selected Nodes</button>
  </div>
</div>
```

**JavaScript Tree Logic:**
```javascript
class OpcUaTree {
    constructor() {
        this.sessionId = null;
        this.selectedNodes = new Set();
    }
    
    async connect(ip, username, password, anonymous) {
        const response = await fetch('/api/opcua/connect', {
            method: 'POST',
            body: JSON.stringify({ opcua_ip: ip, ... })
        });
        const data = await response.json();
        this.sessionId = data.session_id;
        await this.loadRootNodes();
    }
    
    async loadRootNodes() {
        const nodes = await this.browseNodes(null);
        this.renderNodes(nodes, document.getElementById('tree-container'));
    }
    
    async browseNodes(nodeId) {
        const url = `/api/opcua/browse?session_id=${this.sessionId}` +
                    (nodeId ? `&node_id=${nodeId}` : '');
        const response = await fetch(url);
        return await response.json();
    }
    
    renderNodes(nodes, container) {
        nodes.forEach(node => {
            const nodeEl = this.createNodeElement(node);
            container.appendChild(nodeEl);
        });
    }
    
    createNodeElement(node) {
        const div = document.createElement('div');
        div.className = 'tree-node';
        
        // Expand/collapse icon
        if (node.has_children) {
            const icon = document.createElement('span');
            icon.className = 'expand-icon';
            icon.textContent = '▶';
            icon.onclick = () => this.toggleNode(node, div);
            div.appendChild(icon);
        }
        
        // Checkbox for selection
        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.onchange = () => this.toggleSelection(node);
        div.appendChild(checkbox);
        
        // Node label
        const label = document.createElement('span');
        label.textContent = node.display_name;
        div.appendChild(label);
        
        return div;
    }
    
    async toggleNode(node, element) {
        if (!node.children_loaded) {
            const children = await this.browseNodes(node.node_id);
            const childContainer = document.createElement('div');
            childContainer.className = 'tree-children';
            this.renderNodes(children, childContainer);
            element.appendChild(childContainer);
        }
        element.classList.toggle('expanded');
    }
    
    toggleSelection(node) {
        if (this.selectedNodes.has(node.node_id)) {
            this.selectedNodes.delete(node.node_id);
        } else {
            this.selectedNodes.add(node.node_id);
        }
        this.updateSelectedList();
    }
}
```

**CSS Styling:**
```css
.tree-view {
    max-height: 500px;
    overflow-y: auto;
    border: 1px solid #ccc;
    padding: 10px;
}

.tree-node {
    padding: 4px;
    cursor: pointer;
    user-select: none;
}

.tree-node:hover {
    background: #f0f0f0;
}

.tree-children {
    margin-left: 20px;
    display: none;
}

.tree-node.expanded > .tree-children {
    display: block;
}

.expand-icon {
    display: inline-block;
    width: 16px;
    transition: transform 0.2s;
}

.tree-node.expanded > .expand-icon {
    transform: rotate(90deg);
}
```

**Tasks:**
- [ ] Create modal HTML structure
- [ ] Implement OpcUaTree class
- [ ] Add lazy loading on node expansion
- [ ] Add node selection with checkboxes
- [ ] Add search/filter functionality
- [ ] Add loading indicators
- [ ] Add error handling
- [ ] Style the tree view

---

## Summary: Practical Implementation Plan

**Total Time: 5-7 hours**

### Phase 1: Backend Cleanup (1 hour)
- Audit `opcua_poller.rs` for unused functions
- Document the 8 core functions we actually use
- No async refactoring - keep what works

### Phase 2: API Endpoints (2-3 hours)
- Use `tokio::spawn_blocking` to wrap sync OPC-UA calls
- Three endpoints: `connect`, `get-nodes`, `select-nodes`
- In-memory session storage with cleanup
- Same pattern as GUI's worker thread (proven to work)

### Phase 3: Frontend Tree (2-3 hours)
- Modal with connection form
- Tree component with lazy loading
- Checkbox selection
- Integrate with existing config generation

**Key Principle**: Don't refactor what works. The sync OPC-UA code is battle-tested with real Siemens servers. Just wrap it properly in `spawn_blocking` and build the UI on top.

## Testing Strategy

### Backend Tests
**No new tests needed** - the OPC-UA functions are already tested via GUI usage with real servers.

### API Tests
```bash
# Manual testing with curl
curl -X POST http://localhost:8000/api/opcua/connect \
  -H "Content-Type: application/json" \
  -d '{"opcua_ip":"192.168.1.100","anonymous":true}'

curl -X POST http://localhost:8000/api/opcua/get-nodes \
  -H "Content-Type: application/json" \
  -d '{"session_id":"xxx","node_id":null}'
```

### Frontend Tests
- **Manual testing with real Siemens OPC-UA server** (when available)
- Test lazy loading on node expansion
- Test selection and deselection
- Test error handling (connection failures, timeouts)
- **No fake server testing** - waste of time without real data

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Blocking async runtime** | High | Use `spawn_blocking` for all OPC-UA ops |
| **Session leaks** | Medium | Implement TTL and cleanup task |
| **Large tree memory** | Medium | Lazy loading + pagination |
| **Connection timeouts** | Medium | Proper timeout handling + retry logic |
| **Concurrent sessions** | Low | Session isolation with unique IDs |

## Success Criteria

- [ ] Can connect to OPC-UA server from browser
- [ ] Can browse node tree with lazy loading
- [ ] Can select multiple nodes
- [ ] Selected nodes appear in config generation
- [ ] No blocking of async runtime
- [ ] Proper error handling and user feedback
- [ ] Works with both anonymous and authenticated connections
- [ ] Session cleanup prevents memory leaks

## Next Steps

### Immediate Actions

1. **Audit Backend** (30 min)
   ```bash
   # Find all usages of OpcUaPoller functions
   cd src
   rg "OpcUaPoller::" --type rust
   rg "\.browse_complete_structure" --type rust
   rg "\.load_node_children" --type rust
   rg "\.get_namespace_info" --type rust
   ```
   - Document which functions are actually called
   - Mark unused functions with `#[allow(dead_code)]` or delete

2. **Start with One Endpoint** (1-2 hours)
   - Implement just `POST /api/opcua/connect`
   - Test with `spawn_blocking`
   - Verify it works with real server before continuing

3. **Build Minimal Frontend** (1 hour)
   - Just connection form + display root nodes
   - No tree yet, just a list
   - Prove the concept works

4. **Iterate**
   - Add `get-nodes` endpoint + lazy loading
   - Add `select-nodes` endpoint + checkboxes
   - Polish UI

### Don't Do
- ❌ Don't refactor to async
- ❌ Don't write tests for fake servers
- ❌ Don't build the whole thing at once
- ❌ Don't touch working code

### Do
- ✅ Use `spawn_blocking` for all OPC-UA calls
- ✅ Copy patterns from GUI (it works!)
- ✅ Test incrementally with real server
- ✅ Keep it simple

## Questions to Answer

1. **Session storage**: In-memory HashMap or Redis?
   - **Answer**: Start with in-memory, add Redis later if needed

2. **Pagination**: How many nodes per request?
   - **Answer**: 100 nodes per page, configurable

3. **Search**: Client-side or server-side?
   - **Answer**: Client-side for now, server-side later

4. **Authentication**: API key or session-based?
   - **Answer**: Session-based, API key for other endpoints

5. **Caching**: Cache node trees?
   - **Answer**: No caching initially, add if performance issues
