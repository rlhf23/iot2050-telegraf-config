use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use bollard::Docker;
use serde::Serialize;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, error};

#[derive(Clone)]
struct AppState {
    docker: Arc<Docker>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

#[derive(Serialize)]
struct ContainerInfo {
    name: String,
    status: String,
    state: String,
}

#[derive(Serialize)]
struct ContainersResponse {
    success: bool,
    containers: Vec<ContainerInfo>,
}

// Whitelist of containers that can be managed
const ALLOWED_CONTAINERS: &[&str] = &["grafana", "influxdb", "prometheus", "telegraf"];

fn is_allowed_container(name: &str) -> bool {
    ALLOWED_CONTAINERS.contains(&name)
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("docker_api_service=info,tower_http=debug")
        .init();

    info!("Starting Docker API Service...");

    // Connect to Docker socket
    let docker = match Docker::connect_with_socket_defaults() {
        Ok(docker) => {
            info!("Successfully connected to Docker socket");
            Arc::new(docker)
        }
        Err(e) => {
            error!("Failed to connect to Docker: {}", e);
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
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/containers", get(list_containers))
        .route("/api/containers/:name/restart", post(restart_container))
        .route("/api/containers/:name/start", post(start_container))
        .route("/api/containers/:name/stop", post(stop_container))
        .layer(cors)
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("Failed to bind to port 8000");

    info!("API Service listening on 0.0.0.0:8000");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
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
