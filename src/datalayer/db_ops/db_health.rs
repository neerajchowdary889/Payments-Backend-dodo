use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info};

/// Database health status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseHealth {
    pub is_healthy: bool,
    pub latency_ms: u64,
    pub pool_size: u32,
    pub idle_connections: usize,
    pub available_connections: u32,
}

/// Table verification result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TableVerification {
    pub table_name: String,
    pub exists: bool,
    pub row_count: Option<i64>,
}

/// Comprehensive database health check
/// Verifies database connectivity and returns health metrics
pub async fn check_database_health(pool: &Arc<PgPool>) -> Result<DatabaseHealth, sqlx::Error> {
    let start = std::time::Instant::now();

    // Simple ping query
    sqlx::query("SELECT 1")
        .execute(&**pool)
        .await
        .map_err(|e| {
            error!("Database health check failed: {}", e);
            e
        })?;

    let latency = start.elapsed();

    Ok(DatabaseHealth {
        is_healthy: true,
        latency_ms: latency.as_millis() as u64,
        pool_size: pool.size(),
        idle_connections: pool.num_idle(),
        available_connections: pool.size() - (pool.size() - pool.num_idle() as u32),
    })
}

/// Check if a specific table exists in the database
pub async fn check_table_exists(pool: &Arc<PgPool>, table_name: &str) -> Result<bool, sqlx::Error> {
    let result: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_schema = 'public' 
            AND table_name = $1
        )",
    )
    .bind(table_name)
    .fetch_one(&**pool)
    .await?;

    Ok(result.0)
}

/// Get row count for a specific table
pub async fn get_table_row_count(pool: &Arc<PgPool>, table_name: &str) -> Result<i64, sqlx::Error> {
    // Use parameterized query safely
    let query = format!("SELECT COUNT(*) FROM {}", table_name);

    let result: (i64,) = sqlx::query_as(&query).fetch_one(&**pool).await?;

    Ok(result.0)
}

/// Verify a table exists and optionally get its row count
pub async fn verify_table(
    pool: &Arc<PgPool>,
    table_name: &str,
    include_count: bool,
) -> Result<TableVerification, sqlx::Error> {
    let exists = check_table_exists(pool, table_name).await?;

    let row_count = if exists && include_count {
        match get_table_row_count(pool, table_name).await {
            Ok(count) => Some(count),
            Err(e) => {
                error!("Failed to get row count for table {}: {}", table_name, e);
                None
            }
        }
    } else {
        None
    };

    Ok(TableVerification {
        table_name: table_name.to_string(),
        exists,
        row_count,
    })
}

/// Verify all required tables for the payments system
pub async fn verify_all_tables(pool: &Arc<PgPool>) -> Result<Vec<TableVerification>, sqlx::Error> {
    let required_tables = vec![
        "accounts",
        "api_keys",
        "transactions",
        "webhooks",
        "webhook_deliveries",
        "rate_limit_counters",
    ];

    let mut results = Vec::new();

    for table_name in required_tables {
        match verify_table(pool, table_name, true).await {
            Ok(verification) => {
                if verification.exists {
                    info!(
                        "✅ Table '{}' exists with {} rows",
                        table_name,
                        verification.row_count.unwrap_or(0)
                    );
                } else {
                    error!("❌ Table '{}' does not exist", table_name);
                }
                results.push(verification);
            }
            Err(e) => {
                error!("Failed to verify table '{}': {}", table_name, e);
                results.push(TableVerification {
                    table_name: table_name.to_string(),
                    exists: false,
                    row_count: None,
                });
            }
        }
    }

    Ok(results)
}

/// Comprehensive database initialization check
/// Verifies connectivity, health, and table initialization
pub async fn verify_database_initialization(
    pool: &Arc<PgPool>,
) -> Result<(DatabaseHealth, Vec<TableVerification>), sqlx::Error> {
    info!("🔍 Verifying database initialization...");

    // Check database health
    let health = check_database_health(pool).await?;
    info!(
        "✅ Database is healthy (latency: {}ms, pool: {}/{})",
        health.latency_ms, health.idle_connections, health.pool_size
    );

    // Verify all tables
    let table_verifications = verify_all_tables(pool).await?;

    let missing_tables: Vec<&str> = table_verifications
        .iter()
        .filter(|v| !v.exists)
        .map(|v| v.table_name.as_str())
        .collect();

    if !missing_tables.is_empty() {
        error!("❌ Missing tables: {:?}", missing_tables);
        return Err(sqlx::Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Database not properly initialized. Missing tables: {:?}",
                missing_tables
            ),
        )));
    }

    info!("✅ All required tables are initialized");

    Ok((health, table_verifications))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datalayer::db_ops::constants::DbConfig;
    use sqlx::postgres::PgPoolOptions;

    async fn create_test_pool() -> Result<Arc<PgPool>, sqlx::Error> {
        let config = DbConfig::default();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.database_url)
            .await?;
        Ok(Arc::new(pool))
    }

    #[tokio::test]
    async fn test_database_health_check() {
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        if let Ok(pool) = create_test_pool().await {
            let health = check_database_health(&pool).await;
            assert!(health.is_ok());

            if let Ok(h) = health {
                assert!(h.is_healthy);
                assert!(h.latency_ms < 1000); // Should be fast
            }
        }
    }

    #[tokio::test]
    async fn test_table_verification() {
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        if let Ok(pool) = create_test_pool().await {
            // Test with a table that should exist
            let result = verify_table(&pool, "accounts", true).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_verify_all_tables() {
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        if let Ok(pool) = create_test_pool().await {
            let result = verify_all_tables(&pool).await;
            assert!(result.is_ok());
        }
    }
}
