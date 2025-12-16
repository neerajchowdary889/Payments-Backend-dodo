use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use payments_backend_dodo::controllayer::health::{
    HealthResponse, health_check, liveness, readiness,
};
use serde_json;
use tower::ServiceExt;

/// Helper function to create a test router with health endpoints
fn create_test_router() -> Router {
    Router::new()
        .route("/health", axum::routing::get(health_check))
        .route("/health/live", axum::routing::get(liveness))
        .route("/health/ready", axum::routing::get(readiness))
}

#[tokio::test]
async fn test_health_check_returns_200() {
    let app = create_test_router();

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
}

#[tokio::test]
async fn test_health_check_response_structure() {
    let app = create_test_router();

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

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let health_response: HealthResponse = serde_json::from_slice(&body).unwrap();

    // Verify response structure
    assert_eq!(health_response.status, "healthy");
    assert!(!health_response.version.is_empty());
    assert!(health_response.timestamp > 0);
    assert!(health_response.database.is_some());
}

#[tokio::test]
async fn test_health_check_includes_version() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let health_response: HealthResponse = serde_json::from_slice(&body).unwrap();

    // Version should match Cargo.toml version
    assert_eq!(health_response.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn test_health_check_includes_timestamp() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let health_response: HealthResponse = serde_json::from_slice(&body).unwrap();

    // Timestamp should be recent (within last 10 seconds)
    let now = chrono::Utc::now().timestamp();
    assert!(health_response.timestamp <= now);
    assert!(health_response.timestamp >= now - 10);
}

#[tokio::test]
async fn test_health_check_database_status() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let health_response: HealthResponse = serde_json::from_slice(&body).unwrap();

    // Verify database health is included
    assert!(health_response.database.is_some());
    let db_health = health_response.database.unwrap();
    assert_eq!(db_health.status, "healthy");
    assert!(db_health.latency_ms.is_some());
}

#[tokio::test]
async fn test_liveness_probe_returns_200() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_liveness_probe_no_body() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    // Liveness probe should return empty body
    assert_eq!(body.len(), 0);
}

#[tokio::test]
async fn test_readiness_probe_returns_200() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_readiness_probe_no_body() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    // Readiness probe should return empty body
    assert_eq!(body.len(), 0);
}

#[tokio::test]
async fn test_health_endpoints_are_idempotent() {
    let app = create_test_router();

    // Call health check multiple times
    for _ in 0..3 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_health_check_json_content_type() {
    let app = create_test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());

    assert!(content_type.is_some());
    assert!(content_type.unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_health_response_serialization() {
    use payments_backend_dodo::controllayer::health::DatabaseHealth;

    let health_response = HealthResponse {
        status: "healthy".to_string(),
        version: "0.1.0".to_string(),
        timestamp: 1234567890,
        database: Some(DatabaseHealth {
            status: "healthy".to_string(),
            latency_ms: Some(10),
        }),
    };

    let json = serde_json::to_string(&health_response).unwrap();

    assert!(json.contains("healthy"));
    assert!(json.contains("0.1.0"));
    assert!(json.contains("1234567890"));
    assert!(json.contains("database"));
}

#[tokio::test]
async fn test_health_response_deserialization() {
    let json = r#"{
        "status": "healthy",
        "version": "0.1.0",
        "timestamp": 1234567890,
        "database": {
            "status": "healthy",
            "latency_ms": 10
        }
    }"#;

    let health_response: HealthResponse = serde_json::from_str(json).unwrap();

    assert_eq!(health_response.status, "healthy");
    assert_eq!(health_response.version, "0.1.0");
    assert_eq!(health_response.timestamp, 1234567890);
    assert!(health_response.database.is_some());
}

#[tokio::test]
async fn test_health_response_without_database() {
    let json = r#"{
        "status": "healthy",
        "version": "0.1.0",
        "timestamp": 1234567890
    }"#;

    let health_response: HealthResponse = serde_json::from_str(json).unwrap();

    assert_eq!(health_response.status, "healthy");
    assert!(health_response.database.is_none());
}

#[tokio::test]
async fn test_concurrent_health_checks() {
    use tokio::task::JoinSet;

    let mut set = JoinSet::new();

    // Spawn 10 concurrent health check requests
    for _ in 0..10 {
        set.spawn(async {
            let app = create_test_router();
            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/health")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            response.status()
        });
    }

    // All should return 200 OK
    while let Some(result) = set.join_next().await {
        let status = result.unwrap();
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test]
async fn test_health_check_performance() {
    use std::time::Instant;

    let app = create_test_router();
    let start = Instant::now();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let duration = start.elapsed();

    assert_eq!(response.status(), StatusCode::OK);
    // Health check should be fast (< 100ms)
    assert!(duration.as_millis() < 100);
}
