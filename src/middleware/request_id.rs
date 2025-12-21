use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

/// Middleware to add a unique request ID to each request
/// The request ID is added to both the request extensions and response headers
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    // Generate a unique request ID
    let request_id = Uuid::new_v4();

    // Add to request extensions for use in handlers
    request.extensions_mut().insert(request_id);

    // Log the request with ID
    tracing::info!(
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
        "Incoming request"
    );

    // Process the request
    let mut response = next.run(request).await;

    // Add request ID to response headers
    response.headers_mut().insert(
        header::HeaderName::from_static("x-request-id"),
        request_id.to_string().parse().unwrap(),
    );

    response
}

/// Extract request ID from request extensions
pub fn get_request_id(request: &Request) -> Option<Uuid> {
    request.extensions().get::<Uuid>().copied()
}
