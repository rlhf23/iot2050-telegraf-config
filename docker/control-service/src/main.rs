use std::sync::Arc;
use tracing::info;

use control_service::{AppState, ControlConfig, PlcClientManager};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("control_service=debug,tower_http=debug")
        .init();

    info!("Starting Control Service...");

    let config_path = std::env::var("CONTROL_CONFIG").unwrap_or_else(|_| "/config/control/config.toml".to_string());

    info!("Loading config from: {}", config_path);

    let config = match ControlConfig::load(&config_path) {
        Ok(c) => {
            info!(
                "Loaded {} button(s) for PLC at {}",
                c.buttons.len(),
                c.plc.ip
            );
            c
        }
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    let client = PlcClientManager::new(config.plc.clone());

    let state = AppState {
        client,
        config: Arc::new(config),
    };

    let app = control_service::routes::create_routes(state);

    let addr = std::env::var("CONTROL_SERVICE_ADDR").unwrap_or_else(|_| "0.0.0.0:8002".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    info!("Control Service listening on {}", addr);

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}