use crate::datalayer::CRUD::api_key::ApiKeyBuilder;
use crate::datalayer::db_ops::constants::POOL_STATE_TRACKER;
use crate::datalayer::helper::backoff::ExponentialBackoff;
use crate::errors::errors::ServiceError;
use chrono::{DateTime, Duration, Timelike, Utc};
use sqlx::postgres::PgPool;
use uuid::Uuid;

/// Rate limit window type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    Minute,
    Hour,
}

impl WindowType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WindowType::Minute => "minute",
            WindowType::Hour => "hour",
        }
    }

    /// Get the duration for this window type
    pub fn duration(&self) -> Duration {
        match self {
            WindowType::Minute => Duration::minutes(1),
            WindowType::Hour => Duration::hours(1),
        }
    }

    /// Get the start of the current window
    pub fn window_start(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            WindowType::Minute => {
                // Round down to the start of the current minute
                now.with_second(0).unwrap().with_nanosecond(0).unwrap()
            }
            WindowType::Hour => {
                // Round down to the start of the current hour
                now.with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
                    .with_nanosecond(0)
                    .unwrap()
            }
        }
    }
}

/// Rate limit check result
#[derive(Debug)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub limit: i32,
    pub remaining: i32,
    pub reset_at: DateTime<Utc>,
    pub window_type: WindowType,
}

/// Rate limiter for API keys
pub struct RateLimiter;

impl RateLimiter {
    /// Check if a request is allowed under rate limits
    /// Returns Ok(RateLimitResult) if allowed, Err if rate limit exceeded
    pub async fn check_rate_limit(
        api_key_id: Uuid,
        api_key_prefix: &str,
    ) -> Result<Vec<RateLimitResult>, ServiceError> {
        // Get the API key to check its rate limits
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
            .expect_rate_limit_per_minute()
            .expect_rate_limit_per_hour()
            .expect_created_at()
            .expect_last_used_at()
            .expect_permissions()
            .expect_expires_at()
            .expect_revoked_at()
            .read(Some(&mut conn))
            .await?;

        // Check if API key is active
        if api_key.status != "active" {
            tracker.return_connection(conn);
            return Err(ServiceError::ValidationError(format!(
                "API key {} is not active (status: {})",
                api_key_prefix, api_key.status
            )));
        }

        // Check if API key is revoked
        if api_key.revoked_at.is_some() {
            tracker.return_connection(conn);
            return Err(ServiceError::ValidationError(format!(
                "API key {} has been revoked",
                api_key_prefix
            )));
        }

        let now = Utc::now();
        let mut results = Vec::new();

        // Check minute rate limit
        if let Some(minute_limit) = api_key.rate_limit_per_minute {
            let result =
                Self::check_window(&mut conn, api_key_id, WindowType::Minute, minute_limit, now)
                    .await?;

            if !result.allowed {
                tracker.return_connection(conn);
                return Err(ServiceError::RateLimitExceeded {
                    limit: minute_limit,
                    window: "minute".to_string(),
                    reset_at: result.reset_at,
                });
            }
            results.push(result);
        }

        // Check hour rate limit
        if let Some(hour_limit) = api_key.rate_limit_per_hour {
            let result =
                Self::check_window(&mut conn, api_key_id, WindowType::Hour, hour_limit, now)
                    .await?;

            if !result.allowed {
                tracker.return_connection(conn);
                return Err(ServiceError::RateLimitExceeded {
                    limit: hour_limit,
                    window: "hour".to_string(),
                    reset_at: result.reset_at,
                });
            }
            results.push(result);
        }

        tracker.return_connection(conn);
        Ok(results)
    }

    /// Check and increment counter for a specific time window
    async fn check_window(
        conn: &mut sqlx::PgConnection,
        api_key_id: Uuid,
        window_type: WindowType,
        limit: i32,
        now: DateTime<Utc>,
    ) -> Result<RateLimitResult, ServiceError> {
        let window_start = window_type.window_start(now);
        let reset_at = window_start + window_type.duration();

        // Try to get existing counter for this window
        let existing_count: Option<i32> = sqlx::query_scalar(
            "SELECT request_count FROM rate_limit_counters 
             WHERE api_key_id = $1 AND window_type = $2 AND window_start = $3",
        )
        .bind(api_key_id)
        .bind(window_type.as_str())
        .bind(window_start)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;

        let current_count = existing_count.unwrap_or(0);

        // Check if limit would be exceeded
        if current_count >= limit {
            return Ok(RateLimitResult {
                allowed: false,
                limit,
                remaining: 0,
                reset_at,
                window_type,
            });
        }

        // Increment or create counter
        if existing_count.is_some() {
            // Update existing counter
            sqlx::query(
                "UPDATE rate_limit_counters 
                 SET request_count = request_count + 1 
                 WHERE api_key_id = $1 AND window_type = $2 AND window_start = $3",
            )
            .bind(api_key_id)
            .bind(window_type.as_str())
            .bind(window_start)
            .execute(&mut *conn)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;
        } else {
            // Create new counter
            sqlx::query(
                "INSERT INTO rate_limit_counters (api_key_id, window_start, window_type, request_count)
                 VALUES ($1, $2, $3, 1)
                 ON CONFLICT (api_key_id, window_start, window_type) 
                 DO UPDATE SET request_count = rate_limit_counters.request_count + 1",
            )
            .bind(api_key_id)
            .bind(window_start)
            .bind(window_type.as_str())
            .execute(&mut *conn)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;
        }

        Ok(RateLimitResult {
            allowed: true,
            limit,
            remaining: limit - current_count - 1,
            reset_at,
            window_type,
        })
    }

