use std::sync::Arc;
use tracing::info;

use plc_service::{AppState, ButtonsConfig, PlcClientManager};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("plc_service=debug,tower_http=debug")
        .init();

    info!("Starting PLC Service...");

    let config_path = std::env::var("BUTTONS_CONFIG").unwrap_or_else(|_| "/config/plc/buttons.toml".to_string());

    info!("Loading config from: {}", config_path);

    let config = match ButtonsConfig::load(&config_path) {
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

    let app = plc_service::routes::create_routes(state);

    let addr = std::env::var("PLC_SERVICE_ADDR").unwrap_or_else(|_| "0.0.0.0:8001".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    info!("PLC Service listening on {}", addr);

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}