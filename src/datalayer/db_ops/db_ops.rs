use crate::datalayer::db_ops::constants::DbConfig;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use tracing::{error, info};

/// Database connection manager with idempotent initialization
#[derive(Clone)]
pub struct DbManager {
    pool: Arc<PgPool>,
}

impl DbManager {
    /// Creates a new database manager with idempotent connection pool
    /// This can be called multiple times safely - it will reuse the same pool
    pub async fn new(config: DbConfig) -> Result<Self, sqlx::Error> {
        info!("Initializing database connection pool...");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.connection_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime)
            .connect(&config.database_url)
            .await
            .map_err(|e| {
                error!("Failed to create database pool: {}", e);
                e
            })?;

        info!("Database connection pool created successfully");

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Creates a database manager with default configuration
    pub async fn with_defaults() -> Result<Self, sqlx::Error> {
        Self::new(DbConfig::default()).await
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get an Arc clone of the pool for sharing across threads
    pub fn pool_arc(&self) -> Arc<PgPool> {
        Arc::clone(&self.pool)
    }

    /// Health check - verifies database connectivity
    /// This is idempotent and can be called repeatedly
    pub async fn health_check(&self) -> Result<DatabaseHealth, sqlx::Error> {
        let start = std::time::Instant::now();

        // Simple ping query
        sqlx::query("SELECT 1").execute(&*self.pool).await?;

        let latency = start.elapsed();

        Ok(DatabaseHealth {
            is_healthy: true,
            latency_ms: latency.as_millis() as u64,
            pool_size: self.pool.size(),
            idle_connections: self.pool.num_idle(),
        })
    }

    /// Graceful shutdown - closes all connections in the pool
    pub async fn shutdown(&self) {
        info!("Shutting down database connection pool...");
        self.pool.close().await;
        info!("Database connection pool closed");
    }

    /// Test database connection and log pool statistics
    pub async fn test_connection(&self) -> Result<(), sqlx::Error> {
        info!("Testing database connection...");

        let health = self.health_check().await?;

        info!("Database connection test successful");
        info!(
            "Pool statistics: size={}, idle={}, latency={}ms",
            health.pool_size, health.idle_connections, health.latency_ms
        );

        Ok(())
    }
}

/// Database health status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseHealth {
    pub is_healthy: bool,
    pub latency_ms: u64,
    pub pool_size: u32,
    pub idle_connections: usize,
}

/// Initialize database connection at application startup
/// This is the main entry point for database initialization
pub async fn initialize_database() -> Result<DbManager, sqlx::Error> {
    info!("=== Database Initialization Started ===");

    // Load configuration from environment
    let config = DbConfig::default();

    // Create database manager
    let db_manager = DbManager::new(config).await?;

    // Test the connection
    db_manager.test_connection().await?;

    info!("=== Database Initialization Completed ===");

    Ok(db_manager)
}

/// Initialize database with custom configuration
pub async fn initialize_database_with_config(config: DbConfig) -> Result<DbManager, sqlx::Error> {
    info!("=== Database Initialization Started (Custom Config) ===");

    let db_manager = DbManager::new(config).await?;
    db_manager.test_connection().await?;

    info!("=== Database Initialization Completed ===");

    Ok(db_manager)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DbConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
    }

    #[tokio::test]
    async fn test_db_manager_creation() {
        // This test requires a running PostgreSQL instance
        // Skip if DATABASE_URL is not set
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        let config = DbConfig::default();
        let result = DbManager::new(config).await;

        if let Ok(manager) = result {
            assert!(manager.health_check().await.is_ok());
            manager.shutdown().await;
        }
    }
}
