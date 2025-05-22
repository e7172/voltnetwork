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
    
    // If we have bootstrap nodes and our SMT is empty (new node), try to sync state
    if !bootstrap_nodes.is_empty() {
        let root = {
            let smt_lock = smt.lock().unwrap();
            smt_lock.root()
        };
        
        // Check if we have an empty root (all zeros)
        let is_empty_root = root.iter().all(|&b| b == 0);
        
        if is_empty_root {
            info!("New node detected with empty state. Attempting to sync state from bootstrap nodes...");
            
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

    // Register metrics if enabled
    if opt.metrics {
        register_metrics();
        let metrics_addr = opt.metrics_addr.parse()?;
        metrics::start_metrics_server(metrics_addr).await?;
        info!("Metrics server listening on {}", opt.metrics_addr);
    }

    // Create a channel for broadcasting messages
    let (gossip_tx, mut gossip_rx) = tokio::sync::mpsc::channel::<network::types::MintMsg>(100);
    
    // Start JSON-RPC server if enabled
    if opt.rpc {
        let rpc_addr = opt.rpc_addr.parse()?;
        let smt_clone = smt.clone();
        let proof_store_clone = proof_store.clone();
        
        // Create a shared reference to the gossip sender
        let gossip_tx = Arc::new(Mutex::new(gossip_tx));
        
        rpc::start_rpc_server(rpc_addr, smt_clone, proof_store_clone, local_peer_id, gossip_tx).await?;
        info!("JSON-RPC server listening on {}", opt.rpc_addr);
    }
    
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
                handle_update(update, &smt, &proof_store).await?;
            }
            NetworkEvent::PeerDiscovered(peer_id) => {
                info!("Discovered peer: {}", peer_id);
                metrics::PEER_COUNT.inc();
            }
            NetworkEvent::PeerDisconnected(peer_id) => {
                info!("Disconnected from peer: {}", peer_id);
                metrics::PEER_COUNT.dec();
            }
            NetworkEvent::PeerIdentified(peer_id, addr) => {
                info!("Identified peer {} at {}", peer_id, addr);
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
) -> Result<(), NodeError> {
    debug!("Received update: {}", update);
    metrics::UPDATE_COUNTER.inc();

    // Verify the proofs
    let root = {
        let smt = smt.lock().unwrap();
        smt.root()
    };

    // Verify the sender's proof
    if !update.proof_from.verify(root, &update.from) {
        return Err(NodeError::InvalidProof("sender".to_string()));
    }

    // Verify the recipient's proof
    if !update.proof_to.verify(root, &update.to) {
        return Err(NodeError::InvalidProof("recipient".to_string()));
    }

    // Verify the signature
    verify_signature(&update)?;

    // Update the SMT
    {
        let mut smt = smt.lock().unwrap();
        smt.transfer(&update.from, &update.to, update.amount, update.nonce)?;
        
        // State is automatically persisted to RocksDB by the transfer method
    }

    // Store the updated proofs
    let new_root = {
        let smt = smt.lock().unwrap();
        smt.root()
    };

    // Generate and store new proofs
    {
        let smt = smt.lock().unwrap();

        // Generate and store proof for sender
        let sender_proof = smt.gen_proof(&update.from)?;
        proof_store.put_proof(&update.from, &new_root, &sender_proof)?;

        // Generate and store proof for recipient
        let recipient_proof = smt.gen_proof(&update.to)?;
        proof_store.put_proof(&update.to, &new_root, &recipient_proof)?;
    }

    info!(
        "Processed transfer from {:?} to {:?} of {} tokens",
        update.from, update.to, update.amount
    );
    metrics::TRANSACTION_COUNTER.inc();

    Ok(())
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
        Err(e) => return Err(NodeError::InvalidSignature(format!("Invalid public key: {}", e))),
    };
    
    // Convert the core::types::Signature to ed25519_dalek::Signature
    let signature_bytes = update.signature.0;
    let signature = match Signature::from_bytes(&signature_bytes) {
        Ok(sig) => sig,
        Err(e) => return Err(NodeError::InvalidSignature(format!("Invalid signature format: {}", e))),
    };
    
    // Create the message to verify
    // The message should contain all the data that was signed
    let mut message = Vec::new();
    message.extend_from_slice(&update.from);
    message.extend_from_slice(&update.to);
    
    // Add amount as bytes (little-endian)
    message.extend_from_slice(&update.amount.to_le_bytes());
    
    // Add nonce as bytes (little-endian)
    message.extend_from_slice(&update.nonce.to_le_bytes());
    
    // Add root hash
    message.extend_from_slice(&update.root);
    
    // Verify the signature
    match public_key.verify(&message, &signature) {
        Ok(_) => Ok(()),
        Err(e) => Err(NodeError::InvalidSignature(format!("Signature verification failed: {}", e))),
    }
}