    /// Clean up old rate limit counters (should be run periodically)
    pub async fn cleanup_old_counters(pool: &PgPool) -> Result<u64, ServiceError> {
        let cutoff = Utc::now() - Duration::hours(2); // Keep last 2 hours

        let result = sqlx::query("DELETE FROM rate_limit_counters WHERE window_start < $1")
            .bind(cutoff)
            .execute(pool)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?;

        Ok(result.rows_affected())
    }

    /// Get current rate limit status without incrementing
    pub async fn get_rate_limit_status(
        api_key_id: Uuid,
    ) -> Result<Vec<RateLimitResult>, ServiceError> {
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
            .expect_rate_limit_per_minute()
            .expect_rate_limit_per_hour()
            .expect_created_at()
            .read(Some(&mut conn))
            .await?;

        let now = Utc::now();
        let mut results = Vec::new();

        // Check minute rate limit status
        if let Some(minute_limit) = api_key.rate_limit_per_minute {
            let window_start = WindowType::Minute.window_start(now);
            let reset_at = window_start + WindowType::Minute.duration();

            let current_count: i32 = sqlx::query_scalar(
                "SELECT COALESCE(request_count, 0) FROM rate_limit_counters 
                 WHERE api_key_id = $1 AND window_type = 'minute' AND window_start = $2",
            )
            .bind(api_key_id)
            .bind(window_start)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?
            .unwrap_or(0);

            results.push(RateLimitResult {
                allowed: current_count < minute_limit,
                limit: minute_limit,
                remaining: (minute_limit - current_count).max(0),
                reset_at,
                window_type: WindowType::Minute,
            });
        }

        // Check hour rate limit status
        if let Some(hour_limit) = api_key.rate_limit_per_hour {
            let window_start = WindowType::Hour.window_start(now);
            let reset_at = window_start + WindowType::Hour.duration();

            let current_count: i32 = sqlx::query_scalar(
                "SELECT COALESCE(request_count, 0) FROM rate_limit_counters 
                 WHERE api_key_id = $1 AND window_type = 'hour' AND window_start = $2",
            )
            .bind(api_key_id)
            .bind(window_start)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| ServiceError::DatabaseError(e.to_string()))?
            .unwrap_or(0);

            results.push(RateLimitResult {
                allowed: current_count < hour_limit,
                limit: hour_limit,
                remaining: (hour_limit - current_count).max(0),
                reset_at,
                window_type: WindowType::Hour,
            });
        }

