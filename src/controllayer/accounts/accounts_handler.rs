use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::Acquire; // Required for begin() method on connections
use tracing::instrument;
use uuid::Uuid;

use crate::{
    datalayer::{
        CRUD::{
            accounts::AccountBuilder, api_key::ApiKeyBuilder,
            helper::apikey_generator::generate_api_key,
        },
        db_ops::constants::POOL_STATE_TRACKER,
    },
    errors::errors::create_error_response,
    handlers::accounts::{
        AccountResponse, CreateAccountRequest, CreateAccountResponse, GetAccountRequest,
        PutBalanceRequest, UpdateAccountRequest,
    },
};

/// Create a new account with an API key
///
/// This handler:
/// 1. Validates the request payload
/// 2. Checks for duplicate accounts (by business_name or email)
/// 3. Creates the account with balance = 0
/// 4. Generates a secure API key
/// 5. Hashes and stores the API key
/// 6. Returns account details with the plain-text API key (only time it's shown)
///
/// Note: Uses SQL-level transactions to ensure atomic creation of both account and API key
#[instrument(fields(service = "/api/v1/accounts"))]
pub async fn create_account(Json(payload): Json<CreateAccountRequest>) -> Response {
    tracing::info!(
        business_name = %payload.business_name,
        email = %payload.email,
        currency = %payload.currency,
        "Creating new account"
    );

    // Get database connection pool
    let tracker = match POOL_STATE_TRACKER.get() {
        Some(t) => t,
        None => {
            tracing::error!("Database connection pool not initialized");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_unavailable",
                "Database connection pool not initialized",
                None,
            );
        }
    };

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

    let mut guard = ConnectionGuard(None);
    let mut conn = match tracker.get_connection().await {
        Ok(c) => {
            guard.0 = Some(c);
            guard.0.as_mut().unwrap()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get database connection");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "Failed to connect to database",
                None,
            );
        }
    };

    // Begin SQL transaction for atomic account + API key creation
    if let Err(e) = sqlx::query("BEGIN").execute(&mut **conn).await {
        tracing::error!(error = %e, "Failed to begin transaction");
        return create_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "database_error",
            "Failed to begin transaction",
            None,
        );
    }

    tracing::info!("Started database transaction for account creation");

    // Create the account within the transaction using AccountBuilder
    let account = match AccountBuilder::new()
        .business_name(payload.business_name.clone())
        .email(payload.email.clone())
        .currency(payload.currency.clone())
        .status("active".to_string())
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_balance()
        .expect_currency()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .create(Some(conn))
        .await
    {
        Ok(acc) => {
            tracing::info!(
                account_id = %acc.id,
                business_name = %acc.business_name,
                "Account created successfully in transaction"
            );
            acc
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create account");
            // Rollback the transaction
            let _ = sqlx::query("ROLLBACK").execute(&mut **conn).await;
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "account_creation_failed",
                &format!("Failed to create account: {}", e),
                None,
            );
        }
    };

    // Generate API key
    let (api_key, key_hash, key_prefix) = generate_api_key(true);

    tracing::info!(
        account_id = %account.id,
        key_prefix = %key_prefix,
        "Generated API key for account"
    );

    // Store the API key in the database within the same transaction using ApiKeyBuilder
    match ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash)
        .key_prefix(key_prefix)
        .name("Default API Key".to_string())
        .status("active".to_string())
        .permissions(serde_json::json!(["read", "write"]))
        .expect_id()
        .expect_account_id()
        .expect_key_prefix()
        .expect_key_hash()
        .expect_name()
        .expect_permissions()
        .expect_status()
        .expect_last_used_at()
        .expect_expires_at()
        .expect_created_at()
        .expect_revoked_at()
        .create(Some(conn))
        .await
    {
        Ok(api_key_record) => {
            tracing::info!(
                account_id = %account.id,
                api_key_id = %api_key_record.id,
                "API key created and stored successfully in transaction"
            );
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                account_id = %account.id,
                "Failed to create API key, rolling back account creation"
            );
            // Rollback the transaction
            let _ = sqlx::query("ROLLBACK").execute(&mut **conn).await;
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "api_key_creation_failed",
                "Failed to create API key, account creation rolled back",
                None,
            );
        }
    }

    // Commit the transaction - both account and API key are created atomically
    match sqlx::query("COMMIT").execute(&mut **conn).await {
        Ok(_) => {
            tracing::info!(
                account_id = %account.id,
                "Transaction committed successfully - account and API key created atomically"
            );
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                account_id = %account.id,
                "Failed to commit transaction"
            );
            // Attempt rollback
            let _ = sqlx::query("ROLLBACK").execute(&mut **conn).await;
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "transaction_commit_failed",
                "Failed to commit transaction",
                None,
            );
        }
    }

    // Prepare response
    let response = CreateAccountResponse {
        account: AccountResponse {
            id: account.id,
            business_name: account.business_name,
            email: account.email,
            balance: Some(account.balance),
            currency: Some(account.currency),
            status: Some(account.status),
            created_at: Some(account.created_at.to_rfc3339()),
        },
        api_key, // Plain-text API key - only shown once!
    };

    tracing::info!(
        account_id = %response.account.id,
        "Account creation completed successfully"
    );

    (StatusCode::CREATED, Json(response)).into_response()
}

