//! Configuration for the CLI wallet.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Configuration for the CLI wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// The node to connect to
    pub node: String,
    /// The network to connect to
    pub network: String,
    /// The gas price to use for transactions
    pub gas_price: u64,
    /// The gas limit to use for transactions
    pub gas_limit: u64,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            node: "http://localhost:8545".to_string(),
            network: "mainnet".to_string(),
            gas_price: 1,
            gas_limit: 21000,
        }
    }
}

impl WalletConfig {
    /// Loads configuration from a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        let config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Saves configuration to a file.
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}