        tracker.return_connection(conn);
        Ok(results)
    }

    /// Check rate limit with automatic retry using exponential backoff
    ///
    /// This method automatically retries with exponential backoff if a rate limit
    /// is exceeded, up to `max_retries` times.
    ///
    /// # Arguments
    ///
    /// * `api_key_id` - The UUID of the API key
    /// * `api_key_prefix` - The prefix of the API key (for error messages)
    /// * `max_retries` - Maximum number of retry attempts (default: 3)
    /// * `base_delay_ms` - Base delay in milliseconds for exponential backoff (default: 100ms)
    /// * `max_delay_ms` - Maximum delay cap in milliseconds (default: 5000ms)
    ///
    /// # Returns
    ///
    /// * `Ok(RateLimitResult)` - If the request is allowed (either immediately or after retries)
    /// * `Err(ServiceError)` - If rate limit is still exceeded after all retries, or other errors
    ///
    /// # Example
    ///
    /// ```rust
    /// // Use default settings (3 retries, 100ms base, 5s max)
    /// let result = RateLimiter::check_with_retry(api_key_id, &api_key_prefix, 3, 100, 5000).await?;
    /// ```
    pub async fn check_with_retry(
        api_key_id: Uuid,
        api_key_prefix: &str,
        max_retries: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<RateLimitResult, ServiceError> {
        let mut attempt = 0;

        loop {
            match Self::check_rate_limit(api_key_id, api_key_prefix).await {
                Ok(results) => {
                    // Success! Return the first result (minute or hour limit)
                    if let Some(result) = results.into_iter().next() {
                        return Ok(result);
                    } else {
                        // No rate limits configured
                        return Err(ServiceError::ValidationError(
                            "No rate limits configured for this API key".to_string(),
                        ));
                    }
                }
                Err(ServiceError::RateLimitExceeded {
                    limit,
                    window,
                    reset_at,
                }) => {
                    // Rate limit exceeded
                    if attempt >= max_retries {
                        // Exhausted all retries, return the error
                        return Err(ServiceError::RateLimitExceeded {
                            limit,
                            window,
                            reset_at,
                        });
                    }

                    // Create backoff calculator with configured settings
                    let mut backoff = ExponentialBackoff::new();
                    backoff.set_base_delay_ms(base_delay_ms);
                    backoff.set_max_delay_ms(max_delay_ms);

                    // Calculate backoff delay
                    let backoff_ms = backoff.calculate(attempt);

                    println!(
                        "⏳ Rate limit exceeded (attempt {}), retrying in {}ms...",
                        attempt + 1,
                        backoff_ms
                    );

                    // Sleep for the backoff duration
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;

                    attempt += 1;
                }
                Err(e) => {
                    // Other error (not rate limit), return immediately
                    return Err(e);
                }
            }
        }
    }

    /// Check rate limit with retry, waiting until reset time if all retries fail
    ///
    /// This is a more aggressive retry strategy that will wait until the rate limit
    /// window resets if exponential backoff retries are exhausted.
    ///
    /// # Arguments
    ///
    /// * `api_key_id` - The UUID of the API key
    /// * `api_key_prefix` - The prefix of the API key (for error messages)
    /// * `max_retries` - Maximum number of retry attempts before waiting for reset
    /// * `base_delay_ms` - Base delay in milliseconds for exponential backoff
    /// * `max_delay_ms` - Maximum delay cap in milliseconds
    ///
    /// # Returns
    ///
    /// * `Ok(RateLimitResult)` - If the request is eventually allowed
    /// * `Err(ServiceError)` - For non-rate-limit errors
    ///
    /// # Warning
    ///
    /// This function can block for up to 1 minute (or 1 hour for hourly limits).
    /// Use with caution in production environments.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Will wait until rate limit resets if retries fail
    /// let result = RateLimiter::check_with_wait_for_reset(
    ///     api_key_id,
    ///     &api_key_prefix,
    ///     3,
    ///     100,
    ///     5000
    /// ).await?;
    /// ```
    pub async fn check_with_wait_for_reset(
        api_key_id: Uuid,
        api_key_prefix: &str,
        max_retries: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
    ) -> Result<RateLimitResult, ServiceError> {
        // First try with exponential backoff retries
        match Self::check_with_retry(
            api_key_id,
            api_key_prefix,
            max_retries,
            base_delay_ms,
            max_delay_ms,
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(ServiceError::RateLimitExceeded {
                limit: _,
                window,
                reset_at,
            }) => {
                // Calculate time until reset
                let now = Utc::now();
                let wait_duration = reset_at.signed_duration_since(now);

                if wait_duration.num_seconds() > 0 {
                    let wait_ms = wait_duration.num_milliseconds() as u64;

                    println!(
                        "⏰ Waiting for rate limit reset ({} window) in {}ms...",
                        window, wait_ms
                    );

                    // Wait until reset time
                    tokio::time::sleep(tokio::time::Duration::from_millis(wait_ms)).await;

                    // Try one more time after reset
                    match Self::check_rate_limit(api_key_id, api_key_prefix).await {
                        Ok(results) => {
                            if let Some(result) = results.into_iter().next() {
                                Ok(result)
                            } else {
                                Err(ServiceError::ValidationError(
                                    "No rate limits configured for this API key".to_string(),
                                ))
                            }
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    // Reset time has already passed, try immediately
                    match Self::check_rate_limit(api_key_id, api_key_prefix).await {
                        Ok(results) => {
                            if let Some(result) = results.into_iter().next() {
                                Ok(result)
                            } else {
                                Err(ServiceError::ValidationError(
                                    "No rate limits configured for this API key".to_string(),
                                ))
                            }
                        }
                        Err(e) => Err(e),
                    }
                }
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_type_duration() {
        assert_eq!(WindowType::Minute.duration(), Duration::minutes(1));
        assert_eq!(WindowType::Hour.duration(), Duration::hours(1));
    }

    #[test]
    fn test_window_start_minute() {
        let now = Utc::now();
        let window_start = WindowType::Minute.window_start(now);

        // Should be at the start of the current minute
        assert_eq!(window_start.second(), 0);
        assert_eq!(window_start.nanosecond(), 0);
    }

    #[test]
    fn test_window_start_hour() {
        let now = Utc::now();
        let window_start = WindowType::Hour.window_start(now);

        // Should be at the start of the current hour
        assert_eq!(window_start.minute(), 0);
        assert_eq!(window_start.second(), 0);
        assert_eq!(window_start.nanosecond(), 0);
    }
}
