//! Core primitives for the chainless token transfer network.
//!
//! This crate provides the fundamental types and operations for a stateless token
//! transfer network, including Sparse Merkle Trees, cryptographic proofs, and
//! account management.

pub mod errors;
pub mod proofs;
pub mod smt;
pub mod types;

// Re-export commonly used types
pub use errors::CoreError;
pub use proofs::Proof;
pub use smt::SMT;
pub use types::{AccountLeaf, Address, Balance, Nonce};
