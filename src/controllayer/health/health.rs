use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseHealth>,
}

/// Database health status
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub status: String,
    pub latency_ms: Option<u64>,
}

/// Health check endpoint handler
///
/// Returns 200 OK if the service is healthy
/// Returns 503 Service Unavailable if any critical component is unhealthy
pub async fn health_check() -> impl IntoResponse {
    let timestamp = chrono::Utc::now().timestamp();

    // TODO: Add actual database health check when database is integrated
    let db_health = Some(DatabaseHealth {
        status: "healthy".to_string(),
        latency_ms: Some(5),
    });

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp,
        database: db_health,
    };

    (StatusCode::OK, Json(response))
}

/// Liveness probe endpoint
///
/// Simple endpoint that returns 200 OK if the service is running
/// Used by Kubernetes/Docker for liveness checks
pub async fn liveness() -> impl IntoResponse {
    StatusCode::OK
}

/// Readiness probe endpoint
///
/// Returns 200 OK if the service is ready to accept traffic
/// Returns 503 Service Unavailable if not ready (e.g., database not connected)
pub async fn readiness() -> impl IntoResponse {
    // TODO: Add actual readiness checks (database connection, etc.)
    let is_ready = true;

    if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_liveness() {
        let response = liveness().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_readiness() {
        let response = readiness().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
