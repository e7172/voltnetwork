//! Configuration for the node daemon.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Configuration for the node daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Network configuration
    pub network: NetworkConfig,
    /// RPC configuration
    pub rpc: RpcConfig,
    /// Metrics configuration
    pub metrics: MetricsConfig,
    /// Storage configuration
    pub storage: StorageConfig,
}

/// Network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen address for the P2P network
    pub listen_addr: String,
    /// Bootstrap nodes to connect to
    pub bootstrap_nodes: Vec<String>,
    /// Maximum number of peers to connect to
    pub max_peers: usize,
}

/// RPC configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Whether to enable the RPC server
    pub enabled: bool,
    /// Listen address for the RPC server
    pub listen_addr: String,
    /// CORS allowed origins
    pub cors_domains: Vec<String>,
}

/// Metrics configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Whether to enable the metrics server
    pub enabled: bool,
    /// Listen address for the metrics server
    pub listen_addr: String,
}

/// Storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to the data directory
    pub data_dir: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                listen_addr: "/ip4/0.0.0.0/tcp/9000".to_string(),
                bootstrap_nodes: Vec::new(),
                max_peers: 50,
            },
            rpc: RpcConfig {
                enabled: false,
                listen_addr: "127.0.0.1:8545".to_string(),
                cors_domains: vec!["*".to_string()],
            },
            metrics: MetricsConfig {
                enabled: false,
                listen_addr: "127.0.0.1:9090".to_string(),
            },
            storage: StorageConfig {
                data_dir: "./data".to_string(),
            },
        }
    }
}

impl NodeConfig {
    /// Loads configuration from a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        let config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Saves configuration to a file.
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}
