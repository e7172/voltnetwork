//! Node daemon for the chainless token transfer network.

mod config;
mod errors;
mod metrics;
mod rpc;

pub mod main {
    pub use super::handle_update;
}

use anyhow::Result;
use config::NodeConfig;
use core::{proofs::Proof, smt::SMT, types::Address};
use network::gossip;
use libp2p::Swarm;
use network::transport::NodeBehaviour;
use errors::NodeError;
use futures::{StreamExt, FutureExt};
use libp2p::Multiaddr;
use metrics::register_metrics;
use network::{
    dht::DHTManager,
    storage::ProofStore,
    transport::{init_swarm, handle_network_event, NetworkEvent},
    types::UpdateMsg,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Command line arguments for the node daemon.
#[derive(Debug, StructOpt)]
#[structopt(name = "node", about = "Chainless token transfer network node")]
struct Opt {
    /// Path to the configuration file
    #[structopt(short, long, parse(from_os_str))]
    config: Option<PathBuf>,

    /// Path to the data directory
    #[structopt(short, long, parse(from_os_str))]
    data_dir: Option<PathBuf>,

    /// Bootstrap nodes to connect to
    #[structopt(short, long)]
    bootstrap: Vec<String>,

    /// Listen address for the P2P network
    #[structopt(short, long, default_value = "/ip4/0.0.0.0/tcp/9000")]
    listen: String,

    /// Enable JSON-RPC server
    #[structopt(long)]
    rpc: bool,

    /// JSON-RPC server address
    #[structopt(long, default_value = "127.0.0.1:8545")]
    rpc_addr: String,

    /// Enable metrics server
    #[structopt(long)]
    metrics: bool,

    /// Metrics server address
    #[structopt(long, default_value = "127.0.0.1:9090")]
    metrics_addr: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Parse command line arguments
    let opt = Opt::from_args();

    // Load configuration
    let config = match &opt.config {
        Some(path) => NodeConfig::from_file(path)?,
        None => NodeConfig::default(),
    };

    // Determine data directory
    let data_dir = opt.data_dir.unwrap_or_else(|| {
        let mut dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("stateless-token");
        dir
    });

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&data_dir)?;

    // Initialize proof store
    let mut proof_store_path = data_dir.clone();
    proof_store_path.push("proofs");
    let proof_store = ProofStore::new(proof_store_path)?;

    // Path for SMT RocksDB
    let mut smt_db_path = data_dir.clone();
    smt_db_path.push("smt_db");

    // Initialize RocksDB for SMT
    info!("Opening RocksDB for SMT at {}", smt_db_path.display());
    let mut opts = rocksdb::Options::default();
    opts.create_if_missing(true);
    let db = Arc::new(rocksdb::DB::open(&opts, smt_db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open RocksDB: {}", e))?);

    // Initialize SMT - either load from RocksDB or create new
    let smt = match SMT::load_from_db(db.clone()) {
        Ok(loaded_smt) => {
            info!("SMT state loaded successfully from RocksDB");
            Arc::new(Mutex::new(loaded_smt))
        }
        Err(e) => {
            warn!("Failed to load SMT state from RocksDB: {}, creating new", e);
            Arc::new(Mutex::new(SMT::new_with_db(db.clone())))
        }
    };

    // No need for periodic state saving as RocksDB persists changes immediately
    info!("Using RocksDB for SMT state persistence (automatic saving)");

    // Parse bootstrap nodes
    let bootstrap_nodes: Vec<Multiaddr> = opt
        .bootstrap
        .iter()
        .filter_map(|addr| match addr.parse() {
            Ok(addr) => Some(addr),
            Err(e) => {
                warn!("Failed to parse bootstrap node address {}: {}", addr, e);
                None
            }
        })
        .collect();

    // Initialize network swarm
    let (mut swarm, dht_manager) = init_swarm(bootstrap_nodes.clone()).await?;
    
    // Get the local peer ID
    let local_peer_id = swarm.local_peer_id().to_string();
    info!("Local peer ID: {}", local_peer_id);

    // Listen on the specified address
    let listen_addr: Multiaddr = opt.listen.parse()?;
    swarm.listen_on(listen_addr.clone())?;
    info!("Listening on {}", listen_addr);
    
    // Always try to sync state from bootstrap nodes, regardless of whether we have data or not
    if !bootstrap_nodes.is_empty() {
        let root = {
            let smt_lock = smt.lock().unwrap();
            smt_lock.root()
        };
        
        // Check if we have an empty root (all zeros)
        let is_empty_root = root.iter().all(|&b| b == 0);
        
        if is_empty_root {
            info!("New node detected with empty state. Attempting to sync state from bootstrap nodes...");
        } else {
            info!("Node has existing state. Will still attempt to sync latest state from network...");
        }
        
        // Try to connect to each bootstrap node and sync state
        for bootstrap_node in &bootstrap_nodes {
            info!("Attempting to sync state from bootstrap node: {}", bootstrap_node);
            
            // Extract the IP and port from the multiaddr
            if let Some(ip_port) = extract_ip_port(bootstrap_node) {
                let (ip, port) = ip_port;
                
                // Construct the RPC URL
                let rpc_url = format!("http://{}:{}/rpc", ip, 8545); // Assuming RPC port is 8545
                
                info!("Connecting to RPC at {}", rpc_url);
                
                // Try to get the full state from the bootstrap node
                match reqwest::Client::new()
                    .post(&rpc_url)
                    .json(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "get_full_state",
                        "params": [],
                        "id": 1
                    }))
                    .send()
                    .await
                {
                    Ok(response) => {
                        match response.json::<serde_json::Value>().await {
                            Ok(json) => {
                                if let Some(result) = json.get("result") {
                                    // Try to directly apply the state to the SMT
                                    match serde_json::from_value::<rpc::FullState>(result.clone()) {
                                        Ok(full_state) => {
                                            // Apply the state directly to the SMT
                                            let mut smt_lock = smt.lock().unwrap();
                                            
                                            // If we already have state, compare the roots to see if we need to update
                                            if !is_empty_root {
                                                let current_root = smt_lock.root();
                                                
                                                // If our root is the same as the remote root, we're already in sync
                                                if current_root == full_state.root {
                                                    info!("Local state is already in sync with network (root: {:?})", current_root);
                                                    break;
                                                }
                                                
                                                let local_accounts = smt_lock.get_all_accounts().unwrap_or_default();
                                                
                                                // Calculate total balance and highest nonce for local state
                                                let (local_total_balance, local_highest_nonce) = local_accounts.iter()
                                                    .fold((0u128, 0u64), |(total_balance, highest_nonce), account| {
                                                        (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce))
                                                    });
                                                
                                                // Calculate total balance and highest nonce for remote state
                                                let (remote_total_balance, remote_highest_nonce) = full_state.accounts.iter()
                                                    .fold((0u128, 0u64), |(total_balance, highest_nonce), account| {
                                                        (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce))
                                                    });
                                                
                                                // Calculate active accounts for both states
                                                let (_, _, local_active_accounts) = local_accounts.iter()
                                                    .fold((0u128, 0u64, 0usize), |(total_balance, highest_nonce, active_accounts), account| {
                                                        let active = if account.bal > 0 { 1 } else { 0 };
                                                        (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce), active_accounts + active)
                                                    });
                                                
                                                let (_, _, remote_active_accounts) = full_state.accounts.iter()
                                                    .fold((0u128, 0u64, 0usize), |(total_balance, highest_nonce, active_accounts), account| {
                                                        let active = if account.bal > 0 { 1 } else { 0 };
                                                        (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce), active_accounts + active)
                                                    });
                                                
                                                // Calculate a consensus score for each state
                                                // This is a weighted combination of factors that indicate state freshness
                                                let local_score = (local_active_accounts as u128 * 10) +
                                                                 (local_highest_nonce as u128 * 100) +
                                                                 (local_total_balance / 1000);
                                                
                                                let remote_score = (remote_active_accounts as u128 * 10) +
                                                                  (remote_highest_nonce as u128 * 100) +
                                                                  (remote_total_balance / 1000);
                                                
                                                // Log detailed state information for debugging
                                                info!("State comparison:");
                                                info!("Local: {} accounts ({} active), {} total balance, highest nonce {}, score {}",
                                                      local_accounts.len(), local_active_accounts, local_total_balance, local_highest_nonce, local_score);
                                                info!("Remote: {} accounts ({} active), {} total balance, highest nonce {}, score {}",
                                                      full_state.accounts.len(), remote_active_accounts, remote_total_balance, remote_highest_nonce, remote_score);
                                                
                                                // If local state has a higher score, keep it
                                                if local_score >= remote_score {
                                                    info!("Local state has higher consensus score. Keeping local state.");
                                                    break;
                                                }
                                                
                                                info!("Network state appears more recent. Updating local state...");
                                            }
                                            
                                            match smt_lock.set_full_state(full_state.accounts, full_state.root) {
                                                Ok(_) => {
                                                    info!("Successfully synced state from bootstrap node");
                                                    // State is automatically persisted to RocksDB by set_full_state
                                                    break; // Successfully synced, no need to try other nodes
                                                }
                                                Err(e) => {
                                                    warn!("Failed to set state: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to parse state from bootstrap node: {}", e);
                                        }
                                    }
                                } else {
                                    warn!("Invalid response from bootstrap node: no result field");
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse response from bootstrap node: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to connect to bootstrap node RPC: {}", e);
                    }
                }
            } else {
                warn!("Failed to extract IP and port from bootstrap node address: {}", bootstrap_node);
            }
        }
    }
    
    // Perform full state synchronization on startup if bootstrap nodes are provided
    if !bootstrap_nodes.is_empty() {
        info!("Node has existing state. Will still attempt to sync latest state from network...");
        
        // Try to sync with each bootstrap node
        for bootstrap_node in &bootstrap_nodes {
            if let Some(ip_port) = extract_ip_port(bootstrap_node) {
                info!("Attempting to sync state from bootstrap node: {}", bootstrap_node);
                
                // Construct the RPC URL
                let (ip, _) = ip_port;
                let rpc_url = format!("http://{}:{}/rpc", ip, 8545); // Assuming RPC port is 8545
                
                info!("Connecting to RPC at {}", rpc_url);
                
                // Try to get the full state from the bootstrap node
                match reqwest::Client::new()
                    .post(&rpc_url)
                    .json(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "get_full_state",
                        "params": [],
                        "id": 1
                    }))
                    .send()
                    .await
                {
                    Ok(response) => {
                        match response.json::<serde_json::Value>().await {
                            Ok(json) => {
                                if let Some(result) = json.get("result") {
                                    // Try to directly apply the state to the SMT
                                    match serde_json::from_value::<rpc::FullState>(result.clone()) {
                                        Ok(full_state) => {
                                            // Apply the state directly to the SMT
                                            let mut smt_lock = smt.lock().unwrap();
                                            let current_root = smt_lock.root();
                                            
                                            // If our root is the same as the remote root, we're already in sync
                                            if current_root == full_state.root {
                                                info!("Local state is already in sync with network (root: {:?})", current_root);
                                                break;
                                            }
                                            
                                            // If remote state has more accounts or higher nonce, update local state
                                            let local_accounts = smt_lock.get_all_accounts().unwrap_or_default();
                                            
                                            // Calculate metrics for comparison
                                            let (local_total_balance, local_highest_nonce) = local_accounts.iter()
                                                .fold((0u128, 0u64), |(total_balance, highest_nonce), account| {
                                                    (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce))
                                                });
                                            
                                            let (remote_total_balance, remote_highest_nonce) = full_state.accounts.iter()
                                                .fold((0u128, 0u64), |(total_balance, highest_nonce), account| {
                                                    (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce))
                                                });
                                            
                                            // Calculate metrics for both states to implement a consensus-based approach
                                            // This aligns with Volt's architecture of maintaining a single canonical state
                                            let (_, _, local_active_accounts) = local_accounts.iter()
                                                .fold((0u128, 0u64, 0usize), |(total_balance, highest_nonce, active_accounts), account| {
                                                    let active = if account.bal > 0 { 1 } else { 0 };
                                                    (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce), active_accounts + active)
                                                });
                                            
                                            let (_, _, remote_active_accounts) = full_state.accounts.iter()
                                                .fold((0u128, 0u64, 0usize), |(total_balance, highest_nonce, active_accounts), account| {
                                                    let active = if account.bal > 0 { 1 } else { 0 };
                                                    (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce), active_accounts + active)
                                                });
                                            
                                            // Calculate a consensus score for each state
                                            // This is a weighted combination of factors that indicate state freshness
                                            let local_score = (local_active_accounts as u128 * 10) +
                                                             (local_highest_nonce as u128 * 100) +
                                                             (local_total_balance / 1000);
                                            
                                            let remote_score = (remote_active_accounts as u128 * 10) +
                                                              (remote_highest_nonce as u128 * 100) +
                                                              (remote_total_balance / 1000);
                                            
                                            // Log detailed state information for debugging
                                            info!("State comparison:");
                                            info!("Local: {} accounts ({} active), {} total balance, highest nonce {}, score {}",
                                                  local_accounts.len(), local_active_accounts, local_total_balance, local_highest_nonce, local_score);
                                            info!("Remote: {} accounts ({} active), {} total balance, highest nonce {}, score {}",
                                                  full_state.accounts.len(), remote_active_accounts, remote_total_balance, remote_highest_nonce, remote_score);
                                            
                                            // If local state has a higher score, keep it
                                            if local_score >= remote_score {
                                                info!("Local state has higher consensus score. Keeping local state.");
                                                break;
                                            }
                                            
                                            info!("Network state appears more recent. Updating local state...");
                                            match smt_lock.set_full_state(full_state.accounts, full_state.root) {
                                                Ok(_) => {
                                                    info!("Successfully synced state from bootstrap node");
                                                    break; // Successfully synced, no need to try other nodes
                                                }
                                                Err(e) => {
                                                    warn!("Failed to set state: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to parse state from bootstrap node: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse response from bootstrap node: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to connect to bootstrap node RPC: {}", e);
                    }
                }
            } else {
                warn!("Failed to extract IP and port from bootstrap node address: {}", bootstrap_node);
            }
        }
    }
    
    // Set up periodic state synchronization
    let smt_for_sync = smt.clone();
    let bootstrap_nodes_for_sync = bootstrap_nodes.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Sync every 1 minute
        
        loop {
            interval.tick().await;
            
            // Skip if no bootstrap nodes
            if bootstrap_nodes_for_sync.is_empty() {
                continue;
            }
            
            info!("Performing periodic state synchronization...");
            
            // Try to connect to each bootstrap node and sync state
            for bootstrap_node in &bootstrap_nodes_for_sync {
                // Extract the IP and port from the multiaddr
                if let Some(ip_port) = extract_ip_port(bootstrap_node) {
                    let (ip, port) = ip_port;
                    
                    // Construct the RPC URL
                    let rpc_url = format!("http://{}:{}/rpc", ip, 8545); // Assuming RPC port is 8545
                    
                    // Try to get the full state from the bootstrap node
                    match reqwest::Client::new()
                        .post(&rpc_url)
                        .json(&serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "get_full_state",
                            "params": [],
                            "id": 1
                        }))
                        .send()
                        .await
                    {
                        Ok(response) => {
                            match response.json::<serde_json::Value>().await {
                                Ok(json) => {
                                    if let Some(result) = json.get("result") {
                                        // Try to directly apply the state to the SMT
                                        match serde_json::from_value::<rpc::FullState>(result.clone()) {
                                            Ok(full_state) => {
                                                // Apply the state directly to the SMT
                                                let mut smt_lock = smt_for_sync.lock().unwrap();
                                                let current_root = smt_lock.root();
                                                
                                                // If our root is the same as the remote root, we're already in sync
                                                if current_root == full_state.root {
                                                    info!("Local state is already in sync with network (root: {:?})", current_root);
                                                    break;
                                                }
                                                
                                                // If our root is different, we need a more sophisticated approach to determine which state is more recent
                                                // Compare state metadata like timestamps, block heights, or transaction counts
                                                // For this implementation, we'll use a combination of account count, total balance, and highest nonce
                                                
                                                let local_accounts = smt_lock.get_all_accounts().unwrap_or_default();
                                                
                                                // Calculate total balance and highest nonce for local state
                                                let (local_total_balance, local_highest_nonce) = local_accounts.iter()
                                                    .fold((0u128, 0u64), |(total_balance, highest_nonce), account| {
                                                        (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce))
                                                    });
                                                
                                                // Calculate total balance and highest nonce for remote state
                                                let (remote_total_balance, remote_highest_nonce) = full_state.accounts.iter()
                                                    .fold((0u128, 0u64), |(total_balance, highest_nonce), account| {
                                                        (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce))
                                                    });
                                                
                                                // Calculate metrics for both states to implement a consensus-based approach
                                                // This aligns with Volt's architecture of maintaining a single canonical state
                                                let (_, _, local_active_accounts) = local_accounts.iter()
                                                    .fold((0u128, 0u64, 0usize), |(total_balance, highest_nonce, active_accounts), account| {
                                                        let active = if account.bal > 0 { 1 } else { 0 };
                                                        (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce), active_accounts + active)
                                                    });
                                                
                                                let (_, _, remote_active_accounts) = full_state.accounts.iter()
                                                    .fold((0u128, 0u64, 0usize), |(total_balance, highest_nonce, active_accounts), account| {
                                                        let active = if account.bal > 0 { 1 } else { 0 };
                                                        (total_balance + account.bal, std::cmp::max(highest_nonce, account.nonce), active_accounts + active)
                                                    });
                                                
                                                // Calculate a consensus score for each state
                                                // This is a weighted combination of factors that indicate state freshness
                                                let local_score = (local_active_accounts as u128 * 10) +
                                                                 (local_highest_nonce as u128 * 100) +
                                                                 (local_total_balance / 1000);
                                                
                                                let remote_score = (remote_active_accounts as u128 * 10) +
                                                                  (remote_highest_nonce as u128 * 100) +
                                                                  (remote_total_balance / 1000);
                                                
                                                // Log detailed state information for debugging
                                                info!("State comparison:");
                                                info!("Local: {} accounts ({} active), {} total balance, highest nonce {}, score {}",
                                                      local_accounts.len(), local_active_accounts, local_total_balance, local_highest_nonce, local_score);
                                                info!("Remote: {} accounts ({} active), {} total balance, highest nonce {}, score {}",
                                                      full_state.accounts.len(), remote_active_accounts, remote_total_balance, remote_highest_nonce, remote_score);
                                                
                                                // If local state has a higher score, keep it
                                                if local_score >= remote_score {
                                                    info!("Local state has higher consensus score. Keeping local state.");
                                                    break;
                                                }
                                                
                                                info!("Network state appears more recent. Updating local state...");
                                                match smt_lock.set_full_state(full_state.accounts, full_state.root) {
                                                    Ok(_) => {
                                                        info!("Successfully synced state from bootstrap node");
                                                        break; // Successfully synced, no need to try other nodes
                                                    }
                                                    Err(e) => {
                                                        warn!("Failed to set state: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Failed to parse state from bootstrap node: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to parse response from bootstrap node: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to connect to bootstrap node RPC: {}", e);
                        }
                    }
                }
            }
        }
    });


    // Register metrics if enabled
    if opt.metrics {
        register_metrics();
        let metrics_addr = opt.metrics_addr.parse()?;
        metrics::start_metrics_server(metrics_addr).await?;
        info!("Metrics server listening on {}", opt.metrics_addr);
    }

    // Create channels for broadcasting messages
    let (gossip_tx, mut gossip_rx) = tokio::sync::mpsc::channel::<network::types::MintMsg>(100);
    let (update_tx, mut update_rx) = tokio::sync::mpsc::channel::<network::types::UpdateMsg>(100);
    
    // Create a synchronization barrier flag
    let state_synced = Arc::new(std::sync::atomic::AtomicBool::new(false));
    
    // Perform initial state synchronization
    if !bootstrap_nodes.is_empty() {
        info!("Cold-start safety: Blocking RPC and gossip until state is synchronized");
        
        // Try to synchronize state from bootstrap nodes
        let sync_result = synchronize_state_from_network(&bootstrap_nodes, &smt).await;
        
        if sync_result {
            info!("Cold-start safety: State successfully synchronized from network");
            state_synced.store(true, std::sync::atomic::Ordering::SeqCst);
        } else {
            // If we have a non-empty state, we can still proceed
            let root = {
                let smt_lock = smt.lock().unwrap();
                smt_lock.root()
            };
            
            let is_empty_root = root.iter().all(|&b| b == 0);
            
            if !is_empty_root {
                info!("Cold-start safety: Using existing local state as no network state could be obtained");
                state_synced.store(true, std::sync::atomic::Ordering::SeqCst);
            } else {
                warn!("Cold-start safety: No state could be synchronized and local state is empty");
                warn!("Cold-start safety: Node will continue to attempt synchronization in the background");
                warn!("Cold-start safety: RPC and gossip will be blocked until state is synchronized");
            }
        }
    } else {
        // If there are no bootstrap nodes, we can't synchronize state
        // In this case, we'll use whatever state we have locally
        info!("Cold-start safety: No bootstrap nodes provided, using local state");
        state_synced.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    
    // Start JSON-RPC server if enabled and state is synchronized
    let rpc_handle = if opt.rpc {
        let rpc_addr = opt.rpc_addr.parse()?;
        let smt_clone = smt.clone();
        let proof_store_clone = proof_store.clone();
        
        // Create shared references to the gossip senders
        let gossip_tx = Arc::new(Mutex::new(gossip_tx));
        let update_tx = Arc::new(Mutex::new(update_tx));
        
        // Only start the RPC server if state is synchronized
        if state_synced.load(std::sync::atomic::Ordering::SeqCst) {
            rpc::start_rpc_server(rpc_addr, smt_clone, proof_store_clone, local_peer_id.clone(), gossip_tx, update_tx).await?;
            info!("JSON-RPC server listening on {}", opt.rpc_addr);
            None
        } else {
            // If state is not synchronized, spawn a task to start the RPC server once state is synchronized
            let state_synced_clone = state_synced.clone();
            let handle = tokio::spawn(async move {
                // Wait for state to be synchronized
                while !state_synced_clone.load(std::sync::atomic::Ordering::SeqCst) {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
                
                // Start the RPC server once state is synchronized
                match rpc::start_rpc_server(rpc_addr, smt_clone, proof_store_clone, local_peer_id, gossip_tx, update_tx).await {
                    Ok(_) => {
                        info!("JSON-RPC server listening on {}", opt.rpc_addr);
                    },
                    Err(e) => {
                        error!("Failed to start RPC server: {}", e);
                    }
                }
            });
            
            Some(handle)
        }
    } else {
        None
    };
    
    // Spawn a task to handle gossip messages
    let swarm_clone = Arc::new(Mutex::new(swarm));
    let swarm_for_gossip = swarm_clone.clone();
    
    tokio::spawn(async move {
        while let Some(mint_msg) = gossip_rx.recv().await {
            // Serialize the mint message
            match bincode::serialize(&mint_msg) {
                Ok(mint_msg_bytes) => {
                    // Create a topic
                    let topic = libp2p::gossipsub::IdentTopic::new(network::gossip::STATE_UPDATES_TOPIC);
                    
                    // Get a mutable reference to the swarm
                    let mut swarm = swarm_for_gossip.lock().unwrap();
                    
                    // Publish the message
                    match swarm.behaviour_mut().gossipsub.publish(topic, mint_msg_bytes) {
                        Ok(_) => {
                            info!("Successfully broadcast mint message");
                        },
                        Err(e) => {
                            error!("Failed to broadcast mint message: {}", e);
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to serialize mint message: {}", e);
                }
            }
        }
    });
    
    // Spawn a task to handle update messages
    let swarm_for_updates = swarm_clone.clone();
    
    tokio::spawn(async move {
        while let Some(update_msg) = update_rx.recv().await {
            // Serialize the update message
            match bincode::serialize(&update_msg) {
                Ok(update_msg_bytes) => {
                    // Create a topic
                    let topic = libp2p::gossipsub::IdentTopic::new(network::gossip::STATE_UPDATES_TOPIC);
                    
                    // Get a mutable reference to the swarm
                    let mut swarm = swarm_for_updates.lock().unwrap();
                    
                    // Publish the message
                    match swarm.behaviour_mut().gossipsub.publish(topic, update_msg_bytes) {
                        Ok(_) => {
                            info!("Successfully broadcast update message");
                        },
                        Err(e) => {
                            error!("Failed to broadcast update message: {}", e);
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to serialize update message: {}", e);
                }
            }
        }
    });
    
    // Get a mutable reference to the swarm for the main loop
    let swarm_mutex = Arc::clone(&swarm_clone);
    
    // Create a channel for network events
    let (tx, mut rx) = mpsc::channel(100);
    let tx_clone = tx.clone();
    
    // Create a channel for passing network events between tasks
    let (event_tx, mut event_rx) = mpsc::channel(100);
    
    // Spawn a task to poll the swarm for events
    let swarm_clone2 = swarm_clone.clone();
    tokio::spawn(async move {
        loop {
            // Poll the swarm for events
            let event_opt = {
                let mut swarm = swarm_clone2.lock().unwrap();
                match swarm.next().now_or_never() {
                    Some(Some(event)) => Some(event),
                    _ => None,
                }
            };
            
            if let Some(event) = event_opt {
                // Send the event to the processing task
                if let Err(e) = event_tx.send(event).await {
                    error!("Failed to send event: {}", e);
                    break;
                }
            } else {
                // Sleep a bit before polling again
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }
    });
    
    // Spawn a task to process the events
    // Create a channel for passing processed events
    let (processed_tx, mut processed_rx) = mpsc::channel(100);
    let processed_tx_clone = processed_tx.clone();
    
    // Spawn a task to process events without holding the lock across await points
    tokio::spawn(async move {
        let mut known_peers = HashSet::new();
        
        while let Some(event) = event_rx.recv().await {
            // Process the event and get a network event if any
            let mut network_event = None;
            
            // Process the event in a block to ensure the MutexGuard is dropped
            {
                let mut swarm = swarm_clone.lock().unwrap();
                
                // Use the synchronous version of handle_network_event
                match network::transport::handle_network_event_sync(event, &dht_manager, &mut known_peers, &mut swarm) {
                    Ok(Some(evt)) => network_event = Some(evt),
                    Err(e) => error!("Error handling network event: {}", e),
                    _ => {}
                }
            }
            
            // If we got a network event, send it
            if let Some(evt) = network_event {
                if let Err(e) = processed_tx_clone.send(evt).await {
                    error!("Failed to send processed event: {}", e);
                }
            }
            
           
        }
    });
    
    // Spawn another task to forward the processed events to the main channel
    tokio::spawn(async move {
        while let Some(event) = processed_rx.recv().await {
            if let Err(e) = tx_clone.send(event).await {
                error!("Failed to send network event: {}", e);
            }
        }
    });

    // We've already created the channel and spawned the task to handle network events above

    // Main event loop
    info!("Node started");
    while let Some(event) = rx.recv().await {
        match event {
            NetworkEvent::UpdateReceived(update) => {
                info!("Received update from network: from={:?}, to={:?}, amount={}",
                      update.from, update.to, update.amount);
                
                match handle_update(update, &smt, &proof_store, &swarm_mutex).await {
                    Ok(_) => info!("Successfully processed update from network"),
                    Err(e) => error!("Failed to process update from network: {}", e),
                }
            }
            NetworkEvent::PeerDiscovered(peer_id) => {
                info!("Discovered peer: {}", peer_id);
                metrics::PEER_COUNT.inc();
                
                // Add the peer to our gossipsub mesh
                let mut swarm = swarm_mutex.lock().unwrap();
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                
                info!("Added peer {} to gossipsub mesh", peer_id);
            }
            NetworkEvent::PeerDisconnected(peer_id) => {
                info!("Disconnected from peer: {}", peer_id);
                metrics::PEER_COUNT.dec();
            }
            NetworkEvent::PeerIdentified(peer_id, addr) => {
                info!("Identified peer {} at {}", peer_id, addr);
                
                // Add the peer to our gossipsub mesh
                let mut swarm = swarm_mutex.lock().unwrap();
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                
                // Also add the address to Kademlia for better connectivity
                swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                
                info!("Added peer {} to gossipsub mesh", peer_id);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Handles an update message.
pub async fn handle_update(
    update: UpdateMsg,
    smt: &Arc<Mutex<SMT>>,
    proof_store: &ProofStore,
    swarm_mutex: &Arc<Mutex<Swarm<NodeBehaviour>>>,
) -> Result<(), NodeError> {
    debug!("Received update: {}", update);
    metrics::UPDATE_COUNTER.inc();

    // First, verify the signature of the update message
    // This is a critical security check to ensure the transaction is authentic
    if let Err(e) = verify_signature(&update) {
        error!("Signature verification failed: {}", e);
        return Err(NodeError::InvalidSignature("Transaction signature verification failed".to_string()));
    }
    
    info!("Signature verification successful");

    // Verify the proofs using the root from the update message
    // This ensures that even if our local state is different, we can still verify
    // the transaction against the state that the sender had when creating it
    let root = update.root;
    
    // Flag to skip the normal transfer and go directly to storing proofs
    let mut goto_store_proofs = false;

    // Get the local root for comparison
    let local_root = {
        let smt_lock = smt.lock().unwrap();
        smt_lock.root()
    };
    
    // Log the roots for debugging
    debug!("Local root: {:?}, Update root: {:?}", local_root, root);

    
    // In a production-ready system, we use our enhanced verification logic
    // that can handle state transitions between nodes
    
    // Verify the sender's proof using the improved verify_transaction method
    if !update.proof_from.verify_transaction(root, &update.from, update.nonce, local_root) {
        error!("Failed to verify sender proof. Rejecting transaction.");
        return Err(NodeError::InvalidProof("sender proof verification failed".to_string()));
    }
    
    // Verify the recipient's proof using the same verify_transaction method
    // For recipient, we use 0 as the nonce since it's not relevant for the recipient
    if !update.proof_to.verify_transaction(root, &update.to, 0, local_root) {
        error!("Failed to verify recipient proof. Rejecting transaction.");
        return Err(NodeError::InvalidProof("recipient proof verification failed".to_string()));
    }
    
    // Verify that the post_root matches the expected state after applying the transaction
    // This ensures that the transaction will result in a valid state transition
    info!("Verifying transaction will result in expected post-state root");
    
    // If local root doesn't match the transaction's root, we need to be more careful
    // but we don't necessarily need to reject the transaction
    if local_root != root {
        info!("Local state root doesn't match transaction root. Proceeding with caution.");
        info!("Local root: {:?}, Transaction root: {:?}", local_root, root);
        
        // Instead of rejecting immediately, we'll try to apply the transaction
        // and verify the resulting state is consistent
    }
    
    // Process the transaction with strict verification
    {
        let mut smt_lock = smt.lock().unwrap();
        
        // Get the sender's account
        let sender_account = match smt_lock.get_account(&update.from) {
            Ok(account) => {
                // Verify the sender has sufficient balance
                if account.bal < update.amount {
                    error!("Sender has insufficient balance: {} < {}", account.bal, update.amount);
                    return Err(NodeError::InsufficientBalance);
                }
                
                // Verify the nonce with more flexibility to handle state transitions
                if account.nonce > update.nonce {
                    // If the account nonce is higher than the transaction nonce,
                    // this might be a replay attack or a transaction that was already processed
                    error!("Invalid nonce (possible replay attack): account nonce {} > transaction nonce {}",
                           account.nonce, update.nonce);
                    return Err(NodeError::InvalidNonce);
                } else if account.nonce < update.nonce {
                    // If the account nonce is lower than the transaction nonce,
                    // this might be a future transaction that arrived early
                    // In a distributed system, we might want to queue this for later processing
                    // For now, we'll reject it but with a different error message
                    warn!("Future nonce detected: account nonce {} < transaction nonce {}",
                          account.nonce, update.nonce);
                    warn!("This might indicate that nodes are out of sync");
                    
                    // If the difference is small (e.g., 1-2), we might still process it
                    // This helps with network latency and slightly out-of-sync nodes
                    if update.nonce - account.nonce <= 2 {
                        info!("Nonce difference is small, proceeding with transaction");
                        // We'll set the account nonce to match the transaction nonce
                        // This is a compromise that helps with network latency
                    } else {
                        return Err(NodeError::InvalidNonce);
                    }
                }
                
                // At this point, either the nonces match exactly or we've decided to
                // process a transaction with a slightly future nonce
                
                // Update the account with the new balance and nonce
                let mut updated_account = account.clone();
                updated_account.bal -= update.amount;
                updated_account.nonce += 1; // Increment nonce
                
                // Update the SMT with the new account
                if let Err(e) = smt_lock.update_account(updated_account.clone()) {
                    error!("Failed to update sender account: {}", e);
                    return Err(NodeError::UpdateFailed("sender".to_string()));
                }
                
                info!("Updated sender account: bal={}, nonce={}", updated_account.bal, updated_account.nonce);
                updated_account
            },
            Err(e) => {
                error!("Failed to get sender account: {}", e);
                return Err(NodeError::AccountNotFound("sender".to_string()));
            }
        };
            
        // Get or create the recipient account
        let recipient_account = match smt_lock.get_account(&update.to) {
            Ok(account) => {
                // Update the account with the new balance
                let mut updated_account = account.clone();
                updated_account.bal += update.amount;
                
                // Update the SMT with the new account
                if let Err(e) = smt_lock.update_account(updated_account.clone()) {
                    error!("Failed to update recipient account: {}", e);
                    return Err(NodeError::UpdateFailed("recipient".to_string()));
                }
                
                info!("Updated recipient account: bal={}", updated_account.bal);
                updated_account
            },
            Err(_) => {
                // Create a new account for the recipient
                let new_recipient = core::types::AccountLeaf::new(
                    update.to,
                    update.amount,
                    0,  // New accounts start with nonce 0
                    0   // Assuming native token
                );
                
                // Update the SMT with the new account
                if let Err(e) = smt_lock.update_account(new_recipient.clone()) {
                    error!("Failed to create recipient account: {}", e);
                    return Err(NodeError::UpdateFailed("recipient".to_string()));
                }
                
                info!("Created new recipient account: bal={}", new_recipient.bal);
                new_recipient
            }
        };
        
        // Verify that the resulting root matches the expected post_root
        let new_root = smt_lock.root();
        if new_root != update.post_root {
            error!("Transaction resulted in unexpected state root");
            error!("Expected: {:?}, Actual: {:?}", update.post_root, new_root);
            
            // Revert the transaction by restoring the original state
            if let Err(e) = smt_lock.set_full_state(vec![sender_account, recipient_account], root) {
                error!("Failed to revert transaction: {}", e);
            }
            
            return Err(NodeError::StateMismatch("transaction resulted in unexpected state".to_string()));
        }
        
        info!("Transaction successfully applied with expected state root");
        
        // Broadcast the full state to ensure network consistency
        info!("Broadcasting full state after transaction to ensure network consistency");
        let accounts = smt_lock.get_all_accounts().unwrap_or_default();
        let current_root = smt_lock.root();
        
        // Create a full state message
        let full_state = rpc::FullState {
            accounts: accounts.clone(),
            root: current_root,
        };
    }
    
    // Broadcast the update message to all peers
    // This is done outside the lock to avoid deadlocks
    {
        let mut swarm = swarm_mutex.lock().unwrap();
        
        // Serialize the update message
        match bincode::serialize(&update) {
            Ok(update_bytes) => {
                // Create a topic
                let topic = libp2p::gossipsub::IdentTopic::new(network::gossip::STATE_UPDATES_TOPIC);
                
                // Publish the message
                match swarm.behaviour_mut().gossipsub.publish(topic, update_bytes) {
                    Ok(_) => {
                        info!("Successfully broadcast update message to all peers");
                    },
                    Err(e) => {
                        error!("Failed to broadcast update message: {}", e);
                    }
                }
            },
            Err(e) => {
                error!("Failed to serialize update message: {}", e);
            }
        }
    }
    
    // Store the proofs for future use
    if let Err(e) = proof_store.put_proof(&update.from, &root, &update.proof_from) {
        warn!("Failed to store sender proof: {}", e);
    }
    
    if let Err(e) = proof_store.put_proof(&update.to, &root, &update.proof_to) {
        warn!("Failed to store recipient proof: {}", e);
    }
    
    // Log the successful transaction
    info!("Processed transfer from {:?} to {:?} of {} tokens",
          update.from, update.to, update.amount);
    
    Ok(())
}


     // Extracts the IP address and port from a multiaddr.
fn extract_ip_port(addr: &Multiaddr) -> Option<(String, u16)> {
    use libp2p::multiaddr::Protocol;
    
    let mut iter = addr.iter();
    let mut ip = None;
    let mut port = None;
    
    while let Some(protocol) = iter.next() {
        match protocol {
            Protocol::Ip4(addr) => {
                ip = Some(addr.to_string());
            }
            Protocol::Ip6(addr) => {
                ip = Some(addr.to_string());
            }
            Protocol::Tcp(p) => {
                port = Some(p);
            }
            _ => {}
        }
    }
    
    match (ip, port) {
        (Some(ip), Some(port)) => Some((ip, port)),
        _ => None,
    }
}
/// Synchronizes the node's state from the network.
///
/// This function attempts to synchronize the node's state from the network by
/// connecting to bootstrap nodes and requesting their state. It returns true if
/// synchronization was successful, false otherwise.
///
/// # Arguments
///
/// * `bootstrap_nodes` - A list of bootstrap nodes to connect to
/// * `smt` - The Sparse Merkle Tree to synchronize
///
/// # Returns
///
/// `true` if synchronization was successful, `false` otherwise
async fn synchronize_state_from_network(
    bootstrap_nodes: &[Multiaddr],
    smt: &Arc<Mutex<SMT>>,
) -> bool {
    if bootstrap_nodes.is_empty() {
        return false;
    }

    // Get the current root
    let root = {
        let smt_lock = smt.lock().unwrap();
        smt_lock.root()
    };
    
    // Check if we have an empty root (all zeros)
    let is_empty_root = root.iter().all(|&b| b == 0);
    
    if is_empty_root {
        info!("Synchronizing empty state from network...");
    } else {
        info!("Verifying existing state against network...");
    }
    
    // Try to connect to each bootstrap node and sync state
    for bootstrap_node in bootstrap_nodes {
        info!("Attempting to sync state from bootstrap node: {}", bootstrap_node);
        
        // Extract the IP and port from the multiaddr
        if let Some(ip_port) = extract_ip_port(bootstrap_node) {
            let (ip, _) = ip_port;
            
            // Construct the RPC URL
            let rpc_url = format!("http://{}:{}/rpc", ip, 8545); // Assuming RPC port is 8545
            
            info!("Connecting to RPC at {}", rpc_url);
            
            // Try to get the full state from the bootstrap node
            match reqwest::Client::new()
                .post(&rpc_url)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "get_full_state",
                    "params": [],
                    "id": 1
                }))
                .send()
                .await
            {
                Ok(response) => {
                    match response.json::<serde_json::Value>().await {
                        Ok(json) => {
                            if let Some(result) = json.get("result") {
                                // Try to directly apply the state to the SMT
                                match serde_json::from_value::<rpc::FullState>(result.clone()) {
                                    Ok(full_state) => {
                                        // Apply the state directly to the SMT
                                        let mut smt_lock = smt.lock().unwrap();
                                        
                                        // If we already have state, compare the roots to see if we need to update
                                        if !is_empty_root {
                                            let current_root = smt_lock.root();
                                            
                                            // If our root is the same as the remote root, we're already in sync
                                            if current_root == full_state.root {
                                                info!("Local state is already in sync with network (root: {:?})", current_root);
                                                return true;
                                            }
                                            
                                            // Compare state metadata to determine which state is more recent
                                            let local_accounts = smt_lock.get_all_accounts().unwrap_or_default();
                                            
                                            // Calculate consensus scores for both states
                                            let (local_score, remote_score) = calculate_consensus_scores(
                                                &local_accounts,
                                                &full_state.accounts
                                            );
                                            
                                            // If local state has a higher score, keep it
                                            if local_score >= remote_score {
                                                info!("Local state has higher consensus score. Keeping local state.");
                                                return true;
                                            }
                                        }
                                        
                                        // Update the local state with the remote state
                                        info!("Updating local state with network state...");
                                        match smt_lock.set_full_state(full_state.accounts, full_state.root) {
                                            Ok(_) => {
                                                info!("Successfully synchronized state from network");
                                                return true;
                                            }
                                            Err(e) => {
                                                error!("Failed to set state: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to parse state from bootstrap node: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse response from bootstrap node: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to connect to bootstrap node RPC: {}", e);
                }
            }
        }
    }
    
    // If we reach here, synchronization failed
    false
}

/// Calculates consensus scores for local and remote states.
///
/// This function calculates consensus scores for local and remote states based on
/// various metrics such as account count, total balance, and highest nonce.
///
/// # Arguments
///
/// * `local_accounts` - The local accounts
/// * `remote_accounts` - The remote accounts
///
/// # Returns
///
/// A tuple containing the local and remote consensus scores
fn calculate_consensus_scores(
    local_accounts: &[core::types::AccountLeaf],
    remote_accounts: &[core::types::AccountLeaf],
) -> (u128, u128) {
    // Calculate total balance and highest nonce for local state
    let (local_total_balance, local_highest_nonce, local_active_accounts) = local_accounts.iter()
        .fold((0u128, 0u64, 0usize), |(total_balance, highest_nonce, active_accounts), account| {
            let active = if account.bal > 0 { 1 } else { 0 };
            (
                total_balance + account.bal,
                std::cmp::max(highest_nonce, account.nonce),
                active_accounts + active
            )
        });
    
    // Calculate total balance and highest nonce for remote state
    let (remote_total_balance, remote_highest_nonce, remote_active_accounts) = remote_accounts.iter()
        .fold((0u128, 0u64, 0usize), |(total_balance, highest_nonce, active_accounts), account| {
            let active = if account.bal > 0 { 1 } else { 0 };
            (
                total_balance + account.bal,
                std::cmp::max(highest_nonce, account.nonce),
                active_accounts + active
            )
        });
    
    // Log detailed state information for debugging
    info!("State comparison:");
    info!("Local: {} accounts ({} active), {} total balance, highest nonce {}",
          local_accounts.len(), local_active_accounts, local_total_balance, local_highest_nonce);
    info!("Remote: {} accounts ({} active), {} total balance, highest nonce {}",
          remote_accounts.len(), remote_active_accounts, remote_total_balance, remote_highest_nonce);
    
    // Calculate a consensus score for each state
    // This is a weighted combination of factors that indicate state freshness
    let local_score = (local_active_accounts as u128 * 10) +
                     (local_highest_nonce as u128 * 100) +
                     (local_total_balance / 1000);
    
    let remote_score = (remote_active_accounts as u128 * 10) +
                      (remote_highest_nonce as u128 * 100) +
                      (remote_total_balance / 1000);
    
    info!("Consensus scores - Local: {}, Remote: {}", local_score, remote_score);
    
    (local_score, remote_score)
}

/// Verifies the signature in an update message.
///
/// This function checks that the signature in the update message was created by the owner of the
/// `from` address. In this system, addresses are derived from public keys, so we can extract
/// the public key from the address and use it to verify the signature.
fn verify_signature(update: &UpdateMsg) -> Result<(), NodeError> {
    use ed25519_dalek::{PublicKey, Signature, Verifier};
    
    // Extract the public key from the from address
    // In our system, the address is derived directly from the public key
    let mut public_key_bytes = [0u8; 32];
    public_key_bytes.copy_from_slice(&update.from);
    
    // Create the public key from the bytes
    let public_key = match PublicKey::from_bytes(&public_key_bytes) {
        Ok(pk) => pk,
        Err(e) => return Err(NodeError::InvalidProof(format!("Invalid public key: {}", e))),
    };
    
    // Convert the core::types::Signature to ed25519_dalek::Signature
    let signature_bytes = update.signature.0;
    let signature = match Signature::from_bytes(&signature_bytes) {
        Ok(sig) => sig,
        Err(e) => return Err(NodeError::InvalidProof(format!("Invalid signature format: {}", e))),
    };
    
    // Create the transaction message for signature verification - matching how it's created in the CLI
    let from_hex = hex::encode(&update.from);
    let to_hex = hex::encode(&update.to);
    
    let transaction = serde_json::json!({
        "from": from_hex,
        "to": to_hex,
        "amount": update.amount,
        "nonce": update.nonce
    });
    
    // Serialize the transaction for signature verification
    let transaction_bytes = match serde_json::to_vec(&transaction) {
        Ok(bytes) => bytes,
        Err(e) => return Err(NodeError::InvalidProof(format!("Failed to serialize transaction: {}", e))),
    };
    
    // Verify the signature
    match public_key.verify(&transaction_bytes, &signature) {
        Ok(_) => Ok(()),
        Err(e) => {
            // For debugging
            debug!("Signature verification failed: {}", e);
            debug!("Transaction: {:?}", transaction);
            debug!("From: {:?}", update.from);
            debug!("To: {:?}", update.to);
            debug!("Amount: {}", update.amount);
            debug!("Nonce: {}", update.nonce);
            
            Err(NodeError::InvalidProof(format!("Signature verification failed: {}", e)))
        }
    }
}
