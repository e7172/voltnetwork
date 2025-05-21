//! Error types for the core crate.

use thiserror::Error;

/// Errors that can occur in the core crate.
#[derive(Error, Debug)]
pub enum CoreError {
    /// Error when trying to update an account with insufficient balance.
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance {
        /// The required balance
        required: u128,
        /// The available balance
        available: u128,
    },

    /// Error when trying to update an account with an invalid nonce.
    #[error("Invalid nonce: expected {expected}, got {actual}")]
    InvalidNonce {
        /// The expected nonce
        expected: u64,
        /// The actual nonce
        actual: u64,
    },

    /// Error when a proof verification fails.
    #[error("Proof verification failed: {0}")]
    ProofVerificationFailed(String),

    /// Error when a signature verification fails.
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    /// Error when an operation on the Sparse Merkle Tree fails.
    #[error("SMT error: {0}")]
    SMTError(String),

    /// Error when serialization or deserialization fails.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Error when minting would exceed the maximum supply.
    #[error("Minting {mint_amount} tokens would exceed the maximum supply of {max_supply} (current supply: {current_supply})")]
    ExceedsMaxSupply {
        /// The maximum supply
        max_supply: u128,
        /// The current supply
        current_supply: u128,
        /// The amount being minted
        mint_amount: u128,
    },

    /// Error when a token is not found.
    #[error("Token not found: {0}")]
    TokenNotFound(u64),

    /// Error when an operation is unauthorized.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Error when the token supply overflows.
    #[error("Token supply overflow")]
    SupplyOverflow,

    /// Error when trying to burn more tokens than are available.
    #[error("Insufficient supply: required {required}, available {available}")]
    InsufficientSupply {
        /// The required supply
        required: u128,
        /// The available supply
        available: u128,
    },

    /// Error when trying to update an account with an invalid token ID.
    #[error("Invalid token ID: expected {expected}, got {actual}")]
    InvalidTokenId {
        /// The expected token ID
        expected: u64,
        /// The actual token ID
        actual: u64,
    },
}
