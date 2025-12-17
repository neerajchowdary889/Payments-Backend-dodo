use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub mod DBTables {
    pub const ACCOUNTS: &str = "accounts";
    pub const API_KEYS: &str = "api_keys";
    pub const RATE_LIMIT_COUNTERS: &str = "rate_limit_counters";
    pub const TRANSACTIONS: &str = "transactions";
    pub const WEBHOOK_DELIVERIES: &str = "webhook_deliveries";
    pub const WEBHOOKS: &str = "webhooks";
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

// Some of the countries currencies where dodo payments are supported (as per website : https://dodopayments.com)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    // Americas
    USD, // United States Dollar
    CAD, // Canadian Dollar
    BRL, // Brazil Real
    ARS, // Argentina Peso

    // Europe
    EUR, // Euro (Germany, Netherlands, etc.)
    GBP, // British Pound
    CHF, // Swiss Franc (optional but common)

    // Middle East
    AED, // UAE Dirham (Dubai)
    KWD, // Kuwaiti Dinar

    // Asia
    INR, // Indian Rupee
    CNY, // Chinese Yuan
    KRW, // South Korean Won
    JPY, // Japanese Yen

    // Oceania
    AUD, // Australian Dollar
}
