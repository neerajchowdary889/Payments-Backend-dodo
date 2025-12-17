use super::types::Account;
use crate::datalayer::db_ops::constants::POOL_STATE_TRACKER;
use crate::errors::errors::ServiceError;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use crate::datalayer::CRUD::helper;
/// Builder for creating new accounts
#[derive(Debug, Default)]
pub struct AccountBuilder {
    business_name: Option<String>,
    email: Option<String>,
    currency: Option<String>,
    balance: Option<i64>,
    status: Option<String>,
    metadata: Option<serde_json::Value>,
}

impl AccountBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn business_name(mut self, business_name: String) -> Self {
        self.business_name = Some(business_name);
        self
    }

    pub fn email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    pub fn currency(mut self, currency: String) -> Self {
        self.currency = Some(currency);
        self
    }

    pub fn balance(mut self, balance: i64) -> Self {
        self.balance = Some(balance);
        self
    }

    pub fn status(mut self, status: String) -> Self {
        self.status = Some(status);
        self
    }

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Write the account to the database
    /// If pool is None, uses the global singleton pool
    /// Atomic operation either commit or reject all changes
    pub async fn write_to_db(self, pool: Option<&Arc<PgPool>>) -> Result<Uuid, ServiceError> {
        // Validate required fields
        let business_name = self
            .business_name
            .ok_or_else(|| ServiceError::MissingRequiredField("business_name".to_string()))?;
        let email = self
            .email
            .ok_or_else(|| ServiceError::MissingRequiredField("email".to_string()))?;

        // Validate email format (basic check)
        if !helper::email_regex::is_valid_email(&email) {
            return Err(ServiceError::ValidationError(
                "Invalid email format".to_string(),
            ));
        }

        // Use defaults for optional fields
        let currency = self.currency.unwrap_or_else(|| "USD".to_string());
        let balance = self.balance.unwrap_or(0);
        let status = self.status.unwrap_or_else(|| "active".to_string());

        // Validate balance is non-negative
        if balance < 0 {
            return Err(ServiceError::InvalidTransactionAmount);
        }

        // Get pool reference
        let pool = match pool {
            Some(p) => p,
            None => {
                &POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?
                    .pool
            }
        };

        // Use sqlx::query_as for better type safety
        // This approach is cleaner and more maintainable than query!
        let account_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO accounts (
                business_name,
                email,
                currency,
                balance,
                status,
                metadata,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            RETURNING id
            "#,
        )
        .bind(&business_name)
        .bind(&email)
        .bind(&currency)
        .bind(balance)
        .bind(&status)
        .bind(&self.metadata)
        .fetch_one(pool.as_ref())
        .await
        .map_err(|e| {
            // Better error handling - check for specific errors
            match &e {
                sqlx::Error::Database(db_err) => {
                    // Check for unique constraint violation (duplicate email)
                    if db_err.code().as_deref() == Some("23505") {
                        ServiceError::AccountAlreadyExists(email.clone())
                    } else {
                        ServiceError::DatabaseError(e.to_string())
                    }
                }
                _ => ServiceError::DatabaseError(e.to_string()),
            }
        })?;

        Ok(account_id)
    }
}

/// Account database operations
pub struct AccountDB;

impl AccountDB {
    /// Get account by ID
    pub async fn get_by_id(id: Uuid, pool: Option<&Arc<PgPool>>) -> Result<Account, ServiceError> {
        let pool = match pool {
            Some(p) => p,
            None => {
                &POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?
                    .pool
            }
        };

        let account = sqlx::query_as::<_, Account>(r#"SELECT * FROM accounts WHERE id = $1"#)
            .bind(id)
            .fetch_one(pool.as_ref())
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => ServiceError::AccountNotFound(id.to_string()),
                _ => ServiceError::DatabaseError(e.to_string()),
            })?;

        Ok(account)
    }

    /// Get account by email
    pub async fn get_by_email(
        email: &str,
        pool: Option<&Arc<PgPool>>,
    ) -> Result<Account, ServiceError> {
        let pool = match pool {
            Some(p) => p,
            None => {
                &POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?
                    .pool
            }
        };

        let account = sqlx::query_as::<_, Account>(r#"SELECT * FROM accounts WHERE email = $1"#)
            .bind(email)
            .fetch_one(pool.as_ref())
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => ServiceError::AccountNotFound(email.to_string()),
                _ => ServiceError::DatabaseError(e.to_string()),
            })?;

        Ok(account)
    }

    /// Update account balance
    pub async fn update_balance(
        id: Uuid,
        new_balance: i64,
        pool: Option<&Arc<PgPool>>,
    ) -> Result<(), ServiceError> {
        let pool = match pool {
            Some(p) => p,
            None => {
                &POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?
                    .pool
            }
        };

        sqlx::query!(
            r#"UPDATE accounts SET balance = $1, updated_at = NOW() WHERE id = $2"#,
            new_balance,
            id
        )
        .execute(pool.as_ref())
        .await
        .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
