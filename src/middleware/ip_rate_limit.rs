use crate::{
    datalayer::CRUD::redis::redis::RateLimitCounter,
    datalayer::helper::backoff::ExponentialBackoff, errors::errors::create_error_response,
    state::AppState,
};
use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::SocketAddr;

/// IP-based rate limiting middleware for public endpoints
/// Limits requests per IP address to prevent abuse
pub async fn ip_rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let request_id = request
        .extensions()
        .get::<uuid::Uuid>()
        .map(|id| id.to_string());

    let endpoint = request.uri().path().to_string();
    let ip = addr.ip().to_string();

    // Configuration for IP-based rate limiting
    const SOFT_LIMIT: u32 = 10; // Start backoff after 10 requests
    const HARD_LIMIT: u32 = 30; // Block after 30 requests
    const WINDOW_SECONDS: i64 = 60; // 1 minute window

    // Use IP address as the identifier in Redis
    let redis_conn = (*state.redis).clone();
    let mut counter = RateLimitCounter::new(redis_conn.clone());

    // Get current count for this IP + endpoint
    let current_count = counter
        .get_count_by_key(&format!("ip:{}:{}", ip, endpoint))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get rate limit count");
            create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "rate_limit_error",
                "Failed to check rate limit",
                request_id.clone(),
            )
        })?;

    // Check hard limit
    if current_count >= HARD_LIMIT {
        tracing::warn!(
            ip = %ip,
            endpoint = %endpoint,
            count = current_count,
            "IP rate limit exceeded"
        );

        return Err(crate::errors::errors::ServiceError::RateLimitExceeded {
            limit: HARD_LIMIT as i32,
            window: format!("{}s", WINDOW_SECONDS),
            reset_at: chrono::Utc::now() + chrono::Duration::seconds(WINDOW_SECONDS),
        }
        .into_response());
    }

    // Apply backoff if over soft limit
    if current_count >= SOFT_LIMIT {
        let attempts_over_soft = current_count - SOFT_LIMIT;

        let mut backoff = ExponentialBackoff::new();
        backoff.set_base_delay_ms(1000); // 1 second base delay
        backoff.set_max_delay_ms(10000); // 10 second max delay

        let delay_ms = backoff.calculate(attempts_over_soft);

        tracing::warn!(
            ip = %ip,
            endpoint = %endpoint,
            count = current_count + 1,
            delay_ms = delay_ms,
            "IP rate limit soft threshold exceeded, applying backoff"
        );

        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }

    // Increment counter
    counter
        .increment_count_by_key(&format!("ip:{}:{}", ip, endpoint))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to increment rate limit count");
            create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "rate_limit_error",
                "Failed to update rate limit",
                request_id,
            )
        })?;

    // Request allowed, proceed
    let response = next.run(request).await;

    Ok(response)
}
