use axum::{
    Extension, Json,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    datalayer::{
        CRUD::webhook::{
            create_webhook as create_webhook_db, delete_webhook as delete_webhook_db,
            get_webhooks_for_account,
        },
        db_ops::constants::POOL_STATE_TRACKER,
    },
    middleware::auth::AuthenticatedApiKey,
};

// ===== REQUEST DTOs =====

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub account_id: Uuid,
    pub url: String,
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteWebhookRequest {
    pub webhook_id: Uuid,
}

// ===== RESPONSE DTOs =====

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub id: Uuid,
    pub account_id: Uuid,
    pub url: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct WebhooksListResponse {
    pub webhooks: Vec<WebhookResponse>,
    pub total: usize,
}

// ===== HANDLERS =====

/// POST /api/v1/webhooks/set
/// Create a new webhook for the authenticated account
#[instrument(fields(service = "/api/v1/webhooks/set"))]
pub async fn create_webhook(
    Extension(auth_info): Extension<AuthenticatedApiKey>,
    Json(payload): Json<CreateWebhookRequest>,
) -> Response {
    tracing::info!(
        account_id = %auth_info.account_id,
        requested_account_id = %payload.account_id,
        url = %payload.url,
        "Creating webhook"
    );

    // Verify requested account_id matches authenticated user's account
    if payload.account_id != auth_info.account_id {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": {
                    "code": "UNAUTHORIZED",
                    "message": "You can only create webhooks for your own account"
                }
            })),
        )
            .into_response();
    }

    // Validate URL format
    if !payload.url.starts_with("http://") && !payload.url.starts_with("https://") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": {
                    "code": "INVALID_URL",
                    "message": "Webhook URL must start with http:// or https://"
                }
            })),
        )
            .into_response();
    }

    // Get database connection
    let tracker = match POOL_STATE_TRACKER.get() {
        Some(t) => t,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "code": "DATABASE_ERROR",
                        "message": "Database connection unavailable"
                    }
                })),
            )
                .into_response();
        }
    };

    let mut conn = match tracker.get_connection().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get database connection");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "code": "DATABASE_ERROR",
                        "message": "Failed to connect to database"
                    }
                })),
            )
                .into_response();
        }
    };

    // Create webhook
    match create_webhook_db(auth_info.account_id, payload.url, payload.secret, &mut conn).await {
        Ok(webhook) => {
            tracker.return_connection(conn);

            let response = WebhookResponse {
                id: webhook.id,
                account_id: webhook.account_id,
                url: webhook.url,
                status: webhook.status,
                created_at: webhook.created_at.to_rfc3339(),
            };

            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracker.return_connection(conn);
            e.into_response()
        }
    }
}

/// POST /api/v1/webhooks/unset
/// Delete a webhook
#[instrument(fields(service = "/api/v1/webhooks/unset"))]
pub async fn delete_webhook(
    Extension(auth_info): Extension<AuthenticatedApiKey>,
    Json(payload): Json<DeleteWebhookRequest>,
) -> Response {
    tracing::info!(
        account_id = %auth_info.account_id,
        webhook_id = %payload.webhook_id,
        "Deleting webhook"
    );

    let tracker = match POOL_STATE_TRACKER.get() {
        Some(t) => t,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "code": "DATABASE_ERROR",
                        "message": "Database connection unavailable"
                    }
                })),
            )
                .into_response();
        }
    };

    let mut conn = match tracker.get_connection().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get database connection");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "code": "DATABASE_ERROR",
                        "message": "Failed to connect to database"
                    }
                })),
            )
                .into_response();
        }
    };

    match delete_webhook_db(payload.webhook_id, auth_info.account_id, &mut conn).await {
        Ok(_) => {
            tracker.return_connection(conn);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Webhook deleted successfully"
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracker.return_connection(conn);
            e.into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetWebhooksQuery {
    pub account_id: Uuid,
}

/// GET /api/v1/webhooks/info?account_id=xxx
/// Get all webhooks for the authenticated account
#[instrument(fields(service = "/api/v1/webhooks/info"))]
pub async fn get_webhooks(
    Extension(auth_info): Extension<AuthenticatedApiKey>,
    Query(params): Query<GetWebhooksQuery>,
) -> Response {
    tracing::info!(
        account_id = %auth_info.account_id,
        requested_account_id = %params.account_id,
        "Fetching webhooks"
    );

    // Verify requested account_id matches authenticated user's account
    if params.account_id != auth_info.account_id {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": {
                    "code": "UNAUTHORIZED",
                    "message": "You can only view webhooks for your own account"
                }
            })),
        )
            .into_response();
    }

    let tracker = match POOL_STATE_TRACKER.get() {
        Some(t) => t,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "code": "DATABASE_ERROR",
                        "message": "Database connection unavailable"
                    }
                })),
            )
                .into_response();
        }
    };

    let mut conn = match tracker.get_connection().await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get database connection");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "code": "DATABASE_ERROR",
                        "message": "Failed to connect to database"
                    }
                })),
            )
                .into_response();
        }
    };

    match get_webhooks_for_account(auth_info.account_id, &mut conn).await {
        Ok(webhooks) => {
            tracker.return_connection(conn);

            let response_webhooks: Vec<WebhookResponse> = webhooks
                .into_iter()
                .map(|w| WebhookResponse {
                    id: w.id,
                    account_id: w.account_id,
                    url: w.url,
                    status: w.status,
                    created_at: w.created_at.to_rfc3339(),
                })
                .collect();

            let total = response_webhooks.len();

            (
                StatusCode::OK,
                Json(WebhooksListResponse {
                    webhooks: response_webhooks,
                    total,
                }),
            )
                .into_response()
        }
        Err(e) => {
            tracker.return_connection(conn);
            e.into_response()
        }
    }
}
