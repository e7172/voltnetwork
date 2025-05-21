//! Wallet implementation for the CLI.

use crate::errors::WalletError;
use bip32::{Mnemonic, XPrv};
use core::types::Address;
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

/// A wallet for the chainless token transfer network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// The BIP39 mnemonic for the wallet
    mnemonic: String,
    /// The current account index
    account_index: u32,
}

impl Wallet {
    /// Creates a new wallet with a random mnemonic.
    pub fn new() -> Result<Self, WalletError> {
        let mnemonic = Mnemonic::random(OsRng, Default::default());
        
        Ok(Self {
            mnemonic: mnemonic.phrase().to_string(),
            account_index: 0,
        })
    }

    /// Loads a wallet from a file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, WalletError> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        let wallet = serde_json::from_str(&contents)?;
        Ok(wallet)
    }

    /// Saves a wallet to a file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), WalletError> {
        let contents = serde_json::to_string_pretty(self)?;
        
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut file = File::create(path)?;
        file.write_all(contents.as_bytes())?;
        
        Ok(())
    }

    /// Gets the mnemonic for the wallet.
    pub fn mnemonic(&self) -> &str {
        &self.mnemonic
    }

    /// Gets the current account index.
    pub fn account_index(&self) -> u32 {
        self.account_index
    }

    /// Sets the account index.
    pub fn set_account_index(&mut self, index: u32) {
        self.account_index = index;
    }

    /// Gets the keypair for the current account.
    pub fn keypair(&self) -> Result<Keypair, WalletError> {
        // Parse the mnemonic
        let mnemonic = Mnemonic::new(self.mnemonic.as_str(), Default::default())?;
        
        // Derive the seed
        let seed = mnemonic.to_seed("");
        
        // Derive the private key using BIP32
        let root = XPrv::derive_from_path(seed, &format!("m/44'/0'/{}'", self.account_index).parse()?)?;
        
        // Convert to ed25519 keypair
        let secret = root.to_bytes();
        let mut hasher = Sha256::new();
        hasher.update(&secret);
        let result = hasher.finalize();
        
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&result);
        
        let secret_key = SecretKey::from_bytes(&seed)?;
        let public_key = PublicKey::from(&secret_key);
        
        Ok(Keypair {
            secret: secret_key,
            public: public_key,
        })
    }

    /// Gets the address for the current account.
    pub fn address(&self) -> Result<Address, WalletError> {
        let keypair = self.keypair()?;
        let public_key = keypair.public.to_bytes();
        
        // Use the public key directly as the address
        // This ensures compatibility with the node's signature verification
        let mut address = [0u8; 32];
        if public_key.len() >= 32 {
            address.copy_from_slice(&public_key[0..32]);
        } else {
            // If the public key is shorter than 32 bytes (unlikely), pad with zeros
            address[..public_key.len()].copy_from_slice(&public_key);
        }
        
        Ok(address)
    }

    /// Signs a message with the current account's private key.
    pub fn sign(&self, message: &[u8]) -> Result<Signature, WalletError> {
        let keypair = self.keypair()?;
        let signature = keypair.sign(message);
        Ok(signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wallet_creation() {
        let wallet = Wallet::new().unwrap();
        assert!(!wallet.mnemonic().is_empty());
        assert_eq!(wallet.account_index(), 0);
    }

    #[test]
    fn test_wallet_save_load() {
        let dir = tempdir().unwrap();
        let wallet_path = dir.path().join("wallet.dat");
        
        let wallet = Wallet::new().unwrap();
        wallet.save(&wallet_path).unwrap();
        
        let loaded = Wallet::load(&wallet_path).unwrap();
        assert_eq!(wallet.mnemonic(), loaded.mnemonic());
        assert_eq!(wallet.account_index(), loaded.account_index());
    }

    #[test]
    fn test_wallet_address() {
        let wallet = Wallet::new().unwrap();
        let address = wallet.address().unwrap();
        
        // Address should be 32 bytes
        assert_eq!(address.len(), 32);
        
        // Same wallet should produce same address
        let address2 = wallet.address().unwrap();
        assert_eq!(address, address2);
    }

    #[test]
    fn test_wallet_signing() {
        let wallet = Wallet::new().unwrap();
        let message = b"Hello, world!";
        
        let signature = wallet.sign(message).unwrap();
        
        // Verify the signature
        let keypair = wallet.keypair().unwrap();
        keypair.verify(message, &signature).unwrap();
    }
}
