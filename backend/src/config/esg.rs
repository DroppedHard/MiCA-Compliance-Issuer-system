use crate::domain::{EsgEstimate, EsgMethodology};

// Demonstration baseline derived from Cambridge Centre for Alternative Finance,
// "Ethereum after the Merge – A Change in Power" (July 2026), using 7.87 GWh,
// 2.37 ktCO2e and an assumed 400,000,000 Ethereum transactions per year.
// These values allocate network-wide impact uniformly per transaction. They are
// estimates for research/demo purposes, not direct measurements of this token.
pub const METHODOLOGY_VERSION: &str = "ccaf-ethereum-pos-2026-demo-v1";
pub const ANNUAL_TRANSACTIONS_ASSUMPTION: u64 = 400_000_000;
pub const LOWER_ENERGY_MILLIWH_PER_TRANSACTION: u64 = 3_150;
pub const BEST_GUESS_ENERGY_MILLIWH_PER_TRANSACTION: u64 = 19_675;
pub const UPPER_ENERGY_MILLIWH_PER_TRANSACTION: u64 = 28_725;
pub const EMISSIONS_MILLIGRAM_CO2E_PER_TRANSACTION: u64 = 5_925;
pub const RENEWABLE_PER_MILLE: u64 = 393;
pub const NUCLEAR_PER_MILLE: u64 = 170;
pub const FOSSIL_PER_MILLE: u64 = 436;
pub const SOURCE_URL: &str = "https://www.jbs.cam.ac.uk/wp-content/uploads/2026/07/ccaf-2026-ethereum-after-the-merge-report.pdf";

pub fn methodology() -> EsgMethodology {
    EsgMethodology {
        version: METHODOLOGY_VERSION,
        annual_transactions_assumption: ANNUAL_TRANSACTIONS_ASSUMPTION,
        lower_energy_wh_per_transaction: 3.15,
        best_guess_energy_wh_per_transaction: 19.675,
        upper_energy_wh_per_transaction: 28.725,
        emissions_g_co2e_per_transaction: 5.925,
        renewable_percent: 39.3,
        nuclear_percent: 17.0,
        fossil_percent: 43.6,
        source_name: "Cambridge Centre for Alternative Finance — Ethereum after the Merge (2026)",
        source_url: SOURCE_URL,
        note: "Zakres 1,26–11,49 GWh/rok i best guess 7,87 GWh/rok pochodzą ze scenariuszy sprzętowych Cambridge dla Ethereum PoS. System dzieli każdy scenariusz przez założone 400 mln transakcji rocznie. To alokacja demonstracyjna, nie pomiar marginalnego zużycia tokenu ani statystyczny przedział ufności.",
    }
}

pub fn estimate(date_utc: String, transaction_count: u64, status: &'static str) -> EsgEstimate {
    let lower = transaction_count.saturating_mul(LOWER_ENERGY_MILLIWH_PER_TRANSACTION);
    let best = transaction_count.saturating_mul(BEST_GUESS_ENERGY_MILLIWH_PER_TRANSACTION);
    let upper = transaction_count.saturating_mul(UPPER_ENERGY_MILLIWH_PER_TRANSACTION);
    EsgEstimate {
        date_utc,
        status,
        transaction_count,
        data_origin: "observed",
        energy_lower_wh: lower as f64 / 1_000.0,
        energy_best_guess_wh: best as f64 / 1_000.0,
        energy_upper_wh: upper as f64 / 1_000.0,
        emissions_g_co2e: transaction_count.saturating_mul(EMISSIONS_MILLIGRAM_CO2E_PER_TRANSACTION)
            as f64
            / 1_000.0,
        renewable_energy_wh: best.saturating_mul(RENEWABLE_PER_MILLE) as f64 / 1_000_000.0,
        nuclear_energy_wh: best.saturating_mul(NUCLEAR_PER_MILLE) as f64 / 1_000_000.0,
        fossil_energy_wh: best.saturating_mul(FOSSIL_PER_MILLE) as f64 / 1_000_000.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn applies_the_documented_per_transaction_baseline() {
        let result = estimate("2026-08-22".to_owned(), 2, "provisional");
        assert_eq!(result.energy_lower_wh, 6.3);
        assert_eq!(result.energy_best_guess_wh, 39.35);
        assert_eq!(result.energy_upper_wh, 57.45);
        assert_eq!(result.emissions_g_co2e, 11.85);
        assert_eq!(result.renewable_energy_wh, 15.46455);
    }
}
