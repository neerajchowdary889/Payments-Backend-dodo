use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        idempotency_key: String,
    },
    /// Debit: Remove funds from an account (to_account is null)
    Debit {
        from_account: Uuid,
        amount: f64,
        currency: String,
        description: Option<String>,
        idempotency_key: String,
    },
    /// Transfer: Move funds between accounts
    Transfer {
        from_account: Uuid,
        to_account: Uuid,
        amount: f64,
        currency: String,
        description: Option<String>,
        idempotency_key: String,
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
pub async fn transfer(
    Json(payload): Json<TransferRequest>,
) -> Result<(StatusCode, Json<TransferResponse>), StatusCode> {
    // TODO: Implement transfer
    // 1. Validate request
    // 2. Match on transfer type
    // 3. For credit: Call TransferHelper::credit()
    // 4. For debit: Call TransferHelper::debit()
    // 5. For transfer: Call TransferHelper::transfer()
    // 6. Return transaction details

    match &payload {
        TransferRequest::Credit {
            to_account, amount, ..
        } => {
            tracing::info!(
                to_account = %to_account,
                amount = %amount,
                "Processing credit transfer"
            );
        }
        TransferRequest::Debit {
            from_account,
            amount,
            ..
        } => {
            tracing::info!(
                from_account = %from_account,
                amount = %amount,
                "Processing debit transfer"
            );
        }
        TransferRequest::Transfer {
            from_account,
            to_account,
            amount,
            ..
        } => {
            tracing::info!(
                from_account = %from_account,
                to_account = %to_account,
                amount = %amount,
                "Processing transfer"
            );
        }
    }

    Err(StatusCode::NOT_IMPLEMENTED)
}

/// GET /api/v1/transfer/:id
/// Get transfer details by ID
pub async fn get_transfer(
    Path(transfer_id): Path<Uuid>,
) -> Result<Json<TransferResponse>, StatusCode> {
    // TODO: Implement get transfer
    // 1. Fetch transaction using TransactionBuilder
    // 2. Return transaction details

    tracing::info!(transfer_id = %transfer_id, "Getting transfer details");

    Err(StatusCode::NOT_IMPLEMENTED)
}

/// GET /api/v1/transfer
/// List transfers with optional filtering
pub async fn list_transfers(
    Query(params): Query<ListTransfersQuery>,
) -> Result<Json<TransferListResponse>, StatusCode> {
    // TODO: Implement list transfers
    // 1. Parse query parameters
    // 2. Fetch transactions using TransactionBuilder
    // 3. Return paginated list

    tracing::info!(
        account_id = ?params.account_id,
        limit = ?params.limit,
        offset = ?params.offset,
        "Listing transfers"
    );

    Err(StatusCode::NOT_IMPLEMENTED)
}
