//! Storage implementation for the network layer.

use crate::errors::NetworkError;
use core::{proofs::Proof, types::Address};
use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A key in the proof store, consisting of an address and a root hash.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProofKey {
    /// The address the proof is for
    address: Address,
    /// The root hash for which the proof was generated
    root: [u8; 32],
}

/// A wrapper around RocksDB for storing and retrieving proofs.
#[derive(Clone)]
pub struct ProofStore {
    /// The RocksDB instance
    db: Arc<Mutex<DB>>,
}

impl ProofStore {
    /// Creates a new proof store at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, NetworkError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        let db = DB::open(&opts, path)
            .map_err(|e| NetworkError::StorageError(e.to_string()))?;
        
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
        })
    }

    /// Stores a proof for an address and root hash.
    pub fn put_proof(
        &self,
        address: &Address,
        root: &[u8; 32],
        proof: &Proof,
    ) -> Result<(), NetworkError> {
        let key = ProofKey {
            address: *address,
            root: *root,
        };
        
        let key_bytes = bincode::serialize(&key)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;
        
        let proof_bytes = bincode::serialize(proof)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;
        
        self.db
            .lock()
            .unwrap()
            .put(key_bytes, proof_bytes)
            .map_err(|e| NetworkError::StorageError(e.to_string()))?;
        
        Ok(())
    }

    /// Retrieves a proof for an address and root hash.
    pub fn get_proof(
        &self,
        address: &Address,
        root: &[u8; 32],
    ) -> Result<Proof, NetworkError> {
        let key = ProofKey {
            address: *address,
            root: *root,
        };
        
        let key_bytes = bincode::serialize(&key)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;
        
        let proof_bytes = self
            .db
            .lock()
            .unwrap()
            .get(key_bytes)
            .map_err(|e| NetworkError::StorageError(e.to_string()))?
            .ok_or_else(|| NetworkError::ProofNotFound(*address))?;
        
        let proof = bincode::deserialize(&proof_bytes)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;
        
        Ok(proof)
    }

    /// Checks if a proof exists for an address and root hash.
    pub fn has_proof(&self, address: &Address, root: &[u8; 32]) -> Result<bool, NetworkError> {
        let key = ProofKey {
            address: *address,
            root: *root,
        };
        
        let key_bytes = bincode::serialize(&key)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;
        
        let exists = self
            .db
            .lock()
            .unwrap()
            .get(key_bytes)
            .map_err(|e| NetworkError::StorageError(e.to_string()))?
            .is_some();
        
        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::types::AccountLeaf;
    use rand::Rng;
    use tempfile::tempdir;

    #[test]
    fn test_proof_store() {
        let dir = tempdir().unwrap();
        let store = ProofStore::new(dir.path()).unwrap();
        
        let mut rng = rand::thread_rng();
        let mut address = [0u8; 32];
        rng.fill(&mut address);
        
        let mut root = [0u8; 32];
        rng.fill(&mut root);
        
        // Create a simple proof
        let leaf_hash = AccountLeaf::new_empty(address, 0).hash(); // Use native token (token_id = 0)
        let siblings = vec![[0u8; 32]];
        let path = vec![false];
        let proof = Proof::new(siblings, leaf_hash, path);
        
        // Store the proof
        store.put_proof(&address, &root, &proof).unwrap();
        
        // Check that the proof exists
        assert!(store.has_proof(&address, &root).unwrap());
        
        // Retrieve the proof
        let retrieved = store.get_proof(&address, &root).unwrap();
        assert_eq!(retrieved.leaf_hash, proof.leaf_hash);
        assert_eq!(retrieved.siblings.len(), proof.siblings.len());
        assert_eq!(retrieved.path, proof.path);
    }
}
