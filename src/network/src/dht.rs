//! DHT implementation for storing and retrieving proofs.

use crate::errors::NetworkError;
use crate::types::{ProofRequest, ProofResponse};
use core::{proofs::Proof, types::Address};
use futures::channel::oneshot;
use libp2p::kad::{
    record::Key, Kademlia, KademliaEvent, QueryId, QueryResult, Record,
    GetRecordOk, GetRecordError,
};
use libp2p::swarm::SwarmEvent;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::timeout;

/// The timeout for DHT operations in seconds.
const DHT_TIMEOUT_SECS: u64 = 30;

/// A pending DHT get operation.
struct PendingGet {
    /// The sender for the response
    sender: oneshot::Sender<Result<Proof, NetworkError>>,
    /// The address being queried
    address: Address,
    /// The root hash being queried
    root: [u8; 32],
}

/// A pending DHT put operation.
struct PendingPut {
    /// The sender for the response
    sender: oneshot::Sender<Result<(), NetworkError>>,
    /// The address being stored
    address: Address,
    /// The root hash being stored
    root: [u8; 32],
}

/// A manager for DHT operations.
#[derive(Clone)]
pub struct DHTManager {
    /// Pending get operations by query ID
    pending_gets: Arc<Mutex<HashMap<QueryId, PendingGet>>>,
    /// Pending put operations by query ID
    pending_puts: Arc<Mutex<HashMap<QueryId, PendingPut>>>,
}

