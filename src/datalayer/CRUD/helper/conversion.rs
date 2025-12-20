use crate::datalayer::CRUD::types::Currency;
use crate::errors::errors::ServiceError;

impl Currency {
    /// Returns how much 1 unit of this currency is worth in USD
    pub fn usd_rate(self) -> f64 {
        match self {
            Currency::USD => 1.0,

            // Europe
            Currency::EUR => 1.08,
            Currency::GBP => 1.27,
            Currency::CHF => 1.12,

            // Middle East
            Currency::AED => 0.27, // 1 AED ≈ 0.272 USD
            Currency::KWD => 3.25, // 1 KWD ≈ 3.25 USD

            // Asia
            Currency::INR => 0.012,
            Currency::CNY => 0.14,
            Currency::KRW => 0.00077,
            Currency::JPY => 0.0067,

            // Americas
            Currency::CAD => 0.74,
            Currency::BRL => 0.20,
            Currency::ARS => 0.0011,

            // Oceania
            Currency::AUD => 0.66,
        }
    }
}

pub fn map_currency(currency: String) -> Result<Currency, ServiceError> {
    match currency.to_lowercase().as_str() {
        "usd" => Ok(Currency::USD),
        "eur" => Ok(Currency::EUR),
        "gbp" => Ok(Currency::GBP),
        "chf" => Ok(Currency::CHF),
        "aed" => Ok(Currency::AED),
        "kwd" => Ok(Currency::KWD),
        "inr" => Ok(Currency::INR),
        "cny" => Ok(Currency::CNY),
        "krw" => Ok(Currency::KRW),
        "jpy" => Ok(Currency::JPY),
        "cad" => Ok(Currency::CAD),
        "brl" => Ok(Currency::BRL),
        "ars" => Ok(Currency::ARS),
        "aud" => Ok(Currency::AUD),
        _ => Err(ServiceError::InvalidCurrency),
    }
}

pub fn to_usd(amount: f64, currency: Currency) -> Result<f64, ServiceError> {
    // if currency not exist in the enum, return error
    if currency.usd_rate() == 0.0 {
        return Err(ServiceError::InvalidCurrency);
    }

    Ok(amount * currency.usd_rate())
}

