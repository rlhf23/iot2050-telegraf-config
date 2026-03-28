use axum::{
    extract::State,
    extract::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{AppState, SessionDiscoveredData};

// Import from main project
use sie_generate_config::backend::opcua_poller::OpcUaPoller;

/// Ensure OPC-UA IP has port appended (default 4840)
fn ensure_opcua_port(ip: String) -> String {
    if ip.contains(':') {
        ip
    } else {
        format!("{}:4840", ip)
    }
}

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
    info!("Request details: {} file(s), anonymous={}", request.filenames.len(), request.anonymous);

    // Create OpcUaConnectionConfig for namespace polling
    let config = sie_generate_config::OpcUaConnectionConfig {
        ip: ensure_opcua_port(request.opcua_ip.clone()),
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
    };

    // Run OpcUaPoller in a blocking task to avoid runtime-in-runtime issues
    let filenames = request.filenames.clone();
    let namespace_map = match tokio::task::spawn_blocking(move || {
        let poller = OpcUaPoller::new(config)?;
        poller.get_namespace_info(&filenames)
    }).await {
        Ok(Ok(map)) => map,
        Ok(Err(e)) => {
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
        Err(e) => {
            error!("Task join error: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PollNamespacesResponse {
                    success: false,
                    message: format!("Internal error: {}", e),
                    mappings: vec![],
                }),
            ));
        }
    };

    // Convert to response format
    let mut mappings = Vec::new();
    let mut matched_count = 0;
    
    for filename in &request.filenames {
        let namespace_index = namespace_map.get(filename).copied().unwrap_or(0);
        if namespace_index > 0 {
            matched_count += 1;
        }
        mappings.push(NamespaceMapping {
            filename: filename.clone(),
            namespace: namespace_index.to_string(),
            namespace_index,
        });
        info!("  File '{}' -> namespace {}", filename, namespace_index);
    }

    info!("Successfully mapped {} out of {} file(s) to namespaces", matched_count, mappings.len());

    let message = if matched_count == 0 {
        format!("Warning: No files were matched to server namespaces. All files defaulted to namespace 0.")
    } else if matched_count < mappings.len() {
        format!("Partially successful: {} out of {} files matched to server namespaces", matched_count, mappings.len())
    } else {
        format!("Successfully retrieved namespaces for all {} file(s)", mappings.len())
    };

    Ok(Json(PollNamespacesResponse {
        success: true,
        message,
        mappings,
    }))
}

#[derive(Deserialize)]
pub struct DiscoverRequest {
    pub session_id: String,
    pub opcua_ip: String,
    pub opcua_username: Option<String>,
    pub opcua_password: Option<String>,
    pub anonymous: bool,
}

#[derive(Serialize)]
pub struct DiscoveredNamespaceInfo {
    pub index: u16,
    pub name: String,
    pub variable_count: usize,
}

#[derive(Serialize)]
pub struct DiscoverResponse {
    pub success: bool,
    pub message: String,
    pub namespaces: Vec<DiscoveredNamespaceInfo>,
    pub total_variables: usize,
}

/// Discover all namespaces and variables from an OPC-UA server's ServerInterfaces
pub async fn discover_namespaces(
    State(state): State<AppState>,
    Json(request): Json<DiscoverRequest>,
) -> Result<Json<DiscoverResponse>, (StatusCode, Json<DiscoverResponse>)> {
    info!("Discovering namespaces from OPC-UA server: {}", request.opcua_ip);

    // Create OpcUaConnectionConfig for discovery
    let config = sie_generate_config::OpcUaConnectionConfig {
        ip: ensure_opcua_port(request.opcua_ip.clone()),
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
    };

    // Run OpcUaPoller in a blocking task to avoid runtime-in-runtime issues
    let discovered_data = match tokio::task::spawn_blocking(move || {
        let poller = OpcUaPoller::new(config)?;
        poller.discover_all_namespaces()
    }).await {
        Ok(Ok(data)) => data,
        Ok(Err(e)) => {
            error!("Failed to discover namespaces: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DiscoverResponse {
                    success: false,
                    message: format!("Failed to discover namespaces: {}", e),
                    namespaces: vec![],
                    total_variables: 0,
                }),
            ));
        }
        Err(e) => {
            error!("Task join error: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DiscoverResponse {
                    success: false,
                    message: format!("Internal error: {}", e),
                    namespaces: vec![],
                    total_variables: 0,
                }),
            ));
        }
    };

    // Store discovered data in session
    let session_data = SessionDiscoveredData {
        discovered_data: discovered_data.clone(),
        opcua_ip: request.opcua_ip.clone(),
    };
    state.session_store.insert(request.session_id.clone(), session_data);

    // Convert to response format
    let namespaces: Vec<DiscoveredNamespaceInfo> = discovered_data
        .namespaces
        .into_iter()
        .map(|ns| DiscoveredNamespaceInfo {
            index: ns.index,
            name: ns.name,
            variable_count: ns.variable_count,
        })
        .collect();

    let total_variables = namespaces.iter().map(|ns| ns.variable_count).sum();

    info!(
        "Successfully discovered {} namespaces with {} total variables",
        namespaces.len(),
        total_variables
    );

    Ok(Json(DiscoverResponse {
        success: true,
        message: format!(
            "Successfully discovered {} namespaces with {} variables",
            namespaces.len(),
            total_variables
        ),
        namespaces,
        total_variables,
    }))
}
