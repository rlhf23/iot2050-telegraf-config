use crate::buttons::ButtonsConfig;
use crate::s7_client::{PlcClientManager, PlcStatus};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub client: PlcClientManager,
    pub config: Arc<ButtonsConfig>,
}

#[derive(Debug, Serialize)]
pub struct ButtonInfo {
    pub id: usize,
    pub name: String,
    pub address: String,
    pub mode: String,
    pub momentary_duration_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ButtonsResponse {
    pub success: bool,
    pub buttons: Vec<ButtonInfo>,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub success: bool,
    pub status: PlcStatus,
}

#[derive(Debug, Serialize)]
pub struct ReadResponse {
    pub success: bool,
    pub value: Option<bool>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct WriteRequest {
    pub id: usize,
    pub value: bool,
}

#[derive(Debug, Serialize)]
pub struct WriteResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ToggleResponse {
    pub success: bool,
    pub new_value: Option<bool>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
}

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/plc/buttons", get(list_buttons))
        .route("/api/plc/status", get(get_status))
        .route("/api/plc/read/:id", get(read_button))
        .route("/api/plc/write", post(write_button))
        .route("/api/plc/toggle/:id", post(toggle_button))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state)
}

async fn list_buttons(State(state): State<AppState>) -> Json<ButtonsResponse> {
    let buttons: Vec<ButtonInfo> = state
        .config
        .buttons
        .iter()
        .enumerate()
        .map(|(i, b)| ButtonInfo {
            id: i,
            name: b.name.clone(),
            address: b.address.clone(),
            mode: match b.mode {
                crate::buttons::ButtonMode::Toggle => "toggle".to_string(),
                crate::buttons::ButtonMode::Momentary => "momentary".to_string(),
            },
            momentary_duration_ms: if b.mode == crate::buttons::ButtonMode::Momentary {
                Some(b.momentary_duration_ms)
            } else {
                None
            },
        })
        .collect();

    Json(ButtonsResponse {
        success: true,
        buttons,
    })
}

async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let status = state.client.status().await;
    Json(StatusResponse {
        success: true,
        status,
    })
}

async fn read_button(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> Result<Json<ReadResponse>, (StatusCode, Json<ErrorResponse>)> {
    let button = state
        .config
        .buttons
        .get(id)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                message: format!("Button {} not found", id),
            }),
        ))?;

    let addr = button.parse_address().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                message: e.to_string(),
            }),
        )
    })?;

    match state.client.read_bool(&addr).await {
        Ok(value) => Ok(Json(ReadResponse {
            success: true,
            value: Some(value),
            message: "Read successful".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                message: e.to_string(),
            }),
        )),
    }
}

async fn write_button(
    State(state): State<AppState>,
    Json(payload): Json<WriteRequest>,
) -> Result<Json<WriteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let button = state
        .config
        .buttons
        .get(payload.id)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                message: format!("Button {} not found", payload.id),
            }),
        ))?;

    let addr = button.parse_address().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                message: e.to_string(),
            }),
        )
    })?;

    match state.client.write_bool(&addr, payload.value).await {
        Ok(()) => Ok(Json(WriteResponse {
            success: true,
            message: format!("Written {} to {}", payload.value, button.address),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                message: e.to_string(),
            }),
        )),
    }
}

async fn toggle_button(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> Result<Json<ToggleResponse>, (StatusCode, Json<ErrorResponse>)> {
    let button = state
        .config
        .buttons
        .get(id)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                message: format!("Button {} not found", id),
            }),
        ))?;

    let addr = button.parse_address().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                success: false,
                message: e.to_string(),
            }),
        )
    })?;

    match state.client.toggle_bool(&addr).await {
        Ok(new_value) => Ok(Json(ToggleResponse {
            success: true,
            new_value: Some(new_value),
            message: format!("Toggled {} to {}", button.address, new_value),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                message: e.to_string(),
            }),
        )),
    }
}