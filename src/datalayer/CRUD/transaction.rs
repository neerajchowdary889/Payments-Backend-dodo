use super::types::{Transaction, TransactionStatus, TransactionType};
use crate::datalayer::CRUD::accounts::AccountBuilder;
use crate::datalayer::CRUD::helper::transaction::TransactionHelper;
use crate::datalayer::CRUD::money::{self, conversion};
use crate::datalayer::CRUD::sql_generator::sql_generator::{
    FluentInsert, FluentSelect, FluentUpdate,
};
use crate::datalayer::CRUD::types::Transactions;
use crate::datalayer::db_ops::constants::{DEFAULT_CURRENCY, DENOMINATOR, POOL_STATE_TRACKER};
use crate::errors::errors::ServiceError;
use chrono::Utc;
use sea_query::Value;
use sqlx::FromRow;
use sqlx::Postgres;
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// Builder for Transaction operations
///
/// ## Money Representation
///
/// All monetary amounts in transactions are stored as `i64` integers with 4 decimal
/// places of precision using the DENOMINATOR constant (10000).
///
/// - Storage unit = dollars * DENOMINATOR
/// - Example: $10.50 = 105000 storage units
/// - See `crate::datalayer::CRUD::money` for conversion utilities
#[derive(Debug, Clone, Default)]
pub struct TransactionBuilder {
    // Transaction fields
    pub id: Option<Uuid>,
    pub transaction_type: Option<TransactionType>,
    pub from_account_id: Option<Uuid>,
    pub to_account_id: Option<Uuid>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub status: Option<TransactionStatus>,
    pub idempotency_key: Option<String>,
    pub parent_tx_key: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,

    // Flags for dynamic RETURNING/SELECT
    pub get_id: Option<bool>,
    pub get_transaction_type: Option<bool>,
    pub get_from_account_id: Option<bool>,
    pub get_to_account_id: Option<bool>,
    pub get_amount: Option<bool>,
    pub get_currency: Option<bool>,
    pub get_status: Option<bool>,
    pub get_idempotency_key: Option<bool>,
    pub get_parent_tx_key: Option<bool>,
    pub get_description: Option<bool>,
    pub get_metadata: Option<bool>,
    pub get_error_code: Option<bool>,
    pub get_error_message: Option<bool>,
    pub get_created_at: Option<bool>,
    pub get_completed_at: Option<bool>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    // Setter methods
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn transaction_type(mut self, transaction_type: TransactionType) -> Self {
        self.transaction_type = Some(transaction_type);
        self
    }

    pub fn from_account_id(mut self, from_account_id: Uuid) -> Self {
        self.from_account_id = Some(from_account_id);
        self
    }

    pub fn to_account_id(mut self, to_account_id: Uuid) -> Self {
        self.to_account_id = Some(to_account_id);
        self
    }

    pub fn amount(mut self, amount: f64) -> Self {
        self.amount = Some(amount);
        self
    }

    pub fn currency(mut self, currency: String) -> Self {
        self.currency = Some(currency);
        self
    }

