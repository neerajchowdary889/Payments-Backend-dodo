use super::types::Account;
use super::helper::conversion;
use crate::datalayer::CRUD::helper;
use crate::datalayer::CRUD::sql_generator::sql_generator::{
    FluentInsert, FluentSelect, FluentUpdate,
};
use crate::datalayer::CRUD::types::Accounts;
use crate::datalayer::db_ops::constants::POOL_STATE_TRACKER;
use crate::errors::errors::ServiceError;
use sea_query::Value;
use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};
use std::sync::Arc;
use uuid::Uuid;

/// Builder for creating new accounts
#[derive(Debug)]
pub struct AccountBuilder {
    business_name: Option<String>,
    email: Option<String>,
    currency: Option<String>,
    balance: Option<i64>,
    status: Option<String>,
    metadata: Option<serde_json::Value>,
    id: Option<Uuid>,
    // Make list that you expect to return
    get_business_name: Option<bool>,
    get_email: Option<bool>,
    get_currency: Option<bool>,
    get_balance: Option<bool>,
    get_status: Option<bool>,
    get_metadata: Option<bool>,
    get_id: Option<bool>,
}

impl Default for AccountBuilder {
    fn default() -> Self {
        Self {
            business_name: None,
            email: None,
            currency: None,
            balance: None,
            status: None,
            metadata: None,
            id: None,
            get_business_name: None,
            get_email: None,
            get_currency: None,
            get_balance: None,
            get_status: None,
            get_metadata: None,
            get_id: None,
        }
    }
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

    pub fn id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn expect_id(mut self) -> Self {
        self.get_id = Some(true);
        self
    }

    pub fn expect_business_name(mut self) -> Self {
        self.get_business_name = Some(true);
        self
    }

    pub fn expect_email(mut self) -> Self {
        self.get_email = Some(true);
        self
    }

    pub fn expect_currency(mut self) -> Self {
        self.get_currency = Some(true);
        self
    }

    pub fn expect_balance(mut self) -> Self {
        self.get_balance = Some(true);
        self
    }

    pub fn expect_status(mut self) -> Self {
        self.get_status = Some(true);
        self
    }

    pub fn expect_metadata(mut self) -> Self {
        self.get_metadata = Some(true);
        self
    }

