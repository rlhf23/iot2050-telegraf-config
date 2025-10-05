use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, delete},
    Router,
};
use bollard::Docker;
use futures::stream::TryStreamExt;
use serde::Serialize;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, error};

mod config;
mod generator;
mod deploy;
mod opcua;

#[derive(Clone)]
pub struct AppState {
    pub docker: Arc<Docker>,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct ContainerInfo {
    pub name: String,
    pub status: String,
    pub state: String,
}

#[derive(Serialize)]
pub struct ContainersResponse {
    pub success: bool,
    pub containers: Vec<ContainerInfo>,
}

// Whitelist of containers that can be managed
const ALLOWED_CONTAINERS: &[&str] = &["grafana", "influxdb", "prometheus", "telegraf"];

pub fn is_allowed_container(name: &str) -> bool {
    ALLOWED_CONTAINERS.contains(&name)
}

// Create the app router - exposed for testing
pub fn create_app() -> Router {
    // Connect to Docker socket
    let docker = match Docker::connect_with_socket_defaults() {
        Ok(docker) => Arc::new(docker),
        Err(e) => {
            eprintln!("Failed to connect to Docker: {}", e);
            std::process::exit(1);
        }
    };

    let state = AppState { docker };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    Router::new()
        .route("/health", get(health_check))
        .route("/api/containers", get(list_containers))
        .route("/api/containers/:name/restart", post(restart_container))
        .route("/api/containers/:name/start", post(start_container))
        .route("/api/containers/:name/stop", post(stop_container))
        .route("/api/containers/:name/logs", get(get_container_logs))
        // Configuration management endpoints
        .route("/api/session/create", get(config::create_session))
        .route("/api/config/upload", post(config::upload_files))
        .route("/api/config/files/:session_id", get(config::list_files))
        .route("/api/config/files/:session_id/:filename", delete(config::delete_file))
        .route("/api/config/generate", post(config::generate_config))
        .route("/api/config/deploy", post(deploy::deploy_config))
        // OPC-UA endpoints
        .route("/api/opcua/poll-namespaces", post(opcua::poll_namespaces))
        .layer(cors)
        .with_state(state)
}

async fn health_check() -> Json<ApiResponse> {
    Json(ApiResponse {
        success: true,
        message: "API service is healthy".to_string(),
    })
}

async fn list_containers(
    State(state): State<AppState>,
) -> Result<Json<ContainersResponse>, StatusCode> {
    use bollard::container::ListContainersOptions;

    let options = Some(ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    });

    match state.docker.list_containers(options).await {
        Ok(containers) => {
            let container_list: Vec<ContainerInfo> = containers
                .into_iter()
                .filter_map(|c| {
                    let name = c.names?.first()?.trim_start_matches('/').to_string();
                    if is_allowed_container(&name) {
                        Some(ContainerInfo {
                            name,
                            status: c.status?,
                            state: c.state?,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            Ok(Json(ContainersResponse {
                success: true,
                containers: container_list,
            }))
        }
        Err(e) => {
            error!("Failed to list containers: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn restart_container(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {
    if !is_allowed_container(&name) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse {
                success: false,
                message: format!("Container '{}' is not allowed to be restarted", name),
            }),
        ));
    }

    info!("Restarting container: {}", name);

    match state.docker.restart_container(&name, None).await {
        Ok(_) => {
            info!("Successfully restarted container: {}", name);
            Ok(Json(ApiResponse {
                success: true,
                message: format!("Container '{}' restarted successfully", name),
            }))
        }
        Err(e) => {
            error!("Failed to restart container {}: {}", name, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    message: format!("Failed to restart container: {}", e),
                }),
            ))
        }
    }
}

async fn start_container(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {
    if !is_allowed_container(&name) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse {
                success: false,
                message: format!("Container '{}' is not allowed to be started", name),
            }),
        ));
    }

    info!("Starting container: {}", name);

    match state.docker.start_container::<String>(&name, None).await {
        Ok(_) => {
            info!("Successfully started container: {}", name);
            Ok(Json(ApiResponse {
                success: true,
                message: format!("Container '{}' started successfully", name),
            }))
        }
        Err(e) => {
            error!("Failed to start container {}: {}", name, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    message: format!("Failed to start container: {}", e),
                }),
            ))
        }
    }
}

async fn stop_container(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {
    if !is_allowed_container(&name) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse {
                success: false,
                message: format!("Container '{}' is not allowed to be stopped", name),
            }),
        ));
    }

    info!("Stopping container: {}", name);

    match state.docker.stop_container(&name, None).await {
        Ok(_) => {
            info!("Successfully stopped container: {}", name);
            Ok(Json(ApiResponse {
                success: true,
                message: format!("Container '{}' stopped successfully", name),
            }))
        }
        Err(e) => {
            error!("Failed to stop container {}: {}", name, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    message: format!("Failed to stop container: {}", e),
                }),
            ))
        }
    }
}

async fn get_container_logs(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<String, (StatusCode, Json<ApiResponse>)> {
    if !is_allowed_container(&name) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse {
                success: false,
                message: format!("Container '{}' is not allowed", name),
            }),
        ));
    }

    info!("Fetching logs for container: {}", name);

    use bollard::container::LogsOptions;

    let options = Some(LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail: "200".to_string(),
        ..Default::default()
    });

    match state.docker.logs(&name, options).try_collect::<Vec<_>>().await {
        Ok(logs) => {
            use bollard::container::LogOutput;
            let log_text: String = logs
                .into_iter()
                .map(|log| match log {
                    LogOutput::StdOut { message } => String::from_utf8_lossy(&message).to_string(),
                    LogOutput::StdErr { message } => String::from_utf8_lossy(&message).to_string(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("");
            
            info!("Successfully fetched logs for container: {}", name);
            Ok(log_text)
        }
        Err(e) => {
            error!("Failed to fetch logs for container {}: {}", name, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    success: false,
                    message: format!("Failed to fetch logs: {}", e),
                }),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_allowed_container() {
        // Test allowed containers
        assert!(is_allowed_container("grafana"));
        assert!(is_allowed_container("influxdb"));
        assert!(is_allowed_container("prometheus"));
        assert!(is_allowed_container("telegraf"));

        // Test disallowed containers
        assert!(!is_allowed_container("nginx"));
        assert!(!is_allowed_container("api-service"));
        assert!(!is_allowed_container("random-container"));
        assert!(!is_allowed_container(""));
    }

    #[test]
    fn test_allowed_containers_list() {
        // Verify the whitelist contains expected containers
        assert_eq!(ALLOWED_CONTAINERS.len(), 4);
        assert!(ALLOWED_CONTAINERS.contains(&"grafana"));
        assert!(ALLOWED_CONTAINERS.contains(&"influxdb"));
        assert!(ALLOWED_CONTAINERS.contains(&"prometheus"));
        assert!(ALLOWED_CONTAINERS.contains(&"telegraf"));
    }
}
