use super::types::ApiKey;
use crate::datalayer::CRUD::accounts::AccountBuilder;
use crate::datalayer::CRUD::sql_generator::sql_generator::{
    FluentInsert, FluentSelect, FluentUpdate,
};

use crate::datalayer::CRUD::types::ApiKeys;
use crate::datalayer::db_ops::constants::POOL_STATE_TRACKER;
use crate::errors::errors::ServiceError;
use chrono::{DateTime, Utc};
use sea_query::Value;
use sqlx::FromRow;
use sqlx::Postgres;
use sqlx::pool::PoolConnection;
use uuid::Uuid;

/// Builder for creating and managing API keys
///
/// ## Security Note
///
/// API keys are stored as hashed values for security. The `key_hash` field
/// should contain a securely hashed version of the actual API key.
/// The `key_prefix` stores the first few characters for identification.
#[derive(Debug, Clone)]
pub struct ApiKeyBuilder {
    id: Option<Uuid>,
    account_id: Option<Uuid>,
    key_hash: Option<String>,
    key_prefix: Option<String>,
    name: Option<String>,
    status: Option<String>,
    rate_limit_per_minute: Option<i32>,
    rate_limit_per_hour: Option<i32>,
    permissions: Option<serde_json::Value>,
    last_used_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    // Fields to return
    get_id: Option<bool>,
    get_account_id: Option<bool>,
    get_key_hash: Option<bool>,
    get_key_prefix: Option<bool>,
    get_name: Option<bool>,
    get_status: Option<bool>,
    get_rate_limit_per_minute: Option<bool>,
    get_rate_limit_per_hour: Option<bool>,
    get_permissions: Option<bool>,
    get_last_used_at: Option<bool>,
    get_expires_at: Option<bool>,
    get_created_at: Option<bool>,
    get_revoked_at: Option<bool>,
}

impl Default for ApiKeyBuilder {
    fn default() -> Self {
        Self {
            id: None,
            account_id: None,
            key_hash: None,
            key_prefix: None,
            name: None,
            status: None,
            rate_limit_per_minute: None,
            rate_limit_per_hour: None,
            permissions: None,
            last_used_at: None,
            expires_at: None,
            revoked_at: None,
            get_id: None,
            get_account_id: None,
            get_key_hash: None,
            get_key_prefix: None,
            get_name: None,
            get_status: None,
            get_rate_limit_per_minute: None,
            get_rate_limit_per_hour: None,
            get_permissions: None,
            get_last_used_at: None,
            get_expires_at: None,
            get_created_at: None,
            get_revoked_at: None,
        }
    }
}