/// Update an account's balance and optionally currency
///
/// This handler:
/// 1. Authorization check (API key belongs to account)
/// 2. Validates the request (positive balance)
/// 3. Fetches the existing account
/// 4. Updates the balance and optionally the currency
/// 5. Returns the updated account details
#[instrument(fields(service = "/api/v1/accounts/putbalance"))]
pub async fn update_balance(
    request: axum::extract::Request,
    Json(payload): Json<PutBalanceRequest>,
) -> Response {
    tracing::info!(
        account_id = %payload.account_id,
        balance = %payload.balance,
        currency = ?payload.currency,
        "Updating account balance"
    );

    // Extract authenticated API key info from request extensions
    let auth_info = request
        .extensions()
        .get::<crate::middleware::auth::AuthenticatedApiKey>();

    let authenticated_account_id = match auth_info {
        Some(info) => info.account_id,
        None => {
            tracing::error!("No authentication info found in request");
            return create_error_response(
                StatusCode::UNAUTHORIZED,
                "missing_authentication",
                "Authentication required",
                None,
            );
        }
    };

    // Authorization check: Verify the API key belongs to the account being accessed
    if authenticated_account_id != payload.account_id {
        tracing::warn!(
            authenticated_account_id = %authenticated_account_id,
            requested_account_id = %payload.account_id,
            "Authorization failed: API key does not belong to the requested account"
        );
        return create_error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "You do not have permission to access this account",
            None,
        );
    }

    // Validate balance is non-negative
    if payload.balance < 0.0 {
        tracing::warn!(
            account_id = %payload.account_id,
            balance = %payload.balance,
            "Invalid balance: must be non-negative"
        );
        return create_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_balance",
            "Balance must be non-negative",
            None,
        );
    }

    // Get database connection pool
    let tracker = match POOL_STATE_TRACKER.get() {
        Some(t) => t,
        None => {
            tracing::error!("Database connection pool not initialized");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_unavailable",
                "Database connection pool not initialized",
                None,
            );
        }
    };

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

    let mut guard = ConnectionGuard(None);
    let mut conn = match tracker.get_connection().await {
        Ok(c) => {
            guard.0 = Some(c);
            guard.0.as_mut().unwrap()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get database connection");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "Failed to connect to database",
                None,
            );
        }
    };

    // First, verify the account exists by reading it
    let existing_account = match AccountBuilder::new()
        .id(payload.account_id)
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_balance()
        .expect_currency()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .read(Some(&mut conn))
        .await
    {
        Ok(acc) => {
            tracing::debug!(
                account_id = %acc.id,
                current_balance = %acc.balance,
                "Found existing account"
            );
            acc
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                account_id = %payload.account_id,
                "Account not found"
            );
            return create_error_response(
                StatusCode::NOT_FOUND,
                "account_not_found",
                &format!("Account with ID {} not found", payload.account_id),
                None,
            );
        }
    };

    // Update the account with new balance and optionally currency
    let updated_account = match AccountBuilder::new()
        .id(payload.account_id)
        .balance(payload.balance)
        .currency(payload.currency.unwrap_or(existing_account.currency))
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_balance()
        .expect_currency()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .update(Some(&mut conn))
        .await
    {
        Ok(acc) => {
            tracing::info!(
                account_id = %acc.id,
                old_balance = %existing_account.balance,
                new_balance = %acc.balance,
                currency = %acc.currency,
                "Account balance updated successfully"
            );
            acc
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                account_id = %payload.account_id,
                "Failed to update account balance"
            );
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "update_failed",
                &format!("Failed to update account balance: {}", e),
                None,
            );
        }
    };

    // Prepare response
    let response = AccountResponse {
        id: updated_account.id,
        business_name: updated_account.business_name,
        email: updated_account.email,
        balance: Some(updated_account.balance),
        currency: Some(updated_account.currency),
        status: Some(updated_account.status),
        created_at: Some(updated_account.created_at.to_rfc3339()),
    };

    tracing::info!(
        account_id = %response.id,
        "Balance update completed successfully"
    );

    (StatusCode::OK, Json(response)).into_response()
}

