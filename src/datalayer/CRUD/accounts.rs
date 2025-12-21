use super::types::Account;
use crate::datalayer::CRUD::helper;
use crate::datalayer::CRUD::money::money;
use crate::datalayer::CRUD::sql_generator::sql_generator::{
    FluentInsert, FluentSelect, FluentUpdate,
};
use crate::datalayer::CRUD::types::Accounts;
use crate::datalayer::db_ops::constants::{DEFAULT_CURRENCY, POOL_STATE_TRACKER};
use crate::errors::errors::ServiceError;
use sea_query::{Cond, Expr, PostgresQueryBuilder, Query, Value};
use sqlx::FromRow;
use sqlx::Postgres;
use sqlx::pool::PoolConnection;
use uuid::Uuid;

/// Builder for creating new accounts
///
/// ## Money Representation
///
/// Account balances are stored as `i64` integers with 4 decimal places of precision
/// using the DENOMINATOR constant (10000).
///
/// - Storage unit = dollars * DENOMINATOR  
/// - Example: $10.50 = 105000 storage units
/// - See `crate::datalayer::CRUD::money` for conversion utilities
#[derive(Debug)]
pub struct AccountBuilder {
    business_name: Option<String>,
    email: Option<String>,
    currency: Option<String>,
    balance: Option<f64>,
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
    get_created_at: Option<bool>,
    get_updated_at: Option<bool>,
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
            get_created_at: None,
            get_updated_at: None,
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

    pub fn balance(mut self, balance: f64) -> Self {
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

    pub fn expect_created_at(mut self) -> Self {
        self.get_created_at = Some(true);
        self
    }

    pub fn expect_updated_at(mut self) -> Self {
        self.get_updated_at = Some(true);
        self
    }

    /// Create a new account in the database
    /// Create is not associated with any money operations so no need to convert balance to storage units
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
            return Err(ServiceError::InsufficientPermissions(
                "Balance must be 0".to_string(),
            ));
        }

        // Capture flags for returning
        let get_id = self.get_id.unwrap_or(true);
        let get_business_name = self.get_business_name.unwrap_or(false);
        let get_email = self.get_email.unwrap_or(false);
        let get_currency = self.get_currency.unwrap_or(false);
        let get_balance = self.get_balance.unwrap_or(false);
        let get_status = self.get_status.unwrap_or(false);
        let get_metadata = self.get_metadata.unwrap_or(false);
        // Default timestamps to true because Account struct needs them?
        // Or should we mandate explicit expect?
        // If we want create() -> Result<Account>, we usually need all fields.
        // Let's default them to true if not specified, similar to ID? No, ID defaults true via unwrap_or(true).
        // Account struct requires them. So we should probably default true for create/update/read
        // if the target is Account struct.
        // But for flexible query we might not want them.
        // However, since the function returns Result<Account>, we MUST have them.
        // So for `create`, we should default them to true.
        let get_created_at = self.get_created_at.unwrap_or(true);
        let get_updated_at = self.get_updated_at.unwrap_or(true);

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
            if get_created_at {
                insert = insert.returning(Accounts::CreatedAt);
            }
            if get_updated_at {
                insert = insert.returning(Accounts::UpdatedAt);
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
            Some(c) => {
                // Connection provided by caller - don't manage it with guard
                &mut **c
            }
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

        if Self::check_exists(&business_name, &email, db_conn)
            .await
            .unwrap_or(false)
        {
            return Err(ServiceError::AccountAlreadyExists(format!(
                "email:{} or business name:{} already exists",
                email, business_name
            )));
        }

        let (sql, values) = build_insert();
        let query = sqlx::query::<Postgres>(&sql);
        let query = bind_query(query, values);

        let row = query.fetch_one(db_conn).await.map_err(handle_error)?;
        let result =
            Account::from_row(&row).map_err(|e| ServiceError::DatabaseError(e.to_string()));

        result
    }

