use tracing::info;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("docker_api_service=info,tower_http=debug")
        .init();

    info!("Starting Docker API Service...");

    let app = docker_api_service::create_app();

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .expect("Failed to bind to port 8000");

    info!("API Service listening on 0.0.0.0:8000");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}
