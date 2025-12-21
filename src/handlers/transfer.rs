use axum::{
    Extension, Json,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedApiKey;

// ===== REQUEST DTOs =====

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TransferRequest {
    /// Credit: Add funds to an account (from_account is null)
    Credit {
        to_account: Uuid,
        amount: f64,
        currency: String,
        description: Option<String>,
        #[serde(default)]
        idempotency_key: Option<String>,
    },
    /// Debit: Remove funds from an account (to_account is null)
    Debit {
        from_account: Uuid,
        amount: f64,
        currency: String,
        description: Option<String>,
        #[serde(default)]
        idempotency_key: Option<String>,
    },
    /// Transfer: Move funds between accounts
    Transfer {
        from_account: Uuid,
        to_account: Uuid,
        amount: f64,
        currency: String,
        description: Option<String>,
        #[serde(default)]
        idempotency_key: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
pub struct ListTransfersQuery {
    pub account_id: Option<Uuid>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ===== RESPONSE DTOs =====

#[derive(Debug, Serialize)]
pub struct TransferResponse {
    pub id: Uuid,
    pub transfer_type: String,
    pub from_account: Option<Uuid>,
    pub to_account: Option<Uuid>,
    pub amount: f64,
    pub currency: String,
    pub status: String,
    pub description: Option<String>,
    pub created_at: String,
    pub idempotency_key: String,
    pub parent_tx_key: String,
}

#[derive(Debug, Serialize)]
pub struct TransferListResponse {
    pub transfers: Vec<TransferResponse>,
    pub total: i32,
    pub limit: i32,
    pub offset: i32,
}

// ===== HANDLERS =====

/// POST /api/v1/transfer
/// Execute a transfer (credit, debit, or transfer)
#[instrument(fields(service = "/api/v1/transfer"))]
pub async fn transfer(
    auth_info: axum::Extension<crate::middleware::auth::AuthenticatedApiKey>,
    Json(payload): Json<TransferRequest>,
) -> Response {
    info!("Executing transfer with payload: {:?}", payload);
    // Delegate to controller
    crate::controllayer::transfers::execute_transfer(auth_info, Json(payload)).await
}

/// GET /api/v1/transfer/:id
/// Get transfer details by parent key
/// this will return the information about the transaction based on the parent key
#[instrument(fields(service = "/api/v1/transfer/:id"))]
pub async fn get_transfer_byparentkey(
    Extension(auth_info): Extension<AuthenticatedApiKey>,
    Path(parent_key): Path<String>,
) -> Response {
    tracing::info!(parent_key = %parent_key, account_id = %auth_info.account_id, "Getting transfer details by parent key");

    match crate::controllayer::transfers::queries::get_transfer_by_parent_key(
        parent_key,
        auth_info.account_id,
    )
    .await
    {
        Ok(transfers) => (StatusCode::OK, Json(transfers)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/v1/transfer/info/:id
/// Get transfer details by ID
/// this will return the information about the transaction based on the id
#[instrument(fields(service = "/api/v1/transfer/info/:id"))]
pub async fn get_transfer_byid(
    Extension(auth_info): Extension<AuthenticatedApiKey>,
    Path(transfer_id): Path<Uuid>,
) -> Response {
    tracing::info!(transfer_id = %transfer_id, account_id = %auth_info.account_id, "Getting transfer details by ID");

    match crate::controllayer::transfers::queries::get_transfer_by_id(
        transfer_id,
        auth_info.account_id,
    )
    .await
    {
        Ok(transfer) => (StatusCode::OK, Json(transfer)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/v1/transfer/list
/// List transfers with optional filtering
/// return all the Transaction::Type Transfer transactions by the user
#[instrument(fields(service = "/api/v1/transfer/list"))]
pub async fn list_transfers(
    Extension(auth_info): Extension<AuthenticatedApiKey>,
    Query(params): Query<ListTransfersQuery>,
) -> Response {
    tracing::info!(
        account_id = %auth_info.account_id,
        requested_account_id = ?params.account_id,
        limit = ?params.limit,
        offset = ?params.offset,
        "Listing transfers"
    );

    // Require account_id parameter
    let requested_account_id = match params.account_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": {
                        "code": "MISSING_ACCOUNT_ID",
                        "message": "account_id parameter is required"
                    }
                })),
            )
                .into_response();
        }
    };

    // Verify requested account_id matches authenticated user's account
    if requested_account_id != auth_info.account_id {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": {
                    "code": "UNAUTHORIZED",
                    "message": "You can only view transfers for your own account"
                }
            })),
        )
            .into_response();
    }

    match crate::controllayer::transfers::queries::list_transfers(
        auth_info.account_id,
        params.limit,
        params.offset,
    )
    .await
    {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(e) => e.into_response(),
    }
}
