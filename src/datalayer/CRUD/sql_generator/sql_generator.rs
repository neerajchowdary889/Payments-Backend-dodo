use sea_query::{Alias, Expr, Iden, Order, PostgresQueryBuilder, Query, Value};

/* ----------------------------- FLUENT BUILDER WRAPPER ----------------------------- */

// Helper to check if a Value is None (NULL)
fn is_value_none(val: &Value) -> bool {
    matches!(
        val,
        Value::Bool(None)
            | Value::TinyInt(None)
            | Value::SmallInt(None)
            | Value::Int(None)
            | Value::BigInt(None)
            | Value::TinyUnsigned(None)
            | Value::SmallUnsigned(None)
            | Value::Unsigned(None)
            | Value::BigUnsigned(None)
            | Value::Float(None)
            | Value::Double(None)
            | Value::String(None)
            | Value::Char(None)
            | Value::Bytes(None)
            | Value::Json(None)
            | Value::ChronoDate(None)
            | Value::ChronoTime(None)
            | Value::ChronoDateTime(None)
            | Value::ChronoDateTimeUtc(None)
            | Value::ChronoDateTimeLocal(None)
            | Value::ChronoDateTimeWithTimeZone(None)
            | Value::Uuid(None)
    )
}

// --- INSERT ---

pub struct FluentInsert {
    table: Option<Alias>,
    values: Vec<(Alias, Value)>,
    returning: Vec<Alias>,
}

impl FluentInsert {
    pub fn into<T: Iden>(table: T) -> Self {
        Self {
            table: Some(Alias::new(table.to_string())),
            values: vec![],
            returning: vec![],
        }
    }

    pub fn value<C: Iden, V: Into<Value>>(mut self, col: C, v: V) -> Self {
        let val: Value = v.into();
        if !is_value_none(&val) {
            self.values.push((Alias::new(col.to_string()), val));
        }
        self
    }

    pub fn returning<C: Iden>(mut self, col: C) -> Self {
        self.returning.push(Alias::new(col.to_string()));
        self
    }

    pub fn render(self) -> (String, sea_query::Values) {
        let mut query = Query::insert();

        if let Some(table) = self.table {
            query.into_table(table);
        }

        // Unzip columns and values
        let (cols, vals): (Vec<Alias>, Vec<Value>) = self.values.into_iter().unzip();

        query.columns(cols);
        query.values_panic(vals.into_iter().map(|v| sea_query::SimpleExpr::Value(v)));

        if !self.returning.is_empty() {
            query.returning(Query::returning().columns(self.returning));
        }

        query.build(PostgresQueryBuilder)
    }
}

// --- SELECT ---

pub struct FluentSelect {
    table: Option<Alias>,
    columns: Vec<Alias>,
    filters: Vec<(Alias, Value)>,
    conditions: Vec<sea_query::SimpleExpr>, // Generic WHERE conditions
    joins: Vec<(Alias, sea_query::SimpleExpr)>, // (Table, ON Condition)
    limit: Option<u64>,
    offset: Option<u64>,
    order_by: Option<(Alias, Order)>,
}

impl FluentSelect {
    pub fn from<T: Iden>(table: T) -> Self {
        Self {
            table: Some(Alias::new(table.to_string())),
            columns: vec![],
            filters: vec![],
            conditions: vec![],
            joins: vec![],
            limit: None,
            offset: None,
            order_by: None,
        }
    }

    pub fn column<C: Iden>(mut self, col: C) -> Self {
        self.columns.push(Alias::new(col.to_string()));
        self
    }

    pub fn filter<C: Iden, V: Into<Value>>(mut self, col: C, v: V) -> Self {
        let val: Value = v.into();
        if !is_value_none(&val) {
            self.filters.push((Alias::new(col.to_string()), val));
        }
        self
    }

    pub fn and_where<E: Into<sea_query::SimpleExpr>>(mut self, cond: E) -> Self {
        self.conditions.push(cond.into());
        self
    }

    pub fn join<T: Iden, E: Into<sea_query::SimpleExpr>>(mut self, table: T, condition: E) -> Self {
        self.joins
            .push((Alias::new(table.to_string()), condition.into()));
        self
    }

    pub fn limit(mut self, n: u64) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn order_by<C: Iden>(mut self, col: C, order: Order) -> Self {
        self.order_by = Some((Alias::new(col.to_string()), order));
        self
    }

