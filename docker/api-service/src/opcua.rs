use axum::{
    extract::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

// Import from main project
use sie_generate_config::{TelegrafConfig, backend::opcua_poller::{OpcUaPoller, OpcUaNode}};

#[derive(Deserialize)]
pub struct PollNamespacesRequest {
    pub session_id: String,
    pub opcua_ip: String,
    pub opcua_username: Option<String>,
    pub opcua_password: Option<String>,
    pub anonymous: bool,
    pub filenames: Vec<String>,
}

#[derive(Serialize)]
pub struct NamespaceMapping {
    pub filename: String,
    pub namespace: String,
    pub namespace_index: u16,
}

#[derive(Serialize)]
pub struct PollNamespacesResponse {
    pub success: bool,
    pub message: String,
    pub mappings: Vec<NamespaceMapping>,
}

/// Poll OPC-UA server for namespace information using the main project's OpcUaPoller
pub async fn poll_namespaces(
    Json(request): Json<PollNamespacesRequest>,
) -> Result<Json<PollNamespacesResponse>, (StatusCode, Json<PollNamespacesResponse>)> {
    info!("Polling OPC-UA server for namespaces: {}", request.opcua_ip);

    // Create TelegrafConfig for OpcUaPoller
    let config = TelegrafConfig {
        ip: request.opcua_ip.clone(),
        username: if request.anonymous {
            String::new()
        } else {
            request.opcua_username.unwrap_or_default()
        },
        password: if request.anonymous {
            String::new()
        } else {
            request.opcua_password.unwrap_or_default()
        },
        folder: std::path::PathBuf::new(), // Not needed for namespace polling
        iot_host: String::new(),
        iot_username: String::new(),
        iot_password: String::new(),
        output_format: None,
        include_test_inputs: false,
        selected_opcua_nodes: vec![],
        listener_files: vec![],
    };

    // Create OpcUaPoller
    let poller = match OpcUaPoller::new(config) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create OPC-UA poller: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PollNamespacesResponse {
                    success: false,
                    message: format!("Failed to create OPC-UA poller: {}", e),
                    mappings: vec![],
                }),
            ));
        }
    };

    // Get namespace information for the uploaded files
    let namespace_map = match poller.get_namespace_info(&request.filenames) {
        Ok(map) => map,
        Err(e) => {
            error!("Failed to get namespace info: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PollNamespacesResponse {
                    success: false,
                    message: format!("Failed to get namespace info from OPC-UA server: {}", e),
                    mappings: vec![],
                }),
            ));
        }
    };

    // Convert to response format
    let mut mappings = Vec::new();
    for filename in &request.filenames {
        let namespace_index = namespace_map.get(filename).copied().unwrap_or(0);
        mappings.push(NamespaceMapping {
            filename: filename.clone(),
            namespace: namespace_index.to_string(),
            namespace_index,
        });
    }

    info!("Successfully mapped {} file(s) to namespaces", mappings.len());

    Ok(Json(PollNamespacesResponse {
        success: true,
        message: format!("Successfully retrieved namespaces from OPC-UA server"),
        mappings,
    }))
}

// ============================================================================
// OPC-UA Browser Session Management
// ============================================================================

/// Session data for OPC-UA browser
#[derive(Clone)]
struct OpcUaBrowserSession {
    config: TelegrafConfig,
    root_nodes: Vec<OpcUaNode>,
    created_at: Instant,
    last_accessed: Instant,
}

/// Global session storage
type SessionStore = Arc<RwLock<HashMap<String, OpcUaBrowserSession>>>;

lazy_static::lazy_static! {
    static ref OPCUA_SESSIONS: SessionStore = Arc::new(RwLock::new(HashMap::new()));
}

// ============================================================================
// OPC-UA Browser API Endpoints
// ============================================================================

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
    pub node_count: usize,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
}

