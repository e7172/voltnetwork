//! Node daemon for the chainless token transfer network.

pub mod config;
pub mod errors;
pub mod metrics;
pub mod rpc;
pub mod tests;

pub mod main {
    pub use super::handle_update;
    pub use super::handle_mint;
}

use anyhow::Result;
use config::NodeConfig;
use core::{proofs::Proof, smt::SMT, types::Address};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Verifier};
use errors::NodeError;
use network::{
    storage::ProofStore,
    types::{MintMsg, UpdateMsg},
};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Loads a keypair from the filesystem based on the address.
///
/// The keypair is stored in a file at `<data_dir>/keypairs/<hex_address>.key`.
/// The file contains 64 bytes: the first 32 are the Ed25519 secret key seed,
/// and the next 32 are the corresponding public key bytes.
fn keypair_from_address(address: &Address) -> Result<Keypair, NodeError> {
    // Get the data directory from the environment or use a default
    let data_dir = std::env::var("NODE_DATA_DIR")
        .unwrap_or_else(|_| dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string());
    
    // Build the path to the keypair file
    let address_hex = hex::encode(address);
    let keypair_path = std::path::Path::new(&data_dir)
        .join("keypairs")
        .join(format!("{}.key", address_hex));
    
    // Read the keypair file
    let raw = std::fs::read(&keypair_path)
        .map_err(|e| NodeError::InvalidSignature(
            format!("Failed to read keypair file for address {}: {}", address_hex, e)
        ))?;
    
    // Ensure the file contains exactly 64 bytes
    if raw.len() != 64 {
        return Err(NodeError::InvalidSignature(
            format!("Invalid keypair file size for address {}: expected 64 bytes, got {}", 
                address_hex, raw.len())
        ));
    }
    
    // Extract the secret key (first 32 bytes) and public key (next 32 bytes)
    let secret_key = SecretKey::from_bytes(&raw[..32])
        .map_err(|e| NodeError::InvalidSignature(
            format!("Invalid secret key for address {}: {}", address_hex, e)
        ))?;
    
    let public_key = PublicKey::from(&secret_key);
    
    // Verify that the public key matches the stored public key
    if public_key.as_bytes() != &raw[32..64] {
        return Err(NodeError::InvalidSignature(
            format!("Public key mismatch for address {}", address_hex)
        ));
    }
    
    // Verify that the address matches the hash of the public key
    let mut hasher = Sha256::new();
    hasher.update(public_key.as_bytes());
    let result = hasher.finalize();
    
    let mut derived_addr = [0u8; 32];
    derived_addr.copy_from_slice(&result);
    
    if &derived_addr != address {
        return Err(NodeError::InvalidSignature(
            format!("Address mismatch for keypair file {}", address_hex)
        ));
    }
    
    // Return the valid keypair
    Ok(Keypair {
        secret: secret_key,
        public: public_key,
    })
}

/// Handles an update message.
pub async fn handle_update(
    update: UpdateMsg,
    smt: &Arc<Mutex<SMT>>,
    proof_store: &ProofStore,
) -> Result<(), NodeError> {
    debug!("Received update: {}", update);

    // Verify the proofs
    let root = {
        let smt = smt.lock().unwrap();
        smt.root()
    };

    // Verify the sender's proof
    if !update.proof_from.verify(root, &update.from) {
        return Err(NodeError::InvalidProof("sender".to_string()));
    }

    // Verify the recipient's proof
    if !update.proof_to.verify(root, &update.to) {
        return Err(NodeError::InvalidProof("recipient".to_string()));
    }

    // Verify the signature
    // First, get the public key from the sender's address
    // In our implementation, the address is derived from the public key
    let keypair = match keypair_from_address(&update.from) {
        Ok(kp) => kp,
        Err(e) => {
            warn!("Failed to derive public key from address: {}", e);
            return Err(NodeError::InvalidSignature("Invalid public key".to_string()));
        }
    };
    let public_key = keypair.public;

    // Create a signature object from the signature bytes
    let signature = match Signature::from_bytes(&update.signature.0) {
        Ok(sig) => sig,
        Err(e) => {
            warn!("Invalid signature format: {}", e);
            return Err(NodeError::InvalidSignature("Invalid signature format".to_string()));
        }
    };

    // Create a copy of the update message with an empty signature for verification
    let unsigned_update = UpdateMsg {
        from: update.from,
        to: update.to,
        token_id: update.token_id,
        amount: update.amount,
        root: update.root,
        post_root: update.post_root,
        proof_from: update.proof_from.clone(),
        proof_to: update.proof_to.clone(),
        nonce: update.nonce,
        signature: core::types::Signature([0u8; 64]), // Empty signature for verification
    };

    // Serialize the message for verification (same as how it was signed)
    let message = match bincode::serialize(&unsigned_update) {
        Ok(msg) => msg,
        Err(e) => {
            warn!("Failed to serialize message for verification: {}", e);
            return Err(NodeError::InvalidSignature("Serialization error".to_string()));
        }
    };

    // Verify the signature
    if let Err(e) = public_key.verify(&message, &signature) {
        warn!("Signature verification failed: {}", e);
        return Err(NodeError::InvalidSignature("Signature verification failed".to_string()));
    }

    // Update the SMT
    {
        let mut smt = smt.lock().unwrap();
        smt.transfer(&update.from, &update.to, update.amount, update.nonce)?;
    }

    // Store the updated proofs
    let new_root = {
        let smt = smt.lock().unwrap();
        smt.root()
    };

    // Generate and store new proofs
    {
        let smt = smt.lock().unwrap();

        // Generate and store proof for sender
        let sender_proof = smt.gen_proof(&update.from)?;
        proof_store.put_proof(&update.from, &new_root, &sender_proof)?;

        // Generate and store proof for recipient
        let recipient_proof = smt.gen_proof(&update.to)?;
        proof_store.put_proof(&update.to, &new_root, &recipient_proof)?;
    }

    info!(
        "Processed transfer from {:?} to {:?} of {} tokens",
        update.from, update.to, update.amount
    );

    Ok(())
}

