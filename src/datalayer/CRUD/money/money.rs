use crate::datalayer::db_ops::constants::DENOMINATOR;
use crate::errors::errors::ServiceError;
use crate::datalayer::CRUD::helper::conversion::{map_currency, to_usd, from_usd};

/// Money Handling Utilities
///
/// This module provides utilities for handling money with precision.
///
/// ## Money Representation
///
/// All money values are stored as `i64` integers representing the smallest unit
/// with 4 decimal places of precision using the DENOMINATOR constant (10000).
///
/// ### Examples:
/// - $10.00 = 100000 storage units (10.00 * 10000)
/// - $10.50 = 105000 storage units (10.50 * 10000)
/// - $10.5678 = 105678 storage units (10.5678 * 10000)
/// - $0.0001 = 1 storage unit (0.0001 * 10000)
///
/// ### Why integers?
/// Floating-point arithmetic can introduce precision errors. By storing money
/// as integers, we ensure exact calculations without rounding errors.

/// Convert dollars (with up to 4 decimal places) to storage units
///
/// # Arguments
/// * `dollars` - Amount in dollars (e.g., 10.50)
///
/// # Returns
/// * Storage units as i64 (e.g., 105000)
///
/// # Examples
/// ```
/// let storage_units = to_storage_units(10.50);
/// assert_eq!(storage_units, 105000);
/// ```
pub fn to_storage_units(dollars: f64) -> i64 {
    (dollars * DENOMINATOR as f64).round() as i64
}

// Convert to USD using the helper module then convert to storage units
pub fn to_storage_units_with_conversion(amount: f64, currency: String) -> i64 {
    // Match the currency str with Currency
    let curr_temp = map_currency(currency).unwrap();
    let usd = to_usd(amount, curr_temp).unwrap();
    to_storage_units(usd)
}
/// Convert storage units to dollars
///
/// # Arguments
/// * `units` - Amount in storage units (e.g., 105000)
///
/// # Returns
/// * Amount in dollars as f64 (e.g., 10.50)
///
/// # Examples
/// ```
/// let dollars = from_storage_units(105000);
/// assert_eq!(dollars, 10.50);
/// ```
pub fn from_storage_units(units: i64) -> f64 {
    units as f64 / DENOMINATOR as f64
}

pub fn from_storage_units_with_conversion(units: i64, currency: String) -> f64 {
    let curr_temp = map_currency(currency).unwrap();
    let usd = from_storage_units(units);
    from_usd(usd, curr_temp).unwrap()
}
    

/// Validate that an amount is positive and within acceptable bounds
///
/// # Arguments
/// * `amount` - Amount in storage units to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(ServiceError)` if invalid
///
/// # Validation Rules
/// - Amount must be positive (> 0)
/// - Amount must not exceed i64::MAX to prevent overflow
pub fn validate_amount(amount: i64) -> Result<(), ServiceError> {
    if amount <= 0 {
        return Err(ServiceError::InvalidTransactionAmount);
    }

    // Additional validation: ensure amount is reasonable
    // This prevents potential overflow issues in calculations
    if amount > i64::MAX / 2 {
        return Err(ServiceError::ValidationError(
            "Amount exceeds maximum allowed value".to_string(),
        ));
    }

    Ok(())
}

/// Validate that a balance is non-negative
///
/// # Arguments
/// * `balance` - Balance in storage units to validate
///
/// # Returns
/// * `Ok(())` if valid
/// * `Err(ServiceError)` if invalid
pub fn validate_balance(balance: i64) -> Result<(), ServiceError> {
    if balance < 0 {
        return Err(ServiceError::ValidationError(
            "Balance cannot be negative".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_storage_units() {
        assert_eq!(to_storage_units(10.0), 100000);
        assert_eq!(to_storage_units(10.50), 105000);
        assert_eq!(to_storage_units(10.5678), 105678);
        assert_eq!(to_storage_units(0.0001), 1);
        assert_eq!(to_storage_units(0.0), 0);
    }

    #[test]
    fn test_from_storage_units() {
        assert_eq!(from_storage_units(100000), 10.0);
        assert_eq!(from_storage_units(105000), 10.50);
        assert_eq!(from_storage_units(105678), 10.5678);
        assert_eq!(from_storage_units(1), 0.0001);
        assert_eq!(from_storage_units(0), 0.0);
    }

    #[test]
    fn test_validate_amount() {
        assert!(validate_amount(1).is_ok());
        assert!(validate_amount(100000).is_ok());
        assert!(validate_amount(0).is_err());
        assert!(validate_amount(-1).is_err());
        assert!(validate_amount(i64::MAX).is_err()); // Too large
    }

    #[test]
    fn test_validate_balance() {
        assert!(validate_balance(0).is_ok());
        assert!(validate_balance(100000).is_ok());
        assert!(validate_balance(-1).is_err());
    }

    #[test]
    fn test_round_trip_conversion() {
        let original = 10.5678;
        let storage = to_storage_units(original);
        let converted_back = from_storage_units(storage);
        assert_eq!(converted_back, original);
    }
}
