use sqlx::Postgres;
use sqlx::pool::PoolConnection;
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
pub struct PoolStateTracker {
    pub db_config: DbConfig,
    pub current_connections: Vec<PoolConnection<Postgres>>,
    pub current_connections_count: u32,
}