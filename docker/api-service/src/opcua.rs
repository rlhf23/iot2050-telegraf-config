use axum::{
    extract::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

// Import from main project
use sie_generate_config::{TelegrafConfig, backend::opcua_poller::OpcUaPoller};

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
