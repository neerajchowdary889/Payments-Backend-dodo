use crate::datalayer::db_ops::constants::DbConfig;
use crate::datalayer::db_ops::constants::types::PoolStateTracker;
use crate::datalayer::db_ops::db_health::{
    DatabaseHealth, TableVerification, check_database_health, verify_database_initialization,
};
use std::sync::Arc;
use tracing::{error, info};

/// Database operations manager
/// Provides high-level abstraction for database operations
/// Uses PoolStateTracker for connection management
pub struct DbOps {
    tracker: &'static PoolStateTracker,
}

impl DbOps {
    /// Creates a new DbOps instance with default configuration
    /// Initializes the global PoolStateTracker singleton
    pub async fn new() -> Result<Self, sqlx::Error> {
        info!("🔧 Initializing DbOps with default configuration...");

        // Build config using builder pattern
        let config = DbConfig::new();

        Self::with_config(config).await
    }

    /// Creates a new DbOps instance with custom configuration
    /// Uses builder pattern for DbConfig
    pub async fn with_config(config: DbConfig) -> Result<Self, sqlx::Error> {
        info!("🔧 Initializing DbOps with custom configuration...");
        info!("   - Max connections: {}", config.max_connections);
        info!("   - Min connections: {}", config.min_connections);

        // Initialize the global PoolStateTracker
        let tracker = PoolStateTracker::new(Some(config)).await?;

        info!("✅ DbOps initialized successfully");

        Ok(Self { tracker })
    }

    /// Get a reference to the PoolStateTracker
    pub fn tracker(&self) -> &'static PoolStateTracker {
        self.tracker
    }

    /// Get a reference to the underlying pool
    pub fn pool(&self) -> &Arc<sqlx::PgPool> {
        &self.tracker.pool
    }

    /// Check database health
    pub async fn health_check(&self) -> Result<DatabaseHealth, sqlx::Error> {
        info!("🏥 Performing database health check...");
        let health = check_database_health(&self.tracker.pool).await?;

        info!(
            "✅ Database health: latency={}ms, pool={}/{}, available={}",
            health.latency_ms,
            health.idle_connections,
            health.pool_size,
            health.available_connections
        );

        Ok(health)
    }

    /// Verify database initialization
    /// Checks connectivity, health, and table initialization
    pub async fn verify_initialization(
        &self,
    ) -> Result<(DatabaseHealth, Vec<TableVerification>), sqlx::Error> {
        info!("🔍 Verifying complete database initialization...");

        let (health, tables) = verify_database_initialization(&self.tracker.pool).await?;

        info!("✅ Database verification complete");
        info!("   - Health: OK ({}ms latency)", health.latency_ms);
        info!("   - Tables verified: {}", tables.len());

        Ok((health, tables))
    }

    /// Test database connection
    /// Simple connectivity test with logging
    pub async fn test_connection(&self) -> Result<(), sqlx::Error> {
        info!("🔌 Testing database connection...");

        let health = self.health_check().await?;

        info!("✅ Connection test successful");
        info!("   - Latency: {}ms", health.latency_ms);
        info!("   - Pool size: {}", health.pool_size);
        info!("   - Idle connections: {}", health.idle_connections);

        Ok(())
    }

    /// Get pool statistics
    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            size: self.tracker.pool.size(),
            idle: self.tracker.pool.num_idle(),
            available: self.tracker.available_connections(),
        }
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) {
        info!("🛑 Shutting down database operations...");
        self.tracker.pool.close().await;
        info!("✅ Database operations shut down successfully");
    }
}

/// Pool statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolStats {
    pub size: u32,
    pub idle: usize,
    pub available: u32,
}

/// Initialize database operations at application startup
/// This is the main entry point for database initialization
pub async fn initialize_database() -> Result<DbOps, sqlx::Error> {
    info!("=== 🚀 Database Initialization Started ===");

    // Create DbOps with default configuration
    let db_ops = DbOps::new().await?;

    // Test the connection
    db_ops.test_connection().await?;

    // Verify database initialization (tables, etc.)
    db_ops.verify_initialization().await?;

    info!("=== ✅ Database Initialization Completed ===");

    Ok(db_ops)
}

