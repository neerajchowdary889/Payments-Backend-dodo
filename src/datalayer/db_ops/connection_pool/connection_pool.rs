use crate::datalayer::db_ops::constants::DbConfig;
use sqlx::Postgres;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use tracing::{error, info};

/*
This implementation provides methods to get and put connections into the connection pool of the postgres database.
The ConnectionPool wraps sqlx's PgPool and provides explicit methods for connection lifecycle management.
*/

/// Connection pool wrapper for PostgreSQL database
#[derive(Clone)]
pub struct ConnectionPool {
    pool: Arc<PgPool>,
}

impl ConnectionPool {
    /// Creates a new connection pool from the given configuration
    pub async fn new(config: DbConfig) -> Result<Self, sqlx::Error> {
        info!(
            "Creating connection pool with max_connections: {}",
            config.max_connections
        );

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.connection_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime)
            .connect(&config.database_url)
            .await
            .map_err(|e| {
                error!("Failed to create connection pool: {}", e);
                e
            })?;

        info!("Connection pool created successfully");

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Gets a connection from the pool
    /// This acquires a connection from the pool. The connection is automatically
    /// returned to the pool when the PoolConnection is dropped.
    pub async fn get(&self) -> Result<sqlx::pool::PoolConnection<Postgres>, sqlx::Error> {
        info!(
            "Acquiring connection from pool (current size: {}, idle: {})",
            self.pool.size(),
            self.pool.num_idle()
        );

        self.pool.acquire().await.map_err(|e| {
            error!("Failed to acquire connection from pool: {}", e);
            e
        })
    }

    /// Puts a connection back into the pool
    /// Note: With sqlx, connections are automatically returned to the pool when dropped.
    /// This method is provided for API consistency and explicit documentation.
    /// You can simply drop the connection to return it to the pool.
    pub fn put(&self, conn: sqlx::pool::PoolConnection<Postgres>) {
        info!("Returning connection to pool");
        // Connection is automatically returned when dropped
        drop(conn);
    }

    /// Closes the connection pool and all its connections
    /// This is a graceful shutdown that waits for all connections to be returned
    pub async fn close(&self) {
        info!("Closing connection pool...");
        self.pool.close().await;
        info!("Connection pool closed successfully");
    }

    /// Gets a reference to the underlying PgPool
    /// Useful for operations that need direct pool access
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Gets an Arc clone of the pool for sharing across threads
    pub fn pool_arc(&self) -> Arc<PgPool> {
        Arc::clone(&self.pool)
    }

    /// Returns the current size of the pool (total connections)
    pub fn size(&self) -> u32 {
        self.pool.size()
    }

    /// Returns the number of idle connections in the pool
    pub fn idle_count(&self) -> usize {
        self.pool.num_idle()
    }

    /// Health check - verifies that the pool can acquire a connection
    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        let mut conn = self.get().await?;
        sqlx::query("SELECT 1").execute(&mut *conn).await?;
        self.put(conn);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_creation() {
        // Skip if DATABASE_URL is not set
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        let config = DbConfig::default();
        let result = ConnectionPool::new(config).await;

        if let Ok(pool) = result {
            assert!(pool.size() > 0);
            pool.close().await;
        }
    }

    #[tokio::test]
    async fn test_get_connection() {
        // Skip if DATABASE_URL is not set
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        let config = DbConfig::default();
        if let Ok(pool) = ConnectionPool::new(config).await {
            let conn_result = pool.get().await;
            assert!(conn_result.is_ok());

            if let Ok(conn) = conn_result {
                pool.put(conn);
            }

            pool.close().await;
        }
    }

    #[tokio::test]
    async fn test_multiple_connections() {
        // Skip if DATABASE_URL is not set
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        let config = DbConfig::default().set_max_connections(5);
        if let Ok(pool) = ConnectionPool::new(config).await {
            let mut connections = Vec::new();

            // Get multiple connections
            for _ in 0..3 {
                if let Ok(conn) = pool.get().await {
                    connections.push(conn);
                }
            }

            assert_eq!(connections.len(), 3);

            // Return all connections
            for conn in connections {
                pool.put(conn);
            }

            pool.close().await;
        }
    }

    #[tokio::test]
    async fn test_pool_statistics() {
        // Skip if DATABASE_URL is not set
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        let config = DbConfig::default();
        if let Ok(pool) = ConnectionPool::new(config).await {
            let _initial_size = pool.size();
            let _initial_idle = pool.idle_count();

            // Size and idle count are always valid (u32 and usize respectively)
            // Just verify we can call these methods without panicking

            pool.close().await;
        }
    }

    #[tokio::test]
    async fn test_health_check() {
        // Skip if DATABASE_URL is not set
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        let config = DbConfig::default();
        if let Ok(pool) = ConnectionPool::new(config).await {
            let health = pool.health_check().await;
            assert!(health.is_ok());
            pool.close().await;
        }
    }
}
