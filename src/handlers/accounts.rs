use crate::{controllayer::accounts::accounts_handler, datalayer::CRUD::helper::conversion};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};
use uuid::Uuid;
// ===== REQUEST DTOs =====

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub business_name: String,
    pub email: String,
    pub currency: String,
}

#[derive(Debug, Deserialize)]
pub struct PutBalanceRequest {
    pub account_id: Uuid,
    pub balance: f64,
    pub currency: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetAccountRequest {
    pub account_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountRequest {
    pub business_name: Option<String>,
    pub email: Option<String>,
    pub status: Option<String>,
}

// ===== RESPONSE DTOs =====

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountResponse {
    pub id: Uuid,
    pub business_name: String,
    pub email: String,
    pub balance: Option<f64>,
    pub currency: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateAccountResponse {
    pub account: AccountResponse,
    pub api_key: String, // Plain-text API key (only shown once)
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
///
/// This endpoint delegates to the controller layer which handles:
/// 1. Request validation
/// 2. Duplicate account checking
/// 3. Account creation with AccountBuilder
/// 4. API key generation and secure storage
/// 5. Returning account details with the plain-text API key (only shown once)
#[instrument(fields(service = "/api/v1/accounts"))]
pub async fn create_account(payload: Json<CreateAccountRequest>) -> Response {
    info!(
        business_name = %payload.business_name,
        email = %payload.email,
        currency = %payload.currency,
        "Creating new account"
    );
    // Delegate to the controller handler
    let response = accounts_handler::create_account(payload).await;

    // Return the response from the controller handler
    // log the response code
    info!(
        response_code = %response.status(),
        "Account created successfully"
    );
    // return the response
    response
}

/// PUT /api/v1/accounts/:id/balance
/// Update account balance
///
/// This endpoint delegates to the controller layer which handles:
/// 1. Authorization check (API key belongs to account)
/// 2. Balance validation (non-negative)
/// 3. Account existence verification
/// 4. Balance and optional currency update
/// 5. Returning updated account details
#[instrument(fields(service = "/api/v1/accounts/:id/putbalance"))]
pub async fn put_balance(
    Path(account_id): Path<Uuid>,
    Json(payload): Json<PutBalanceRequest>,
    request: axum::extract::Request,
) -> Response {
    info!(
        account_id = %payload.account_id,
        balance = %payload.balance,
        currency = ?payload.currency,
        "Updating account balance"
    );

    // Create the request payload with account_id from path
    let request_payload = Json(PutBalanceRequest {
        account_id,
        balance: payload.balance,
        currency: payload.currency,
    });

    // Delegate to the controller handler with request for auth
    let response = accounts_handler::update_balance(request, request_payload).await;

    // Return the response from the controller handler
    // log the response code
    info!(
        response_code = %response.status(),
        "Account balance updated successfully"
    );
    // return the response
    response
}

/// GET /api/v1/accounts/:id
/// Get account details
///
/// This endpoint delegates to the controller layer which handles:
/// 1. Authorization check (API key belongs to account)
/// 2. Account lookup by ID
/// 3. Returning account details
#[instrument(fields(service = "/api/v1/accounts/:id"))]
pub async fn get_account(
    Path(account_id): Path<Uuid>,
    request: axum::extract::Request,
) -> Response {
    info!(account_id = %account_id, "Getting account details");

    // Create the request payload with account_id from path
    let payload = Json(GetAccountRequest { account_id });

    // Delegate to the controller handler with the full request for auth checking
    let response = accounts_handler::get_account(request, payload).await;

    // Return the response from the controller handler
    // log the response code
    info!(response_code = %response.status(), "Account details retrieved successfully");
    // return the response
    response
}

/// PATCH /api/v1/accounts/:id
/// Update account details
///
/// This endpoint delegates to the controller layer which handles:
/// 1. Authorization check (API key belongs to account)
/// 2. Account lookup and update
/// 3. Returning updated account details
#[instrument(fields(service = "/api/v1/accounts/:id"))]
pub async fn update_account(
    Path(account_id): Path<Uuid>,
    Json(payload): Json<UpdateAccountRequest>,
    request: axum::extract::Request,
) -> Response {
    info!(
        account_id = %account_id,
        business_name = ?payload.business_name,
        email = ?payload.email,
        status = ?payload.status,
        "Updating account details"
    );

    // Delegate to the controller handler with account_id and payload
    let response = accounts_handler::update_account(request, account_id, Json(payload)).await;

    // Log the response
    info!(
        response_code = %response.status(),
        "Account update completed"
    );

    response
}

#[derive(Debug, Deserialize)]
pub struct GetBalanceRequest {
    pub currency: Option<String>, // Optional currency to convert balance to
}

/// GET /api/v1/accounts/:id/balance
/// Get account balance
///
/// This endpoint returns only the balance information for an account
/// Optionally accepts a currency query parameter to convert the balance
/// Authorization: API key must belong to the account
#[instrument(fields(service = "/api/v1/accounts/:id/balance"))]
pub async fn get_balance(
    Path(account_id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<GetBalanceRequest>,
    request: axum::extract::Request,
) -> Response {
    info!(
        account_id = %account_id,
        requested_currency = ?query.currency,
        "Getting account balance"
    );

    // Create the request payload with account_id from path
    let payload = Json(GetAccountRequest { account_id });

    // Reuse the get_account controller function (which includes authorization)
    let response = accounts_handler::get_account(request, payload).await;

    // Check if we got the account successfully
    if response.status() != StatusCode::OK {
        return response;
    }

    // Extract the body and parse it to get AccountResponse
    let (parts, body) = response.into_parts();

    // Convert body to bytes and parse as AccountResponse
    match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => {
            match serde_json::from_slice::<AccountResponse>(&bytes) {
                Ok(account) => {
                    let account_balance = account.balance.unwrap_or(0.0);
                    let account_currency_str = account
                        .currency
                        .clone()
                        .unwrap_or_else(|| "USD".to_string());

                    // Determine target currency: use requested currency if provided, otherwise use account's currency
                    let target_currency_str =
                        query.currency.unwrap_or(account_currency_str.clone());

                    // Convert balance from USD to target currency
                    let converted_balance = if let Ok(target_currency) =
                        conversion::map_currency(target_currency_str.clone())
                    {
                        conversion::from_usd(account_balance, target_currency)
                            .unwrap_or(account_balance)
                    } else {
                        // If currency mapping fails, use original balance
                        error!("Failed to map currency: {}", target_currency_str);
                        account_balance
                    };

                    // Extract only balance information
                    let balance_response = BalanceResponse {
                        account_id: account.id,
                        balance: converted_balance,
                        currency: target_currency_str,
                    };

                    info!(
                        account_id = %balance_response.account_id,
                        balance = %balance_response.balance,
                        currency = %balance_response.currency,
                        "Account balance retrieved and converted"
                    );

                    (StatusCode::OK, Json(balance_response)).into_response()
                }
                Err(e) => {
                    error!("Failed to parse account response: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to parse account data",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to read response body: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read response").into_response()
        }
    }
}