#[instrument(fields(service = "/api/v1/accounts/get"))]
pub async fn get_account(
    request: axum::extract::Request,
    Json(payload): Json<GetAccountRequest>,
) -> Response {
    tracing::info!(account_id = %payload.account_id, "Getting account");

    // Extract authenticated API key info from request extensions
    let auth_info = request
        .extensions()
        .get::<crate::middleware::auth::AuthenticatedApiKey>();

    let authenticated_account_id = match auth_info {
        Some(info) => info.account_id,
        None => {
            tracing::error!("No authentication info found in request");
            return create_error_response(
                StatusCode::UNAUTHORIZED,
                "missing_authentication",
                "Authentication required",
                None,
            );
        }
    };

    // Authorization check: Verify the API key belongs to the account being accessed
    if authenticated_account_id != payload.account_id {
        tracing::warn!(
            authenticated_account_id = %authenticated_account_id,
            requested_account_id = %payload.account_id,
            "Authorization failed: API key does not belong to the requested account"
        );
        return create_error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "You do not have permission to access this account",
            None,
        );
    }

    // Get database connection pool
    let tracker = match POOL_STATE_TRACKER.get() {
        Some(t) => t,
        None => {
            tracing::error!("Database connection pool not initialized");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_unavailable",
                "Database connection pool not initialized",
                None,
            );
        }
    };

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

    let mut guard = ConnectionGuard(None);
    let mut conn = match tracker.get_connection().await {
        Ok(c) => {
            guard.0 = Some(c);
            guard.0.as_mut().unwrap()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get database connection");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "Failed to connect to database",
                None,
            );
        }
    };

    // First, verify the account exists by reading it
    let existing_account = match AccountBuilder::new()
        .id(payload.account_id)
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_balance()
        .expect_currency()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .read(Some(&mut conn))
        .await
    {
        Ok(acc) => {
            tracing::debug!(
                account_id = %acc.id,
                current_balance = %acc.balance,
                "Found existing account"
            );
            acc
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                account_id = %payload.account_id,
                "Account not found"
            );
            return create_error_response(
                StatusCode::NOT_FOUND,
                "account_not_found",
                &format!("Account with ID {} not found", payload.account_id),
                None,
            );
        }
    };

    // Prepare response
    let response = AccountResponse {
        id: existing_account.id,
        business_name: existing_account.business_name,
        email: existing_account.email,
        balance: Some(existing_account.balance),
        currency: Some(existing_account.currency),
        status: Some(existing_account.status),
        created_at: Some(existing_account.created_at.to_rfc3339()),
    };

    tracing::info!(
        account_id = %response.id,
        "Account retrieved successfully"
    );

    (StatusCode::OK, Json(response)).into_response()
}

