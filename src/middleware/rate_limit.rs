use crate::{
    datalayer::CRUD::rate_limiter::RateLimiter, errors::errors::create_error_response,
    middleware::auth::AuthenticatedApiKey, state::AppState,
};
use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};

/// Rate limiting middleware with exponential backoff
/// Applies per-endpoint, per-API-key rate limiting using Redis
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let request_id = request
        .extensions()
        .get::<uuid::Uuid>()
        .map(|id| id.to_string());

    // Get authenticated API key from request extensions
    let auth_key = request
        .extensions()
        .get::<AuthenticatedApiKey>()
        .ok_or_else(|| {
            create_error_response(
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Request must be authenticated before rate limiting",
                request_id.clone(),
            )
        })?;

    // Get endpoint path for rate limiting
    let endpoint = request.uri().path().to_string();

    // Initialize rate limiter
    let limiter = RateLimiter::new();

    // Check rate limit with automatic backoff
    let redis_conn = (*state.redis).clone();

    limiter
        .check_with_backoff(
            auth_key.api_key_id,
            &auth_key.key_prefix,
            &endpoint,
            redis_conn.clone(),
        )
        .await
        .map_err(|e| {
            tracing::warn!(
                api_key_id = %auth_key.api_key_id,
                endpoint = %endpoint,
                error = ?e,
                "Rate limit exceeded"
            );

            match e {
                crate::errors::errors::ServiceError::RateLimitExceeded {
                    limit,
                    window,
                    reset_at,
                } => {
                    let mut response = create_error_response(
                        StatusCode::TOO_MANY_REQUESTS,
                        "rate_limit_exceeded",
                        &format!("Rate limit exceeded for endpoint: {}", window),
                        request_id.clone(),
                    );

                    // Add rate limit headers
                    let headers = response.headers_mut();
                    headers.insert(
                        header::HeaderName::from_static("x-ratelimit-limit"),
                        limit.to_string().parse().unwrap(),
                    );
                    headers.insert(
                        header::HeaderName::from_static("x-ratelimit-remaining"),
                        "0".parse().unwrap(),
                    );
                    headers.insert(
                        header::HeaderName::from_static("x-ratelimit-reset"),
                        reset_at.timestamp().to_string().parse().unwrap(),
                    );
                    headers.insert(header::RETRY_AFTER, "60".parse().unwrap());

                    response
                }
                _ => create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "rate_limit_error",
                    "Failed to check rate limit",
                    request_id,
                ),
            }
        })?;

    // Request allowed, proceed
    let response = next.run(request).await;

    Ok(response)
}
