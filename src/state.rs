use redis::{Client, aio::ConnectionManager};
use std::sync::Arc;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// Redis client for rate limiting counters
    pub redis: Arc<ConnectionManager>,
}

impl AppState {
    /// Create new application state
    pub async fn new(redis_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize Redis client
        let redis_client = Client::open(redis_url)?;
        let redis_conn = ConnectionManager::new(redis_client).await?;

        Ok(Self {
            redis: Arc::new(redis_conn),
        })
    }
}
