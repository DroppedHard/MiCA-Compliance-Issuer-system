pub mod bank;
pub mod blockchain;
pub mod cache;
pub mod casp;
pub mod sqlite;

// Transitional re-exports preserve the public paths used by existing binaries
// and tests while adapters are grouped by their external boundary.
pub use bank::mock_bank_client;
pub use blockchain::{ethereum, token_issuer};
pub use casp::reporting_http as casp_reporting_http;
pub use sqlite::{
    address_restrictions as address_restriction_sqlite, asset_state as asset_state_sqlite,
    casp_reporting as casp_reporting_sqlite, issuance as issuance_sqlite,
    operation_decisions as operation_decision_sqlite, redemption as redemption_sqlite,
    wind_down as wind_down_sqlite,
};