/// Update an account's details (business_name, email, status)
///
/// This handler:
/// 1. Authorization check (API key belongs to account)
/// 2. Validates the request
/// 3. Updates only the provided fields (optional fields are preserved if not provided)
/// 4. Returns the updated account details
///
/// Note: This does NOT update balance or currency - use update_balance for that
#[instrument(fields(service = "/api/v1/accounts/update"))]
pub async fn update_account(
    request: axum::extract::Request,
    account_id: Uuid,
    Json(payload): Json<UpdateAccountRequest>,
) -> Response {
    tracing::info!(
        account_id = %account_id,
        email = ?payload.email,
        business_name = ?payload.business_name,
        status = ?payload.status,
        "Updating account details"
    );

    // Extract authenticated API key info from request extensions
    let auth_info = request
        .extensions()
        .get::<crate::middleware::auth::AuthenticatedApiKey>();

    let authenticated_account_id = match auth_info {
        Some(info) => info.account_id,
        None => {
            tracing::error!("No authentication info found in request");
            return create_error_response(
                StatusCode::UNAUTHORIZED,
                "missing_authentication",
                "Authentication required",
                None,
            );
        }
    };

    // Authorization check: Verify the API key belongs to the account being accessed
    if authenticated_account_id != account_id {
        tracing::warn!(
            authenticated_account_id = %authenticated_account_id,
            requested_account_id = %account_id,
            "Authorization failed: API key does not belong to the requested account"
        );
        return create_error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "You do not have permission to access this account",
            None,
        );
    }

    // Get database connection pool
    let tracker = match POOL_STATE_TRACKER.get() {
        Some(t) => t,
        None => {
            tracing::error!("Database connection pool not initialized");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_unavailable",
                "Database connection pool not initialized",
                None,
            );
        }
    };

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

    let mut guard = ConnectionGuard(None);
    let mut conn = match tracker.get_connection().await {
        Ok(c) => {
            guard.0 = Some(c);
            guard.0.as_mut().unwrap()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get database connection");
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "Failed to connect to database",
                None,
            );
        }
    };

    // Build the update with only the fields that are provided
    let mut builder = AccountBuilder::new().id(account_id);

    // Only set fields that are provided (Some)
    if let Some(business_name) = payload.business_name {
        builder = builder.business_name(business_name);
    }
    if let Some(email) = payload.email {
        builder = builder.email(email);
    }
    if let Some(status) = payload.status {
        builder = builder.status(status);
    }

    // Update the account details
    let account = match builder
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_balance()
        .expect_currency()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .update(Some(&mut conn))
        .await
    {
        Ok(acc) => {
            tracing::info!(
                account_id = %acc.id,
                business_name = %acc.business_name,
                email = %acc.email,
                "Account updated successfully"
            );
            acc
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                account_id = %account_id,
                "Failed to update account"
            );
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "update_failed",
                &format!("Failed to update account: {}", e),
                None,
            );
        }
    };

    // Prepare response
    let response = AccountResponse {
        id: account.id,
        business_name: account.business_name,
        email: account.email,
        balance: Some(account.balance),
        currency: Some(account.currency),
        status: Some(account.status),
        created_at: Some(account.created_at.to_rfc3339()),
    };

    tracing::info!(
        account_id = %response.id,
        "Account update completed successfully"
    );

    (StatusCode::OK, Json(response)).into_response()
}