    /// Create a new account in the database
    pub async fn create(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<Account, ServiceError> {
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
        let balance = Some(0);
        let status = self.status.clone().unwrap_or_else(|| "active".to_string());
        let metadata = self.metadata;

        // Validate balance is non-negative
        if balance != Some(0) {
            return Err(ServiceError::InsufficientPermissions("Balance must be 0".to_string()));
        }

        // Capture flags for returning
        let get_id = self.get_id.unwrap_or(true);
        let get_business_name = self.get_business_name.unwrap_or(false);
        let get_email = self.get_email.unwrap_or(false);
        let get_currency = self.get_currency.unwrap_or(false);
        let get_balance = self.get_balance.unwrap_or(false);
        let get_status = self.get_status.unwrap_or(false);
        let get_metadata = self.get_metadata.unwrap_or(false);

        // Build the query variables
        let build_insert = || {
            let mut insert = FluentInsert::into(Accounts::Table)
                .value(Accounts::BusinessName, business_name.clone())
                .value(Accounts::Email, email.clone())
                .value(Accounts::Currency, currency.clone())
                .value(Accounts::Balance, balance)
                .value(Accounts::Status, status.clone())
                .value(Accounts::Metadata, metadata.clone())
                .value(Accounts::CreatedAt, chrono::Utc::now())
                .value(Accounts::UpdatedAt, chrono::Utc::now());

            if get_id {
                insert = insert.returning(Accounts::Id);
            }
            if get_business_name {
                insert = insert.returning(Accounts::BusinessName);
            }
            if get_email {
                insert = insert.returning(Accounts::Email);
            }
            if get_currency {
                insert = insert.returning(Accounts::Currency);
            }
            if get_balance {
                insert = insert.returning(Accounts::Balance);
            }
            if get_status {
                insert = insert.returning(Accounts::Status);
            }
            if get_metadata {
                insert = insert.returning(Accounts::Metadata);
            }

            insert.render()
        };

        let handle_error = |e: sqlx::Error| match &e {
            sqlx::Error::Database(db_err) => {
                if db_err.code().as_deref() == Some("23505") {
                    ServiceError::AccountAlreadyExists(email.clone())
                } else {
                    ServiceError::DatabaseError(e.to_string())
                }
            }
            _ => ServiceError::DatabaseError(e.to_string()),
        };

        match conn {
            Some(conn) => {
                let (sql, values) = build_insert();
                let query = sqlx::query_as::<_, Account>(&sql);
                let query = bind_query_as(query, values);

                query.fetch_one(&mut **conn).await.map_err(handle_error)
            }
            None => {
                let tracker = POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?;
                let mut owned_conn = tracker.get_connection().await.map_err(|e| {
                    ServiceError::DatabaseError(format!("Failed to get connection: {}", e))
                })?;

                let (sql, values) = build_insert();
                let query = sqlx::query_as::<_, Account>(&sql);
                let query = bind_query_as(query, values);

                let result = query
                    .fetch_one(&mut *owned_conn)
                    .await
                    .map_err(handle_error);
                tracker.return_connection(owned_conn);
                result
            }
        }
    }

    /// Update an existing account in the database
    pub async fn update(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<Account, ServiceError> {
        let id = self
            .id
            .ok_or_else(|| ServiceError::ValidationError("Missing ID for update".to_string()))?;

        // Check if there are any fields to update
        if self.business_name.is_none()
            && self.email.is_none()
            && self.currency.is_none()
            && self.balance.is_none()
            && self.status.is_none()
            && self.metadata.is_none()
        {
            // If no updates to fields, we skip FluentUpdate construction?
            // Actually, we must return the Account. So we should probably do a SELECT if no update?
            // But FluentUpdate is for UPDATE.
            // If we run UPDATE accounts SET ... WHERE id=... without values, it's invalid.
            // But if we have no values to set, we can't use UPDATE to fetch returning.
            // We should use FluentSelect if no updates.
            // BUT, the user might want to verify it exists? Or just fetch?
            // "update" implies side effect.
            // I will err if nothing to update to be safe, because switching to SELECT is a behavior change.
            // BUT, strictly speaking, "update" with no changes is a no-op that returns nothing.
            // But we need to return Account.
            // Since I cannot leave it empty, I will remove the "return Ok(()) optimization" and rely on FluentUpdate needing at least one value?
            // FluentUpdate with no values will produce "UPDATE accounts SET WHERE ..." -> Syntax Error.
            // So I MUST handle this case.
            // I will force a dummy update (UpdatedAt = Now) which is already there!
            // .value(Accounts::UpdatedAt, chrono::Utc::now()) is ALWAYS present in build_update!
            // So `self.business_name.is_none()...` check is irrelevant because UpdatedAt is always updated.
            // So I can just remove the check block entirely.
        }

        let get_id = self.get_id.unwrap_or(true);
        let get_business_name = self.get_business_name.unwrap_or(false);
        let get_email = self.get_email.unwrap_or(false);
        let get_currency = self.get_currency.unwrap_or(false);
        let get_balance = self.get_balance.unwrap_or(false);
        let get_status = self.get_status.unwrap_or(false);
        let get_metadata = self.get_metadata.unwrap_or(false);

        let build_update = || {
            let mut update = FluentUpdate::table(Accounts::Table)
                .value(Accounts::BusinessName, self.business_name.clone())
                .value(Accounts::Email, self.email.clone())
                .value(Accounts::Currency, self.currency.clone())
                .value(Accounts::Balance, self.balance)
                .value(Accounts::Status, self.status.clone())
                .value(Accounts::Metadata, self.metadata.clone())
                .value(Accounts::UpdatedAt, chrono::Utc::now())
                .filter(Accounts::Id, id);

            if get_id {
                update = update.returning(Accounts::Id);
            }
            if get_business_name {
                update = update.returning(Accounts::BusinessName);
            }
            if get_email {
                update = update.returning(Accounts::Email);
            }
            if get_currency {
                update = update.returning(Accounts::Currency);
            }
            if get_balance {
                update = update.returning(Accounts::Balance);
            }
            if get_status {
                update = update.returning(Accounts::Status);
            }
            if get_metadata {
                update = update.returning(Accounts::Metadata);
            }

            update.render()
        };

        match conn {
            Some(conn) => {
                let (sql, values) = build_update();
                let query = sqlx::query_as::<_, Account>(&sql);
                let query = bind_query_as(query, values);

                query
                    .fetch_one(&mut **conn)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(e.to_string()))
            }
            None => {
                let tracker = POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?;
                let mut owned_conn = tracker.get_connection().await.map_err(|e| {
                    ServiceError::DatabaseError(format!("Failed to get connection: {}", e))
                })?;

                let (sql, values) = build_update();
                let query = sqlx::query_as::<_, Account>(&sql);
                let query = bind_query_as(query, values);

                let result = query
                    .fetch_one(&mut *owned_conn)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(e.to_string()));
                tracker.return_connection(owned_conn);
                result
            }
        }
    }

