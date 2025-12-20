use crate::datalayer::CRUD::rate_limiter::RateLimiter;
use crate::datalayer::helper::backoff::ExponentialBackoff;
use crate::errors::errors::ServiceError;
use uuid::Uuid;

/// Rate limiter with automatic exponential backoff retry
///
/// This wrapper around RateLimiter automatically retries requests that hit rate limits
/// using exponential backoff with jitter.
pub struct RateLimiterWithBackoff {
    max_retries: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
}

impl Default for RateLimiterWithBackoff {
    fn default() -> Self {
        Self {
            max_retries: 3,     // Retry up to 3 times
            base_delay_ms: 100, // Start with 100ms
            max_delay_ms: 5000, // Cap at 5 seconds
        }
    }
}

impl RateLimiterWithBackoff {
    /// Create a new rate limiter with backoff using default settings
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a rate limiter with custom backoff settings
    ///
    /// # Arguments
    ///
    /// * `max_retries` - Maximum number of retry attempts (default: 3)
    /// * `base_delay_ms` - Base delay in milliseconds for exponential backoff (default: 100ms)
    /// * `max_delay_ms` - Maximum delay cap in milliseconds (default: 5000ms)
    pub fn with_config(max_retries: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            max_delay_ms,
        }
    }

    /// Check rate limit with automatic retry on rate limit exceeded
    ///
    /// This function will automatically retry with exponential backoff if a rate limit
    /// is exceeded, up to `max_retries` times.
    ///
    /// # Arguments
    ///
    /// * `api_key_id` - The UUID of the API key
    /// * `api_key_prefix` - The prefix of the API key (for error messages)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<RateLimitResult>)` - If the request is allowed (either immediately or after retries)
    /// * `Err(ServiceError)` - If rate limit is still exceeded after all retries, or other errors
    ///
    /// # Example
    ///
    /// ```rust
    /// let limiter = RateLimiterWithBackoff::new();
    /// match limiter.check_with_retry(api_key_id, &api_key_prefix).await {
    ///     Ok(results) => {
    ///         // Request allowed, proceed
    ///         println!("Remaining: {}", results[0].remaining);
    ///     }
    ///     Err(ServiceError::RateLimitExceeded { .. }) => {
    ///         // Still rate limited after retries
    ///         return Err(ServiceError::RateLimitExceeded { .. });
    ///     }
    ///     Err(e) => {
    ///         // Other error
    ///         return Err(e);
    ///     }
    /// }
    /// ```
    pub async fn check_with_retry(
        &self,
        api_key_id: Uuid,
        api_key_prefix: &str,
    ) -> Result<crate::datalayer::CRUD::rate_limiter::RateLimitResult, ServiceError> {
        let mut attempt = 0;

        loop {
            match RateLimiter::check_rate_limit(api_key_id, api_key_prefix).await {
                Ok(results) => {
                    // Success! Return the first result (minute or hour limit)
                    if let Some(result) = results.into_iter().next() {
                        return Ok(result);
                    } else {
                        // No rate limits configured, this shouldn't happen but handle gracefully
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
                    if attempt >= self.max_retries {
                        // Exhausted all retries, return the error
                        return Err(ServiceError::RateLimitExceeded {
                            limit,
                            window,
                            reset_at,
                        });
                    }

                    // Create backoff calculator with configured settings
                    let mut backoff = ExponentialBackoff::new();
                    backoff.set_base_delay_ms(self.base_delay_ms);
                    backoff.set_max_delay_ms(self.max_delay_ms);

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

    /// Check rate limit with retry, but wait until reset time if all retries fail
    ///
    /// This is a more aggressive retry strategy that will wait until the rate limit
    /// window resets if exponential backoff retries are exhausted.
    ///
    /// # Arguments
    ///
    /// * `api_key_id` - The UUID of the API key
    /// * `api_key_prefix` - The prefix of the API key (for error messages)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<RateLimitResult>)` - If the request is eventually allowed
    /// * `Err(ServiceError)` - For non-rate-limit errors
    ///
    /// # Warning
    ///
    /// This function can block for up to 1 minute (or 1 hour for hourly limits).
    /// Use with caution in production environments.
    pub async fn check_with_wait_for_reset(
        &self,
        api_key_id: Uuid,
        api_key_prefix: &str,
    ) -> Result<crate::datalayer::CRUD::rate_limiter::RateLimitResult, ServiceError> {
        // First try with exponential backoff retries
        match self.check_with_retry(api_key_id, api_key_prefix).await {
            Ok(result) => Ok(result),
            Err(ServiceError::RateLimitExceeded {
                limit,
                window,
                reset_at,
            }) => {
                // Calculate time until reset
                let now = chrono::Utc::now();
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
                    match RateLimiter::check_rate_limit(api_key_id, api_key_prefix).await {
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
                    match RateLimiter::check_rate_limit(api_key_id, api_key_prefix).await {
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
    fn test_default_config() {
        let limiter = RateLimiterWithBackoff::new();
        assert_eq!(limiter.max_retries, 3);
        assert_eq!(limiter.base_delay_ms, 100);
        assert_eq!(limiter.max_delay_ms, 5000);
    }

    #[test]
    fn test_custom_config() {
        let limiter = RateLimiterWithBackoff::with_config(5, 200, 10000);
        assert_eq!(limiter.max_retries, 5);
        assert_eq!(limiter.base_delay_ms, 200);
        assert_eq!(limiter.max_delay_ms, 10000);
    }
}