    pub fn status(mut self, status: TransactionStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn idempotency_key(mut self, idempotency_key: String) -> Self {
        self.idempotency_key = Some(idempotency_key);
        self
    }

    pub fn parent_tx_key(mut self, parent_tx_key: String) -> Self {
        self.parent_tx_key = Some(parent_tx_key);
        self
    }

    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn error_code(mut self, error_code: String) -> Self {
        self.error_code = Some(error_code);
        self
    }

    pub fn error_message(mut self, error_message: String) -> Self {
        self.error_message = Some(error_message);
        self
    }

    pub fn completed_at(mut self, completed_at: chrono::DateTime<chrono::Utc>) -> Self {
        self.completed_at = Some(completed_at);
        self
    }

    // Expect methods for dynamic RETURNING/SELECT
    pub fn expect_id(mut self) -> Self {
        self.get_id = Some(true);
        self
    }

    pub fn expect_transaction_type(mut self) -> Self {
        self.get_transaction_type = Some(true);
        self
    }

    pub fn expect_from_account_id(mut self) -> Self {
        self.get_from_account_id = Some(true);
        self
    }

    pub fn expect_to_account_id(mut self) -> Self {
        self.get_to_account_id = Some(true);
        self
    }

    pub fn expect_amount(mut self) -> Self {
        self.get_amount = Some(true);
        self
    }

    pub fn expect_currency(mut self) -> Self {
        self.get_currency = Some(true);
        self
    }

    pub fn expect_status(mut self) -> Self {
        self.get_status = Some(true);
        self
    }

    pub fn expect_idempotency_key(mut self) -> Self {
        self.get_idempotency_key = Some(true);
        self
    }

    pub fn expect_parent_tx_key(mut self) -> Self {
        self.get_parent_tx_key = Some(true);
        self
    }

    pub fn expect_description(mut self) -> Self {
        self.get_description = Some(true);
        self
    }

    pub fn expect_metadata(mut self) -> Self {
        self.get_metadata = Some(true);
        self
    }

    pub fn expect_error_code(mut self) -> Self {
        self.get_error_code = Some(true);
        self
    }

    pub fn expect_error_message(mut self) -> Self {
        self.get_error_message = Some(true);
        self
    }

    pub fn expect_created_at(mut self) -> Self {
        self.get_created_at = Some(true);
        self
    }

    pub fn expect_completed_at(mut self) -> Self {
        self.get_completed_at = Some(true);
        self
    }

    /// Create a new transaction in the database
    pub async fn create(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<Transaction, ServiceError> {
        // Validate required fields
        let transaction_type = self.transaction_type.as_ref().ok_or_else(|| {
            ServiceError::ValidationError("Missing transaction_type for create".to_string())
        })?;
        let amount = self.amount.as_ref().ok_or_else(|| {
            ServiceError::ValidationError("Missing amount for create".to_string())
        })?;

        // Validate amount is positive and within bounds
        money::validate_amount(*amount)?;

        let idempotency_key = self.idempotency_key.as_ref().ok_or_else(|| {
            ServiceError::ValidationError("Missing idempotency_key for create".to_string())
        })?;
        let parent_tx_key = self.parent_tx_key.as_ref().ok_or_else(|| {
            ServiceError::ValidationError("Missing parent_tx_key for create".to_string())
        })?;

        // Validate transaction type constraints
        self.validate_transaction_accounts(&transaction_type)?;

        struct ConnectionGuard(Option<PoolConnection<Postgres>>);
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
        let db_conn = match conn {
            Some(c) => c,
            None => {
                // We're acquiring the connection - manage it with guard
                let tracker = POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?;
                let c = tracker.get_connection().await.map_err(|e| {
                    ServiceError::DatabaseError(format!("Failed to get connection: {}", e))
                })?;
                guard.0 = Some(c);
                guard.0.as_mut().unwrap()
            }
        };

        // Before quering this table, check if the parent_tx_key exists for debit and credit transactions
        let validation_check: bool = Self::basic_create_checks(&self, db_conn).await?;
        if !validation_check {
            return Err(ServiceError::ValidationError(
                "Invalid transaction".to_string(),
            ));
        }

        // Check idempotency if key is provided
        let exists = Self::check_idempotency_exists(idempotency_key, &mut **db_conn).await;
        // DEBUG
        println!(
            ">>>> DEBUG :: Checking idempotency key result: {:#?}",
            exists
        );
        if exists.unwrap() == true {
            return Err(ServiceError::DuplicateTransaction(format!(
                "Transaction with idempotency_key '{}' already exists",
                idempotency_key
            )));
        }
        // DEBUG
        println!(
            ">>>> DEBUG :: Checking idempotency key: {} Done",
            idempotency_key
        );
        /*
         - Then convert to storage units
        */
        let mut usd_amount: i64 = 0;
        let mut currency = self.currency.clone();
        if self.amount.is_some() {
            /*
            If currency is not provided, then try to read the currency from the database
            - if that too not provided then use default USD
            */
            if currency.is_none() {
                let account_res = AccountBuilder::new()
                    .id(self.from_account_id.unwrap())
                    .expect_currency()
                    .read(Some(&mut *db_conn))
                    .await;

                if let Ok(account) = account_res {
                    currency = Some(account.currency);
                } else {
                    currency = Some(DEFAULT_CURRENCY.to_string());
                }
            }

            usd_amount = money::to_storage_units_with_conversion(
                self.amount.unwrap(),
                currency.clone().unwrap(),
            );
        }

        let mut status = TransactionStatus::Pending;
        match self.transaction_type.clone() {
            Some(TransactionType::Transfer) => {
                /*
                    If transaction type is transfer
                    1. create a record for transfer transaction.
                    2. In this transfer transaction state of the account table wont be changed
                    3. record will be created as pending
                */
                status = TransactionStatus::Pending;
            }
            Some(TransactionType::Debit) => {
                /*
                    If transaction type is debit
                    1. create a record for debit transaction.
                    2. In this debit transaction state of the account table will be changed
                    3. record will be created as pending
                */
                // Load the current account balance and do subtract opearation
                let _debit_state = TransactionHelper::new(self.clone(), db_conn)
                    .debit(usd_amount.clone())
                    .await?;
                status = TransactionStatus::Pending;
            }
            Some(TransactionType::Credit) => {
                /*
                    If transaction type is credit
                    1. create a record for credit transaction.
                    2. In this credit transaction state of the account table will be changed
                    3. record will be created as completed
                */
                let _credit_state = TransactionHelper::new(self.clone(), db_conn)
                    .credit(usd_amount.clone())
                    .await?;
                status = TransactionStatus::Completed;
            }
            _ => {
                return Err(ServiceError::ValidationError(
                    "Invalid transaction type".to_string(),
                ));
            }
        }

        // DEBUG
        println!(">>> DEBUG :: Status");

        // Determine which fields to return
        let get_id = self.get_id.unwrap_or(true);
        let get_transaction_type = self.get_transaction_type.unwrap_or(true);
        let get_from_account_id = self.get_from_account_id.unwrap_or(false);
        let get_to_account_id = self.get_to_account_id.unwrap_or(false);
        let get_amount = self.get_amount.unwrap_or(true);
        let get_currency = self.get_currency.unwrap_or(true);
        let get_status = self.get_status.unwrap_or(false);
        let get_idempotency_key = self.get_idempotency_key.unwrap_or(true);
        let get_parent_tx_key = self.get_parent_tx_key.unwrap_or(true);
        let get_description = self.get_description.unwrap_or(false);
        let get_metadata = self.get_metadata.unwrap_or(false);
        let get_error_code = self.get_error_code.unwrap_or(false);
        let get_error_message = self.get_error_message.unwrap_or(false);
        let get_created_at = self.get_created_at.unwrap_or(true);
        let get_completed_at = self.get_completed_at.unwrap_or(true);

        // Build the INSERT query
        let build_insert = || {
            // Convert enums to strings for sea-query
            let transaction_type_str = format!("{:?}", transaction_type).to_lowercase();
            let status_str = format!("{:?}", status).to_lowercase();

            let mut insert = FluentInsert::into(Transactions::Table)
                .value(Transactions::TransactionType, transaction_type_str)
                .value(Transactions::FromAccountId, self.from_account_id)
                .value(Transactions::ToAccountId, self.to_account_id)
                .value(Transactions::Amount, usd_amount)
                .value(Transactions::Currency, currency)
                .value(Transactions::Status, status_str)
                .value(Transactions::IdempotencyKey, idempotency_key)
                .value(Transactions::ParentTxKey, parent_tx_key)
                .value(Transactions::Description, self.description.clone())
                .value(Transactions::Metadata, self.metadata.clone())
                .value(Transactions::ErrorCode, self.error_code.clone())
                .value(Transactions::ErrorMessage, self.error_message.clone())
                .value(Transactions::CompletedAt, Utc::now());

            if get_id {
                insert = insert.returning(Transactions::Id);
            }
            if get_transaction_type {
                insert = insert.returning(Transactions::TransactionType);
            }
            if get_from_account_id {
                insert = insert.returning(Transactions::FromAccountId);
            }
            if get_to_account_id {
                insert = insert.returning(Transactions::ToAccountId);
            }
            if get_amount {
                insert = insert.returning(Transactions::Amount);
            }
            if get_currency {
                insert = insert.returning(Transactions::Currency);
            }
            if get_status {
                insert = insert.returning(Transactions::Status);
            }
            if get_idempotency_key {
                insert = insert.returning(Transactions::IdempotencyKey);
            }
            if get_parent_tx_key {
                insert = insert.returning(Transactions::ParentTxKey);
            }
            if get_description {
                insert = insert.returning(Transactions::Description);
            }
            if get_metadata {
                insert = insert.returning(Transactions::Metadata);
            }
            if get_error_code {
                insert = insert.returning(Transactions::ErrorCode);
            }
            if get_error_message {
                insert = insert.returning(Transactions::ErrorMessage);
            }
            if get_created_at {
                insert = insert.returning(Transactions::CreatedAt);
            }
            if get_completed_at {
                insert = insert.returning(Transactions::CompletedAt);
            }

            insert.render()
        };

        // Execute query
        let (sql, values) = build_insert();
        let query = sqlx::query::<Postgres>(&sql);
        let query = bind_query(query, values);

        let row = query.fetch_one(&mut **db_conn).await.map_err(|e| match e {
            sqlx::Error::RowNotFound => ServiceError::DatabaseError(e.to_string()),
            _ => ServiceError::DatabaseError(e.to_string()),
        })?;

        let result =
            Transaction::from_row(&row).map_err(|e| ServiceError::DatabaseError(e.to_string()));

        result
    }

    /// Update an existing transaction in the database
    pub async fn update(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<Transaction, ServiceError> {
        let id = self
            .id
            .ok_or_else(|| ServiceError::ValidationError("Missing ID for update".to_string()))?;

        let get_id = self.get_id.unwrap_or(true);
        let get_transaction_type = self.get_transaction_type.unwrap_or(false);
        let get_from_account_id = self.get_from_account_id.unwrap_or(false);
        let get_to_account_id = self.get_to_account_id.unwrap_or(false);
        let get_amount = self.get_amount.unwrap_or(false);
        let get_currency = self.get_currency.unwrap_or(true);
        let get_status = self.get_status.unwrap_or(false);
        let get_idempotency_key = self.get_idempotency_key.unwrap_or(false);
        let get_description = self.get_description.unwrap_or(false);
        let get_metadata = self.get_metadata.unwrap_or(false);
        let get_error_code = self.get_error_code.unwrap_or(false);
        let get_error_message = self.get_error_message.unwrap_or(false);
        let get_created_at = self.get_created_at.unwrap_or(true);
        let get_completed_at = self.get_completed_at.unwrap_or(true);
        let get_parent_tx_key = self.get_parent_tx_key.unwrap_or(true);

        let build_update = || {
            // Convert enum to string for sea-query
            let status_str = self
                .status
                .as_ref()
                .map(|s| format!("{:?}", s).to_lowercase());

            let mut update = FluentUpdate::table(Transactions::Table)
                .value(Transactions::Status, status_str)
                .value(Transactions::ErrorCode, self.error_code.clone())
                .value(Transactions::ErrorMessage, self.error_message.clone())
                .value(Transactions::CompletedAt, self.completed_at)
                .value(Transactions::Description, self.description.clone())
                .value(Transactions::Metadata, self.metadata.clone())
                .filter(Transactions::Id, id);

            if get_id {
                update = update.returning(Transactions::Id);
            }
            if get_transaction_type {
                update = update.returning(Transactions::TransactionType);
            }
            if get_from_account_id {
                update = update.returning(Transactions::FromAccountId);
            }
            if get_to_account_id {
                update = update.returning(Transactions::ToAccountId);
            }
            if get_amount {
                update = update.returning(Transactions::Amount);
            }
            if get_currency {
                update = update.returning(Transactions::Currency);
            }
            if get_status {
                update = update.returning(Transactions::Status);
            }
            if get_idempotency_key {
                update = update.returning(Transactions::IdempotencyKey);
            }
            if get_parent_tx_key {
                update = update.returning(Transactions::ParentTxKey);
            }
            if get_description {
                update = update.returning(Transactions::Description);
            }
            if get_metadata {
                update = update.returning(Transactions::Metadata);
            }
            if get_error_code {
                update = update.returning(Transactions::ErrorCode);
            }
            if get_error_message {
                update = update.returning(Transactions::ErrorMessage);
            }
            if get_created_at {
                update = update.returning(Transactions::CreatedAt);
            }
            if get_completed_at {
                update = update.returning(Transactions::CompletedAt);
            }

            update.render()
        };

        let (sql, values) = build_update();
        let query = sqlx::query::<Postgres>(&sql);
        let query = bind_query(query, values);

        struct ConnectionGuard(Option<PoolConnection<Postgres>>);
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
        let db_conn = match conn {
            Some(c) => c,
            None => {
                // We're acquiring the connection - manage it with guard
                let tracker = POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?;
                let c = tracker.get_connection().await.map_err(|e| {
                    ServiceError::DatabaseError(format!("Failed to get connection: {}", e))
                })?;
                guard.0 = Some(c);
                guard.0.as_mut().unwrap()
            }
        };

        let row = query.fetch_one(&mut **db_conn).await.map_err(|e| match e {
            sqlx::Error::RowNotFound => ServiceError::DatabaseError(e.to_string()),
            _ => ServiceError::DatabaseError(e.to_string()),
        })?;

        let result =
            Transaction::from_row(&row).map_err(|e| ServiceError::DatabaseError(e.to_string()));

        result
    }

    /// Read a transaction from the database
    pub async fn read(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<Transaction, ServiceError> {
        let get_id = self.get_id.unwrap_or(true);
        let get_transaction_type = self.get_transaction_type.unwrap_or(true);
        let get_from_account_id = self.get_from_account_id.unwrap_or(false);
        let get_to_account_id = self.get_to_account_id.unwrap_or(false);
        let get_amount = self.get_amount.unwrap_or(false);
        let get_currency = self.get_currency.unwrap_or(false);
        let get_status = self.get_status.unwrap_or(false);
        let get_idempotency_key = self.get_idempotency_key.unwrap_or(false);
        let get_parent_tx_key = self.get_parent_tx_key.unwrap_or(false);
        let get_description = self.get_description.unwrap_or(false);
        let get_metadata = self.get_metadata.unwrap_or(false);
        let get_error_code = self.get_error_code.unwrap_or(false);
        let get_error_message = self.get_error_message.unwrap_or(false);
        let get_created_at = self.get_created_at.unwrap_or(true);
        let get_completed_at = self.get_completed_at.unwrap_or(false);

        let build_select = || {
            let mut select = FluentSelect::from(Transactions::Table);

            if get_id {
                select = select.column(Transactions::Id);
            }
            if get_transaction_type {
                select = select.column(Transactions::TransactionType);
            }
            if get_from_account_id {
                select = select.column(Transactions::FromAccountId);
            }
            if get_to_account_id {
                select = select.column(Transactions::ToAccountId);
            }
            if get_amount {
                select = select.column(Transactions::Amount);
            }
            if get_currency {
                select = select.column(Transactions::Currency);
            }
            if get_status {
                select = select.column(Transactions::Status);
            }
            if get_idempotency_key {
                select = select.column(Transactions::IdempotencyKey);
            }
            if get_parent_tx_key {
                select = select.column(Transactions::ParentTxKey);
            }
            if get_description {
                select = select.column(Transactions::Description);
            }
            if get_metadata {
                select = select.column(Transactions::Metadata);
            }
            if get_error_code {
                select = select.column(Transactions::ErrorCode);
            }
            if get_error_message {
                select = select.column(Transactions::ErrorMessage);
            }
            if get_created_at {
                select = select.column(Transactions::CreatedAt);
            }
            if get_completed_at {
                select = select.column(Transactions::CompletedAt);
            }

            // Add filters
            if let Some(id) = self.id {
                select = select.filter(Transactions::Id, id);
            }
            if let Some(ref transaction_type) = self.transaction_type {
                let type_str = format!("{:?}", transaction_type).to_lowercase();
                select = select.filter(Transactions::TransactionType, type_str);
            }
            if let Some(from_account_id) = self.from_account_id {
                select = select.filter(Transactions::FromAccountId, from_account_id);
            }
            if let Some(to_account_id) = self.to_account_id {
                select = select.filter(Transactions::ToAccountId, to_account_id);
            }
            if let Some(ref status) = self.status {
                let status_str = format!("{:?}", status).to_lowercase();
                select = select.filter(Transactions::Status, status_str);
            }
            if let Some(ref idempotency_key) = self.idempotency_key {
                select = select.filter(Transactions::IdempotencyKey, idempotency_key.clone());
            }
            if let Some(ref parent_tx_key) = self.parent_tx_key {
                select = select.filter(Transactions::ParentTxKey, parent_tx_key.clone());
            }

            select.render()
        };

        let (sql, values) = build_select();
        let query = sqlx::query::<Postgres>(&sql);
        let query = bind_query(query, values);

        struct ConnectionGuard(Option<PoolConnection<Postgres>>);
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
        let db_conn = match conn {
            Some(c) => c,
            None => {
                // We're acquiring the connection - manage it with guard
                let tracker = POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?;
                let c = tracker.get_connection().await.map_err(|e| {
                    ServiceError::DatabaseError(format!("Failed to get connection: {}", e))
                })?;
                guard.0 = Some(c);
                guard.0.as_mut().unwrap()
            }
        };

        let row = query.fetch_one(&mut **db_conn).await.map_err(|e| match e {
            sqlx::Error::RowNotFound => ServiceError::DatabaseError(e.to_string()),
            _ => ServiceError::DatabaseError(e.to_string()),
        })?;

        let result =
            Transaction::from_row(&row).map_err(|e| ServiceError::DatabaseError(e.to_string()));

        result
    }

    /// Check if an idempotency key already exists
    pub async fn check_idempotency_exists(
        idempotency_key: &str,
        conn: &mut PgConnection,
    ) -> Result<bool, ServiceError> {
        println!(
            ">>>> DEBUG :: Checking idempotency key: {}",
            idempotency_key
        );
        let query = "SELECT COUNT(*) FROM transactions WHERE idempotency_key = $1";
        let count: i64 = sqlx::query_scalar(query)
            .bind(idempotency_key)
            .fetch_one(conn)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;

        println!(">>>> DEBUG :: Idempotency key exists: {}", count > 0);
        if count > 0 {
            return Ok(true);
        }
        Ok(false)
    }

    /// Validate transaction account constraints based on transaction type
    fn validate_transaction_accounts(
        &self,
        transaction_type: &TransactionType,
    ) -> Result<(), ServiceError> {
        match transaction_type {
            TransactionType::Credit => {
                if self.from_account_id.is_some() {
                    return Err(ServiceError::ValidationError(
                        "Credit transactions should not have from_account_id".to_string(),
                    ));
                }
                if self.to_account_id.is_none() {
                    return Err(ServiceError::ValidationError(
                        "Credit transactions must have to_account_id".to_string(),
                    ));
                }
            }
            TransactionType::Debit => {
                if self.from_account_id.is_none() {
                    return Err(ServiceError::ValidationError(
                        "Debit transactions must have from_account_id".to_string(),
                    ));
                }
                if self.to_account_id.is_some() {
                    return Err(ServiceError::ValidationError(
                        "Debit transactions should not have to_account_id".to_string(),
                    ));
                }
            }
            TransactionType::Transfer => {
                if self.from_account_id.is_none() {
                    return Err(ServiceError::ValidationError(
                        "Transfer transactions must have from_account_id".to_string(),
                    ));
                }
                if self.to_account_id.is_none() {
                    return Err(ServiceError::ValidationError(
                        "Transfer transactions must have to_account_id".to_string(),
                    ));
                }
                if self.from_account_id == self.to_account_id {
                    return Err(ServiceError::ValidationError(
                        "Transfer transactions cannot have the same from_account_id and to_account_id".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    #[allow(unused_doc_comments)]
    async fn basic_create_checks(
        &self,
        conn: &mut PoolConnection<Postgres>,
    ) -> Result<bool, ServiceError> {
        /// 1. Check if the from_account_id exists
        /// 2. if the account is positive and have balance to proceed with the transaction
        /// 3. Check if the to_account_id exists
        /// 4. If transaction_type is debit, check if the tranfer record, parent_tx_key exist and same or not.
        /// 5. If transaction_type is credit, check if the debit record, parent_tx_key exist and same or not.
        // Checks 1 and 2
        // as the key account_id is foreign key, we need to check if the account exists
        // Extract from_account_id before consuming self
        if let Some(from_account_id) = self.from_account_id {
            let from_account = AccountBuilder::new()
                .id(from_account_id)
                .expect_id()
                .expect_business_name()
                .expect_email()
                .expect_balance()
                .expect_currency()
                .expect_status()
                .expect_created_at()
                .expect_updated_at()
                .read(Some(conn))
                .await
                .map_err(|_| ServiceError::AccountNotFound(from_account_id.to_string()))?;

            // Check if account has sufficient balance
            // amount given here is not in usd - so need to convert to usd
            // balance from db would be in the usd
            if let Some(amount) = self.amount {
                // Determine transaction currency
                let transaction_currency = if let Some(ref curr) = self.currency {
                    curr.clone()
                } else {
                    from_account.currency.clone()
                };

                // Convert to Currency enum and then to USD
                let currency_enum = conversion::map_currency(transaction_currency)?;
                let converted_amount = conversion::to_usd(amount, currency_enum)?;

                if from_account.balance < converted_amount {
                    return Err(ServiceError::InsufficientBalance {
                        account_id: from_account_id.to_string(),
                        required: converted_amount,
                    });
                }
            }
        }

        // Check 3
        if let Some(to_account_id) = self.to_account_id {
            let to_account = AccountBuilder::new()
                .id(to_account_id)
                .expect_id()
                .expect_business_name()
                .expect_email()
                .expect_balance()
                .expect_currency()
                .expect_status()
                .expect_created_at()
                .expect_updated_at()
                .read(Some(conn))
                .await;

            if to_account.is_err() {
                return Err(ServiceError::AccountNotFound(to_account_id.to_string()));
            }
        }

        // Check 4: If transaction_type is debit, check if the transfer record with parent_tx_key exists
        // Check 5: If transaction_type is credit, check if the debit record with parent_tx_key exists
        if let Some(transaction_type) = self.transaction_type.as_ref() {
            if let Some(parent_tx_key) = self.parent_tx_key.as_ref() {
                match transaction_type {
                    TransactionType::Debit => {
                        // For Debit, verify that a Transfer transaction exists where:
                        // 1. Transfer parent_tx_key = Debit parent_tx_key (same transaction group)
                        // 2. Transfer from_account_id = Debit from_account_id
                        println!(
                            ">>>> DEBUG :: Debit validation - looking for Transfer with parent_tx_key: {} and from_account: {:?}",
                            parent_tx_key, self.from_account_id
                        );

                        // Skip validation if parent_tx_key is NULL (standalone debit)
                        if parent_tx_key != "NULL" {
                            let transfer_exists = TransactionBuilder::new()
                                .transaction_type(TransactionType::Transfer)
                                .parent_tx_key(parent_tx_key.clone())
                                .from_account_id(self.from_account_id.unwrap())
                                .expect_id()
                                .expect_amount()
                                .expect_parent_tx_key()
                                .expect_currency()
                                .expect_status()
                                .expect_created_at()
                                .expect_idempotency_key()
                                .expect_transaction_type()
                                .expect_from_account_id()
                                .read(Some(conn))
                                .await;

                            println!(
                                ">>>> DEBUG :: Transfer lookup result: {:?}",
                                transfer_exists
                                    .as_ref()
                                    .map(|t| (
                                        &t.id,
                                        &t.transaction_type,
                                        &t.parent_tx_key,
                                        &t.from_account_id
                                    ))
                                    .map_err(|e| format!("{:?}", e))
                            );

                            if transfer_exists.is_err() {
                                return Err(ServiceError::ValidationError(format!(
                                    "No transfer transaction found with parent_tx_key: {} and matching from_account",
                                    parent_tx_key
                                )));
                            }
                        }
                    }
                    TransactionType::Credit => {
                        // For Credit, verify that a Debit transaction with this parent_tx_key exists
                        let debit_exists = TransactionBuilder::new()
                            .transaction_type(TransactionType::Debit)
                            .parent_tx_key(parent_tx_key.clone())
                            .expect_id()
                            .expect_amount()
                            .expect_currency()
                            .expect_status()
                            .expect_transaction_type()
                            .expect_idempotency_key()
                            .expect_parent_tx_key()
                            .expect_created_at()
                            .read(Some(conn))
                            .await;

                        if debit_exists.is_err() {
                            return Err(ServiceError::ValidationError(format!(
                                "No debit transaction found with parent_tx_key: {}",
                                parent_tx_key
                            )));
                        }
                    }
                    TransactionType::Transfer => {
                        // Transfer transactions don't need this validation
                        // Out sqlgenerator doesn't support OR statements for now - so we need to do this twice.
                        // - its twice load on db and known issue.
                        // Check if EITHER idempotency_key OR parent_tx_key exists

                        // Check idempotency_key
                        let idempotency_exists = TransactionBuilder::new()
                            .idempotency_key(self.idempotency_key.clone().unwrap())
                            .expect_id()
                            .read(Some(conn))
                            .await;

                        if idempotency_exists.is_ok() {
                            return Err(ServiceError::ValidationError(format!(
                                "Transaction with idempotency_key '{}' already exists",
                                self.idempotency_key.as_ref().unwrap()
                            )));
                        }

                        // Check parent_tx_key
                        let parent_tx_exists = TransactionBuilder::new()
                            .parent_tx_key(self.parent_tx_key.clone().unwrap())
                            .expect_id()
                            .read(Some(conn))
                            .await;

                        if parent_tx_exists.is_ok() {
                            return Err(ServiceError::ValidationError(format!(
                                "Transaction with parent_tx_key '{}' already exists",
                                self.parent_tx_key.as_ref().unwrap()
                            )));
                        }
                    }
                }
            }
        }

        Ok(true)
    }
}

/// Helper macro to implement binding for different query types
macro_rules! impl_bind_values {
    ($func_name:ident, $query_type:ty) => {
        fn $func_name<'a>(mut query: $query_type, values: sea_query::Values) -> $query_type {
            for value in values.0 {
                query = match value {
                    Value::Bool(v) => query.bind(v),
                    Value::TinyInt(v) => query.bind(v.map(|x| x as i16)),
                    Value::SmallInt(v) => query.bind(v),
                    Value::Int(v) => query.bind(v),
                    Value::BigInt(v) => query.bind(v),
                    Value::TinyUnsigned(v) => query.bind(v.map(|x| x as i16)),
                    Value::SmallUnsigned(v) => query.bind(v.map(|x| x as i32)),
                    Value::Unsigned(v) => query.bind(v.map(|x| x as i64)),
                    Value::BigUnsigned(v) => query.bind(v.map(|x| x as i64)),
                    Value::Float(v) => query.bind(v),
                    Value::Double(v) => query.bind(v),
                    Value::String(v) => query.bind(v.map(|s| *s)),
                    Value::Char(v) => query.bind(v.map(|c| c.to_string())),
                    Value::Bytes(v) => query.bind(v.map(|b| *b)),
                    Value::Uuid(v) => query.bind(v.map(|u| *u)),
                    Value::Json(v) => query.bind(v.map(|j| *j)),
                    Value::ChronoDate(v) => query.bind(v.map(|d| *d)),
                    Value::ChronoTime(v) => query.bind(v.map(|t| *t)),
                    Value::ChronoDateTime(v) => query.bind(v.map(|dt| *dt)),
                    Value::ChronoDateTimeUtc(v) => query.bind(v.map(|dt| *dt)),
                    Value::ChronoDateTimeLocal(v) => query.bind(v.map(|dt| *dt)),
                    Value::ChronoDateTimeWithTimeZone(v) => query.bind(v.map(|dt| *dt)),
                };
            }
            query
        }
    };
}

impl_bind_values!(
    bind_query_as,
    sqlx::query::QueryAs<'a, Postgres, Transaction, sqlx::postgres::PgArguments>
);

impl_bind_values!(
    bind_query,
    sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>
);
