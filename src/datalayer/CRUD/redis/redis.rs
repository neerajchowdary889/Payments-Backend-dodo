use redis::{AsyncCommands, aio::ConnectionManager};
use std::time::Duration;
use uuid::Uuid;

/// Redis helper for rate limiting counters
pub struct RateLimitCounter {
    redis: ConnectionManager,
}

impl RateLimitCounter {
    pub fn new(redis: ConnectionManager) -> Self {
        Self { redis }
    }

    /// Get current request count for an API key and endpoint
    /// Redis key format: rate_limit:{api_key_id}:{endpoint}
    pub async fn get_count(
        &mut self,
        api_key_id: Uuid,
        endpoint: &str,
    ) -> Result<u32, redis::RedisError> {
        let key = format!("rate_limit:{}:{}", api_key_id, endpoint);
        let count: Option<u32> = self.redis.get(&key).await?;
        Ok(count.unwrap_or(0))
    }

    /// Increment request count for an API key and endpoint
    /// Sets TTL of 60 seconds on first increment
    pub async fn increment_count(
        &mut self,
        api_key_id: Uuid,
        endpoint: &str,
    ) -> Result<u32, redis::RedisError> {
        let key = format!("rate_limit:{}:{}", api_key_id, endpoint);

        // Increment counter
        let new_count: u32 = self.redis.incr(&key, 1).await?;

        // Set expiration on first increment
        if new_count == 1 {
            let _: () = self.redis.expire(&key, 60).await?;
        }

        Ok(new_count)
    }

    /// Reset counter for an API key and endpoint
    pub async fn reset_count(
        &mut self,
        api_key_id: Uuid,
        endpoint: &str,
    ) -> Result<(), redis::RedisError> {
        let key = format!("rate_limit:{}:{}", api_key_id, endpoint);
        let _: () = self.redis.del(&key).await?;
        Ok(())
    }

    /// Get time until counter resets (TTL in seconds)
    pub async fn get_reset_time(
        &mut self,
        api_key_id: Uuid,
        endpoint: &str,
    ) -> Result<Option<Duration>, redis::RedisError> {
        let key = format!("rate_limit:{}:{}", api_key_id, endpoint);
        let ttl: i64 = self.redis.ttl(&key).await?;

        if ttl > 0 {
            Ok(Some(Duration::from_secs(ttl as u64)))
        } else {
            Ok(None)
        }
    }

    /// Get current request count for a custom key (e.g., IP-based)
    /// Redis key format: rate_limit:{custom_key}
    pub async fn get_count_by_key(&mut self, custom_key: &str) -> Result<u32, redis::RedisError> {
        let key = format!("rate_limit:{}", custom_key);
        let count: Option<u32> = self.redis.get(&key).await?;
        Ok(count.unwrap_or(0))
    }

    /// Increment request count for a custom key (e.g., IP-based)
    /// Sets TTL of 60 seconds on first increment
    pub async fn increment_count_by_key(
        &mut self,
        custom_key: &str,
    ) -> Result<u32, redis::RedisError> {
        let key = format!("rate_limit:{}", custom_key);

        // Increment counter
        let new_count: u32 = self.redis.incr(&key, 1).await?;

        // Set expiration on first increment
        if new_count == 1 {
            let _: () = self.redis.expire(&key, 60).await?;
        }

        Ok(new_count)
    }

    /// Reset counter for a custom key
    pub async fn reset_count_by_key(&mut self, custom_key: &str) -> Result<(), redis::RedisError> {
        let key = format!("rate_limit:{}", custom_key);
        let _: () = self.redis.del(&key).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running Redis instance
    // Run with: docker-compose up -d redis

    #[tokio::test]
    #[ignore] // Ignore by default, run with --ignored
    async fn test_rate_limit_counter() {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let client = redis::Client::open(redis_url).unwrap();
        let conn = redis::aio::ConnectionManager::new(client).await.unwrap();
        let mut counter = RateLimitCounter::new(conn);

        let api_key_id = Uuid::new_v4();
        let endpoint = "/api/v1/test";

        // Initial count should be 0
        let count = counter.get_count(api_key_id, endpoint).await.unwrap();
        assert_eq!(count, 0);

        // Increment and check
        let count = counter.increment_count(api_key_id, endpoint).await.unwrap();
        assert_eq!(count, 1);

        let count = counter.increment_count(api_key_id, endpoint).await.unwrap();
        assert_eq!(count, 2);

        // Reset
        counter.reset_count(api_key_id, endpoint).await.unwrap();
        let count = counter.get_count(api_key_id, endpoint).await.unwrap();
        assert_eq!(count, 0);
    }
}