impl ApiKeyBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    // Setter methods
    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn account_id(mut self, account_id: Uuid) -> Self {
        self.account_id = Some(account_id);
        self
    }

    pub fn key_hash(mut self, key_hash: String) -> Self {
        self.key_hash = Some(key_hash);
        self
    }

    pub fn key_prefix(mut self, key_prefix: String) -> Self {
        self.key_prefix = Some(key_prefix);
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn status(mut self, status: String) -> Self {
        self.status = Some(status);
        self
    }

    pub fn rate_limit_per_minute(mut self, rate_limit: i32) -> Self {
        self.rate_limit_per_minute = Some(rate_limit);
        self
    }

    pub fn rate_limit_per_hour(mut self, rate_limit: i32) -> Self {
        self.rate_limit_per_hour = Some(rate_limit);
        self
    }

    pub fn permissions(mut self, permissions: serde_json::Value) -> Self {
        self.permissions = Some(permissions);
        self
    }

    pub fn last_used_at(mut self, last_used_at: DateTime<Utc>) -> Self {
        self.last_used_at = Some(last_used_at);
        self
    }

    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn revoked_at(mut self, revoked_at: DateTime<Utc>) -> Self {
        self.revoked_at = Some(revoked_at);
        self
    }

    // Expect methods for field selection
    pub fn expect_id(mut self) -> Self {
        self.get_id = Some(true);
        self
    }

    pub fn expect_account_id(mut self) -> Self {
        self.get_account_id = Some(true);
        self
    }

    pub fn expect_key_hash(mut self) -> Self {
        self.get_key_hash = Some(true);
        self
    }

    pub fn expect_key_prefix(mut self) -> Self {
        self.get_key_prefix = Some(true);
        self
    }

    pub fn expect_name(mut self) -> Self {
        self.get_name = Some(true);
        self
    }

    pub fn expect_status(mut self) -> Self {
        self.get_status = Some(true);
        self
    }

    pub fn expect_rate_limit_per_minute(mut self) -> Self {
        self.get_rate_limit_per_minute = Some(true);
        self
    }

    pub fn expect_rate_limit_per_hour(mut self) -> Self {
        self.get_rate_limit_per_hour = Some(true);
        self
    }

    pub fn expect_permissions(mut self) -> Self {
        self.get_permissions = Some(true);
        self
    }

    pub fn expect_last_used_at(mut self) -> Self {
        self.get_last_used_at = Some(true);
        self
    }

    pub fn expect_expires_at(mut self) -> Self {
        self.get_expires_at = Some(true);
        self
    }

    pub fn expect_created_at(mut self) -> Self {
        self.get_created_at = Some(true);
        self
    }

    pub fn expect_revoked_at(mut self) -> Self {
        self.get_revoked_at = Some(true);
        self
    }

    /// Create a new API key in the database
    pub async fn create(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<ApiKey, ServiceError> {
        // Validate required fields
        let account_id = self
            .account_id
            .ok_or_else(|| ServiceError::MissingRequiredField("account_id".to_string()))?;
        let key_hash = self
            .key_hash
            .ok_or_else(|| ServiceError::MissingRequiredField("key_hash".to_string()))?;
        let key_prefix = self
            .key_prefix
            .ok_or_else(|| ServiceError::MissingRequiredField("key_prefix".to_string()))?;

        // Use defaults for optional fields
        let status = self.status.unwrap_or_else(|| "active".to_string());
        let permissions = self
            .permissions
            .or_else(|| Some(serde_json::json!(["read", "write"])));

        // Capture flags for returning
        let get_id = self.get_id.unwrap_or(true);
        let get_account_id = self.get_account_id.unwrap_or(false);
        let get_key_hash = self.get_key_hash.unwrap_or(false);
        let get_key_prefix = self.get_key_prefix.unwrap_or(false);
        let get_name = self.get_name.unwrap_or(false);
        let get_status = self.get_status.unwrap_or(false);
        let get_permissions = self.get_permissions.unwrap_or(false);
        let get_last_used_at = self.get_last_used_at.unwrap_or(false);
        let get_expires_at = self.get_expires_at.unwrap_or(false);
        let get_created_at = self.get_created_at.unwrap_or(true);
        let get_revoked_at = self.get_revoked_at.unwrap_or(false);

        // Build the insert query
        let build_insert = || {
            let mut insert = FluentInsert::into(ApiKeys::Table)
                .value(ApiKeys::AccountId, account_id)
                .value(ApiKeys::KeyHash, key_hash.clone())
                .value(ApiKeys::KeyPrefix, key_prefix.clone())
                .value(ApiKeys::Name, self.name.clone())
                .value(ApiKeys::Status, status.clone())
                .value(ApiKeys::Permissions, permissions.clone())
                .value(ApiKeys::LastUsedAt, self.last_used_at)
                .value(ApiKeys::ExpiresAt, self.expires_at)
                .value(ApiKeys::CreatedAt, Utc::now())
                .value(ApiKeys::RevokedAt, self.revoked_at);

            if get_id {
                insert = insert.returning(ApiKeys::Id);
            }
            if get_account_id {
                insert = insert.returning(ApiKeys::AccountId);
            }
            if get_key_hash {
                insert = insert.returning(ApiKeys::KeyHash);
            }
            if get_key_prefix {
                insert = insert.returning(ApiKeys::KeyPrefix);
            }
            if get_name {
                insert = insert.returning(ApiKeys::Name);
            }
            if get_status {
                insert = insert.returning(ApiKeys::Status);
            }

            if get_permissions {
                insert = insert.returning(ApiKeys::Permissions);
            }
            if get_last_used_at {
                insert = insert.returning(ApiKeys::LastUsedAt);
            }
            if get_expires_at {
                insert = insert.returning(ApiKeys::ExpiresAt);
            }
            if get_created_at {
                insert = insert.returning(ApiKeys::CreatedAt);
            }
            if get_revoked_at {
                insert = insert.returning(ApiKeys::RevokedAt);
            }

            insert.render()
        };

        let handle_error = |e: sqlx::Error| match &e {
            sqlx::Error::Database(db_err) => {
                if db_err.code().as_deref() == Some("23505") {
                    ServiceError::ValidationError("API key already exists".to_string())
                } else if db_err.code().as_deref() == Some("23503") {
                    ServiceError::AccountNotFound(account_id.to_string())
                } else {
                    ServiceError::DatabaseError(e.to_string())
                }
            }
            _ => ServiceError::DatabaseError(e.to_string()),
        };

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

        // Verify account exists
        let account_check = AccountBuilder::new()
            .id(account_id)
            .expect_id()
            .expect_business_name()
            .expect_email()
            .expect_balance()
            .expect_currency()
            .expect_status()
            .expect_created_at()
            .expect_updated_at()
            .read(Some(db_conn))
            .await;

        if account_check.is_err() {
            return Err(ServiceError::AccountNotFound(account_id.to_string()));
        }

        let (sql, values) = build_insert();
        let query = sqlx::query::<Postgres>(&sql);
        let query = bind_query(query, values);

        let row = query
            .fetch_one(&mut **db_conn)
            .await
            .map_err(handle_error)?;
        let result = ApiKey::from_row(&row).map_err(|e| ServiceError::DatabaseError(e.to_string()));

        result
    }

    /// Update an existing API key in the database
    pub async fn update(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<ApiKey, ServiceError> {
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

        let id = self
            .id
            .ok_or_else(|| ServiceError::ValidationError("Missing ID for update".to_string()))?;

        let get_id = self.get_id.unwrap_or(true);
        let get_account_id = self.get_account_id.unwrap_or(false);
        let get_key_hash = self.get_key_hash.unwrap_or(false);
        let get_key_prefix = self.get_key_prefix.unwrap_or(false);
        let get_name = self.get_name.unwrap_or(false);
        let get_status = self.get_status.unwrap_or(false);
        let get_permissions = self.get_permissions.unwrap_or(false);
        let get_last_used_at = self.get_last_used_at.unwrap_or(false);
        let get_expires_at = self.get_expires_at.unwrap_or(false);
        let get_created_at = self.get_created_at.unwrap_or(false);
        let get_revoked_at = self.get_revoked_at.unwrap_or(false);

        let build_update = || {
            let mut update = FluentUpdate::table(ApiKeys::Table)
                .value(ApiKeys::Name, self.name.clone())
                .value(ApiKeys::Status, self.status.clone())

                .value(ApiKeys::Permissions, self.permissions.clone())
                .value(ApiKeys::LastUsedAt, self.last_used_at)
                .value(ApiKeys::ExpiresAt, self.expires_at)
                .value(ApiKeys::RevokedAt, self.revoked_at)
                .filter(ApiKeys::Id, id);

            if get_id {
                update = update.returning(ApiKeys::Id);
            }
            if get_account_id {
                update = update.returning(ApiKeys::AccountId);
            }
            if get_key_hash {
                update = update.returning(ApiKeys::KeyHash);
            }
            if get_key_prefix {
                update = update.returning(ApiKeys::KeyPrefix);
            }
            if get_name {
                update = update.returning(ApiKeys::Name);
            }
            if get_status {
                update = update.returning(ApiKeys::Status);
            }

            if get_permissions {
                update = update.returning(ApiKeys::Permissions);
            }
            if get_last_used_at {
                update = update.returning(ApiKeys::LastUsedAt);
            }
            if get_expires_at {
                update = update.returning(ApiKeys::ExpiresAt);
            }
            if get_created_at {
                update = update.returning(ApiKeys::CreatedAt);
            }
            if get_revoked_at {
                update = update.returning(ApiKeys::RevokedAt);
            }

            update.render()
        };

        let (sql, values) = build_update();
        let query = sqlx::query::<Postgres>(&sql);
        let query = bind_query(query, values);

        let row = query
            .fetch_one(&mut **db_conn)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;
        let result = ApiKey::from_row(&row).map_err(|e| ServiceError::DatabaseError(e.to_string()));

        result
    }

    /// Read an API key from the database based on set fields (filters)
    pub async fn read(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<ApiKey, ServiceError> {
        let get_id = self.get_id.unwrap_or(true);
        let get_account_id = self.get_account_id.unwrap_or(false);
        let get_key_hash = self.get_key_hash.unwrap_or(false);
        let get_key_prefix = self.get_key_prefix.unwrap_or(false);
        let get_name = self.get_name.unwrap_or(false);
        let get_status = self.get_status.unwrap_or(false);
        let get_permissions = self.get_permissions.unwrap_or(false);
        let get_last_used_at = self.get_last_used_at.unwrap_or(false);
        let get_expires_at = self.get_expires_at.unwrap_or(false);
        let get_created_at = self.get_created_at.unwrap_or(true);
        let get_revoked_at = self.get_revoked_at.unwrap_or(false);

        let build_select = || {
            let mut select = FluentSelect::from(ApiKeys::Table);

            // Add columns
            if get_id {
                select = select.column(ApiKeys::Id);
            }
            if get_account_id {
                select = select.column(ApiKeys::AccountId);
            }
            if get_key_hash {
                select = select.column(ApiKeys::KeyHash);
            }
            if get_key_prefix {
                select = select.column(ApiKeys::KeyPrefix);
            }
            if get_name {
                select = select.column(ApiKeys::Name);
            }
            if get_status {
                select = select.column(ApiKeys::Status);
            }
            if get_permissions {
                select = select.column(ApiKeys::Permissions);
            }
            if get_last_used_at {
                select = select.column(ApiKeys::LastUsedAt);
            }
            if get_expires_at {
                select = select.column(ApiKeys::ExpiresAt);
            }
            if get_created_at {
                select = select.column(ApiKeys::CreatedAt);
            }
            if get_revoked_at {
                select = select.column(ApiKeys::RevokedAt);
            }

            // Add filters
            if let Some(id) = self.id {
                select = select.filter(ApiKeys::Id, id);
            }
            if let Some(account_id) = self.account_id {
                select = select.filter(ApiKeys::AccountId, account_id);
            }
            if let Some(key_hash) = self.key_hash.as_ref() {
                select = select.filter(ApiKeys::KeyHash, key_hash.clone());
            }
            if let Some(key_prefix) = self.key_prefix.as_ref() {
                select = select.filter(ApiKeys::KeyPrefix, key_prefix.clone());
            }
            if let Some(status) = self.status.as_ref() {
                select = select.filter(ApiKeys::Status, status.clone());
            }

            select.render()
        };

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
            Some(c) => &mut **c,
            None => {
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

        let (sql, values) = build_select();
        let query = sqlx::query::<Postgres>(&sql);
        let query = bind_query(query, values);

        let row = query.fetch_one(db_conn).await.map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                ServiceError::ValidationError("API key not found".to_string())
            }
            _ => ServiceError::DatabaseError(e.to_string()),
        })?;

        let result = ApiKey::from_row(&row).map_err(|e| ServiceError::DatabaseError(e.to_string()));

        result
    }

    /// Revoke an API key (soft delete)
    pub async fn revoke(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<ApiKey, ServiceError> {
        let id = self
            .id
            .ok_or_else(|| ServiceError::ValidationError("Missing ID for revoke".to_string()))?;

        ApiKeyBuilder::new()
            .id(id)
            .status("revoked".to_string())
            .revoked_at(Utc::now())
            .expect_id()
            .expect_status()
            .expect_revoked_at()
            .update(conn)
            .await
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
    bind_query,
    sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>
);
