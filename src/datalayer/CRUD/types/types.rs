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
pub enum Transactions {
    Table,
    Id,
    TransactionType,
    FromAccountId,
    ToAccountId,
    Amount,
    Currency,
    Status,
    IdempotencyKey,
    Description,
    Metadata,
    ErrorCode,
    ErrorMessage,
    CreatedAt,
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
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub id: Uuid,
    pub business_name: String,
    pub email: String,
    pub balance: i64,
    pub currency: String,
    pub status: String,
    #[sqlx(default)]
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub rate_limit_per_minute: Option<i32>,
    pub rate_limit_per_hour: Option<i32>,
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: Uuid,
    pub transaction_type: TransactionType,
    pub from_account_id: Option<Uuid>,
    pub to_account_id: Option<Uuid>,
    pub amount: i64,
    pub currency: String,
    pub status: TransactionStatus,
    pub idempotency_key: Option<String>,
    pub description: Option<String>,
    #[sqlx(default)]
    pub metadata: Option<serde_json::Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
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
