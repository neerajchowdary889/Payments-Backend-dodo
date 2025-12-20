use crate::datalayer::CRUD::money::from_storage_units;
use chrono::{DateTime, Utc};
use sea_query::Iden;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub mod db_tables {
    pub const ACCOUNTS: &str = "accounts";
    pub const API_KEYS: &str = "api_keys";
    pub const RATE_LIMIT_COUNTERS: &str = "rate_limit_counters";
    pub const TRANSACTIONS: &str = "transactions";
    pub const WEBHOOK_DELIVERIES: &str = "webhook_deliveries";
    pub const WEBHOOKS: &str = "webhooks";
}

// --- SEA QUERY IDENS ---

#[derive(Iden)]
pub enum Accounts {
    Table,
    Id,
    #[iden = "business_name"]
    BusinessName,
    Email,
    Balance,
    Currency,
    Status,
    Metadata,
    #[iden = "created_at"]
    CreatedAt,
    #[iden = "updated_at"]
    UpdatedAt,
}

#[derive(Iden)]
pub enum ApiKeys {
    Table,
    Id,
    #[iden = "account_id"]
    AccountId,
    #[iden = "key_hash"]
    KeyHash,
    #[iden = "key_prefix"]
    KeyPrefix,
    Name,
    Status,
    Permissions,
    #[iden = "last_used_at"]
    LastUsedAt,
    #[iden = "expires_at"]
    ExpiresAt,
    #[iden = "created_at"]
    CreatedAt,
    #[iden = "revoked_at"]
    RevokedAt,
}

#[derive(Iden)]
pub enum Transactions {
    Table,
    Id,
    #[iden = "transaction_type"]
    TransactionType,
    #[iden = "from_account_id"]
    FromAccountId,
    #[iden = "to_account_id"]
    ToAccountId,
    Amount,
    Currency,
    Status,
    #[iden = "idempotency_key"]
    IdempotencyKey,
    #[iden = "parent_tx_key"]
    ParentTxKey,
    Description,
    Metadata,
    #[iden = "error_code"]
    ErrorCode,
    #[iden = "error_message"]
    ErrorMessage,
    #[iden = "created_at"]
    CreatedAt,
    #[iden = "completed_at"]
    CompletedAt,
}

#[derive(Iden)]
pub enum Webhooks {
    Table,
    Id,
    AccountId,
    Url,
    Secret,
    Events,
    Status,
    MaxRetries,
    RetryBackoffSeconds,
    ConsecutiveFailures,
    LastFailureAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    USD,
    CAD,
    BRL,
    ARS, // Americas
    EUR,
    GBP,
    CHF, // Europe
    AED,
    KWD, // Middle East
    INR,
    CNY,
    KRW,
    JPY, // Asia
    AUD, // Oceania
}

/// Account struct matching the accounts table schema
///
/// Note: `balance` is stored in database as BIGINT (storage units with DENOMINATOR=10000)
/// but exposed as f64 (dollars) in the API. Conversion happens automatically via FromRow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub business_name: String,
    pub email: String,
    pub balance: f64, // API uses dollars, DB stores as i64 storage units
    pub currency: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Custom FromRow implementation to convert balance from i64 (storage units) to f64 (dollars)
impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Account {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let balance_storage_units: i64 = row.try_get("balance")?;

        Ok(Account {
            id: row.try_get("id")?,
            business_name: row.try_get("business_name")?,
            email: row.try_get("email")?,
            balance: from_storage_units(balance_storage_units),
            currency: row.try_get("currency")?,
            status: row.try_get("status")?,
            metadata: row.try_get("metadata").ok(),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

// --- API KEYS ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub account_id: Uuid,
    pub key_hash: String,
    pub key_prefix: String,
    pub name: Option<String>,
    pub status: String,
    pub permissions: Option<serde_json::Value>, // JSONB ["read", "write"]
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

// --- TRANSACTIONS ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum TransactionType {
    Credit,
    Debit,
    Transfer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
    Reversed,
}

/// Transaction struct matching the transactions table schema
///
/// Note: `amount` is stored in database as BIGINT (storage units with DENOMINATOR=10000)
/// but exposed as f64 (dollars) in the API. Conversion happens automatically via FromRow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub transaction_type: TransactionType,
    pub from_account_id: Option<Uuid>,
    pub to_account_id: Option<Uuid>,
    pub amount: f64, // API uses dollars, DB stores as i64 storage units
    pub currency: String,
    pub status: TransactionStatus,
    pub idempotency_key: String,
    pub parent_tx_key: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// Custom FromRow implementation to convert amount from i64 (storage units) to f64 (dollars)
impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Transaction {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let amount_storage_units: i64 = row.try_get("amount")?;

        Ok(Transaction {
            id: row.try_get("id")?,
            transaction_type: row.try_get("transaction_type")?,
            from_account_id: row.try_get("from_account_id").ok(),
            to_account_id: row.try_get("to_account_id").ok(),
            amount: from_storage_units(amount_storage_units),
            currency: row.try_get("currency")?,
            status: row.try_get("status")?,
            idempotency_key: row.try_get("idempotency_key")?,
            parent_tx_key: row.try_get("parent_tx_key")?,
            description: row.try_get("description").ok(),
            metadata: row.try_get("metadata").ok(),
            error_code: row.try_get("error_code").ok(),
            error_message: row.try_get("error_message").ok(),
            created_at: row.try_get("created_at")?,
            completed_at: row.try_get("completed_at").ok(),
        })
    }
}

// --- WEBHOOKS ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Webhook {
    pub id: Uuid,
    pub account_id: Uuid,
    pub url: String,
    pub secret: String,
    pub events: serde_json::Value, // JSONB
    pub status: String,
    pub max_retries: Option<i32>,
    pub retry_backoff_seconds: Option<i32>,
    pub consecutive_failures: Option<i32>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub transaction_id: Option<Uuid>,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub attempt_count: Option<i32>,
    pub max_attempts: Option<i32>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub http_status_code: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
}

// --- RATE LIMITS ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RateLimitCounter {
    pub id: Uuid,
    pub api_key_id: Uuid,
    pub window_start: DateTime<Utc>,
    pub window_type: String,
    pub request_count: i32,
    pub created_at: DateTime<Utc>,
}