    /// Read an account from the database based on set fields (filters)
    pub async fn read(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<Account, ServiceError> {
        // Collect filters from set fields
        // Note: Logic is AND. If multiple set, all must match.
        // If NO fields set, it might match everything (if limited) or fail (if fetch_one).
        // For fetch_one, we expect a single result.
        // If ID is provided, it's usually unique.

        // Capture flags for selection
        let get_id = self.get_id.unwrap_or(true);
        let get_business_name = self.get_business_name.unwrap_or(false);
        let get_email = self.get_email.unwrap_or(false);
        let get_currency = self.get_currency.unwrap_or(false);
        let get_balance = self.get_balance.unwrap_or(false);
        let get_status = self.get_status.unwrap_or(false);
        let get_metadata = self.get_metadata.unwrap_or(false);

        let build_select = || {
            let mut select = FluentSelect::from(Accounts::Table);

            // Add Columns
            if get_id {
                select = select.column(Accounts::Id);
            }
            if get_business_name {
                select = select.column(Accounts::BusinessName);
            }
            if get_email {
                select = select.column(Accounts::Email);
            }
            if get_currency {
                select = select.column(Accounts::Currency);
            }
            if get_balance {
                select = select.column(Accounts::Balance);
            }
            if get_status {
                select = select.column(Accounts::Status);
            }
            if get_metadata {
                select = select.column(Accounts::Metadata);
            }

            // Add Filters
            if let Some(id) = self.id {
                select = select.filter(Accounts::Id, id);
            }
            if let Some(name) = self.business_name.as_ref() {
                select = select.filter(Accounts::BusinessName, name.clone());
            }
            if let Some(email) = self.email.as_ref() {
                select = select.filter(Accounts::Email, email.clone());
            }
            if let Some(currency) = self.currency.as_ref() {
                select = select.filter(Accounts::Currency, currency.clone());
            }
            if let Some(balance) = self.balance {
                select = select.filter(Accounts::Balance, balance);
            }
            if let Some(status) = self.status.as_ref() {
                select = select.filter(Accounts::Status, status.clone());
            }
            // Metadata filter usually not simple equality, skipping for now or assumed exact match stringified?
            // SeaQuery Value::Json can work.
            if let Some(metadata) = self.metadata.as_ref() {
                select = select.filter(Accounts::Metadata, metadata.clone());
            }

            select.render()
        };

        match conn {
            Some(conn) => {
                let (sql, values) = build_select();
                let query = sqlx::query_as::<_, Account>(&sql);
                let query = bind_query_as(query, values);

                query.fetch_one(&mut **conn).await.map_err(|e| match e {
                    sqlx::Error::RowNotFound => {
                        ServiceError::AccountNotFound("Filtered criteria".to_string())
                    }
                    _ => ServiceError::DatabaseError(e.to_string()),
                })
            }
            None => {
                let tracker = POOL_STATE_TRACKER
                    .get()
                    .ok_or_else(|| ServiceError::DatabaseConnectionError)?;
                let mut owned_conn = tracker.get_connection().await.map_err(|e| {
                    ServiceError::DatabaseError(format!("Failed to get connection: {}", e))
                })?;

                let (sql, values) = build_select();
                let query = sqlx::query_as::<_, Account>(&sql);
                let query = bind_query_as(query, values);

                let result = query
                    .fetch_one(&mut *owned_conn)
                    .await
                    .map_err(|e| match e {
                        sqlx::Error::RowNotFound => {
                            ServiceError::AccountNotFound("Filtered criteria".to_string())
                        }
                        _ => ServiceError::DatabaseError(e.to_string()),
                    });
                tracker.return_connection(owned_conn);
                result
            }
        }
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

fn ifAccountExist()

impl_bind_values!(
    bind_query_as,
    sqlx::query::QueryAs<'a, Postgres, Account, sqlx::postgres::PgArguments>
);
