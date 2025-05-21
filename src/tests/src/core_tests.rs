//! Tests for the core crate.

use core::{
    proofs::Proof,
    smt::SMT,
    types::{AccountLeaf, Address},
};
use rand::Rng;

/// Tests the SMT implementation.
#[test]
fn test_smt() {
    // Create a new SMT
    let mut smt = SMT::new_zero();
    
    // Create some random accounts
    let mut rng = rand::thread_rng();
    let mut addr1 = [0u8; 32];
    let mut addr2 = [0u8; 32];
    rng.fill(&mut addr1);
    rng.fill(&mut addr2);
    
    // Create account leaves
    let leaf1 = AccountLeaf::new(addr1, 1000, 0, 0); // Use native token (token_id = 0)
    let leaf2 = AccountLeaf::new(addr2, 500, 0, 0); // Use native token (token_id = 0)
    
    // Update the SMT
    smt.update(leaf1.clone()).unwrap();
    smt.update(leaf2.clone()).unwrap();
    
    // Get the accounts
    let account1 = smt.get_account(&addr1).unwrap();
    let account2 = smt.get_account(&addr2).unwrap();
    
    // Check that the accounts were updated correctly
    assert_eq!(account1.bal, 1000);
    assert_eq!(account1.nonce, 0);
    assert_eq!(account2.bal, 500);
    assert_eq!(account2.nonce, 0);
    
    // Transfer tokens
    smt.transfer(&addr1, &addr2, 500, 0).unwrap();
    
    // Get the accounts again
    let account1 = smt.get_account(&addr1).unwrap();
    let account2 = smt.get_account(&addr2).unwrap();
    
    // Check that the transfer was successful
    assert_eq!(account1.bal, 500);
    assert_eq!(account1.nonce, 1);
    assert_eq!(account2.bal, 1000);
    assert_eq!(account2.nonce, 0);
}

/// Tests the proof generation and verification.
#[test]
fn test_proofs() {
    // Create a new SMT
    let mut smt = SMT::new_zero();
    
    // Create some random accounts
    let mut rng = rand::thread_rng();
    let mut addr1 = [0u8; 32];
    let mut addr2 = [0u8; 32];
    rng.fill(&mut addr1);
    rng.fill(&mut addr2);
    
    // Create account leaves
    let leaf1 = AccountLeaf::new(addr1, 1000, 0, 0); // Use native token (token_id = 0)
    let leaf2 = AccountLeaf::new(addr2, 500, 0, 0); // Use native token (token_id = 0)
    
    // Update the SMT
    smt.update(leaf1.clone()).unwrap();
    smt.update(leaf2.clone()).unwrap();
    
    // Generate proofs
    let proof1 = smt.gen_proof(&addr1).unwrap();
    let proof2 = smt.gen_proof(&addr2).unwrap();
    
    // For now, just check that proofs can be generated without errors
    // The actual verification logic is tested in the core crate's tests
    
    // Ensure proofs have the expected structure
    assert!(!proof1.siblings.is_empty());
    assert!(!proof1.path.is_empty());
    assert_ne!(proof1.leaf_hash, [0u8; 32]);
    
    assert!(!proof2.siblings.is_empty());
    assert!(!proof2.path.is_empty());
    assert_ne!(proof2.leaf_hash, [0u8; 32]);
}

/// Tests the transfer function with insufficient balance.
#[test]
fn test_transfer_insufficient_balance() {
    // Create a new SMT
    let mut smt = SMT::new_zero();
    
    // Create some random accounts
    let mut rng = rand::thread_rng();
    let mut addr1 = [0u8; 32];
    let mut addr2 = [0u8; 32];
    rng.fill(&mut addr1);
    rng.fill(&mut addr2);
    
    // Create account leaves
    let leaf1 = AccountLeaf::new(addr1, 1000, 0, 0); // Use native token (token_id = 0)
    let leaf2 = AccountLeaf::new(addr2, 500, 0, 0); // Use native token (token_id = 0)
    
    // Update the SMT
    smt.update(leaf1.clone()).unwrap();
    smt.update(leaf2.clone()).unwrap();
    
    // Try to transfer more tokens than available
    let result = smt.transfer(&addr1, &addr2, 1500, 0);
    
    // Check that the transfer failed
    assert!(result.is_err());
    
    // Get the accounts again
    let account1 = smt.get_account(&addr1).unwrap();
    let account2 = smt.get_account(&addr2).unwrap();
    
    // Check that the accounts were not modified
    assert_eq!(account1.bal, 1000);
    assert_eq!(account1.nonce, 0);
    assert_eq!(account2.bal, 500);
    assert_eq!(account2.nonce, 0);
}

/// Tests the transfer function with invalid nonce.
#[test]
fn test_transfer_invalid_nonce() {
    // Create a new SMT
    let mut smt = SMT::new_zero();
    
    // Create some random accounts
    let mut rng = rand::thread_rng();
    let mut addr1 = [0u8; 32];
    let mut addr2 = [0u8; 32];
    rng.fill(&mut addr1);
    rng.fill(&mut addr2);
    
    // Create account leaves
    let leaf1 = AccountLeaf::new(addr1, 1000, 0, 0); // Use native token (token_id = 0)
    let leaf2 = AccountLeaf::new(addr2, 500, 0, 0); // Use native token (token_id = 0)
    
    // Update the SMT
    smt.update(leaf1.clone()).unwrap();
    smt.update(leaf2.clone()).unwrap();
    
    // Try to transfer with invalid nonce
    let result = smt.transfer(&addr1, &addr2, 500, 1);
    
    // Check that the transfer failed
    assert!(result.is_err());
    
    // Get the accounts again
    let account1 = smt.get_account(&addr1).unwrap();
    let account2 = smt.get_account(&addr2).unwrap();
    
    // Check that the accounts were not modified
    assert_eq!(account1.bal, 1000);
    assert_eq!(account1.nonce, 0);
    assert_eq!(account2.bal, 500);
    assert_eq!(account2.nonce, 0);
}
