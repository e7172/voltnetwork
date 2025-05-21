//! Initialize seed command for the CLI wallet.

use crate::errors::WalletError;
use crate::wallet::Wallet;
use anyhow::Result;
use std::path::Path;
use tracing::{debug, info};

/// Runs the init-seed command.
pub async fn run<P: AsRef<Path>>(wallet_path: P) -> Result<(), WalletError> {
    // Check if the wallet file already exists
    if wallet_path.as_ref().exists() {
        return Err(WalletError::WalletError(
            "Wallet file already exists. Use export-seed to view the seed.".to_string(),
        ));
    }

    // Create a new wallet
    let wallet = Wallet::new()?;
    debug!("Created new wallet with mnemonic: {}", wallet.mnemonic());

    // Save the wallet
    wallet.save(&wallet_path)?;
    info!("Wallet saved to {}", wallet_path.as_ref().display());

    // Get the address
    let address = wallet.address()?;
    info!("Wallet address: {}", hex::encode(address));

    Ok(())
}
