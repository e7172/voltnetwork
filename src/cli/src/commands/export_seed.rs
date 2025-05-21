//! Export seed command for the CLI wallet.

use crate::errors::WalletError;
use crate::wallet::Wallet;
use anyhow::Result;
use std::path::Path;
use tracing::{debug, info};

/// Runs the export-seed command.
pub async fn run<P: AsRef<Path>>(wallet_path: P) -> Result<String, WalletError> {
    // Check if the wallet file exists
    if !wallet_path.as_ref().exists() {
        return Err(WalletError::WalletError(
            "Wallet file does not exist. Use init-seed to create a new wallet.".to_string(),
        ));
    }

    // Load the wallet
    let wallet = Wallet::load(&wallet_path)?;
    debug!("Loaded wallet from {}", wallet_path.as_ref().display());

    // Get the mnemonic
    let mnemonic = wallet.mnemonic().to_string();
    info!("Retrieved mnemonic from wallet");

    Ok(mnemonic)
}