    pub fn render(self) -> (String, sea_query::Values) {
        let mut query = Query::select();

        if let Some(table) = self.table {
            query.from(table);
        }

        if !self.columns.is_empty() {
            query.columns(self.columns);
        } else {
            query.column(Alias::new("*"));
        }

        for (table, condition) in self.joins {
            query.left_join(table, condition);
        }

        for (col, val) in self.filters {
            query.and_where(Expr::col(col).eq(val));
        }

        for cond in self.conditions {
            query.and_where(cond);
        }

        if let Some(l) = self.limit {
            query.limit(l);
        }

        if let Some(o) = self.offset {
            query.offset(o);
        }

        if let Some((col, order)) = self.order_by {
            query.order_by(col, order);
        }

        query.build(PostgresQueryBuilder)
    }
}
// --- UPDATE ---

pub struct FluentUpdate {
    table: Option<Alias>,
    values: Vec<(Alias, Value)>,
    filters: Vec<(Alias, Value)>,
    returning: Vec<Alias>,
}

impl FluentUpdate {
    pub fn table<T: Iden>(table: T) -> Self {
        Self {
            table: Some(Alias::new(table.to_string())),
            values: vec![],
            filters: vec![],
            returning: vec![],
        }
    }

    pub fn value<C: Iden, V: Into<Value>>(mut self, col: C, v: V) -> Self {
        // For UPDATE, we also skip setting a field if value is None
        let val: Value = v.into();
        if !is_value_none(&val) {
            self.values.push((Alias::new(col.to_string()), val));
        }
        self
    }

    pub fn filter<C: Iden, V: Into<Value>>(mut self, col: C, v: V) -> Self {
        let val: Value = v.into();
        if !is_value_none(&val) {
            self.filters.push((Alias::new(col.to_string()), val));
        }
        self
    }

    pub fn returning<C: Iden>(mut self, col: C) -> Self {
        self.returning.push(Alias::new(col.to_string()));
        self
    }

    pub fn render(self) -> (String, sea_query::Values) {
        let mut query = Query::update();

        if let Some(table) = self.table {
            query.table(table);
        }

        // values
        let (cols, vals): (Vec<Alias>, Vec<Value>) = self.values.into_iter().unzip();
        query.values(
            cols.into_iter()
                .zip(vals.into_iter().map(|v| sea_query::SimpleExpr::Value(v))),
        );

        // filters
        for (col, val) in self.filters {
            query.and_where(Expr::col(col).eq(val));
        }

        if !self.returning.is_empty() {
            query.returning(Query::returning().columns(self.returning));
        }

        query.build(PostgresQueryBuilder)
    }
}

// --- DELETE ---

pub struct FluentDelete {
    table: Option<Alias>,
    filters: Vec<(Alias, Value)>,
    returning: Vec<Alias>,
}

impl FluentDelete {
    pub fn from<T: Iden>(table: T) -> Self {
        Self {
            table: Some(Alias::new(table.to_string())),
            filters: vec![],
            returning: vec![],
        }
    }

    pub fn filter<C: Iden, V: Into<Value>>(mut self, col: C, v: V) -> Self {
        let val: Value = v.into();
        if !is_value_none(&val) {
            self.filters.push((Alias::new(col.to_string()), val));
        }
        self
    }

    pub fn returning<C: Iden>(mut self, col: C) -> Self {
        self.returning.push(Alias::new(col.to_string()));
        self
    }

