use crate::datalayer::CRUD::api_key::ApiKeyBuilder;
use crate::datalayer::CRUD::redis::redis::RateLimitCounter;
use crate::datalayer::db_ops::constants::POOL_STATE_TRACKER;
use crate::datalayer::helper::backoff::ExponentialBackoff;
use crate::errors::errors::ServiceError;
use redis::aio::ConnectionManager;
use uuid::Uuid;

/// Backoff-based rate limiter
///
/// Uses soft and hard limits:
/// - Soft limit (default: 5): Start applying exponential backoff
/// - Hard limit (default: 15): Completely reject request
pub struct RateLimiter {
    soft_limit: u32,
    hard_limit: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            soft_limit: 5,
            hard_limit: 15,
            base_delay_ms: 1000,
            max_delay_ms: 20000,
        }
    }
}

impl RateLimiter {
    /// Create a new rate limiter with default settings
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a rate limiter with custom settings
    ///
    /// # Arguments
    ///
    /// * `soft_limit` - Number of requests before backoff starts (default: 5)
    /// * `hard_limit` - Maximum requests before blocking (default: 15)
    /// * `base_delay_ms` - Base delay for exponential backoff (default: 100ms)
    /// * `max_delay_ms` - Maximum delay cap (default: 5000ms)
    pub fn with_config(
        soft_limit: u32,
        hard_limit: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Self {
        Self {
            soft_limit,
            hard_limit,
            base_delay_ms,
            max_delay_ms,
        }
    }

    /// Check rate limit with backoff for an API key and endpoint
    /// Automatically tracks request count in Redis
    pub async fn check_with_backoff(
        &self,
        api_key_id: Uuid,
        api_key_prefix: &str,
        endpoint: &str,
        redis_conn: ConnectionManager,
    ) -> Result<(), ServiceError> {
        // Validate API key first
        self.validate_api_key(api_key_id, api_key_prefix).await?;

        // Get current count from Redis
        let mut counter = RateLimitCounter::new(redis_conn.clone());
        let current_count = counter
            .get_count(api_key_id, endpoint)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("Redis error: {}", e)))?;

        // Check hard limit
        if current_count >= self.hard_limit {
            return Err(ServiceError::RateLimitExceeded {
                limit: self.hard_limit as i32,
                window: endpoint.to_string(),
                reset_at: chrono::Utc::now() + chrono::Duration::seconds(60),
            });
        }

        // Apply backoff if over soft limit
        if current_count >= self.soft_limit {
            let attempts_over_soft = current_count - self.soft_limit;

            let mut backoff = ExponentialBackoff::new();
            backoff.set_base_delay_ms(self.base_delay_ms);
            backoff.set_max_delay_ms(self.max_delay_ms);

            let delay_ms = backoff.calculate(attempts_over_soft);

            tracing::warn!(
                api_key_id = %api_key_id,
                endpoint = %endpoint,
                request_count = current_count + 1,
                delay_ms = delay_ms,
                "Rate limit soft threshold exceeded, applying backoff"
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }

        // Increment counter in Redis
        counter
            .increment_count(api_key_id, endpoint)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("Redis error: {}", e)))?;

        Ok(())
    }

    /// Validate that the API key exists and is active
    async fn validate_api_key(
        &self,
        api_key_id: Uuid,
        api_key_prefix: &str,
    ) -> Result<(), ServiceError> {
        let tracker = POOL_STATE_TRACKER
            .get()
            .ok_or_else(|| ServiceError::DatabaseConnectionError)?;

        let mut conn = tracker
            .get_connection()
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("Failed to get connection: {}", e)))?;

        let api_key = ApiKeyBuilder::new()
            .id(api_key_id)
            .expect_id()
            .expect_account_id()
            .expect_key_hash()
            .expect_key_prefix()
            .expect_name()
            .expect_status()
            .expect_permissions()
            .expect_last_used_at()
            .expect_expires_at()
            .expect_created_at()
            .expect_revoked_at()
            .read(Some(&mut conn))
            .await?;

        tracker.return_connection(conn);

        // Check if API key is active
        if api_key.status != "active" {
            return Err(ServiceError::ValidationError(format!(
                "API key {} is not active (status: {})",
                api_key_prefix, api_key.status
            )));
        }

        // Check if API key is revoked
        if api_key.revoked_at.is_some() {
            return Err(ServiceError::ValidationError(format!(
                "API key {} has been revoked",
                api_key_prefix
            )));
        }

        Ok(())
    }

    /// Get current request count for an API key and endpoint
    pub async fn get_count(
        &self,
        api_key_id: Uuid,
        endpoint: &str,
        redis_conn: ConnectionManager,
    ) -> Result<u32, ServiceError> {
        let mut counter = RateLimitCounter::new(redis_conn);
        counter
            .get_count(api_key_id, endpoint)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("Redis error: {}", e)))
    }

    /// Reset rate limit counter for an API key and endpoint
    pub async fn reset_count(
        &self,
        api_key_id: Uuid,
        endpoint: &str,
        redis_conn: ConnectionManager,
    ) -> Result<(), ServiceError> {
        let mut counter = RateLimitCounter::new(redis_conn);
        counter
            .reset_count(api_key_id, endpoint)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("Redis error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let limiter = RateLimiter::new();
        assert_eq!(limiter.soft_limit, 5);
        assert_eq!(limiter.hard_limit, 15);
        assert_eq!(limiter.base_delay_ms, 1000);
        assert_eq!(limiter.max_delay_ms, 20000);
    }

    #[test]
    fn test_custom_config() {
        let limiter = RateLimiter::with_config(10, 30, 200, 10000);
        assert_eq!(limiter.soft_limit, 10);
        assert_eq!(limiter.hard_limit, 30);
        assert_eq!(limiter.base_delay_ms, 200);
        assert_eq!(limiter.max_delay_ms, 10000);
    }
}
