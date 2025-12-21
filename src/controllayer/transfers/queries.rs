use crate::{
    datalayer::{
        CRUD::types::{Transaction, TransactionStatus, TransactionType},
        db_ops::constants::POOL_STATE_TRACKER,
    },
    errors::errors::ServiceError,
    handlers::transfer::{TransferListResponse, TransferResponse},
};
use tracing::instrument;
use uuid::Uuid;

/// Get all transactions (Transfer, Debit, Credit) by parent_tx_key
/// Authorization: Verifies user is involved in the transfer (from_account or to_account)
#[instrument(fields(service = "/api/v1/transfer/:id"))]
pub async fn get_transfer_by_parent_key(
    parent_tx_key: String,
    user_account_id: Uuid,
) -> Result<Vec<TransferResponse>, ServiceError> {
    tracing::info!(parent_tx_key = %parent_tx_key, user_account_id = %user_account_id, "Getting transfer by parent key");

    let tracker = POOL_STATE_TRACKER
        .get()
        .ok_or(ServiceError::DatabaseConnectionError)?;

    let mut conn = tracker.get_connection().await?;

    // Query all transactions with this parent_tx_key
    let transactions: Vec<Transaction> = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE parent_tx_key = $1 ORDER BY created_at ASC",
    )
    .bind(&parent_tx_key)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch transactions by parent_tx_key");
        ServiceError::DatabaseError(e.to_string())
    })?;

    if transactions.is_empty() {
        tracker.return_connection(conn);
        return Err(ServiceError::TransactionNotFound(parent_tx_key));
    }

    // Authorization: Check if user is involved in the transfer
    let transfer_txn = transactions
        .iter()
        .find(|t| matches!(t.transaction_type, TransactionType::Transfer));

    if let Some(transfer) = transfer_txn {
        let is_authorized = transfer.from_account_id == Some(user_account_id)
            || transfer.to_account_id == Some(user_account_id);

        if !is_authorized {
            tracker.return_connection(conn);
            return Err(ServiceError::Unauthorized(
                "You are not authorized to view this transfer".to_string(),
            ));
        }
    }

    let responses: Vec<TransferResponse> = transactions
        .into_iter()
        .map(|txn| TransferResponse {
            id: txn.id,
            transfer_type: format!("{:?}", txn.transaction_type).to_lowercase(),
            from_account: txn.from_account_id,
            to_account: txn.to_account_id,
            amount: txn.amount,
            currency: txn.currency,
            status: format!("{:?}", txn.status).to_lowercase(),
            description: txn.description,
            created_at: txn.created_at.to_rfc3339(),
            idempotency_key: txn.idempotency_key,
            parent_tx_key: txn.parent_tx_key,
        })
        .collect();

    tracker.return_connection(conn);
    Ok(responses)
}