    /// Update an existing account in the database
    /// update is associated with the money operations so balance is in storage units, need to convert this to dollars
    pub async fn update(
        self,
        conn: Option<&mut PoolConnection<Postgres>>,
    ) -> Result<Account, ServiceError> {
        // Sort out the connection issues here rather than bottom
        // This should be bottom of the function actually so save connection locking time so that when we see lot of request over period of time
        // - we save locking time significant.
        // But for the quick refactor we made this in top for this funciton.
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
            Some(c) => {
                // Connection provided by caller - don't manage it with guard
                c
            }
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

        let id = self
            .id
            .ok_or_else(|| ServiceError::ValidationError("Missing ID for update".to_string()))?;

        let get_id = self.get_id.unwrap_or(true);
        let get_business_name = self.get_business_name.unwrap_or(false);
        let get_email = self.get_email.unwrap_or(false);
        let get_currency = self.get_currency.unwrap_or(false);
        let get_balance = self.get_balance.unwrap_or(false);
        let get_status = self.get_status.unwrap_or(false);
        let get_metadata = self.get_metadata.unwrap_or(false);
        let get_created_at = self.get_created_at.unwrap_or(false);
        let get_updated_at = self.get_updated_at.unwrap_or(false);

        /*
         If balance is provided, check for the currency - first convert it to USD
         - Then convert to storage units
        */
        let mut usd_balance: i64 = 0;
        let mut currency = self.currency;
        if self.balance.is_some() {
            /*
            If currency is not provided, then try to read the currency from the database
            - if that too not provided then use default USD
            */
            if currency.is_none() {
                let account_res = AccountBuilder::new()
                    .id(id)
                    .expect_currency()
                    .read(Some(&mut *db_conn))
                    .await;

                if let Ok(account) = account_res {
                    currency = Some(account.currency);
                } else {
                    currency = Some(DEFAULT_CURRENCY.to_string());
                }
            }

            usd_balance = money::to_storage_units_with_conversion(
                self.balance.unwrap(),
                currency.clone().unwrap(),
            )
        }

        let build_update = || {
            let mut update = FluentUpdate::table(Accounts::Table)
                .value(Accounts::UpdatedAt, chrono::Utc::now())
                .filter(Accounts::Id, id);

            // Only update fields that are provided
            if let Some(business_name) = self.business_name.clone() {
                update = update.value(Accounts::BusinessName, business_name);
            }
            if let Some(email) = self.email.clone() {
                update = update.value(Accounts::Email, email);
            }
            if let Some(status) = self.status.clone() {
                update = update.value(Accounts::Status, status);
            }
            if let Some(metadata) = self.metadata.clone() {
                update = update.value(Accounts::Metadata, metadata);
            }

            // Only update balance and currency if balance is provided
            if self.balance.is_some() {
                update = update.value(Accounts::Balance, usd_balance);
                if let Some(curr) = currency.clone() {
                    update = update.value(Accounts::Currency, curr);
                }
            }

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
            if get_created_at {
                update = update.returning(Accounts::CreatedAt);
            }
            if get_updated_at {
                update = update.returning(Accounts::UpdatedAt);
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
        let result =
            Account::from_row(&row).map_err(|e| ServiceError::DatabaseError(e.to_string()));

        result
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
        let get_created_at = self.get_created_at.unwrap_or(true);
        let get_updated_at = self.get_updated_at.unwrap_or(true);

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
            if get_created_at {
                select = select.column(Accounts::CreatedAt);
            }
            if get_updated_at {
                select = select.column(Accounts::UpdatedAt);
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
            Some(c) => {
                // Connection provided by caller - don't manage it with guard
                &mut **c
            }
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

        let (sql, values) = build_select();
        let query = sqlx::query::<Postgres>(&sql);
        let query = bind_query(query, values);

        let row = query.fetch_one(db_conn).await.map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                ServiceError::AccountNotFound("Filtered criteria".to_string())
            }
            _ => ServiceError::DatabaseError(e.to_string()),
        })?;

        let result =
            Account::from_row(&row).map_err(|e| ServiceError::DatabaseError(e.to_string()));

        result
    }

    /// Check if an account exists with the given business name OR email
    pub async fn check_exists(
        business_name: &str,
        email: &str,
        conn: &mut sqlx::PgConnection,
    ) -> Result<bool, ServiceError> {
        let (sql, values) = Query::select()
            .column(Accounts::Id)
            .from(Accounts::Table)
            .cond_where(
                Cond::any()
                    .add(Expr::col(Accounts::BusinessName).eq(business_name))
                    .add(Expr::col(Accounts::Email).eq(email)),
            )
            .build(PostgresQueryBuilder);

        let query = sqlx::query_scalar::<_, Uuid>(&sql);
        let query = bind_query_scalar(query, values);

        let result = query.fetch_optional(conn).await;

        match result {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(ServiceError::DatabaseError(e.to_string())),
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

impl_bind_values!(
    bind_query_scalar,
    sqlx::query::QueryScalar<'a, Postgres, Uuid, sqlx::postgres::PgArguments>
);

impl_bind_values!(
    bind_query,
    sqlx::query::Query<'a, Postgres, sqlx::postgres::PgArguments>
);
