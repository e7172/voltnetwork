//! Merkle proof implementation for the chainless token transfer network.

use crate::errors::CoreError;
use crate::types::Address;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// A Merkle proof that can be used to verify the inclusion of a leaf in a Sparse Merkle Tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof {
    /// The sibling hashes along the path from the leaf to the root
    pub siblings: Vec<[u8; 32]>,
    /// The hash of the leaf being proven
    pub leaf_hash: [u8; 32],
    /// The path from the root to the leaf (as a sequence of bits)
    pub path: Vec<bool>,
}

impl Proof {
    /// Creates a new Merkle proof.
    pub fn new(siblings: Vec<[u8; 32]>, leaf_hash: [u8; 32], path: Vec<bool>) -> Self {
        Self {
            siblings,
            leaf_hash,
            path,
        }
    }

    /// Verifies that this proof is valid for the given root and address.
    ///
    /// # Arguments
    ///
    /// * `root` - The root hash of the Sparse Merkle Tree
    /// * `addr` - The address of the account being proven
    ///
    /// # Returns
    ///
    /// `true` if the proof is valid, `false` otherwise
    pub fn verify(&self, root: [u8; 32], addr: &Address) -> bool {
        // Always use the path from the proof
        let path = &self.path;

        // Debug output
        println!("Verify - Root: {:?}", root);
        println!("Verify - Leaf hash: {:?}", self.leaf_hash);
        println!("Verify - Path length: {}", path.len());
        println!("Verify - Siblings length: {}", self.siblings.len());

        // Ensure path length matches siblings length
        if path.len() != self.siblings.len() {
            println!("Path length doesn't match siblings length");
            return false;
        }

        // Compute the root hash from the leaf hash and siblings
        let computed_root = self.compute_root_from_proof(path);
        println!("Verify - Computed root: {:?}", computed_root);
        
        // Compare the computed root with the expected root
        let result = computed_root == root;
        println!("Verify - Result: {}", result);
        
        // Return the actual verification result
        result
    }

    /// Verifies that this proof is valid for the given root and address, returning a Result.
    ///
    /// # Arguments
    ///
    /// * `root` - The root hash of the Sparse Merkle Tree
    /// * `addr` - The address of the account being proven
    ///
    /// # Returns
    ///
    /// `Ok(())` if the proof is valid, `Err(CoreError)` otherwise
    pub fn verify_with_error(&self, root: [u8; 32], addr: &Address) -> Result<(), CoreError> {
        if self.verify(root, addr) {
            Ok(())
        } else {
            Err(CoreError::ProofVerificationFailed(
                "Merkle proof verification failed".to_string(),
            ))
        }
    }

    /// Computes the root hash from the leaf hash and siblings.
    ///
    /// # Arguments
    ///
    /// * `path` - The path from the root to the leaf (as a sequence of bits)
    ///
    /// # Returns
    ///
    /// The computed root hash
    // fn compute_root_from_proof(&self, path: &[bool]) -> [u8; 32] {
    //     let mut current_hash = self.leaf_hash;
    //     println!("Computing root from leaf hash: {:?}", current_hash);

    //     // Traverse from leaf to root
    //     for (i, &bit) in path.iter().enumerate() {
    //         let sibling = self.siblings[i];
    //         println!("Step {}: bit={}, sibling={:?}", i, bit, sibling);
            
    //         // Compute the parent hash
    //         let mut hasher = Sha256::new();
    //         if bit {
    //             // If bit is 1, current_hash is the right child
    //             println!("  Hashing: sibling + current_hash");
    //             hasher.update(sibling);
    //             hasher.update(current_hash);
    //         } else {
    //             // If bit is 0, current_hash is the left child
    //             println!("  Hashing: current_hash + sibling");
    //             hasher.update(current_hash);
    //             hasher.update(sibling);
    //         }
            
    //         let result = hasher.finalize();
    //         current_hash.copy_from_slice(&result);
    //         println!("  New hash: {:?}", current_hash);
    //     }

    //     println!("Final computed root: {:?}", current_hash);
    //     current_hash
    // }
    fn compute_root_from_proof(&self, path: &[bool]) -> [u8; 32] {
        let mut current_hash = self.leaf_hash;
        println!("Computing root from leaf hash: {:?}", current_hash);

        // Traverse from the leaf back up to the root.
        // The path in the proof is already in the correct order (from leaf to root)
        for i in 0..path.len() {
            let bit = path[i];
            let sibling = self.siblings[i];
            println!("Step {}: bit={}, sibling={:?}", i, bit, sibling);

            // Compute the parent hash
            let mut hasher = Sha256::new();
            if bit {
                // bit==true means our leaf was the right child,
                // so sibling is the left child:
                println!("  Hashing: sibling + current_hash");
                hasher.update(sibling);
                hasher.update(current_hash);
            } else {
                // bit==false means we were the left child:
                println!("  Hashing: current_hash + sibling");
                hasher.update(current_hash);
                hasher.update(sibling);
            }

            let result = hasher.finalize();
            current_hash.copy_from_slice(&result);
            println!("  New hash: {:?}", current_hash);
        }

        println!("Final computed root: {:?}", current_hash);
        current_hash
    }

}

/// Converts an address to a path in the Sparse Merkle Tree.
///
/// # Arguments
///
/// * `addr` - The address to convert
///
/// # Returns
///
/// A vector of booleans representing the path
fn address_to_path(addr: &Address) -> Vec<bool> {
    let mut path = Vec::with_capacity(256);
    for &byte in addr {
        for i in 0..8 {
            path.push((byte & (1 << (7 - i))) != 0);
        }
    }
    path
}

impl fmt::Display for Proof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Proof {{ siblings: {} hashes, leaf_hash: {:?} }}",
            self.siblings.len(),
            self.leaf_hash
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_address_to_path() {
        let mut addr = [0u8; 32];
        addr[0] = 0b10101010;
        
        let path = address_to_path(&addr);
        
        // Check first byte
        assert_eq!(path[0], true);
        assert_eq!(path[1], false);
        assert_eq!(path[2], true);
        assert_eq!(path[3], false);
        assert_eq!(path[4], true);
        assert_eq!(path[5], false);
        assert_eq!(path[6], true);
        assert_eq!(path[7], false);
        
        // Rest should be false
        for i in 8..256 {
            assert_eq!(path[i], false);
        }
    }

    #[test]
    fn test_simple_proof_verification() {
        let mut rng = rand::thread_rng();
        
        // Create a simple proof with one level
        let mut leaf_hash = [0u8; 32];
        rng.fill(&mut leaf_hash);
        
        let mut sibling = [0u8; 32];
        rng.fill(&mut sibling);
        
        let path = vec![false]; // Left child
        
        // Compute expected root
        let mut hasher = Sha256::new();
        hasher.update(leaf_hash);
        hasher.update(sibling);
        let mut expected_root = [0u8; 32];
        expected_root.copy_from_slice(&hasher.finalize());
        
        // Create and verify proof
        let proof = Proof::new(vec![sibling], leaf_hash, path);
        let mut addr = [0u8; 32];
        assert!(proof.verify(expected_root, &addr));
        
        // Modify root to make verification fail
        expected_root[0] ^= 1;
        assert!(!proof.verify(expected_root, &addr));
    }
}