/// Get a single transaction by ID
/// Authorization: If debit/credit, checks parent Transfer. If transfer, checks directly.
#[instrument(fields(service = "/api/v1/transfer/info/:id"))]
pub async fn get_transfer_by_id(
    transaction_id: Uuid,
    user_account_id: Uuid,
) -> Result<TransferResponse, ServiceError> {
    tracing::info!(transaction_id = %transaction_id, user_account_id = %user_account_id, "Getting transfer by ID");

    let tracker = POOL_STATE_TRACKER
        .get()
        .ok_or(ServiceError::DatabaseConnectionError)?;

    let mut conn = tracker.get_connection().await?;

    // Query transaction by ID
    let transaction: Transaction = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions WHERE id = $1"
    )
    .bind(transaction_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, transaction_id = %transaction_id, "Failed to fetch transaction by ID");
        match e {
            sqlx::Error::RowNotFound => ServiceError::TransactionNotFound(transaction_id.to_string()),
            _ => ServiceError::DatabaseError(e.to_string()),
        }
    })?;

    // Authorization logic based on transaction type
    match transaction.transaction_type {
        TransactionType::Transfer => {
            // For Transfer, check if user is from_account or to_account
            let is_authorized = transaction.from_account_id == Some(user_account_id)
                || transaction.to_account_id == Some(user_account_id);

            if !is_authorized {
                tracker.return_connection(conn);
                return Err(ServiceError::Unauthorized(
                    "You are not authorized to view this transfer".to_string(),
                ));
            }
        }
        TransactionType::Debit | TransactionType::Credit => {
            // For Debit/Credit, find parent Transfer and check authorization
            let parent_transfer: Result<Transaction, _> = sqlx::query_as::<_, Transaction>(
                "SELECT * FROM transactions WHERE parent_tx_key = $1 AND transaction_type = 'transfer'"
            )
            .bind(&transaction.parent_tx_key)
            .fetch_one(&mut *conn)
            .await;

            match parent_transfer {
                Ok(transfer) => {
                    let is_authorized = transfer.from_account_id == Some(user_account_id)
                        || transfer.to_account_id == Some(user_account_id);

                    if !is_authorized {
                        tracker.return_connection(conn);
                        return Err(ServiceError::Unauthorized(
                            "You are not authorized to view this transaction".to_string(),
                        ));
                    }
                }
                Err(_) => {
                    // No parent transfer found, check if user owns this transaction directly
                    let is_authorized = transaction.from_account_id == Some(user_account_id)
                        || transaction.to_account_id == Some(user_account_id);

                    if !is_authorized {
                        tracker.return_connection(conn);
                        return Err(ServiceError::Unauthorized(
                            "You are not authorized to view this transaction".to_string(),
                        ));
                    }
                }
            }
        }
    }

    tracker.return_connection(conn);

    Ok(TransferResponse {
        id: transaction.id,
        transfer_type: format!("{:?}", transaction.transaction_type).to_lowercase(),
        from_account: transaction.from_account_id,
        to_account: transaction.to_account_id,
        amount: transaction.amount,
        currency: transaction.currency,
        status: format!("{:?}", transaction.status).to_lowercase(),
        description: transaction.description,
        created_at: transaction.created_at.to_rfc3339(),
        idempotency_key: transaction.idempotency_key,
        parent_tx_key: transaction.parent_tx_key,
    })
}

/// List all Transfer-type transactions for the authenticated user
/// Returns transfers where user is either from_account OR to_account
#[instrument(fields(service = "/api/v1/transfer/list"))]
pub async fn list_transfers(
    user_account_id: Uuid,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<TransferListResponse, ServiceError> {
    let limit = limit.unwrap_or(50).min(100);
    let offset = offset.unwrap_or(0);

    tracing::info!(
        user_account_id = %user_account_id,
        limit = %limit,
        offset = %offset,
        "Listing transfers"
    );

    let tracker = POOL_STATE_TRACKER
        .get()
        .ok_or(ServiceError::DatabaseConnectionError)?;

    let mut conn = tracker.get_connection().await?;

    // Query transfers where user is involved (from_account OR to_account)
    let transactions: Vec<Transaction> = sqlx::query_as::<_, Transaction>(
        "SELECT * FROM transactions 
         WHERE transaction_type = 'transfer' 
         AND (from_account_id = $1 OR to_account_id = $1)
         ORDER BY created_at DESC 
         LIMIT $2 OFFSET $3",
    )
    .bind(user_account_id)
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch transfers");
        ServiceError::DatabaseError(e.to_string())
    })?;

    // Get total count
    let count_result: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM transactions 
         WHERE transaction_type = 'transfer' 
         AND (from_account_id = $1 OR to_account_id = $1)",
    )
    .bind(user_account_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;

    let responses: Vec<TransferResponse> = transactions
        .into_iter()
        .map(|txn| TransferResponse {
            id: txn.id,
            transfer_type: format!("{:?}", txn.transaction_type).to_lowercase(),
            from_account: txn.from_account_id,
            to_account: txn.to_account_id,
            amount: txn.amount,
            currency: txn.currency,
            status: format!("{:?}", txn.status).to_lowercase(),
            description: txn.description,
            created_at: txn.created_at.to_rfc3339(),
            idempotency_key: txn.idempotency_key,
            parent_tx_key: txn.parent_tx_key,
        })
        .collect();

    tracker.return_connection(conn);

    Ok(TransferListResponse {
        transfers: responses,
        total: count_result.0 as i32,
        limit,
        offset,
    })
}
