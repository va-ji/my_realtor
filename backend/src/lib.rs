// Library module for testable functions

pub mod ingestion;

/// Calculate rental yield percentage
/// Formula: (weekly_rent × 52 / price) × 100
pub fn calculate_rental_yield(price: i32, weekly_rent: i32) -> Option<f32> {
    if price <= 0 {
        return None;
    }
    Some((weekly_rent as f32 * 52.0 / price as f32) * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rental_yield_calculation() {
        // Test normal case
        let yield_val = calculate_rental_yield(650000, 550);
        assert!(yield_val.is_some());
        let yield_val = yield_val.unwrap();
        assert!((yield_val - 4.4).abs() < 0.01);
    }

    #[test]
    fn test_rental_yield_different_values() {
        // Test with different property values
        let yield_val = calculate_rental_yield(480000, 420);
        assert!(yield_val.is_some());
        let yield_val = yield_val.unwrap();
        assert!((yield_val - 4.55).abs() < 0.01);
    }

    #[test]
    fn test_rental_yield_zero_price() {
        // Test with zero price (should return None)
        let yield_val = calculate_rental_yield(0, 500);
        assert!(yield_val.is_none());
    }

    #[test]
    fn test_rental_yield_negative_price() {
        // Test with negative price (should return None)
        let yield_val = calculate_rental_yield(-100000, 500);
        assert!(yield_val.is_none());
    }

    #[test]
    fn test_rental_yield_high_yield() {
        // Test high yield property (10%)
        let yield_val = calculate_rental_yield(260000, 500);
        assert!(yield_val.is_some());
        let yield_val = yield_val.unwrap();
        assert!((yield_val - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_rental_yield_low_yield() {
        // Test low yield property (2%)
        let yield_val = calculate_rental_yield(1300000, 500);
        assert!(yield_val.is_some());
        let yield_val = yield_val.unwrap();
        assert!((yield_val - 2.0).abs() < 0.01);
    }
}