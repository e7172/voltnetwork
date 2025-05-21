//! Error types for the node daemon.

use std::fmt;
use std::error::Error as StdError;
use core::errors::CoreError;
use network::errors::NetworkError;

/// Errors that can occur in the node daemon.
#[derive(Debug)]
pub enum NodeError {
    /// Error when a core operation fails.
    CoreError(CoreError),

    /// Error when a network operation fails.
    NetworkError(NetworkError),

    /// Error when an RPC operation fails.
    RpcError(String),

    /// Error when a metrics operation fails.
    MetricsError(String),

    /// Error when a configuration operation fails.
    ConfigError(String),

    /// Error when a proof is invalid.
    InvalidProof(String),

    /// Error when a signature is invalid.
    InvalidSignature(String),

    /// Error when an operation is unauthorized.
    Unauthorized(String),
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeError::CoreError(e) => write!(f, "Core error: {}", e),
            NodeError::NetworkError(e) => write!(f, "Network error: {}", e),
            NodeError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            NodeError::MetricsError(msg) => write!(f, "Metrics error: {}", msg),
            NodeError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            NodeError::InvalidProof(msg) => write!(f, "Invalid proof for {}", msg),
            NodeError::InvalidSignature(msg) => write!(f, "Invalid signature: {}", msg),
            NodeError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
        }
    }
}

impl StdError for NodeError {}

impl From<CoreError> for NodeError {
    fn from(error: CoreError) -> Self {
        NodeError::CoreError(error)
    }
}

impl From<NetworkError> for NodeError {
    fn from(error: NetworkError) -> Self {
        NodeError::NetworkError(error)
    }
}