impl DHTManager {
    /// Creates a new DHT manager.
    pub fn new() -> Self {
        Self {
            pending_gets: Arc::new(Mutex::new(HashMap::new())),
            pending_puts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Handles a Kademlia event.
    pub fn handle_event(
        &self,
        event: KademliaEvent,
        kademlia: &mut Kademlia<libp2p::kad::store::MemoryStore>,
    ) {
        match event {
            KademliaEvent::OutboundQueryProgressed { id, result, .. } => {
                match result {
                    QueryResult::GetRecord(Ok(result)) => {
                        // Handle get record result
                        if let Some(pending) = self.pending_gets.lock().unwrap().remove(&id) {
                            match result {
                                GetRecordOk::FoundRecord(record) => {
                                    // Deserialize the proof
                                    match bincode::deserialize::<Proof>(&record.record.value) {
                                        Ok(proof) => {
                                            let _ = pending.sender.send(Ok(proof));
                                        }
                                        Err(e) => {
                                            let _ = pending.sender.send(Err(
                                                NetworkError::SerializationError(e.to_string())
                                            ));
                                        }
                                    }
                                }
                                // Handle multiple records (not used in current libp2p version)
                                _ => {
                                    let _ = pending.sender.send(Err(
                                        NetworkError::ProofNotFound(pending.address)
                                    ));
                                }
                            }
                        }
                    }
                    QueryResult::GetRecord(Err(e)) => {
                        // Handle get record error
                        if let Some(pending) = self.pending_gets.lock().unwrap().remove(&id) {
                            let _ = pending.sender.send(Err(
                                NetworkError::DHTError(format!("Get record error: {:?}", e))
                            ));
                        }
                    }
                    QueryResult::PutRecord(Ok(_)) => {
                        // Handle put record result
                        if let Some(pending) = self.pending_puts.lock().unwrap().remove(&id) {
                            let _ = pending.sender.send(Ok(()));
                        }
                    }
                    QueryResult::PutRecord(Err(e)) => {
                        // Handle put record error
                        if let Some(pending) = self.pending_puts.lock().unwrap().remove(&id) {
                            let _ = pending.sender.send(Err(
                                NetworkError::DHTError(format!("Put record error: {:?}", e))
                            ));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Handles a Kademlia event synchronously.
    pub fn handle_event_sync(
        &self,
        event: KademliaEvent,
        kademlia: &mut Kademlia<libp2p::kad::store::MemoryStore>,
    ) {
        // This is already synchronous, so we can just call the regular handle_event
        self.handle_event(event, kademlia);
    }

    /// Puts a proof in the DHT.
    pub async fn put_proof(
        &self,
        kademlia: &mut Kademlia<libp2p::kad::store::MemoryStore>,
        address: &Address,
        root: &[u8; 32],
        proof: &Proof,
    ) -> Result<(), NetworkError> {
        // Create a key for the proof
        let key_bytes = create_proof_key(address, root);
        let key = Key::new(&key_bytes);

        // Serialize the proof
        let value = bincode::serialize(proof)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;

        // Create a record
        let record = Record {
            key,
            value,
            publisher: None,
            expires: None,
        };

        // Create a channel for the response
        let (sender, receiver) = oneshot::channel();

        // Put the record in the DHT
        let query_id = match kademlia.put_record(record, libp2p::kad::Quorum::Majority) {
            Ok(id) => id,
            Err(e) => return Err(NetworkError::DHTError(format!("Failed to put record: {:?}", e))),
        };

        // Store the pending put
        self.pending_puts.lock().unwrap().insert(
            query_id,
            PendingPut {
                sender,
                address: *address,
                root: *root,
            },
        );

        // Wait for the response with a timeout
        match timeout(
            Duration::from_secs(DHT_TIMEOUT_SECS),
            receiver,
        ).await {
            Ok(result) => result.map_err(|_| NetworkError::Timeout("put_proof".to_string()))?,
            Err(_) => {
                // Remove the pending put on timeout
                self.pending_puts.lock().unwrap().remove(&query_id);
                Err(NetworkError::Timeout("put_proof".to_string()))
            }
        }
    }

    /// Gets a proof from the DHT.
    pub async fn get_proof(
        &self,
        kademlia: &mut Kademlia<libp2p::kad::store::MemoryStore>,
        address: &Address,
        root: &[u8; 32],
    ) -> Result<Proof, NetworkError> {
        // Create a key for the proof
        let key_bytes = create_proof_key(address, root);
        let key = Key::new(&key_bytes);

        // Create a channel for the response
        let (sender, receiver) = oneshot::channel();

        // Get the record from the DHT
        let query_id = kademlia.get_record(key);

        // Store the pending get
        self.pending_gets.lock().unwrap().insert(
            query_id,
            PendingGet {
                sender,
                address: *address,
                root: *root,
            },
        );

        // Wait for the response with a timeout
        match timeout(
            Duration::from_secs(DHT_TIMEOUT_SECS),
            receiver,
        ).await {
            Ok(result) => result.map_err(|_| NetworkError::Timeout("get_proof".to_string()))?,
            Err(_) => {
                // Remove the pending get on timeout
                self.pending_gets.lock().unwrap().remove(&query_id);
                Err(NetworkError::Timeout("get_proof".to_string()))
            }
        }
    }
}

/// Creates a key for a proof in the DHT.
fn create_proof_key(address: &Address, root: &[u8; 32]) -> Vec<u8> {
    let mut key = Vec::with_capacity(64);
    key.extend_from_slice(address);
    key.extend_from_slice(root);
    key
}

/// Puts a proof in the DHT.
///
/// This is a convenience function that wraps the DHT manager's put_proof method.
pub async fn put_proof(
    dht_manager: &DHTManager,
    kademlia: &mut Kademlia<libp2p::kad::store::MemoryStore>,
    address: &Address,
    root: &[u8; 32],
    proof: &Proof,
) -> Result<(), NetworkError> {
    dht_manager.put_proof(kademlia, address, root, proof).await
}

/// Gets a proof from the DHT.
///
/// This is a convenience function that wraps the DHT manager's get_proof method.
pub async fn get_proof(
    dht_manager: &DHTManager,
    kademlia: &mut Kademlia<libp2p::kad::store::MemoryStore>,
    address: &Address,
    root: &[u8; 32],
) -> Result<Proof, NetworkError> {
    dht_manager.get_proof(kademlia, address, root).await
}
