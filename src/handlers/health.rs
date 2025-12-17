use crate::controllayer::health::liveness;
use crate::errors::errors::ServiceError;
use axum::response::IntoResponse;
use tracing::{info, instrument};

/// Health check handler that delegates to the liveness function
#[instrument(fields(service = "health_check"))]
pub async fn health_check() -> Result<impl IntoResponse, ServiceError> {
    // Call the liveness function from controllayer
    info!("Health check request received");
    liveness().await
}