/// Handles a mint message.
pub async fn handle_mint(
    mint: MintMsg,
    smt: &Arc<Mutex<SMT>>,
    proof_store: &ProofStore,
    treasury_address: &Address,
    max_supply: u128,
    current_supply: &mut u128,
) -> Result<(), NodeError> {
    debug!("Received mint: {}", mint);

    // Check if the sender is the treasury
    if &mint.from != treasury_address {
        return Err(NodeError::Unauthorized(format!(
            "Only the treasury ({:?}) can mint tokens, got {:?}",
            treasury_address, mint.from
        )));
    }

    // Verify the proofs
    let root = {
        let smt = smt.lock().unwrap();
        smt.root()
    };

    // Verify the treasury's proof
    if !mint.proof_from.verify(root, &mint.from) {
        return Err(NodeError::InvalidProof("treasury".to_string()));
    }

    // Verify the recipient's proof
    if !mint.proof_to.verify(root, &mint.to) {
        return Err(NodeError::InvalidProof("recipient".to_string()));
    }

    // Verify the signature
    // First, get the public key from the treasury's address
    let keypair = match keypair_from_address(&mint.from) {
        Ok(kp) => kp,
        Err(e) => {
            warn!("Failed to derive public key from treasury address: {}", e);
            return Err(NodeError::InvalidSignature("Invalid treasury key".to_string()));
        }
    };
    let public_key = keypair.public;

    // Create a signature object from the signature bytes
    let signature = match Signature::from_bytes(&mint.signature.0) {
        Ok(sig) => sig,
        Err(e) => {
            warn!("Invalid signature format: {}", e);
            return Err(NodeError::InvalidSignature("Invalid signature format".to_string()));
        }
    };

    // Create a copy of the mint message with an empty signature for verification
    let unsigned_mint = MintMsg {
        from: mint.from,
        to: mint.to,
        token_id: mint.token_id,
        amount: mint.amount,
        root: mint.root,
        proof_from: mint.proof_from.clone(),
        proof_to: mint.proof_to.clone(),
        nonce: mint.nonce,
        signature: core::types::Signature([0u8; 64]), // Empty signature for verification
    };

    // Serialize the message for verification (same as how it was signed)
    let message = match bincode::serialize(&unsigned_mint) {
        Ok(msg) => msg,
        Err(e) => {
            warn!("Failed to serialize message for verification: {}", e);
            return Err(NodeError::InvalidSignature("Serialization error".to_string()));
        }
    };

    // Verify the signature
    if let Err(e) = public_key.verify(&message, &signature) {
        warn!("Signature verification failed: {}", e);
        return Err(NodeError::InvalidSignature("Signature verification failed".to_string()));
    }

    // Update the SMT
    {
        let mut smt = smt.lock().unwrap();
        let new_supply = smt.mint(&mint.from, &mint.to, mint.amount, mint.nonce, max_supply, *current_supply)?;
        *current_supply = new_supply;
    }

    // Store the updated proofs
    let new_root = {
        let smt = smt.lock().unwrap();
        smt.root()
    };

    // Generate and store new proofs
    {
        let smt = smt.lock().unwrap();

        // Generate and store proof for treasury
        let treasury_proof = smt.gen_proof(&mint.from)?;
        proof_store.put_proof(&mint.from, &new_root, &treasury_proof)?;

        // Generate and store proof for recipient
        let recipient_proof = smt.gen_proof(&mint.to)?;
        proof_store.put_proof(&mint.to, &new_root, &recipient_proof)?;
    }

    info!(
        "Processed mint from treasury {:?} to {:?} of {} tokens. New supply: {}",
        mint.from, mint.to, mint.amount, *current_supply
    );

    Ok(())
}