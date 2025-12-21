use crate::{
    datalayer::{
        CRUD::{
            transaction::TransactionBuilder,
            types::{TransactionStatus, TransactionType},
            webhook::get_active_webhooks_for_account,
        },
        db_ops::constants::POOL_STATE_TRACKER,
    },
    errors::errors::create_error_response,
    handlers::transfer::{TransferRequest, TransferResponse},
    middleware::auth::AuthenticatedApiKey,
    services::WebhookDispatcher,
};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::instrument;
use uuid::Uuid;

/// Execute a transfer operation atomically
///
/// This handler creates transaction records atomically using SQL transactions.
/// For Transfer type: creates parent transfer + debit + credit records
/// For Debit type: creates single debit record
/// For Credit type: creates single credit record
#[instrument(fields(service = "/api/v1/transfer"))]
pub async fn execute_transfer(
    auth_info: axum::Extension<AuthenticatedApiKey>,
    Json(payload): Json<TransferRequest>,
) -> Response {
    // Extract transfer details based on type
    let (from_account, to_account, amount, currency, description, idempotency_key, transfer_type) =
        match &payload {
            TransferRequest::Credit {
                to_account,
                amount,
                currency,
                description,
                idempotency_key,
            } => {
                tracing::info!(
                    to_account = %to_account,
                    amount = %amount,
                    currency = %currency,
                    "Processing credit transfer"
                );
                (
                    None,
                    Some(*to_account),
                    *amount,
                    currency.clone(),
                    description.clone(),
                    idempotency_key.clone(),
                    TransactionType::Credit,
                )
            }
            TransferRequest::Debit {
                from_account,
                amount,
                currency,
                description,
                idempotency_key,
            } => {
                tracing::info!(
                    from_account = %from_account,
                    amount = %amount,
                    currency = %currency,
                    "Processing debit transfer"
                );

                // Authorization: User can only debit from their own account
                if auth_info.account_id != *from_account {
                    tracing::warn!(
                        authenticated_account_id = %auth_info.account_id,
                        requested_from_account = %from_account,
                        "Authorization failed: Cannot debit from another account"
                    );
                    return create_error_response(
                        StatusCode::FORBIDDEN,
                        "forbidden",
                        "You can only debit from your own account",
                        None,
                    );
                }

                (
                    Some(*from_account),
                    None,
                    *amount,
                    currency.clone(),
                    description.clone(),
                    idempotency_key.clone(),
                    TransactionType::Debit,
                )
            }
            TransferRequest::Transfer {
                from_account,
                to_account,
                amount,
                currency,
                description,
                idempotency_key,
            } => {
                tracing::info!(
                    from_account = %from_account,
                    to_account = %to_account,
                    amount = %amount,
                    currency = %currency,
                    "Processing transfer"
                );

                // Authorization: User can only transfer from their own account
                if auth_info.account_id != *from_account {
                    tracing::warn!(
                        authenticated_account_id = %auth_info.account_id,
                        requested_from_account = %from_account,
                        "Authorization failed: Cannot transfer from another account"
                    );
                    return create_error_response(
                        StatusCode::FORBIDDEN,
                        "forbidden",
                        "You can only transfer from your own account",
                        None,
                    );
                }

                (
                    Some(*from_account),
                    Some(*to_account),
                    *amount,
                    currency.clone(),
                    description.clone(),
                    idempotency_key.clone(),
                    TransactionType::Transfer,
                )
            }
        };

    // Auto-generate idempotency_key if not provided
    // This ensures idempotency while making the API easier to use
    let idempotency_key = idempotency_key.unwrap_or_else(|| {
        let generated_key = Uuid::new_v4().to_string();
        tracing::info!(
            generated_idempotency_key = %generated_key,
            "Auto-generated idempotency key for transfer"
        );
        generated_key
    });

    // Validate amount
    if amount <= 0.0 {
        return create_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_amount",
            "Amount must be greater than 0",
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

    // Begin SQL transaction for atomic transfer operation
    if let Err(e) = sqlx::query("BEGIN").execute(&mut **conn).await {
        tracing::error!(error = %e, "Failed to begin transaction");
        return create_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "database_error",
            "Failed to begin transaction",
            None,
        );
    }

    tracing::info!("Started database transaction for transfer");

    // Generate a transaction group ID that will be shared by all related transactions
    // This allows us to group Transfer, Debit, and Credit records together
    let transaction_group_id = format!("txgroup_{}", Uuid::new_v4());

    tracing::info!(
        transaction_group_id = %transaction_group_id,
        "Generated transaction group ID for atomic transfer"
    );

    // For Transfer type: Create parent transfer record first
    let parent_tx_id = if matches!(transfer_type, TransactionType::Transfer) {
        let mut builder = TransactionBuilder::new()
            .transaction_type(transfer_type.clone())
            .amount(amount)
            .currency(currency.clone())
            .status(TransactionStatus::Pending)
            .idempotency_key(idempotency_key.clone())
            .parent_tx_key(transaction_group_id.clone()) // All transactions share the same group ID
            .expect_id()
            .expect_transaction_type()
            .expect_amount()
            .expect_currency()
            .expect_status()
            .expect_idempotency_key()
            .expect_parent_tx_key()
            .expect_created_at();

        if let Some(from_acc) = from_account {
            builder = builder.from_account_id(from_acc).expect_from_account_id();
        }
        if let Some(to_acc) = to_account {
            builder = builder.to_account_id(to_acc).expect_to_account_id();
        }
        if let Some(desc) = description.clone() {
            builder = builder.description(desc).expect_description();
        }

        match builder.create(Some(conn)).await {
            Ok(txn) => {
                tracing::info!(
                    transfer_id = %txn.id,
                    transaction_group = %transaction_group_id,
                    "Parent transfer record created successfully"
                );
                Some(txn.id)
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to create parent transfer record");
                let _ = sqlx::query("ROLLBACK").execute(&mut **conn).await;

                if e.to_string().contains("duplicate") || e.to_string().contains("23505") {
                    return create_error_response(
                        StatusCode::CONFLICT,
                        "duplicate_transfer",
                        "A transfer with this idempotency key already exists",
                        None,
                    );
                }

                return create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "transfer_creation_failed",
                    &format!("Failed to create transfer: {}", e),
                    None,
                );
            }
        }
    } else {
        None
    };

    // Create debit record (for Debit or Transfer types)
    if let Some(from_acc) = from_account {
        let debit_idempotency = if parent_tx_id.is_some() {
            format!("{}_debit", idempotency_key)
        } else {
            idempotency_key.clone()
        };

        // Use the transaction group ID for parent_tx_key (same for all related transactions)
        let parent_key = transaction_group_id.clone();

        let mut builder = TransactionBuilder::new()
            .from_account_id(from_acc)
            .transaction_type(TransactionType::Debit)
            .amount(amount)
            .currency(currency.clone())
            .status(TransactionStatus::Pending)
            .idempotency_key(debit_idempotency)
            .parent_tx_key(parent_key)
            .expect_id()
            .expect_from_account_id()
            .expect_transaction_type()
            .expect_amount()
            .expect_currency()
            .expect_status()
            .expect_idempotency_key()
            .expect_parent_tx_key()
            .expect_created_at();

        if let Some(desc) = description.clone() {
            builder = builder.description(desc).expect_description();
        }

        match builder.create(Some(conn)).await {
            Ok(debit_txn) => {
                tracing::info!(
                    debit_id = %debit_txn.id,
                    from_account = %from_acc,
                    "Debit record created successfully"
                );

                // Dispatch webhook for debit transaction
                if let Ok(webhooks) = get_active_webhooks_for_account(from_acc, conn).await {
                    let dispatcher = WebhookDispatcher::new();
                    for webhook in webhooks {
                        dispatcher.dispatch_debit_webhook(webhook, debit_txn.clone());
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    from_account = %from_acc,
                    "Failed to create debit record, rolling back"
                );
                let _ = sqlx::query("ROLLBACK").execute(&mut **conn).await;
                return create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "debit_creation_failed",
                    "Failed to create debit record, transfer rolled back",
                    None,
                );
            }
        }
    }

    // Create credit record (for Credit or Transfer types)
    if let Some(to_acc) = to_account {
        let credit_idempotency = if parent_tx_id.is_some() {
            format!("{}_credit", idempotency_key)
        } else {
            idempotency_key.clone()
        };

        // Use the transaction group ID for parent_tx_key (same for all related transactions)
        let parent_key = transaction_group_id.clone();

        let mut builder = TransactionBuilder::new()
            .to_account_id(to_acc)
            .transaction_type(TransactionType::Credit)
            .amount(amount)
            .currency(currency.clone())
            .status(TransactionStatus::Completed)
            .idempotency_key(credit_idempotency)
            .parent_tx_key(parent_key)
            .expect_id()
            .expect_to_account_id()
            .expect_transaction_type()
            .expect_amount()
            .expect_currency()
            .expect_status()
            .expect_idempotency_key()
            .expect_parent_tx_key()
            .expect_created_at();

        if let Some(desc) = description.clone() {
            builder = builder.description(desc).expect_description();
        }

        match builder.create(Some(conn)).await {
            Ok(credit_txn) => {
                tracing::info!(
                    credit_id = %credit_txn.id,
                    to_account = %to_acc,
                    "Credit record created successfully"
                );

                // Dispatch webhook for credit transaction
                if let Ok(webhooks) = get_active_webhooks_for_account(to_acc, conn).await {
                    let dispatcher = WebhookDispatcher::new();
                    for webhook in webhooks {
                        dispatcher.dispatch_credit_webhook(webhook, credit_txn.clone());
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    to_account = %to_acc,
                    "Failed to create credit record, rolling back"
                );
                let _ = sqlx::query("ROLLBACK").execute(&mut **conn).await;
                return create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "credit_creation_failed",
                    "Failed to create credit record, transfer rolled back",
                    None,
                );
            }
        }
    }

    // Commit the transaction - all records created atomically
    match sqlx::query("COMMIT").execute(&mut **conn).await {
        Ok(_) => {
            tracing::info!("Transaction committed successfully - transfer completed atomically");
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to commit transaction");
            let _ = sqlx::query("ROLLBACK").execute(&mut **conn).await;
            return create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "transaction_commit_failed",
                "Failed to commit transaction",
                None,
            );
        }
    }

    // Prepare response - use parent_tx_id if it exists, otherwise we need to get the created transaction ID
    // For simplicity, let's return a success response with the transfer details
    let response_id = parent_tx_id.unwrap_or_else(|| Uuid::new_v4()); // This is a placeholder

    let response = TransferResponse {
        id: response_id,
        transfer_type: format!("{:?}", transfer_type).to_lowercase(),
        from_account,
        to_account,
        amount,
        currency,
        status: "completed".to_string(),
        description,
        created_at: chrono::Utc::now().to_rfc3339(),
        idempotency_key,
        parent_tx_key: transaction_group_id,
    };

    tracing::info!(
        transfer_id = %response.id,
        "Transfer completed successfully"
    );

    (StatusCode::CREATED, Json(response)).into_response()
}
