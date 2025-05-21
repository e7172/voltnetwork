/// Error types for the network crate.
use std::fmt;
use std::error::Error as StdError;
use core::types::Address;

/// Errors that can occur in the network crate.
#[derive(Debug)]
pub enum NetworkError {
    /// Error when a libp2p operation fails.
    Libp2pError(String),

    /// Error when a DHT operation fails.
    DHTError(String),

    /// Error when a gossip operation fails.
    GossipError(String),

    /// Error when a storage operation fails.
    StorageError(String),

    /// Error when serialization or deserialization fails.
    SerializationError(String),

    /// Error when a proof is not found.
    ProofNotFound(Address),

    /// Error when a timeout occurs.
    Timeout(String),

    /// Error when a peer is not found.
    PeerNotFound(String),

    /// Error when a message is invalid.
    InvalidMessage(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::Libp2pError(msg) => write!(f, "libp2p error: {}", msg),
            NetworkError::DHTError(msg) => write!(f, "DHT error: {}", msg),
            NetworkError::GossipError(msg) => write!(f, "Gossip error: {}", msg),
            NetworkError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            NetworkError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            NetworkError::ProofNotFound(addr) => write!(f, "Proof not found for address: {:?}", addr),
            NetworkError::Timeout(msg) => write!(f, "Timeout waiting for {}", msg),
            NetworkError::PeerNotFound(msg) => write!(f, "Peer not found: {}", msg),
            NetworkError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
        }
    }
}

impl StdError for NetworkError {}


