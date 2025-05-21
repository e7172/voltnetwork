//! Error types for the CLI wallet.

use std::fmt;
use std::error::Error as StdError;

/// Errors that can occur in the CLI wallet.
#[derive(Debug)]
pub enum WalletError {
    /// Error when a file operation fails.
    FileError(std::io::Error),

    /// Error when JSON serialization or deserialization fails.
    JsonError(serde_json::Error),

    /// Error when a BIP32 operation fails.
    Bip32Error(bip32::Error),

    /// Error when an ed25519 operation fails.
    Ed25519Error(ed25519_dalek::SignatureError),

    /// Error when a network operation fails.
    NetworkError(String),

    /// Error when a wallet operation fails.
    WalletError(String),

    /// Error when a proof operation fails.
    ProofError(String),

    /// Error when a transaction operation fails.
    TransactionError(String),

    /// Error when an address is invalid.
    InvalidAddress(String),

    /// Error when an amount is invalid.
    InvalidAmount(String),

    /// Error when a node is unavailable.
    NodeUnavailable(String),

    /// Error when a request to the node fails.
    NodeRequestFailed(String),

    /// Error when the balance is insufficient for a transaction.
    InsufficientBalance(String),
}

impl fmt::Display for WalletError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalletError::FileError(e) => write!(f, "File error: {}", e),
            WalletError::JsonError(e) => write!(f, "JSON error: {}", e),
            WalletError::Bip32Error(e) => write!(f, "BIP32 error: {}", e),
            WalletError::Ed25519Error(e) => write!(f, "Ed25519 error: {}", e),
            WalletError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            WalletError::WalletError(msg) => write!(f, "Wallet error: {}", msg),
            WalletError::ProofError(msg) => write!(f, "Proof error: {}", msg),
            WalletError::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            WalletError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            WalletError::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
            WalletError::NodeUnavailable(msg) => write!(f, "Node unavailable: {}", msg),
            WalletError::NodeRequestFailed(msg) => write!(f, "Node request failed: {}", msg),
            WalletError::InsufficientBalance(msg) => write!(f, "Insufficient balance: {}", msg),
        }
    }
}

impl StdError for WalletError {}

impl From<std::io::Error> for WalletError {
    fn from(error: std::io::Error) -> Self {
        WalletError::FileError(error)
    }
}

impl From<serde_json::Error> for WalletError {
    fn from(error: serde_json::Error) -> Self {
        WalletError::JsonError(error)
    }
}

impl From<bip32::Error> for WalletError {
    fn from(error: bip32::Error) -> Self {
        WalletError::Bip32Error(error)
    }
}

impl From<ed25519_dalek::SignatureError> for WalletError {
    fn from(error: ed25519_dalek::SignatureError) -> Self {
        WalletError::Ed25519Error(error)
    }
}
