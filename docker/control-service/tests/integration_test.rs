use axum::Router;
use control_service::{AppState, ControlConfig, PlcClientManager};
use std::sync::Arc;
use tower::util::ServiceExt;

fn create_test_config() -> ControlConfig {
    ControlConfig {
        plc: control_service::config::PlcConfig {
            ip: "127.0.0.1".to_string(),
            rack: 0,
            slot: 1,
            connection_timeout_ms: 10,
            read_timeout_ms: 10,
            write_timeout_ms: 10,
        },
        buttons: vec![
            control_service::config::ButtonDef {
                name: "TestButton1".to_string(),
                address: "DB1.DBX0.0".to_string(),
                mode: control_service::config::ButtonMode::Toggle,
                momentary_duration_ms: 500,
            },
            control_service::config::ButtonDef {
                name: "TestButton2".to_string(),
                address: "DB1.DBX0.1".to_string(),
                mode: control_service::config::ButtonMode::Momentary,
                momentary_duration_ms: 200,
            },
        ],
    }
}

fn create_test_app() -> Router {
    let config = create_test_config();
    let client = PlcClientManager::new(config.plc.clone());
    let state = AppState {
        client,
        config: Arc::new(config),
    };
    control_service::routes::create_routes(state)
}

#[tokio::test]
async fn test_config_loads_successfully() {
    let config = create_test_config();
    assert_eq!(config.buttons.len(), 2);
    assert_eq!(config.plc.ip, "127.0.0.1");
    assert_eq!(config.plc.read_timeout_ms, 10);
}

#[tokio::test]
async fn test_buttons_endpoint_works() {
    use axum::http::StatusCode;

    let app = create_test_app();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/control/buttons")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["buttons"].as_array().unwrap().len(), 2);
    assert_eq!(json["buttons"][0]["name"], "TestButton1");
    assert_eq!(json["buttons"][0]["mode"], "toggle");
    assert_eq!(json["buttons"][1]["mode"], "momentary");
}

#[tokio::test]
async fn test_status_endpoint_returns_connected_false_when_plc_unreachable() {
    use axum::http::StatusCode;

    let app = create_test_app();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/control/status")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["success"].as_bool().unwrap());
    assert!(!json["status"]["connected"].as_bool().unwrap());
    assert_eq!(json["status"]["ip"], "127.0.0.1");
}

#[tokio::test]
async fn test_read_endpoint_returns_error_when_plc_unreachable() {
    use axum::http::StatusCode;

    let app = create_test_app();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/control/read/0")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(!json["success"].as_bool().unwrap());
    assert!(json["message"].as_str().unwrap().contains("error"));
}

#[tokio::test]
async fn test_write_endpoint_returns_error_when_plc_unreachable() {
    use axum::http::StatusCode;

    let app = create_test_app();
    let body_str = r#"{"id": 0, "value": true}"#;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/control/write")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(body_str.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(!json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_toggle_endpoint_returns_error_when_plc_unreachable() {
    use axum::http::StatusCode;

    let app = create_test_app();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/control/toggle/0")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_read_invalid_button_id_returns_404() {
    use axum::http::StatusCode;

    let app = create_test_app();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/control/read/999")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_write_invalid_button_id_returns_404() {
    use axum::http::StatusCode;

    let app = create_test_app();
    let body_str = r#"{"id": 999, "value": true}"#;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/control/write")
                .header("Content-Type", "application/json")
                .body(axum::body::Body::from(body_str.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_multiple_requests_dont_hang() {
    use std::time::Instant;

    let app = create_test_app();

    let start = Instant::now();

    for _ in 0..5 {
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/control/status")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.status().is_success());
    }

    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 2000,
        "5 requests took {:?}, should be fast even with unreachable PLC",
        elapsed
    );
}

mod address_parsing {
    use control_service::config::parse_s7_address;

    #[test]
    fn test_valid_addresses() {
        let addr = parse_s7_address("DB1.DBX0.0").unwrap();
        assert_eq!(addr.db, 1);
        assert_eq!(addr.byte, 0);
        assert_eq!(addr.bit, 0);

        let addr = parse_s7_address("DB100.DBX50.7").unwrap();
        assert_eq!(addr.db, 100);
        assert_eq!(addr.byte, 50);
        assert_eq!(addr.bit, 7);

        let addr = parse_s7_address("db1.dbx0.0").unwrap();
        assert_eq!(addr.db, 1);

        let addr = parse_s7_address("DB1.0.0").unwrap();
        assert_eq!(addr.db, 1);
        assert_eq!(addr.byte, 0);
        assert_eq!(addr.bit, 0);
    }

    #[test]
    fn test_invalid_addresses() {
        assert!(parse_s7_address("M0.0").is_err());
        assert!(parse_s7_address("DB1").is_err());
        assert!(parse_s7_address("DB1.DBX0.8").is_err());
        assert!(parse_s7_address("DB1.DBX0.-1").is_err());
        assert!(parse_s7_address("").is_err());
    }
}