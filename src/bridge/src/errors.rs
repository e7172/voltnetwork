/// Error types for the bridge crate.
use std::fmt;
use std::error::Error as StdError;

/// Errors that can occur in the bridge crate.
#[derive(Debug)]
pub enum BridgeError {
    /// Error when an Ethereum operation fails.
    EthereumError(String),

    /// Error when a contract operation fails.
    ContractError(String),

    /// Error when a proof operation fails.
    ProofError(String),

    /// Error when a transaction operation fails.
    TransactionError(String),

    /// Error when a signature operation fails.
    SignatureError(String),

    /// Error when an address is invalid.
    InvalidAddress(String),

    /// Error when an amount is invalid.
    InvalidAmount(String),

    /// Error when a proof is invalid.
    InvalidProof(String),

    /// Error when a root is invalid.
    InvalidRoot(String),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BridgeError::EthereumError(msg) => write!(f, "Ethereum error: {}", msg),
            BridgeError::ContractError(msg) => write!(f, "Contract error: {}", msg),
            BridgeError::ProofError(msg) => write!(f, "Proof error: {}", msg),
            BridgeError::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            BridgeError::SignatureError(msg) => write!(f, "Signature error: {}", msg),
            BridgeError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            BridgeError::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
            BridgeError::InvalidProof(msg) => write!(f, "Invalid proof: {}", msg),
            BridgeError::InvalidRoot(msg) => write!(f, "Invalid root: {}", msg),
        }
    }
}

impl StdError for BridgeError {}
