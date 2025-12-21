use crate::{
    datalayer::CRUD::api_key::ApiKeyBuilder, datalayer::db_ops::constants::POOL_STATE_TRACKER,
    errors::errors::create_error_response, state::AppState,
};
use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Authenticated API key information stored in request extensions
#[derive(Debug, Clone)]
pub struct AuthenticatedApiKey {
    pub api_key_id: Uuid,
    pub account_id: Uuid,
    pub key_prefix: String,
}

/// API key authentication middleware
/// Extracts and validates API key from Authorization header
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let request_id = request.extensions().get::<Uuid>().map(|id| id.to_string());

    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let api_key = match auth_header {
        Some(header_value) => {
            // Expected format: "Bearer sk_live_..." or "Bearer sk_test_..."
            if let Some(key) = header_value.strip_prefix("Bearer ") {
                key
            } else {
                return Err(create_error_response(
                    StatusCode::UNAUTHORIZED,
                    "invalid_authorization_header",
                    "Authorization header must use Bearer scheme",
                    request_id,
                ));
            }
        }
        None => {
            return Err(create_error_response(
                StatusCode::UNAUTHORIZED,
                "missing_authorization",
                "Authorization header is required",
                request_id,
            ));
        }
    };

    // Validate API key format (should start with pk_live_ or pk_test_)
    if !api_key.starts_with("pk_") {
        return Err(create_error_response(
            StatusCode::UNAUTHORIZED,
            "invalid_api_key_format",
            "API key must start with 'pk_'",
            request_id,
        ));
    }

    // Hash the API key
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = format!("{:x}", hasher.finalize());
    let key_prefix: String = api_key.chars().take(8).collect();

    // Look up API key in database
    let tracker = POOL_STATE_TRACKER.get().ok_or_else(|| {
        create_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "database_unavailable",
            "Database connection pool not initialized",
            request_id.clone(),
        )
    })?;

    // Connection guard to automatically return connection when scope ends
    struct ConnectionGuard(Option<sqlx::pool::PoolConnection<sqlx::Postgres>>);
    impl Drop for ConnectionGuard {
        fn drop(&mut self) {
            if let Some(c) = self.0.take() {
                if let Some(tracker) = POOL_STATE_TRACKER.get() {
                    tracker.return_connection(c);
                }
            }
        }
    }

    let conn = tracker.get_connection().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get database connection");
        create_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "database_error",
            "Failed to connect to database",
            request_id.clone(),
        )
    })?;

    let mut guard = ConnectionGuard(Some(conn));
    let db_conn = guard.0.as_mut().unwrap();

    let api_key_record = ApiKeyBuilder::new()
        .key_hash(key_hash)
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_permissions()
        .expect_last_used_at()
        .expect_expires_at()
        .expect_created_at()
        .expect_revoked_at()
        .read(Some(&mut *db_conn))
        .await;

    // Connection automatically returned when guard goes out of scope

    let api_key_record = api_key_record.map_err(|e| {
        tracing::warn!(
            key_prefix = %key_prefix,
            error = ?e,
            "API key not found or invalid"
        );
        create_error_response(
            StatusCode::UNAUTHORIZED,
            "invalid_api_key",
            "API key is invalid or does not exist",
            request_id.clone(),
        )
    })?;

    // Check if API key is active
    if api_key_record.status != "active" {
        return Err(create_error_response(
            StatusCode::UNAUTHORIZED,
            "inactive_api_key",
            &format!("API key is not active (status: {})", api_key_record.status),
            request_id,
        ));
    }

    // Check if API key is revoked
    if api_key_record.revoked_at.is_some() {
        return Err(create_error_response(
            StatusCode::UNAUTHORIZED,
            "revoked_api_key",
            "API key has been revoked",
            request_id,
        ));
    }

    // Check if API key is expired
    if let Some(expires_at) = api_key_record.expires_at {
        if expires_at < chrono::Utc::now() {
            return Err(create_error_response(
                StatusCode::UNAUTHORIZED,
                "expired_api_key",
                "API key has expired",
                request_id,
            ));
        }
    }

    // Store authenticated API key info in request extensions
    let auth_info = AuthenticatedApiKey {
        api_key_id: api_key_record.id,
        account_id: api_key_record.account_id,
        key_prefix: api_key_record.key_prefix.clone(),
    };

    request.extensions_mut().insert(auth_info);

    tracing::info!(
        api_key_id = %api_key_record.id,
        account_id = %api_key_record.account_id,
        key_prefix = %key_prefix,
        "API key authenticated successfully"
    );

    Ok(next.run(request).await)
}

/// Helper to extract authenticated API key from request
pub fn get_authenticated_key(request: &Request) -> Option<&AuthenticatedApiKey> {
    request.extensions().get::<AuthenticatedApiKey>()
}
