//! Tests for the bridge crate.

use bridge::bridge::Bridge;
use core::{
    proofs::Proof,
    smt::SMT,
    types::{AccountLeaf, Address},
};
use ethers::{
    core::types::{Address as EthAddress, U256},
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
};
use rand::Rng;
use serial_test::serial;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Tests the bridge contract bindings.
#[test]
#[serial]
#[ignore] // Requires a local Ethereum node
fn test_bridge_bindings() {
    // Create a runtime
    let rt = Runtime::new().unwrap();
    
    // Create a provider
    let provider = Provider::<Http>::try_from("http://localhost:8545").unwrap();
    
    // Create a wallet
    let wallet = "0x0123456789012345678901234567890123456789012345678901234567890123"
        .parse::<LocalWallet>()
        .unwrap();
    
    // Create a signer
    let signer = SignerMiddleware::new(provider, wallet);
    
    // Create a bridge
    rt.block_on(async {
        let bridge = Bridge::new(
            Arc::new(signer),
            "0x0123456789012345678901234567890123456789",
        )
        .await;
        
        // Check that the bridge was created successfully
        assert!(bridge.is_ok());
    });
}

/// Tests the bridge lock function.
#[test]
#[serial]
#[ignore] // Requires a local Ethereum node
fn test_bridge_lock() {
    // Create a runtime
    let rt = Runtime::new().unwrap();
    
    // Create a provider
    let provider = Provider::<Http>::try_from("http://localhost:8545").unwrap();
    
    // Create a wallet
    let wallet = "0x0123456789012345678901234567890123456789012345678901234567890123"
        .parse::<LocalWallet>()
        .unwrap();
    
    // Create a signer
    let signer = SignerMiddleware::new(provider, wallet);
    
    // Create a bridge
    rt.block_on(async {
        let bridge = Bridge::new(
            Arc::new(signer),
            "0x0123456789012345678901234567890123456789",
        )
        .await
        .unwrap();
        
        // Create a random address
        let mut rng = rand::thread_rng();
        let mut addr = [0u8; 32];
        rng.fill(&mut addr);
        
        // Lock some tokens
        let result = bridge.lock(&addr, 1000).await;
        
        // Check that the lock was successful
        assert!(result.is_ok());
    });
}

/// Tests the bridge unlock function.
#[test]
#[serial]
#[ignore] // Requires a local Ethereum node
fn test_bridge_unlock() {
    // Create a runtime
    let rt = Runtime::new().unwrap();
    
    // Create a provider
    let provider = Provider::<Http>::try_from("http://localhost:8545").unwrap();
    
    // Create a wallet
    let wallet = "0x0123456789012345678901234567890123456789012345678901234567890123"
        .parse::<LocalWallet>()
        .unwrap();
    
    // Create a signer
    let signer = SignerMiddleware::new(provider, wallet);
    
    // Create a bridge
    rt.block_on(async {
        let bridge = Bridge::new(
            Arc::new(signer),
            "0x0123456789012345678901234567890123456789",
        )
        .await
        .unwrap();
        
        // Create a random address
        let mut rng = rand::thread_rng();
        let mut addr = [0u8; 32];
        rng.fill(&mut addr);
        
        // Create an SMT
        let mut smt = SMT::new_zero();
        
        // Create an account leaf
        let leaf = AccountLeaf::new(addr, 1000, 0, 0); // Use native token (token_id = 0)
        
        // Update the SMT
        smt.update(leaf.clone()).unwrap();
        
        // Generate a proof
        let proof = smt.gen_proof(&addr).unwrap();
        
        // Update the root in the bridge contract
        let result = bridge.update_root(&smt.root()).await;
        assert!(result.is_ok());
        
        // Unlock some tokens
        let result = bridge
            .unlock(
                "0x0123456789012345678901234567890123456789",
                1000,
                &proof,
                &addr,
            )
            .await;
        
        // Check that the unlock was successful
        assert!(result.is_ok());
    });
}
