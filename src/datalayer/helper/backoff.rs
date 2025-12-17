use rand::Rng;

/// Exponential backoff with jitter calculator
pub struct ExponentialBackoff{
    base_delay_ms: u64,
    max_delay_ms: u64,
}

static mut backoff: ExponentialBackoff = ExponentialBackoff::new();

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {base_delay_ms: 10,max_delay_ms: 300,}
    }
}

impl ExponentialBackoff {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_base_delay_ms(&mut self, base_delay_ms: u64) {
        self.base_delay_ms = base_delay_ms;
    }

    pub fn set_max_delay_ms(&mut self, max_delay_ms: u64) {
        self.max_delay_ms = max_delay_ms;
    }

    /// Calculates the backoff time in milliseconds using exponential backoff with full jitter.
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current retry attempt number (0-indexed)
    /// * `base_delay_ms` - The base delay in milliseconds (default: 100ms recommended)
    /// * `max_delay_ms` - The maximum delay in milliseconds (default: 30000ms recommended)
    ///
    /// # Returns
    ///
    /// The backoff time in milliseconds as a u64
    ///
    /// # Algorithm
    ///
    /// This uses the "Full Jitter" strategy from AWS Architecture Blog:
    /// `sleep = random_between(0, min(max_delay, base_delay * 2^attempt))`
    ///
    /// Benefits:
    /// - Prevents thundering herd problem
    /// - Spreads out retry attempts
    /// - Bounded by max_delay to prevent excessive waits
    ///
    /// # Example
    ///
    /// ```
    /// use datalayer::helper::backoff::ExponentialBackoff;
    ///
    /// let backoff_ms = ExponentialBackoff::calculate(0, 100, 30000);
    /// // First attempt: returns random value between 0 and 100ms
    ///
    /// let backoff_ms = ExponentialBackoff::calculate(3, 100, 30000);
    /// // Fourth attempt: returns random value between 0 and min(800, 30000)ms
    /// ```
    pub fn calculate(attempt: u32, base_delay_ms: u64, max_delay_ms: u64) -> u64 {
        // Calculate exponential delay: base_delay * 2^attempt
        // Use saturating operations to prevent overflow
        let exponential_delay = base_delay_ms.saturating_mul(2u64.saturating_pow(attempt));

        // Cap at max_delay
        let capped_delay = exponential_delay.min(max_delay_ms);

        // Apply full jitter: random value between 0 and capped_delay
        if capped_delay == 0 {
            return 0;
        }

        let mut rng = rand::thread_rng();
        rng.gen_range(0..=capped_delay)
    }

    /// Calculates backoff with default parameters (100ms base, 30s max)
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current retry attempt number (0-indexed)
    ///
    /// # Returns
    ///
    /// The backoff time in milliseconds as a u64
    ///
    /// # Example
    ///
    /// ```
    /// use datalayer::helper::backoff::ExponentialBackoff;
    ///
    /// let backoff_ms = ExponentialBackoff::calculate_default(2);
    /// // Returns random value between 0 and 400ms
    /// ```
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_first_attempt() {
        let backoff = ExponentialBackoff::calculate(0, 100, 30000);
        // First attempt should be between 0 and 100ms
        assert!(backoff <= 100);
    }

    #[test]
    fn test_backoff_increases_exponentially() {
        let backoff1 = ExponentialBackoff::calculate(1, 100, 30000);
        let backoff2 = ExponentialBackoff::calculate(2, 100, 30000);

        // Second attempt max should be 200ms, third should be 400ms
        assert!(backoff1 <= 200);
        assert!(backoff2 <= 400);
    }

    #[test]
    fn test_backoff_respects_max_delay() {
        let backoff = ExponentialBackoff::calculate(20, 100, 5000);
        // Even with high attempt number, should not exceed max_delay
        assert!(backoff <= 5000);
    }

    #[test]
    fn test_backoff_zero_base_delay() {
        let backoff = ExponentialBackoff::calculate(5, 0, 30000);
        assert_eq!(backoff, 0);
    }

    #[test]
    fn test_backoff_default() {
        let backoff = ExponentialBackoff::calculate_default(3);
        // Fourth attempt with defaults: should be between 0 and 800ms
        assert!(backoff <= 800);
    }

    #[test]
    fn test_backoff_overflow_protection() {
        // Test with very high attempt number to ensure no overflow
        let backoff = ExponentialBackoff::calculate(100, 1000, 60000);
        assert!(backoff <= 60000);
    }

    #[test]
    fn test_backoff_jitter_variance() {
        // Run multiple times to ensure we get different values (jitter working)
        let mut values = std::collections::HashSet::new();
        for _ in 0..10 {
            values.insert(ExponentialBackoff::calculate(3, 100, 30000));
        }
        // Should have at least some variance (not all the same value)
        // Note: There's a tiny chance this could fail due to randomness
        assert!(values.len() > 1, "Jitter should produce varied results");
    }
}
