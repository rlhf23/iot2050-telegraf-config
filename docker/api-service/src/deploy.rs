use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info};

use crate::AppState;

#[derive(Deserialize)]
pub struct DeployConfigRequest {
    pub session_id: String,
    pub config_path: String,
}

#[derive(Serialize)]
pub struct DeployConfigResponse {
    pub success: bool,
    pub message: String,
    pub telegraf_status: Option<String>,
}

/// Deploy configuration to Telegraf container
pub async fn deploy_config(
    State(state): State<AppState>,
    Json(request): Json<DeployConfigRequest>,
) -> Result<Json<DeployConfigResponse>, (StatusCode, Json<DeployConfigResponse>)> {
    info!("Deploying config for session: {}", request.session_id);

    // Verify the config file exists
    let config_path = PathBuf::from(&request.config_path);
    if !config_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(DeployConfigResponse {
                success: false,
                message: "Configuration file not found".to_string(),
                telegraf_status: None,
            }),
        ));
    }

    // Read the generated config
    let config_content = match fs::read_to_string(&config_path).await {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read config file: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DeployConfigResponse {
                    success: false,
                    message: "Failed to read configuration file".to_string(),
                    telegraf_status: None,
                }),
            ));
        }
    };

    // Define the target path where Telegraf reads config
    // This should be set via TELEGRAF_CONFIG_PATH env var in docker-compose
    // which mounts ${HOME}/telegraf to /telegraf in the container
    let target_path = std::env::var("TELEGRAF_CONFIG_PATH")
        .unwrap_or_else(|_| "/telegraf/telegraf.conf".to_string());

    // Write the config to the target location
    match fs::write(&target_path, &config_content).await {
        Ok(_) => {
            info!("Successfully wrote config to: {}", target_path);
        }
        Err(e) => {
            error!("Failed to write config to {}: {}", target_path, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DeployConfigResponse {
                    success: false,
                    message: format!("Failed to write configuration to target location: {}", e),
                    telegraf_status: None,
                }),
            ));
        }
    }

    // Restart the Telegraf container using Docker API
    use bollard::container::RestartContainerOptions;
    
    let container_name = "telegraf";
    let restart_options = RestartContainerOptions { t: 10 }; // 10 second timeout

    match state.docker.restart_container(container_name, Some(restart_options)).await {
        Ok(_) => {
            info!("Successfully restarted Telegraf container");
            Ok(Json(DeployConfigResponse {
                success: true,
                message: "Configuration deployed and Telegraf restarted successfully".to_string(),
                telegraf_status: Some("restarted".to_string()),
            }))
        }
        Err(e) => {
            error!("Failed to restart Telegraf container: {}", e);
            // Config was written but restart failed
            Ok(Json(DeployConfigResponse {
                success: true,
                message: format!("Configuration deployed but failed to restart Telegraf: {}", e),
                telegraf_status: Some("restart_failed".to_string()),
            }))
        }
    }
}
