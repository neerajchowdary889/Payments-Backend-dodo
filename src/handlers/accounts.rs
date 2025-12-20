use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===== REQUEST DTOs =====

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub business_name: String,
    pub email: String,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountRequest {
    pub business_name: Option<String>,
    pub email: Option<String>,
    pub status: Option<String>,
}

// ===== RESPONSE DTOs =====

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub id: Uuid,
    pub business_name: String,
    pub email: String,
    pub balance: f64,
    pub currency: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub account_id: Uuid,
    pub balance: f64,
    pub currency: String,
}

// ===== HANDLERS =====

/// POST /api/v1/accounts
/// Create a new account
pub async fn create_account(
    Json(payload): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<AccountResponse>), StatusCode> {
    // TODO: Implement account creation
    // 1. Validate request
    // 2. Create account using AccountBuilder
    // 3. Return account details

    tracing::info!(
        business_name = %payload.business_name,
        email = %payload.email,
        currency = %payload.currency,
        "Creating new account"
    );

    Err(StatusCode::NOT_IMPLEMENTED)
}

/// GET /api/v1/accounts/:id
/// Get account details
pub async fn get_account(
    Path(account_id): Path<Uuid>,
) -> Result<Json<AccountResponse>, StatusCode> {
    // TODO: Implement get account
    // 1. Fetch account using AccountBuilder
    // 2. Return account details

    tracing::info!(account_id = %account_id, "Getting account");

    Err(StatusCode::NOT_IMPLEMENTED)
}

/// PATCH /api/v1/accounts/:id
/// Update account details
pub async fn update_account(
    Path(account_id): Path<Uuid>,
    Json(payload): Json<UpdateAccountRequest>,
) -> Result<Json<AccountResponse>, StatusCode> {
    // TODO: Implement account update
    // 1. Validate request
    // 2. Update account using AccountBuilder
    // 3. Return updated account details

    tracing::info!(
        account_id = %account_id,
        "Updating account"
    );

    Err(StatusCode::NOT_IMPLEMENTED)
}

/// GET /api/v1/accounts/:id/balance
/// Get account balance
pub async fn get_balance(
    Path(account_id): Path<Uuid>,
) -> Result<Json<BalanceResponse>, StatusCode> {
    // TODO: Implement get balance
    // 1. Fetch account using AccountBuilder
    // 2. Return balance details

    tracing::info!(account_id = %account_id, "Getting account balance");

    Err(StatusCode::NOT_IMPLEMENTED)
}
