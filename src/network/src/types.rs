//! Network message types for the chainless token transfer network.

use core::{proofs::Proof, types::Address};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Message for updating the state of the network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateMsg {
    /// The sender's address
    pub from: Address,
    /// The recipient's address
    pub to: Address,
    /// The token ID
    pub token_id: u64,
    /// The amount to transfer
    pub amount: u128,
    /// The current root hash (pre-transaction)
    pub root: [u8; 32],
    /// The expected root hash after the transaction is applied
    pub post_root: [u8; 32],
    /// The proof for the sender's account
    pub proof_from: Proof,
    /// The proof for the recipient's account
    pub proof_to: Proof,
    /// The nonce for this transaction
    pub nonce: u64,
    /// The signature of the sender
    pub signature: core::types::Signature,
}

/// Message for minting new tokens (can only be sent by the treasury).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MintMsg {
    /// The treasury's address (sender)
    pub from: Address,
    /// The recipient's address
    pub to: Address,
    /// The token ID
    pub token_id: u64,
    /// The amount to mint
    pub amount: u128,
    /// The current root hash
    pub root: [u8; 32],
    /// The proof for the treasury's account
    pub proof_from: Proof,
    /// The proof for the recipient's account
    pub proof_to: Proof,
    /// The nonce for this transaction
    pub nonce: u64,
    /// The signature of the treasury
    pub signature: core::types::Signature,
}

impl fmt::Display for MintMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MintMsg {{ from: {:?}, to: {:?}, token_id: {}, amount: {}, nonce: {} }}",
            self.from, self.to, self.token_id, self.amount, self.nonce
        )
    }
}

/// Request for a proof of an account.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofRequest {
    /// The address to get the proof for
    pub address: Address,
    /// The root hash for which to generate the proof
    pub root: [u8; 32],
}

/// Response containing a proof of an account.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofResponse {
    /// The address the proof is for
    pub address: Address,
    /// The root hash for which the proof was generated
    pub root: [u8; 32],
    /// The proof itself
    pub proof: Proof,
}

impl fmt::Display for UpdateMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UpdateMsg {{ from: {:?}, to: {:?}, token_id: {}, amount: {}, nonce: {} }}",
            self.from, self.to, self.token_id, self.amount, self.nonce
        )
    }
}

impl fmt::Display for ProofRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProofRequest {{ address: {:?}, root: {:?} }}",
            self.address, self.root
        )
    }
}

impl fmt::Display for ProofResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProofResponse {{ address: {:?}, root: {:?} }}",
            self.address, self.root
        )
    }
}
