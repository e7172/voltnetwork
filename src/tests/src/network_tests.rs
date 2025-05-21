//! Tests for the network crate.

use core::{
    proofs::Proof,
    smt::SMT,
    types::{AccountLeaf, Address},
};
use network::{
    dht::DHTManager,
    storage::ProofStore,
    transport::{init_swarm, handle_network_event},
    types::UpdateMsg,
};
use rand::Rng;
use serial_test::serial;
use std::collections::HashSet;
use tempfile::tempdir;
use tokio::runtime::Runtime;
use futures::StreamExt;

/// Tests the proof store.
#[test]
fn test_proof_store() {
    // Create a temporary directory for the proof store
    let dir = tempdir().unwrap();
    
    // Create a proof store
    let store = ProofStore::new(dir.path()).unwrap();
    
    // Create a random address and root
    let mut rng = rand::thread_rng();
    let mut addr = [0u8; 32];
    let mut root = [0u8; 32];
    rng.fill(&mut addr);
    rng.fill(&mut root);
    
    // Create a simple proof
    let leaf_hash = AccountLeaf::new_empty(addr, 0).hash(); // Use native token (token_id = 0)
    let siblings = vec![[0u8; 32]];
    let path = vec![false];
    let proof = Proof::new(siblings, leaf_hash, path);
    
    // Store the proof
    store.put_proof(&addr, &root, &proof).unwrap();
    
    // Check that the proof exists
    assert!(store.has_proof(&addr, &root).unwrap());
    
    // Retrieve the proof
    let retrieved = store.get_proof(&addr, &root).unwrap();
    
    // Check that the retrieved proof matches the original
    assert_eq!(retrieved.leaf_hash, proof.leaf_hash);
    assert_eq!(retrieved.siblings.len(), proof.siblings.len());
    assert_eq!(retrieved.path, proof.path);
}

/// Tests the network swarm initialization.
#[test]
#[serial]
fn test_swarm_init() {
    // Create a runtime
    let rt = Runtime::new().unwrap();
    
    // Initialize the swarm
    rt.block_on(async {
        let (swarm, dht_manager) = init_swarm(vec![]).await.unwrap();
        
        // Check that the swarm and DHT manager were created successfully
        // Check that the swarm was created successfully
        // Just verify that the swarm exists
        assert!(swarm.connected_peers().count() == 0);
    });
}

/// Tests the network event handling.
#[test]
#[serial]
fn test_network_event_handling() {
    // Create a runtime
    let rt = Runtime::new().unwrap();
    
    // Initialize the swarm
    rt.block_on(async {
        let (mut swarm, dht_manager) = init_swarm(vec![]).await.unwrap();
        let mut known_peers = HashSet::new();
        
        // Listen on a local address
        swarm.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();
        
        // Wait for the swarm to start listening
        let mut listening = false;
        while !listening {
            match swarm.select_next_some().await {
                event => {
                    match handle_network_event(event, &dht_manager, &mut known_peers, &mut swarm).await {
                        Ok(Some(_)) => {
                            // Event was handled successfully
                        }
                        Ok(None) => {
                            // No event was emitted
                        }
                        Err(e) => {
                            panic!("Error handling network event: {}", e);
                        }
                    }
                    
                    // Check if the swarm is listening
                    if swarm.listeners().count() > 0 {
                        listening = true;
                    }
                }
            }
        }
        
        // Check that the swarm is listening
        assert!(swarm.listeners().count() > 0);
    });
}