pub fn from_usd(amount: f64, currency: Currency) -> Result<f64, ServiceError> {
    // if currency not exist in the enum, return error
    if currency.usd_rate() == 0.0 {
        return Err(ServiceError::InvalidCurrency);
    }

    Ok(amount / currency.usd_rate())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usd_to_usd_conversion() {
        println!("\n=== TEST: USD to USD Conversion ===");
        let amount = 100.0;
        let currency = Currency::USD;

        println!("Converting {} {} to USD", amount, "USD");
        let result = to_usd(amount, currency).unwrap();
        let return_result = from_usd(result, currency).unwrap();
        println!("Result: ${:.2}", result);
        println!("Return Result: ${:.2}", return_result);
        assert_eq!(result, 100.0);
        assert_eq!(return_result, 100.0);
        println!("✅ Test passed: USD to USD conversion is 1:1");
    }

    #[test]
    fn test_eur_to_usd_conversion() {
        println!("\n=== TEST: EUR to USD Conversion ===");
        let amount = 100.0;
        let currency = Currency::EUR;

        println!("Converting {} EUR to USD", amount);
        println!("Exchange rate: 1 EUR = {} USD", currency.usd_rate());

        let result = to_usd(amount, currency).unwrap();
        let return_result = from_usd(result, currency).unwrap();
        println!("Result: ${:.2}", result);
        println!("Return Result: ${:.2}", return_result);
        assert_eq!(result, 108.0);
        assert_eq!(return_result, 100.0);
        println!("✅ Test passed: 100 EUR = $108.00");
    }

    #[test]
    fn test_inr_to_usd_conversion() {
        println!("\n=== TEST: INR to USD Conversion ===");
        let amount = 1000.0;
        let currency = Currency::INR;

        println!("Converting {} INR to USD", amount);
        println!("Exchange rate: 1 INR = {} USD", currency.usd_rate());

        let result = to_usd(amount, currency).unwrap();

        println!("Result: ${:.2}", result);
        assert_eq!(result, 12.0);
        println!("✅ Test passed: 1000 INR = $12.00");
    }

    #[test]
    fn test_gbp_to_usd_conversion() {
        println!("\n=== TEST: GBP to USD Conversion ===");
        let amount = 50.0;
        let currency = Currency::GBP;

        println!("Converting {} GBP to USD", amount);
        println!("Exchange rate: 1 GBP = {} USD", currency.usd_rate());

        let result = to_usd(amount, currency).unwrap();

        println!("Result: ${:.2}", result);
        assert_eq!(result, 63.5);
        println!("✅ Test passed: 50 GBP = $63.50");
    }

    #[test]
    fn test_jpy_to_usd_conversion() {
        println!("\n=== TEST: JPY to USD Conversion ===");
        let amount = 10000.0;
        let currency = Currency::JPY;

        println!("Converting {} JPY to USD", amount);
        println!("Exchange rate: 1 JPY = {} USD", currency.usd_rate());

        let result = to_usd(amount, currency).unwrap();
        let return_result = from_usd(result, currency).unwrap();
        println!("Result: ${:.2}", result);
        println!("Return Result: ${:.2}", return_result);
        assert_eq!(result, 67.0);
        assert_eq!(return_result, 10000.0);
        println!("✅ Test passed: 10000 JPY = $67.00");
    }

    #[test]
    fn test_aed_to_usd_conversion() {
        println!("\n=== TEST: AED to USD Conversion ===");
        let amount = 100.0;
        let currency = Currency::AED;

        println!("Converting {} AED to USD", amount);
        println!("Exchange rate: 1 AED = {} USD", currency.usd_rate());

        let result = to_usd(amount, currency).unwrap();
        let return_result = from_usd(result, currency).unwrap();
        println!("Result: ${:.2}", result);
        println!("Return Result: ${:.2}", return_result);
        assert_eq!(result, 27.0);
        assert_eq!(return_result, 100.0);
        println!("✅ Test passed: 100 AED = $27.00");
    }

    #[test]
    fn test_kwd_to_usd_conversion() {
        println!("\n=== TEST: KWD to USD Conversion (High Value Currency) ===");
        let amount = 10.0;
        let currency = Currency::KWD;

        println!("Converting {} KWD to USD", amount);
        println!("Exchange rate: 1 KWD = {} USD", currency.usd_rate());

        let result = to_usd(amount, currency).unwrap();
        let return_result = from_usd(result, currency).unwrap();
        println!("Result: ${:.2}", result);
        println!("Return Result: ${:.2}", return_result);
        assert_eq!(result, 32.5);
        assert_eq!(return_result, 10.0);
        println!("✅ Test passed: 10 KWD = $32.50 (KWD is high-value currency)");
    }

    #[test]
    fn test_zero_amount_conversion() {
        println!("\n=== TEST: Zero Amount Conversion ===");
        let amount = 0.0;
        let currency = Currency::EUR;

        println!("Converting {} EUR to USD", amount);
        let result = to_usd(amount, currency).unwrap();
        let return_result = from_usd(result, currency).unwrap();
        println!("Result: ${:.2}", result);
        println!("Return Result: ${:.2}", return_result);
        assert_eq!(result, 0.0);
        assert_eq!(return_result, 0.0);
        println!("✅ Test passed: 0 EUR = $0.00");
    }

    #[test]
    fn test_large_amount_conversion() {
        println!("\n=== TEST: Large Amount Conversion ===");
        let amount = 1_000_000.0;
        let currency = Currency::USD;

        println!("Converting ${:.2} USD to USD", amount);
        let result = to_usd(amount, currency).unwrap();
        let return_result = from_usd(result, currency).unwrap();
        println!("Result: ${:.2}", result);
        println!("Return Result: ${:.2}", return_result);
        assert_eq!(result, 1_000_000.0);
        assert_eq!(return_result, 1_000_000.0);
        println!("✅ Test passed: Large amount conversion works correctly");
    }

    #[test]
    fn test_all_currencies_have_valid_rates() {
        println!("\n=== TEST: All Currencies Have Valid Rates ===");

        let currencies = vec![
            (Currency::USD, "USD"),
            (Currency::EUR, "EUR"),
            (Currency::GBP, "GBP"),
            (Currency::CHF, "CHF"),
            (Currency::AED, "AED"),
            (Currency::KWD, "KWD"),
            (Currency::INR, "INR"),
            (Currency::CNY, "CNY"),
            (Currency::KRW, "KRW"),
            (Currency::JPY, "JPY"),
            (Currency::CAD, "CAD"),
            (Currency::BRL, "BRL"),
            (Currency::ARS, "ARS"),
            (Currency::AUD, "AUD"),
        ];

        println!("Checking all {} currencies...", currencies.len());
        for (currency, name) in currencies {
            let rate = currency.usd_rate();
            println!("  {} rate: {:.6} USD", name, rate);
            assert!(rate > 0.0, "{} should have a positive rate", name);

            // Test conversion
            let result = to_usd(100.0, currency);
            assert!(result.is_ok(), "{} conversion should succeed", name);
        }
        println!("✅ Test passed: All currencies have valid positive rates");
    }

    #[test]
    fn test_decimal_precision() {
        println!("\n=== TEST: Decimal Precision ===");
        let amount = 123.45;
        let currency = Currency::EUR;

        println!("Converting {:.2} EUR to USD", amount);
        println!("Exchange rate: 1 EUR = {} USD", currency.usd_rate());

        let result = to_usd(amount, currency).unwrap();
        let expected = 123.45 * 1.08;

        println!("Result: ${:.2}", result);
        println!("Expected: ${:.2}", expected);
        assert!(
            (result - expected).abs() < 0.01,
            "Precision should be maintained"
        );
        println!("✅ Test passed: Decimal precision maintained");
    }
}
