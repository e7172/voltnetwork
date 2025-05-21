//! CLI wallet for the chainless token transfer network.

pub mod commands;
pub mod config;
pub mod errors;
pub mod wallet;

// Re-export commonly used types and functions
pub use commands::{balance, export_seed, init_seed, send};
pub use config::WalletConfig;
pub use errors::WalletError;
pub use wallet::Wallet;