/// POST /api/opcua/connect - Connect to OPC-UA server and browse root nodes
pub async fn connect_opcua(
    Json(request): Json<ConnectRequest>,
) -> Result<Json<ConnectResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Connecting to OPC-UA server: {}", request.opcua_ip);

    // Create TelegrafConfig
    let config = TelegrafConfig {
        ip: request.opcua_ip.clone(),
        username: if request.anonymous {
            String::new()
        } else {
            request.opcua_username.unwrap_or_default()
        },
        password: if request.anonymous {
            String::new()
        } else {
            request.opcua_password.unwrap_or_default()
        },
        folder: std::path::PathBuf::new(),
        iot_host: String::new(),
        iot_username: String::new(),
        iot_password: String::new(),
        output_format: None,
        include_test_inputs: false,
        selected_opcua_nodes: vec![],
        listener_files: vec![],
    };

    // Browse root nodes in blocking task (same pattern as GUI worker)
    let config_clone = config.clone();
    let root_nodes = tokio::task::spawn_blocking(move || {
        let poller = OpcUaPoller::new(config_clone)?;
        poller.browse_complete_structure()
    })
    .await
    .map_err(|e| {
        error!("Task join error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                message: format!("Internal error: {}", e),
            }),
        )
    })?
    .map_err(|e| {
        error!("OPC-UA browse error: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                message: format!("Failed to browse OPC-UA server: {}", e),
            }),
        )
    })?;

    // Create session
    let session_id = uuid::Uuid::new_v4().to_string();
    let node_count = root_nodes.len();
    
    let session = OpcUaBrowserSession {
        config,
        root_nodes,
        created_at: Instant::now(),
        last_accessed: Instant::now(),
    };

    OPCUA_SESSIONS.write().await.insert(session_id.clone(), session);

    info!("OPC-UA session created: {} ({} root nodes)", session_id, node_count);

    Ok(Json(ConnectResponse {
        success: true,
        session_id,
        message: format!("Connected successfully, found {} root nodes", node_count),
        node_count,
    }))
}

#[derive(Deserialize)]
pub struct GetNodesRequest {
    pub session_id: String,
    pub node_id: Option<String>, // None = get root nodes, Some = load children
}

#[derive(Serialize)]
pub struct GetNodesResponse {
    pub success: bool,
    pub nodes: Vec<OpcUaNode>,
}

/// POST /api/opcua/get-nodes - Get root nodes or load children for a specific node
pub async fn get_nodes(
    Json(request): Json<GetNodesRequest>,
) -> Result<Json<GetNodesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut sessions = OPCUA_SESSIONS.write().await;
    let session = sessions.get_mut(&request.session_id).ok_or_else(|| {
        error!("Session not found: {}", request.session_id);
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                message: "Session not found or expired".to_string(),
            }),
        )
    })?;

    // Update last accessed time
    session.last_accessed = Instant::now();

    // If no node_id, return root nodes
    if request.node_id.is_none() {
        info!("Returning {} root nodes for session {}", session.root_nodes.len(), request.session_id);
        return Ok(Json(GetNodesResponse {
            success: true,
            nodes: session.root_nodes.clone(),
        }));
    }

    // Load children for specific node
    let node_id_str = request.node_id.unwrap();
    info!("Loading children for node: {}", node_id_str);

    // Find the parent node in the tree
    let parent_node = find_node_in_tree(&session.root_nodes, &node_id_str).ok_or_else(|| {
        error!("Node not found: {}", node_id_str);
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                message: format!("Node not found: {}", node_id_str),
            }),
        )
    })?;

    // Load children in blocking task
    let config = session.config.clone();
    let children = tokio::task::spawn_blocking(move || {
        let poller = OpcUaPoller::new(config)?;
        poller.load_node_children(&parent_node, 1)
    })
    .await
    .map_err(|e| {
        error!("Task join error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                message: format!("Internal error: {}", e),
            }),
        )
    })?
    .map_err(|e| {
        error!("Failed to load children: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                message: format!("Failed to load node children: {}", e),
            }),
        )
    })?;

    info!("Loaded {} children for node {}", children.len(), node_id_str);

    Ok(Json(GetNodesResponse {
        success: true,
        nodes: children,
    }))
}

/// Helper function to find a node in the tree recursively
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
