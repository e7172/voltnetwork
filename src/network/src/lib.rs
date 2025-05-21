//! Network layer for the chainless token transfer network.
//!
//! This crate provides the P2P networking functionality for the stateless token
//! transfer network, including DHT-based proof storage and retrieval, and
//! gossip-based state updates.

pub mod dht;
pub mod errors;
pub mod gossip;
pub mod storage;
pub mod transport;
pub mod types;

// Re-export commonly used types and functions
pub use dht::{get_proof, put_proof};
pub use errors::NetworkError;
pub use gossip::broadcast_update;
pub use transport::{init_swarm, NetworkEvent};
pub use types::{ProofRequest, ProofResponse, UpdateMsg};
