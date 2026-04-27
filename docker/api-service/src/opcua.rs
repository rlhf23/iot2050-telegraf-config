use axum::{
    extract::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use sie_generate_config::{OpcUaConnectionConfig, backend::opcua_poller::OpcUaPoller};

fn ensure_opcua_port(ip: String) -> String {
    if ip.contains(':') {
        ip
    } else {
        format!("{}:4840", ip)
    }
}

#[derive(Deserialize)]
pub struct PlcTimeRequest {
    pub opcua_ip: String,
    pub opcua_username: Option<String>,
    pub opcua_password: Option<String>,
    pub anonymous: Option<bool>,
}

#[derive(Serialize)]
pub struct PlcTimeResponse {
    pub success: bool,
    pub message: String,
    pub plc_time: Option<String>,
    pub plc_time_offset_ms: Option<i64>,
}

pub async fn plc_time(
    Json(request): Json<PlcTimeRequest>,
) -> Result<Json<PlcTimeResponse>, (StatusCode, Json<PlcTimeResponse>)> {
    info!("Reading PLC time from {}", request.opcua_ip);

    let config = OpcUaConnectionConfig {
        ip: ensure_opcua_port(request.opcua_ip.clone()),
        username: if request.anonymous.unwrap_or(false) || request.opcua_username.is_none() {
            String::new()
        } else {
            request.opcua_username.unwrap_or_default()
        },
        password: if request.anonymous.unwrap_or(false) || request.opcua_password.is_none() {
            String::new()
        } else {
            request.opcua_password.unwrap_or_default()
        },
    };

    match tokio::task::spawn_blocking(move || {
        let poller = OpcUaPoller::new(config)?;
        poller.read_current_time()
    }).await {
        Ok(Ok((plc_time, offset_ms))) => {
            Ok(Json(PlcTimeResponse {
                success: true,
                message: "PLC time retrieved successfully".to_string(),
                plc_time: Some(plc_time.format("%Y-%m-%d %H:%M:%S").to_string()),
                plc_time_offset_ms: Some(offset_ms),
            }))
        }
        Ok(Err(e)) => {
            error!("Failed to read PLC time: {}", e);
            Err((
                StatusCode::BAD_GATEWAY,
                Json(PlcTimeResponse {
                    success: false,
                    message: format!("Failed to read PLC time: {}", e),
                    plc_time: None,
                    plc_time_offset_ms: None,
                }),
            ))
        }
        Err(e) => {
            error!("Task join error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(PlcTimeResponse {
                    success: false,
                    message: format!("Internal error: {}", e),
                    plc_time: None,
                    plc_time_offset_ms: None,
                }),
            ))
        }
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
