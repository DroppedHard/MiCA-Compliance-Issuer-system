pub mod api;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod mock_bank;
pub mod services;

// Compatibility alias retained while downstream binaries migrate to the
// explicit `services` module.
pub use services as application;
