use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::Value;
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