    pub fn render(self) -> (String, sea_query::Values) {
        let mut query = Query::delete();

        if let Some(table) = self.table {
            query.from_table(table);
        }

        for (col, val) in self.filters {
            query.and_where(Expr::col(col).eq(val));
        }

        if !self.returning.is_empty() {
            query.returning(Query::returning().columns(self.returning));
        }

        query.build(PostgresQueryBuilder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fluent_insert_wrapper() {
        use crate::datalayer::CRUD::types::Accounts;
        let now = chrono::Utc::now();
        let (sql, values) = FluentInsert::into(Accounts::Table)
            .value(Accounts::BusinessName, "Fluent Corp")
            .value(Accounts::Email, "fluent@example.com")
            .value(Accounts::Balance, 500)
            .value(Accounts::Currency, "USD")
            .value(Accounts::CreatedAt, now)
            .value(Accounts::UpdatedAt, now)
            .returning(Accounts::Id)
            .render();

        println!("Fluent SQL: {}", sql);
        println!("Fluent Values: {:?}", values);

        assert!(sql.starts_with("INSERT INTO \"accounts\""));
        assert!(sql.contains("\"business_name\", \"email\", \"balance\", \"currency\""));
        assert!(sql.contains("RETURNING \"id\""));
    }

    #[test]
    fn test_fluent_insert_with_none() {
        use crate::datalayer::CRUD::types::Accounts;
        let (sql, values) = FluentInsert::into(Accounts::Table)
            .value(Accounts::BusinessName, "Robust Corp")
            .value(Accounts::Email, Option::<String>::None) // Should be skipped
            .returning(Accounts::Id)
            .render();

        println!("Robust Insert SQL: {}", sql);
        println!("Robust Insert Values: {:?}", values);
        assert!(!sql.contains("\"email\""));
        assert!(sql.contains("\"business_name\""));
    }

    #[test]
    fn test_fluent_select_wrapper() {
        use crate::datalayer::CRUD::types::Accounts;
        let (sql, values) = FluentSelect::from(Accounts::Table)
            .column(Accounts::Id)
            .column(Accounts::Email)
            .filter(Accounts::BusinessName, "Fluent Corp")
            .limit(10)
            .order_by(Accounts::CreatedAt, Order::Desc)
            .render();

        println!("Select SQL: {}", sql);
        println!("Select Values: {:?}", values);

        assert!(sql.starts_with("SELECT \"id\", \"email\" FROM \"accounts\""));
        assert!(sql.contains("WHERE \"business_name\" = $1"));
        // LIMIT is parameterized as $2
        assert!(sql.contains("ORDER BY \"created_at\" DESC LIMIT $2"));
    }

    #[test]
    fn test_fluent_select_join() {
        use crate::datalayer::CRUD::types::{Accounts, Transactions};
        // SELECT * FROM transactions LEFT JOIN accounts ON transactions.from_account_id = accounts.id
        // We must use Expr::col((Table, Column)) to get "table"."column" output
        let (sql, _) = FluentSelect::from(Transactions::Table)
            .join(
                Accounts::Table,
                Expr::col((Transactions::Table, Transactions::FromAccountId))
                    .equals((Accounts::Table, Accounts::Id)),
            )
            .filter(Accounts::BusinessName, "Sender Corp")
            .render();

        println!("Join SQL: {}", sql);
        // SeaQuery 0.30 should render fully qualified identifiers if tuple provided
        assert!(sql.contains(
            "LEFT JOIN \"accounts\" ON \"transactions\".\"from_account_id\" = \"accounts\".\"id\""
        ));
    }

    #[test]
    fn test_complex_query_builder() {
        use crate::datalayer::CRUD::types::{Accounts, Transactions};
        // Complex Query:
        // SELECT * FROM transactions
        // LEFT JOIN accounts ON transactions.from_account_id = accounts.id
        // WHERE business_name = 'Sender Corp'
        // AND amount > 500
        // ORDER BY created_at DESC
        // LIMIT 10

        let (sql, _) = FluentSelect::from(Transactions::Table)
            .join(
                Accounts::Table,
                Expr::col((Transactions::Table, Transactions::FromAccountId))
                    .equals((Accounts::Table, Accounts::Id)),
            )
            .filter(Accounts::BusinessName, "Sender Corp")
            .and_where(Expr::col(Transactions::Amount).gt(500))
            .order_by(Transactions::CreatedAt, Order::Desc)
            .limit(10)
            .render();

        println!("Complex SQL: {}", sql);

        assert!(sql.contains(
            "LEFT JOIN \"accounts\" ON \"transactions\".\"from_account_id\" = \"accounts\".\"id\""
        ));
        assert!(sql.contains("WHERE \"business_name\" = $1"));
        // Amount > 500 should be parameterized as $2
        assert!(sql.contains("AND \"amount\" > $2"));
        assert!(sql.contains("ORDER BY \"created_at\" DESC LIMIT $3"));
    }

    #[test]
    fn test_fluent_update_wrapper() {
        use crate::datalayer::CRUD::types::Accounts;
        let (sql, values) = FluentUpdate::table(Accounts::Table)
            .value(Accounts::Balance, 1000)
            .filter(Accounts::BusinessName, "Fluent Corp")
            .returning(Accounts::UpdatedAt)
            .render();

        println!("Update SQL: {}", sql);
        println!("Update Values: {:?}", values);

        assert!(sql.starts_with("UPDATE \"accounts\" SET \"balance\" = $1"));
        assert!(sql.contains("WHERE \"business_name\" = $2"));
        assert!(sql.contains("RETURNING \"updated_at\""));
    }

    #[test]
    fn test_fluent_delete_wrapper() {
        use crate::datalayer::CRUD::types::Transactions;
        let (sql, values) = FluentDelete::from(Transactions::Table)
            .filter(Transactions::IdempotencyKey, "test-idempotency-key")
            .filter(Transactions::ParentTxKey, "test-parent-key")
            .filter(Transactions::CreatedAt, Option::<String>::None)
            .returning(Transactions::Id)
            .render();

        println!("Delete SQL: {}", sql);
        println!("Delete Values: {:?}", values);

        assert!(sql.starts_with("DELETE FROM \"transactions\""));
        assert!(sql.contains("WHERE \"idempotency_key\" = $1"));
        assert!(sql.contains("RETURNING \"id\""));
    }
}
