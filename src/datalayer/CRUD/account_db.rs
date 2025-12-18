use super::types::Account;
use crate::datalayer::CRUD::helper;
use crate::datalayer::db_ops::constants::POOL_STATE_TRACKER;
use crate::errors::errors::ServiceError;
use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};
use std::sync::Arc;
use uuid::Uuid;
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
    /// If connection is None, gets a connection from the global singleton pool
    pub async fn write_to_db(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<Uuid, ServiceError> {
        // Validate required fields
        let business_name = self
            .business_name
            .ok_or_else(|| ServiceError::MissingRequiredField("business_name".to_string()))?;
        let email = self
            .email
            .ok_or_else(|| ServiceError::MissingRequiredField("email".to_string()))?;

        // Validate email format
        if !helper::email_regex::is_valid_email(&email) {
            return Err(ServiceError::ValidationError(
                "Invalid email format".to_string(),
            ));
        }

        // Use defaults for optional fields
        let currency = self.currency.clone().unwrap_or_else(|| "USD".to_string());
        let balance = self.balance.unwrap_or(0);
        let status = self.status.clone().unwrap_or_else(|| "active".to_string());
        let metadata = self.metadata;

        // Validate balance is non-negative
        if balance < 0 {
            return Err(ServiceError::InvalidTransactionAmount);
        }

        // Execute with provided or acquired connection
        match conn {
            Some(connection) => {
                Self::execute_insert(
                    connection,
                    business_name,
                    email,
                    currency,
                    balance,
                    status,
                    metadata,
                )
                .await
            }
            None => {
                // Get tracker and use its get_connection method
                let tracker = POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?;

                // Get connection using tracker's smart connection management
                let mut owned_conn = tracker.get_connection().await.map_err(|e| {
                    ServiceError::DatabaseError(format!("Failed to get connection: {}", e))
                })?;

                // Execute the insert
                let result = Self::execute_insert(
                    &mut owned_conn,
                    business_name,
                    email,
                    currency,
                    balance,
                    status,
                    metadata,
                )
                .await;

                // Return connection to tracker
                tracker.return_connection(owned_conn);

                result
            }
        }
    }

    /// Helper function to execute the actual insert
    async fn execute_insert(
        conn: &mut sqlx::PgConnection,
        business_name: String,
        email: String,
        currency: String,
        balance: i64,
        status: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<Uuid, ServiceError> {
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
        .bind(&metadata)
        .fetch_one(conn)
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

    /// Update account in the database
    /// If connection is None, gets a connection from the global singleton pool
    pub async fn update(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<(), ServiceError> {
        // If connection is provided, use it; otherwise get one from pool
        match conn {
            Some(connection) => {
                // Use the provided connection
                self.execute(connection).await
            }
            None => {
                // Get tracker and use its get_connection method
                let tracker = POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?;

                // Get connection using tracker's smart connection management
                let mut owned_conn = tracker.get_connection().await.map_err(|e| {
                    ServiceError::DatabaseError(format!("Failed to get connection: {}", e))
                })?;

                // Execute the update
                let result = self.execute(&mut owned_conn).await;

                // Return connection to tracker
                tracker.return_connection(owned_conn);

                result
            }
        }
    }

    /// Helper function to execute the actual update
    async fn execute(self, conn: &mut PoolConnection<Postgres>) -> Result<(), ServiceError> {
        // TODO: Implement your update logic here
        // Example:
        // sqlx::query!(
        //     "UPDATE accounts SET business_name = $1, email = $2, updated_at = NOW() WHERE id = $3",
        //     self.business_name,
        //     self.email,
        //     self.id
        // )
        // .execute(conn)
        // .await
        // .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;

        Ok(())
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
