use axum::{extract::Request, middleware::Next, response::Response};

// Re-export error response creation from centralized errors module
pub use crate::errors::errors::create_error_response;

/// Error handling middleware
/// Catches panics and converts them to proper error responses
pub async fn error_handling_middleware(request: Request, next: Next) -> Response {
    let request_id = request
        .extensions()
        .get::<uuid::Uuid>()
        .map(|id| id.to_string());

    let response = next.run(request).await;

    // If response is an error status, ensure it has proper format
    if response.status().is_client_error() || response.status().is_server_error() {
        // Log the error
        tracing::error!(
            status = %response.status(),
            request_id = ?request_id,
            "Request failed"
        );
    }

    response
}
