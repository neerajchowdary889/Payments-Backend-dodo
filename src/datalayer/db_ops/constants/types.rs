use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::time::Duration;

/// Database configuration structure
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

/// Pool state tracker to monitor active connections
/// Note: In practice, sqlx's PgPool already tracks this internally.
/// This struct is provided if you need custom tracking logic.
///
/// The `available_connections` counter tracks how many connections are available.
/// It starts at `max_connections` and decrements when connections are acquired,
/// increments when they are returned. This is atomic for thread-safety.
pub struct PoolStateTracker {
    pub db_config: DbConfig,
    pub current_connections: Vec<PoolConnection<Postgres>>,
    /// Atomic counter tracking available connections (starts at max_connections)
    pub available_connections: AtomicU32,
    pub pool: Arc<PgPool>,
}
