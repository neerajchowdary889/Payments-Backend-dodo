use rand::Rng;

/// Exponential backoff with jitter calculator
pub struct ExponentialBackoff {
    base_delay_ms: u64,
    max_delay_ms: u64,
}

// Commented out - not used, and causes compilation errors
// static mut backoff: ExponentialBackoff = ExponentialBackoff::new();

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            base_delay_ms: 10,
            max_delay_ms: 300,
        }
    }
}

impl ExponentialBackoff {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_base_delay_ms(&mut self, base_delay_ms: u64) -> &mut Self {
        self.base_delay_ms = base_delay_ms;
        self
    }

    pub fn set_max_delay_ms(&mut self, max_delay_ms: u64) -> &mut Self {
        self.max_delay_ms = max_delay_ms;
        self
    }

    /// Calculates the backoff time in milliseconds using exponential backoff with full jitter.
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current retry attempt number (0-indexed)
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
    pub fn calculate(&self, attempt: u32) -> u64 {
        // Calculate exponential delay: base_delay * 2^attempt
        // Use saturating operations to prevent overflow
        let exponential_delay = self
            .base_delay_ms
            .saturating_mul(2u64.saturating_pow(attempt));

        // Cap at max_delay
        let capped_delay = exponential_delay.min(self.max_delay_ms);

        // Apply full jitter: random value between 0 and capped_delay
        if capped_delay == 0 {
            return 0;
        }

        let mut rng = rand::thread_rng();
        rng.gen_range(0..=capped_delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_first_attempt() {
        let backoff = ExponentialBackoff::new().calculate(0);
        // First attempt should be between 0 and 100ms
        assert!(backoff <= 100);
    }

    #[test]
    fn test_backoff_increases_exponentially() {
        let backoff1 = ExponentialBackoff::new().calculate(1);
        let backoff2 = ExponentialBackoff::new().calculate(2);
        println!("backoff1: {}", backoff1);
        println!("backoff2: {}", backoff2);
        // Second attempt max should be 200ms, third should be 400ms
        assert!(backoff1 <= 200);
        assert!(backoff2 <= 400);
    }

    #[test]
    fn test_backoff_respects_max_delay() {
        let backoff = ExponentialBackoff::new().calculate(20);
        println!("backoff: {}", backoff);
        // Even with high attempt number, should not exceed max_delay
        assert!(backoff <= 5000);
    }

    #[test]
    fn test_backoff_zero_base_delay() {
        let backoff = ExponentialBackoff::new().calculate(5);
        println!("backoff: {}", backoff);
        assert_eq!(backoff, 0);
    }

    #[test]
    fn test_backoff_overflow_protection() {
        // Test with very high attempt number to ensure no overflow
        let backoff = ExponentialBackoff::new().calculate(100);
        println!("backoff: {}", backoff);
        assert!(backoff <= 60000);
    }

    #[test]
    fn test_backoff_jitter_variance() {
        // Run multiple times to ensure we get different values (jitter working)
        let mut values = std::collections::HashSet::new();
        let mut binding = ExponentialBackoff::new();
        let backoff = binding
            .set_base_delay_ms(100)
            .set_max_delay_ms(30000);
        for _ in 0..10 {
            values.insert(backoff.calculate(3));
        }
        println!("values: {:?}", values);
        // Should have at least some variance (not all the same value)
        // Note: There's a tiny chance this could fail due to randomness
        assert!(values.len() > 1, "Jitter should produce varied results");
    }
}
