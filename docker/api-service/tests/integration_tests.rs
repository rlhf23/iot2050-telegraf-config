use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

// Helper function to create the app for testing
fn create_test_app() -> axum::Router {
    // Note: These tests will only work if Docker is available
    // For CI/CD, you might want to mock the Docker client
    docker_api_service::create_app()
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["message"].as_str().unwrap().contains("healthy"));
}

#[tokio::test]
async fn test_list_containers_endpoint() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/containers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 200 even if no containers are running
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["containers"].is_array());
}

#[tokio::test]
async fn test_restart_forbidden_container() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/containers/nginx/restart")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().contains("not allowed"));
}

#[tokio::test]
async fn test_start_forbidden_container() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/containers/random-container/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_stop_forbidden_container() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/containers/api-service/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_restart_allowed_container_not_found() {
    let app = create_test_app();

    // Try to restart an allowed container that doesn't exist
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/containers/grafana/restart")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 500 if container doesn't exist
    // or 200 if it does exist and was restarted
    assert!(
        response.status() == StatusCode::INTERNAL_SERVER_ERROR
            || response.status() == StatusCode::OK
    );
}

#[tokio::test]
async fn test_get_logs_forbidden_container() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/containers/nginx/logs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_logs_allowed_container() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/containers/grafana/logs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 200 if container exists
    // or 500 if it doesn't exist
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

// ============================================================================
// Config Generation Tests
// ============================================================================
// Note: These tests are disabled because they require shared state between
// app instances. The config generation logic is tested via unit tests in
// src/config_test.rs instead.

#[tokio::test]
async fn test_create_session() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/session/create")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], true);
    assert!(json["session_id"].is_string());
    assert!(!json["session_id"].as_str().unwrap().is_empty());
}

// Disabled: Requires shared state between app instances
// See src/config_test.rs for unit tests of config generation logic
/*
#[tokio::test]
async fn test_generate_config_minimal() { ... }

#[tokio::test]
async fn test_generate_config_with_iot_credentials() { ... }
*/

#[tokio::test]
async fn test_generate_config_invalid_session() {
    let app = create_test_app();

    let request_body = json!({
        "session_id": "invalid-session-id",
        "opcua_ip": "192.168.1.100",
        "anonymous": true,
        "output_format": "influxdb",
        "file_configs": []
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/config/generate")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["message"].as_str().unwrap().contains("not found"));
}