/// Initialize database with custom configuration using builder pattern
///
/// # Example
/// ```rust
/// use std::time::Duration;
///
/// let db_ops = initialize_database_with_builder(|config| {
///     config
///         .set_max_connections(20)
///         .set_min_connections(5)
///         .set_connection_timeout(Duration::from_secs(60))
/// }).await?;
/// ```
pub async fn initialize_database_with_builder<F>(builder: F) -> Result<DbOps, sqlx::Error>
where
    F: FnOnce(DbConfig) -> DbConfig,
{
    info!("=== 🚀 Database Initialization Started (Custom Config) ===");

    // Start with default config and apply builder
    let config = builder(DbConfig::new());

    let db_ops = DbOps::with_config(config).await?;
    db_ops.test_connection().await?;
    db_ops.verify_initialization().await?;

    info!("=== ✅ Database Initialization Completed ===");

    Ok(db_ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_db_ops_creation() {
        println!("\n=== TEST: DbOps Creation ===");

        if std::env::var("DATABASE_URL").is_err() {
            println!("⚠️  Skipping test: DATABASE_URL not set");
            return;
        }

        println!("🔧 Creating DbOps with defaults...");
        let result = DbOps::new().await;

        if let Ok(db_ops) = result {
            println!("✅ DbOps created successfully");

            let stats = db_ops.pool_stats();
            println!(
                "📊 Pool stats: size={}, idle={}, available={}",
                stats.size, stats.idle, stats.available
            );

            db_ops.shutdown().await;
        }
    }

    #[tokio::test]
    async fn test_db_ops_with_custom_config() {
        println!("\n=== TEST: DbOps with Custom Config ===");

        if std::env::var("DATABASE_URL").is_err() {
            println!("⚠️  Skipping test: DATABASE_URL not set");
            return;
        }

        println!("🔧 Creating DbOps with custom config...");
        let config = DbConfig::new()
            .set_max_connections(15)
            .set_min_connections(3)
            .set_connection_timeout(Duration::from_secs(45));

        let result = DbOps::with_config(config).await;

        if let Ok(db_ops) = result {
            println!("✅ DbOps created with custom config");
            db_ops.shutdown().await;
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        println!("\n=== TEST: Health Check ===");

        if std::env::var("DATABASE_URL").is_err() {
            println!("⚠️  Skipping test: DATABASE_URL not set");
            return;
        }

        if let Ok(db_ops) = DbOps::new().await {
            println!("🏥 Running health check...");
            let health = db_ops.health_check().await;

            assert!(health.is_ok());

            if let Ok(h) = health {
                println!("✅ Health check passed");
                println!("   - Latency: {}ms", h.latency_ms);
                println!("   - Pool size: {}", h.pool_size);
            }

            db_ops.shutdown().await;
        }
    }

    #[tokio::test]
    async fn test_verify_initialization() {
        println!("\n=== TEST: Verify Initialization ===");

        if std::env::var("DATABASE_URL").is_err() {
            println!("⚠️  Skipping test: DATABASE_URL not set");
            return;
        }

        if let Ok(db_ops) = DbOps::new().await {
            println!("🔍 Verifying database initialization...");
            let result = db_ops.verify_initialization().await;

            if let Ok((health, tables)) = result {
                println!("✅ Verification complete");
                println!("   - Tables verified: {}", tables.len());

                for table in tables {
                    println!(
                        "   - {}: {} rows",
                        table.table_name,
                        table.row_count.unwrap_or(0)
                    );
                }
            }

            db_ops.shutdown().await;
        }
    }

    #[tokio::test]
    async fn test_initialize_database() {
        println!("\n=== TEST: Initialize Database ===");

        if std::env::var("DATABASE_URL").is_err() {
            println!("⚠️  Skipping test: DATABASE_URL not set");
            return;
        }

        println!("🚀 Initializing database...");
        let result = initialize_database().await;

        assert!(result.is_ok());

        if let Ok(db_ops) = result {
            println!("✅ Database initialized successfully");
            db_ops.shutdown().await;
        }
    }
}
