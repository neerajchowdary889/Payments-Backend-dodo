use axum::{Json, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::errors::errors::ServiceError;
use tracing::{info, instrument};

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: i64,
}

/// Liveness probe endpoint
#[instrument(fields(service = "liveness"))]
pub async fn liveness() -> Result<impl IntoResponse, ServiceError> {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now().timestamp(),
    };

    // Log with structured fields
    info!(
        status_code = StatusCode::OK.as_u16(),
        response_status = %response.status,
        version = %response.version,
        "Liveness check successful"
    );

    Ok((StatusCode::OK, Json(response)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_liveness() {
        let response = liveness().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
