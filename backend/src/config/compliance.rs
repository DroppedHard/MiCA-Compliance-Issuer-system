//! Fixed inputs for the demonstrational Embedded Compliance calculations.
//!
//! IMPORTANT: the EUR/USD rate below is an intentionally strong research
//! simplification. It is not a market rate, an ECB reference rate, financial
//! advice, or a production-grade FX methodology. The real currencies are not
//! assumed to trade at parity. The demo uses 1:1 only to keep the MiCA Article
//! 23 threshold calculation deterministic and focused on the state-machine
//! behavior. A production system must replace this module with a dated,
//! auditable exchange-rate provider and an explicitly defined rounding policy.

/// Identifies the exact simplified methodology used in stored calculations.
pub const EUR_USD_RATE_VERSION: &str = "eur-usd-fixed-parity-demo-v1";

/// USD minor units per the corresponding number of EUR minor units.
///
/// Keeping the rate as an integer ratio avoids floating-point monetary math.
/// `1 / 1` means that USD 1.00 is treated as EUR 1.00 for this demo only.
pub const USD_MINOR_UNITS: u128 = 1;
pub const EUR_MINOR_UNITS: u128 = 1;

/// Converts a USD-denominated transaction value into the EUR reporting value
/// used by the demonstrational MiCA threshold calculation.
///
/// With the intentionally simplified 1:1 rate, the numeric value is unchanged.
pub fn usd_minor_to_eur_minor(value_usd_minor: u128) -> Option<u128> {
    value_usd_minor
        .checked_mul(EUR_MINOR_UNITS)
        .map(|value| value / USD_MINOR_UNITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_the_explicit_demo_only_parity_rate() {
        assert_eq!(usd_minor_to_eur_minor(20_000_000_001), Some(20_000_000_001));
        assert_eq!(USD_MINOR_UNITS, EUR_MINOR_UNITS);
        assert!(EUR_USD_RATE_VERSION.contains("fixed-parity-demo"));
    }
}
