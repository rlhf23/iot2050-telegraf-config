use axum::{
    extract::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use sie_generate_config::backend::{
    generate_dashboard as generate_dashboard_json,
    sanitize_uid,
    DashboardConfig,
};

#[derive(Deserialize)]
pub struct GenerateDashboardRequest {
    pub measurements: Vec<String>,
    #[serde(default = "default_bucket")]
    pub bucket: String,
    #[serde(default = "default_datasource")]
    pub datasource_uid: String,
}

fn default_bucket() -> String {
    "telegraf".to_string()
}

fn default_datasource() -> String {
    "InfluxDB".to_string()
}

#[derive(Serialize)]
pub struct GenerateDashboardResponse {
    pub success: bool,
    pub message: String,
    pub dashboard_json: Option<String>,
    pub dashboard_uid: Option<String>,
}

#[derive(Deserialize)]
pub struct DeployDashboardRequest {
    pub dashboard_json: String,
}

#[derive(Serialize)]
pub struct DeployDashboardResponse {
    pub success: bool,
    pub message: String,
}

/// Generate a Grafana dashboard from measurements
pub async fn generate_dashboard(
    Json(request): Json<GenerateDashboardRequest>,
) -> Result<Json<GenerateDashboardResponse>, (StatusCode, Json<GenerateDashboardResponse>)> {
    info!("Generating dashboard for {} measurement(s)", request.measurements.len());

    if request.measurements.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(GenerateDashboardResponse {
                success: false,
                message: "At least one measurement is required".to_string(),
                dashboard_json: None,
                dashboard_uid: None,
            }),
        ));
    }

    // Use first measurement as primary for dashboard UID
    let primary_measurement = &request.measurements[0];
    let dashboard_uid = sanitize_uid(primary_measurement);

    let config = DashboardConfig {
        uid: dashboard_uid.clone(),
        title: format!("{} Monitor", primary_measurement),
        measurements: request.measurements.clone(),
        bucket: request.bucket,
        datasource_uid: request.datasource_uid,
    };

    match generate_dashboard_json(&config, None) {
        Ok(dashboard_json) => {
            info!("Successfully generated dashboard: {}", dashboard_uid);
            Ok(Json(GenerateDashboardResponse {
                success: true,
                message: format!("Dashboard '{}' generated successfully", dashboard_uid),
                dashboard_json: Some(dashboard_json),
                dashboard_uid: Some(dashboard_uid),
            }))
        }
        Err(e) => {
            error!("Failed to generate dashboard: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(GenerateDashboardResponse {
                    success: false,
                    message: format!("Failed to generate dashboard: {}", e),
                    dashboard_json: None,
                    dashboard_uid: None,
                }),
            ))
        }
    }
}

/// Deploy dashboard to Grafana via API
pub async fn deploy_dashboard(
    Json(request): Json<DeployDashboardRequest>,
) -> Result<Json<DeployDashboardResponse>, (StatusCode, Json<DeployDashboardResponse>)> {
    info!("Deploying dashboard to Grafana");

    // Grafana API endpoint (running in same docker network)
    let grafana_url = std::env::var("GRAFANA_URL")
        .unwrap_or_else(|_| "http://grafana:3000".to_string());
    
    let grafana_user = std::env::var("GRAFANA_ADMIN_USER")
        .unwrap_or_else(|_| "admin".to_string());
    let grafana_password = std::env::var("GRAFANA_ADMIN_PASSWORD")
        .unwrap_or_else(|_| "admin".to_string());

    // Deploy via HTTP POST to Grafana API
    let client = reqwest::Client::new();
    let url = format!("{}/api/dashboards/db", grafana_url);
    
    let response = client
        .post(&url)
        .basic_auth(&grafana_user, Some(&grafana_password))
        .header("Content-Type", "application/json")
        .body(request.dashboard_json)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            
            if status.is_success() {
                info!("Dashboard deployed successfully to Grafana");
                Ok(Json(DeployDashboardResponse {
                    success: true,
                    message: "Dashboard deployed successfully to Grafana".to_string(),
                }))
            } else {
                error!("Grafana returned error: {} - {}", status, body);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(DeployDashboardResponse {
                        success: false,
                        message: format!("Grafana API error: {}", status),
                    }),
                ))
            }
        }
        Err(e) => {
            error!("Failed to connect to Grafana: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DeployDashboardResponse {
                    success: false,
                    message: format!("Failed to connect to Grafana: {}", e),
                }),
            ))
        }
    }
}