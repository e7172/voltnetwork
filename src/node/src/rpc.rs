//! JSON-RPC server for the node daemon.

use anyhow::Result;
use core::{proofs::Proof, smt::SMT, types::Address};
use ed25519_dalek::Verifier;
use network::storage::ProofStore;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error, debug};
use warp::{Filter, Rejection, Reply};

/// Full state of the SMT.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FullState {
    /// The accounts in the SMT
    pub accounts: Vec<core::types::AccountLeaf>,
    /// The root hash of the SMT
    pub root: [u8; 32],
}

/// JSON-RPC request.
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    /// JSON-RPC version
    jsonrpc: String,
    /// Method to call
    method: String,
    /// Parameters for the method
    params: serde_json::Value,
    /// Request ID
    id: serde_json::Value,
}

/// JSON-RPC response.
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    /// JSON-RPC version
    jsonrpc: String,
    /// Result of the method call
    result: Option<serde_json::Value>,
    /// Error, if any
    error: Option<JsonRpcError>,
    /// Request ID
    id: serde_json::Value,
}

/// JSON-RPC error.
#[derive(Debug, Serialize)]
struct JsonRpcError {
    /// Error code
    code: i32,
    /// Error message
    message: String,
    /// Additional error data
    data: Option<serde_json::Value>,
}

/// State for the RPC server.
struct RpcState {
    /// The Sparse Merkle Tree
    smt: Arc<Mutex<SMT>>,
    /// The proof store
    proof_store: ProofStore,
    /// The local peer ID
    peer_id: String,
    /// Channel for broadcasting mint messages
    gossip_tx: Arc<Mutex<tokio::sync::mpsc::Sender<network::types::MintMsg>>>,
    /// Channel for broadcasting update messages
    update_tx: Arc<Mutex<tokio::sync::mpsc::Sender<network::types::UpdateMsg>>>,
}

/// Starts the JSON-RPC server.
pub async fn start_rpc_server(
    addr: SocketAddr,
    smt: Arc<Mutex<SMT>>,
    proof_store: ProofStore,
    peer_id: String,
    gossip_tx: Arc<Mutex<tokio::sync::mpsc::Sender<network::types::MintMsg>>>,
    update_tx: Arc<Mutex<tokio::sync::mpsc::Sender<network::types::UpdateMsg>>>,
) -> Result<()> {
    let state = Arc::new(RpcState { smt, proof_store, peer_id, gossip_tx, update_tx });

    let rpc_route = warp::path("rpc")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_rpc);

    tokio::spawn(async move {
        warp::serve(rpc_route).run(addr).await;
    });

    Ok(())
}

/// Provides the RPC state to handlers.
fn with_state(
    state: Arc<RpcState>,
) -> impl Filter<Extract = (Arc<RpcState>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

/// Handles a JSON-RPC request.
async fn handle_rpc(
    request: JsonRpcRequest,
    state: Arc<RpcState>,
) -> Result<impl Reply, Rejection> {
    let id = request.id.clone();

    let result = match request.method.as_str() {
        "getRoot" => handle_get_root(&state),
        "getProof" => handle_get_proof(&request.params, &state),
        "getBalance" => handle_get_balance(&request.params, &state),
        "getBalanceWithToken" => handle_get_balance_with_token(&request.params, &state),
        "getAllBalances" => handle_get_all_balances(&request.params, &state),
        "get_peer_id" => handle_get_peer_id(&state),
        "getNonce" => handle_get_nonce(&request.params, &state),
        "broadcastUpdate" => handle_broadcast_update(&request.params, &state),
        "get_nonce" => handle_get_nonce(&request.params, &state), // Alias for getNonce
        "p3p_issueToken" => handle_issue_token(&request.params, &state),
        "get_proof_with_token" => handle_get_proof_with_token(&request.params, &state),
        "get_nonce_with_token" => handle_get_nonce_with_token(&request.params, &state),
        "p3p_mintToken" => handle_mint_token(&request.params, &state),
        "mint" => handle_mint(&request.params, &state),
        "send" => handle_send(&request.params, &state),
        "get_root" => handle_get_root(&state), // Alias for getRoot
        "get_total_supply" => handle_get_total_supply(&state),
        "get_max_supply" => handle_get_max_supply(&state),
        "broadcast_mint" => handle_broadcast_mint(&request.params, &state),
        "get_full_state" => handle_get_full_state(&state),
        "set_full_state" => handle_set_full_state(&request.params, &state),
        "get_tokens" => handle_get_tokens(&state),
        _ => Err(JsonRpcError {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        }),
    };

    let response = match result {
        Ok(result) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        },
        Err(error) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        },
    };

    Ok(warp::reply::json(&response))
}

/// Handles the getRoot method.
fn handle_get_root(state: &RpcState) -> Result<serde_json::Value, JsonRpcError> {
    let root = {
        let smt = state.smt.lock().unwrap();
        smt.root()
    };

    let root_hex = hex::encode(root);
    Ok(serde_json::json!(root_hex))
}

/// Handles the getProof method.
fn handle_get_proof(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 1 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let address_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid address".to_string(),
        data: None,
    })?;

    // Parse address
    let address_bytes = hex::decode(address_hex.trim_start_matches("0x")).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid address".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if address_bytes.len() != 32 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid address length".to_string(),
            data: None,
        });
    }

    let mut address = [0u8; 32];
    address.copy_from_slice(&address_bytes);

    // Get the current root
    let root = {
        let smt = state.smt.lock().unwrap();
        smt.root()
    };

    // Get the proof
    let proof = {
        let smt = state.smt.lock().unwrap();
        // The gen_proof method should work even for non-existent accounts
        // It will generate a proof for an empty leaf
        smt.gen_proof(&address).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to generate proof".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?
    };

    // Serialize the proof using json! macro to ensure it's not null
    let proof_value = serde_json::to_value(proof).map_err(|e| JsonRpcError {
        code: -32603,
        message: "Internal error".to_string(),
        data: Some(serde_json::to_value(e.to_string()).unwrap()),
    })?;
    
    Ok(serde_json::json!(proof_value))
}

/// Handles the getBalance method.
fn handle_get_balance(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 1 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let address_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid address".to_string(),
        data: None,
    })?;

    // Parse address
    let address_bytes = hex::decode(address_hex.trim_start_matches("0x")).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid address".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if address_bytes.len() != 32 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid address length".to_string(),
            data: None,
        });
    }

    let mut address = [0u8; 32];
    address.copy_from_slice(&address_bytes);

    // Get the account - in production, we need to ensure we're getting the latest state
    let balance = {
        // First, try to get the account from the SMT
        let mut smt = state.smt.lock().unwrap();
        
        // Log the request for debugging
        info!("RPC: Getting balance for address: {:?}", address);
        
        // Try to get the account from the SMT
        match smt.get_account(&address) {
            Ok(account) => {
                info!("RPC: Found account with balance: {}", account.bal);
                account.bal
            },
            Err(e) => {
                // If the account doesn't exist, return a balance of 0
                // This is more user-friendly than returning an error
                warn!("RPC: Account not found: {}", e);
                0
            }
        }
    };

    // Convert the balance to u64 (the CLI expects a u64)
    let balance_u64 = if balance > u64::MAX as u128 {
        u64::MAX // Cap at u64::MAX if the balance is too large
    } else {
        balance as u64
    };

    // Return the balance as a JSON number
    // Make sure to use a format that the CLI can parse
    // Use a direct number value instead of a Number object to ensure it's not null
    Ok(serde_json::json!(balance_u64))
}

/// Handles the get_peer_id method.
fn handle_get_peer_id(state: &RpcState) -> Result<serde_json::Value, JsonRpcError> {
    // Return the peer ID
    Ok(serde_json::to_value(&state.peer_id).map_err(|e| JsonRpcError {
        code: -32603,
        message: "Internal error".to_string(),
        data: Some(serde_json::to_value(e.to_string()).unwrap()),
    })?)
}

/// Handles the getNonce method.
fn handle_get_nonce(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 1 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let address_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid address".to_string(),
        data: None,
    })?;

    // Parse address
    let address_bytes = hex::decode(address_hex.trim_start_matches("0x")).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid address".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if address_bytes.len() != 32 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid address length".to_string(),
            data: None,
        });
    }

    let mut address = [0u8; 32];
    address.copy_from_slice(&address_bytes);

    // Get the account
    let nonce = {
        let mut smt = state.smt.lock().unwrap();
        
        // Log the request for debugging
        info!("RPC: Getting nonce for address: {:?}", address);
        
        match smt.get_account(&address) {
            Ok(account) => {
                info!("RPC: Found account with nonce: {}", account.nonce);
                account.nonce
            },
            Err(e) => {
                // If the account doesn't exist, return a nonce of 0
                // This is more user-friendly than returning an error
                warn!("RPC: Account not found: {}", e);
                0
            }
        }
    };

    // Return the nonce
    Ok(serde_json::json!(nonce))
}

/// Handles the broadcastUpdate method.
fn handle_broadcast_update(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 1 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let update_msg = params[0].clone();

    // Parse the update message
    let update_msg: network::types::UpdateMsg = serde_json::from_value(update_msg).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid update message".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    // Verify the signature
    let message_bytes = bincode::serialize(&network::types::UpdateMsg {
        from: update_msg.from,
        to: update_msg.to,
        token_id: update_msg.token_id,
        amount: update_msg.amount,
        root: update_msg.root,
        post_root: update_msg.post_root,
        proof_from: update_msg.proof_from.clone(),
        proof_to: update_msg.proof_to.clone(),
        nonce: update_msg.nonce,
        signature: core::types::Signature([0u8; 64]), // Empty signature for verification
    })
    .map_err(|e| JsonRpcError {
        code: -32603,
        message: "Failed to serialize message".to_string(),
        data: Some(serde_json::to_value(e.to_string()).unwrap()),
    })?;

    // Verify the signature using ed25519-dalek
    let public_key = ed25519_dalek::PublicKey::from_bytes(&update_msg.from[..32]).map_err(|e| {
        JsonRpcError {
            code: -32603,
            message: "Invalid public key".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    let signature = ed25519_dalek::Signature::from_bytes(&update_msg.signature.0).map_err(|e| {
        JsonRpcError {
            code: -32603,
            message: "Invalid signature".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if let Err(e) = public_key.verify(&message_bytes, &signature) {
        return Err(JsonRpcError {
            code: -32603,
            message: "Invalid signature".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        });
    }

    // Verify the proofs
    {
        let smt = state.smt.lock().unwrap();
        
        // Verify the sender's proof
        if !update_msg.proof_from.verify(update_msg.root, &update_msg.from) {
            return Err(JsonRpcError {
                code: -32603,
                message: "Invalid sender proof".to_string(),
                data: None,
            });
        }

        // Verify the recipient's proof
        if !update_msg.proof_to.verify(update_msg.root, &update_msg.to) {
            return Err(JsonRpcError {
                code: -32603,
                message: "Invalid recipient proof".to_string(),
                data: None,
            });
        }
    }

    // Update the SMT
    {
        let mut smt = state.smt.lock().unwrap();
        
        // Get the sender's account
        let mut sender_account = match smt.get_account_with_token(&update_msg.from, update_msg.token_id) {
            Ok(account) => account,
            Err(_) => {
                return Err(JsonRpcError {
                    code: -32603,
                    message: format!("Sender account not found for token ID {}", update_msg.token_id),
                    data: None,
                });
            }
        };

        // Check the nonce
        if sender_account.nonce != update_msg.nonce {
            return Err(JsonRpcError {
                code: -32603,
                message: format!("Invalid nonce: expected {}, got {}", sender_account.nonce, update_msg.nonce),
                data: None,
            });
        }

        // Check the balance
        if sender_account.bal < update_msg.amount {
            return Err(JsonRpcError {
                code: -32603,
                message: format!("Insufficient balance: {} < {}", sender_account.bal, update_msg.amount),
                data: None,
            });
        }

        // Update the sender's account
        sender_account.bal -= update_msg.amount;
        sender_account.nonce += 1;
        smt.update_account_with_token(sender_account, update_msg.token_id).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to update sender account".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;

        // Get the recipient's account
        let mut recipient_account = match smt.get_account_with_token(&update_msg.to, update_msg.token_id) {
            Ok(account) => account,
            Err(_) => {
                // If the recipient account doesn't exist, create a new one
                core::types::AccountLeaf::new_empty(update_msg.to, update_msg.token_id)
            }
        };

        // Update the recipient's account
        recipient_account.bal += update_msg.amount;
        smt.update_account_with_token(recipient_account, update_msg.token_id).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to update recipient account".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;
    }

    // Generate a transaction hash
    let tx_hash = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(&bincode::serialize(&update_msg).unwrap());
        let result = hasher.finalize();
        hex::encode(result)
    };

    // Return the transaction hash
    Ok(serde_json::json!(tx_hash))
}

/// Handles the get_proof_with_token method.
fn handle_get_proof_with_token(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 2 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let address_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid address".to_string(),
        data: None,
    })?;

    let token_id = params[1].as_u64().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid token ID".to_string(),
        data: None,
    })?;

    // Parse address
    let address_bytes = hex::decode(address_hex.trim_start_matches("0x")).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid address".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if address_bytes.len() != 32 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid address length".to_string(),
            data: None,
        });
    }

    let mut address = [0u8; 32];
    address.copy_from_slice(&address_bytes);

    // Get the proof
    let proof = {
        let smt = state.smt.lock().unwrap();
        smt.gen_proof_with_token(&address, token_id).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to generate proof".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?
    };

    // Serialize the proof
    Ok(serde_json::to_value(proof).map_err(|e| JsonRpcError {
        code: -32603,
        message: "Internal error".to_string(),
        data: Some(serde_json::to_value(e.to_string()).unwrap()),
    })?)
}

/// Handles the get_nonce_with_token method.
fn handle_get_nonce_with_token(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 2 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let address_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid address".to_string(),
        data: None,
    })?;

    let token_id = params[1].as_u64().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid token ID".to_string(),
        data: None,
    })?;

    // Parse address
    let address_bytes = hex::decode(address_hex.trim_start_matches("0x")).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid address".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if address_bytes.len() != 32 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid address length".to_string(),
            data: None,
        });
    }

    let mut address = [0u8; 32];
    address.copy_from_slice(&address_bytes);

    // Get the account
    let nonce = {
        let smt = state.smt.lock().unwrap();
        match smt.get_account_with_token(&address, token_id) {
            Ok(account) => account.nonce,
            Err(_) => {
                // If the account doesn't exist, return a nonce of 0
                // This is more user-friendly than returning an error
                0
            }
        }
    };

    // Return the nonce
    Ok(serde_json::json!(nonce))
}

/// Handles the p3p_issueToken method.
fn handle_issue_token(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 1 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let message_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid message".to_string(),
        data: None,
    })?;

    // Parse the message
    let message_bytes = hex::decode(message_hex).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid message".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    let message: core::types::SystemMsg = bincode::deserialize(&message_bytes).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid message".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    // Process the message
    match message {
        core::types::SystemMsg::IssueToken { issuer, token_id: _, metadata, nonce, signature } => {
            // Verify the signature
            let message_bytes = bincode::serialize(&core::types::SystemMsg::IssueToken {
                issuer,
                token_id: 0, // Will be assigned by the system
                metadata: metadata.clone(),
                nonce,
                signature: core::types::Signature([0u8; 64]), // Empty signature for verification
            })
            .map_err(|e| JsonRpcError {
                code: -32603,
                message: "Failed to serialize message".to_string(),
                data: Some(serde_json::to_value(e.to_string()).unwrap()),
            })?;

            // Verify the signature using ed25519-dalek
            let public_key = ed25519_dalek::PublicKey::from_bytes(&issuer[..32]).map_err(|e| {
                JsonRpcError {
                    code: -32603,
                    message: "Invalid public key".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;

            let signature = ed25519_dalek::Signature::from_bytes(&signature.0).map_err(|e| {
                JsonRpcError {
                    code: -32603,
                    message: "Invalid signature".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;

            if let Err(e) = public_key.verify(&message_bytes, &signature) {
                return Err(JsonRpcError {
                    code: -32603,
                    message: "Invalid signature".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                });
            }

            // Register the token
            let token_id = {
                let mut smt = state.smt.lock().unwrap();
                smt.register_token(&issuer, metadata).map_err(|e| JsonRpcError {
                    code: -32603,
                    message: "Failed to register token".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                })?
            };

            // Return the token ID
            Ok(serde_json::json!(token_id))
        },
        _ => {
            Err(JsonRpcError {
                code: -32602,
                message: "Invalid message type".to_string(),
                data: None,
            })
        }
    }
}

/// Handles the p3p_mintToken method.
fn handle_mint_token(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 1 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let message_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid message".to_string(),
        data: None,
    })?;

    // Parse the message
    let message_bytes = hex::decode(message_hex).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid message".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    let message: core::types::SystemMsg = bincode::deserialize(&message_bytes).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid message".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    // Process the message
    match message {
        core::types::SystemMsg::Mint { from, to, token_id, amount, nonce, signature } => {
            // Verify the signature
            let message_bytes = bincode::serialize(&core::types::SystemMsg::Mint {
                from,
                to,
                token_id,
                amount,
                nonce,
                signature: core::types::Signature([0u8; 64]), // Empty signature for verification
            })
            .map_err(|e| JsonRpcError {
                code: -32603,
                message: "Failed to serialize message".to_string(),
                data: Some(serde_json::to_value(e.to_string()).unwrap()),
            })?;

            // Verify the signature using ed25519-dalek
            let public_key = ed25519_dalek::PublicKey::from_bytes(&from[..32]).map_err(|e| {
                JsonRpcError {
                    code: -32603,
                    message: "Invalid public key".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;

            let signature = ed25519_dalek::Signature::from_bytes(&signature.0).map_err(|e| {
                JsonRpcError {
                    code: -32603,
                    message: "Invalid signature".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;

            if let Err(e) = public_key.verify(&message_bytes, &signature) {
                return Err(JsonRpcError {
                    code: -32603,
                    message: "Invalid signature".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                });
            }

            // Check if the token exists
            {
                let smt = state.smt.lock().unwrap();
                if let Err(e) = smt.get_token(token_id) {
                    return Err(JsonRpcError {
                        code: -32603,
                        message: format!("Token not found: {}", e),
                        data: None,
                    });
                }
            }

            // Mint the tokens
            {
                let mut smt = state.smt.lock().unwrap();
                
                // Get the issuer's account
                let mut issuer_account = match smt.get_account_with_token(&from, token_id) {
                    Ok(account) => account,
                    Err(_) => {
                        // If the issuer account doesn't exist, create a new one
                        core::types::AccountLeaf::new_empty(from, token_id)
                    }
                };

                // Check the nonce
                if issuer_account.nonce != nonce {
                    return Err(JsonRpcError {
                        code: -32603,
                        message: format!("Invalid nonce: expected {}, got {}", issuer_account.nonce, nonce),
                        data: None,
                    });
                }

                // Update the issuer's account
                issuer_account.nonce += 1;
                smt.update_account_with_token(issuer_account, token_id).map_err(|e| JsonRpcError {
                    code: -32603,
                    message: "Failed to update issuer account".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                })?;

                // Get the recipient's account
                let mut recipient_account = match smt.get_account_with_token(&to, token_id) {
                    Ok(account) => account,
                    Err(_) => {
                        // If the recipient account doesn't exist, create a new one
                        core::types::AccountLeaf::new_empty(to, token_id)
                    }
                };

                // Update the recipient's account
                recipient_account.bal += amount;
                smt.update_account_with_token(recipient_account, token_id).map_err(|e| JsonRpcError {
                    code: -32603,
                    message: "Failed to update recipient account".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                })?;
            }

            // Generate a transaction hash
            let tx_hash = {
                let mut hasher = sha2::Sha256::new();
                hasher.update(&message_bytes);
                let result = hasher.finalize();
                hex::encode(result)
            };

            // Return the transaction hash
            Ok(serde_json::json!(tx_hash))
        },
        _ => {
            Err(JsonRpcError {
                code: -32602,
                message: "Invalid message type".to_string(),
                data: None,
            })
        }
    }
}

/// Handles the mint method.
fn handle_mint(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    // Check if we have the right number of parameters
    // We need: [from_address, from_signature, to_address, amount]
    if params.len() != 4 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params. Expected [from_address, from_signature, to_address, amount]".to_string(),
            data: None,
        });
    }

    // Parse from address (must be an authorized minter)
    let from_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid from address".to_string(),
        data: None,
    })?;

    let from_bytes = hex::decode(from_hex.trim_start_matches("0x")).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid from address".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if from_bytes.len() != 32 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid from address length".to_string(),
            data: None,
        });
    }

    let mut from = [0u8; 32];
    from.copy_from_slice(&from_bytes);

    // Parse signature
    let signature_hex = params[1].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid signature".to_string(),
        data: None,
    })?;

    let signature_bytes = hex::decode(signature_hex).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid signature".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if signature_bytes.len() != 64 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid signature length".to_string(),
            data: None,
        });
    }

    let mut signature = [0u8; 64];
    signature.copy_from_slice(&signature_bytes);

    // Parse to address
    let to_hex = params[2].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid to address".to_string(),
        data: None,
    })?;

    let to_bytes = hex::decode(to_hex.trim_start_matches("0x")).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid to address".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if to_bytes.len() != 32 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid to address length".to_string(),
            data: None,
        });
    }

    let mut to = [0u8; 32];
    to.copy_from_slice(&to_bytes);

    // Parse amount
    let amount = params[3].as_u64().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid amount".to_string(),
        data: None,
    })?;

    // Create the message that was signed
    let message = format!("mint:{}:{}", to_hex, amount);
    let message_bytes = message.as_bytes();

    // Verify the signature
    let public_key = ed25519_dalek::PublicKey::from_bytes(&from).map_err(|e| {
        JsonRpcError {
            code: -32603,
            message: "Invalid public key".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    let ed_signature = ed25519_dalek::Signature::from_bytes(&signature).map_err(|e| {
        JsonRpcError {
            code: -32603,
            message: "Invalid signature".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if let Err(e) = public_key.verify(message_bytes, &ed_signature) {
        return Err(JsonRpcError {
            code: -32603,
            message: "Invalid signature".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        });
    }

    // Check if the from address is authorized to mint
    // For now, we'll use a simple check - only the treasury address can mint
    // This would be a configurable list of authorized minters
    let wallet_address = hex::decode("dcc80a50e84955049514913bd424ce6cbdff2bca048c612ab9eecbc7d703fa7e").unwrap_or_default();
    let mut treasury_address = [0u8; 32];
    if wallet_address.len() == 32 {
        treasury_address.copy_from_slice(&wallet_address);
    }
    
    if from != treasury_address {
        return Err(JsonRpcError {
            code: -32603,
            message: "Unauthorized: Only the treasury can mint tokens".to_string(),
            data: None,
        });
    }

    // Mint tokens and prepare for broadcasting
    let (root, proof_from, proof_to, nonce) = {
        let mut smt = state.smt.lock().unwrap();
        
        // Get the treasury account (from address)
        let mut treasury_account = match smt.get_account(&from) {
            Ok(account) => account,
            Err(_) => {
                // If the treasury account doesn't exist, create a new one
                core::types::AccountLeaf::new_empty(from, 0)
            }
        };
        
        // Get the current nonce for the treasury account
        let nonce = treasury_account.nonce;
        
        // Increment the nonce for the treasury account
        treasury_account.nonce += 1;
        smt.update_account(treasury_account).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to update treasury account".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;
        
        // Get the recipient's account
        let mut recipient_account = match smt.get_account(&to) {
            Ok(account) => account,
            Err(_) => {
                // If the recipient account doesn't exist, create a new one
                core::types::AccountLeaf::new_empty(to, 0)
            }
        };

        // Update the recipient's account
        recipient_account.bal += amount as u128;
        smt.update_account(recipient_account).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to update recipient account".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;
        
        // Get the current root
        let root = smt.root();
        
        // Generate proofs for both accounts
        let proof_from = smt.gen_proof(&from).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to generate proof for treasury".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;
        
        let proof_to = smt.gen_proof(&to).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to generate proof for recipient".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;
        
        (root, proof_from, proof_to, nonce)
    };
    
    // Create a MintMsg for broadcasting
    let mint_msg = network::types::MintMsg {
        from,
        to,
        token_id: 0, // Use native token (token_id = 0) for regular mint
        amount: amount as u128,
        root,
        proof_from: proof_from.clone(),
        proof_to: proof_to.clone(),
        nonce,
        signature: core::types::Signature(signature),
    };
    
    // Broadcast the mint message via channel
    let gossip_tx = state.gossip_tx.lock().unwrap();
    if let Err(e) = gossip_tx.try_send(mint_msg.clone()) {
        return Err(JsonRpcError {
            code: -32603,
            message: format!("Failed to broadcast mint message: {}", e),
            data: None,
        });
    }
    
    // Store the proofs in the proof store
    state.proof_store.put_proof(&from, &root, &proof_from)
        .map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to store proof for treasury".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;
    
    state.proof_store.put_proof(&to, &root, &proof_to)
        .map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to store proof for recipient".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;

    // Generate a transaction hash
    let tx_hash = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(&bincode::serialize(&mint_msg).unwrap_or_default());
        let result = hasher.finalize();
        hex::encode(result)
    };

    // Return the transaction hash
    Ok(serde_json::json!(tx_hash))
}

/// Handles the get_total_supply method.
fn handle_get_total_supply(state: &RpcState) -> Result<serde_json::Value, JsonRpcError> {
    // We would need to calculate the total supply from the SMT
    // For now, we'll return a fixed value
    let total_supply = {
        // Since we can't access the accounts directly, we'll return a fixed value
        // We would need to use a method provided by the SMT
        1_000_000_000u128
    };
    
    // Return the total supply as a string to avoid JSON number precision issues
    Ok(serde_json::json!(total_supply.to_string()))
}

/// Handles the get_max_supply method.
fn handle_get_max_supply(_state: &RpcState) -> Result<serde_json::Value, JsonRpcError> {
    // This would be configurable
    // For now, we'll return a fixed value
    let max_supply = u128::MAX;
    
    // Return the max supply as a string to avoid JSON number precision issues
    Ok(serde_json::json!(max_supply.to_string()))
}

/// Handles the broadcast_mint method.
fn handle_broadcast_mint(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 1 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let message_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid message".to_string(),
        data: None,
    })?;

    // Parse the message
    let message_bytes = hex::decode(message_hex).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid message".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    let message: network::types::MintMsg = bincode::deserialize(&message_bytes).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid message".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    // Verify the signature
    let message_bytes = bincode::serialize(&network::types::MintMsg {
        from: message.from,
        to: message.to,
        token_id: message.token_id,
        amount: message.amount,
        root: message.root,
        proof_from: message.proof_from.clone(),
        proof_to: message.proof_to.clone(),
        nonce: message.nonce,
        signature: core::types::Signature([0u8; 64]), // Empty signature for verification
    })
    .map_err(|e| JsonRpcError {
        code: -32603,
        message: "Failed to serialize message".to_string(),
        data: Some(serde_json::to_value(e.to_string()).unwrap()),
    })?;

    // Verify the signature using ed25519-dalek
    let public_key = ed25519_dalek::PublicKey::from_bytes(&message.from[..32]).map_err(|e| {
        JsonRpcError {
            code: -32603,
            message: "Invalid public key".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    let signature = ed25519_dalek::Signature::from_bytes(&message.signature.0).map_err(|e| {
        JsonRpcError {
            code: -32603,
            message: "Invalid signature".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if let Err(e) = public_key.verify(&message_bytes, &signature) {
        return Err(JsonRpcError {
            code: -32603,
            message: "Invalid signature".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        });
    }

    // Verify the proofs
    {
        let smt = state.smt.lock().unwrap();
        
        // Verify the sender's proof
        if !message.proof_from.verify(message.root, &message.from) {
            return Err(JsonRpcError {
                code: -32603,
                message: "Invalid sender proof".to_string(),
                data: None,
            });
        }

        // Verify the recipient's proof
        if !message.proof_to.verify(message.root, &message.to) {
            return Err(JsonRpcError {
                code: -32603,
                message: "Invalid recipient proof".to_string(),
                data: None,
            });
        }
    }

    // Update the SMT
    {
        let mut smt = state.smt.lock().unwrap();
        
        // Get the sender's account (treasury)
        let mut sender_account = match smt.get_account(&message.from) {
            Ok(account) => account,
            Err(_) => {
                return Err(JsonRpcError {
                    code: -32603,
                    message: "Sender account not found".to_string(),
                    data: None,
                });
            }
        };

        // Check the nonce
        if sender_account.nonce != message.nonce {
            return Err(JsonRpcError {
                code: -32603,
                message: format!("Invalid nonce: expected {}, got {}", sender_account.nonce, message.nonce),
                data: None,
            });
        }

        // Update the sender's account (treasury)
        sender_account.nonce += 1;
        smt.update_account_with_token(sender_account, message.token_id).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to update sender account".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;

        // Get the recipient's account
        let mut recipient_account = match smt.get_account(&message.to) {
            Ok(account) => account,
            Err(_) => {
                // If the recipient account doesn't exist, create a new one
                core::types::AccountLeaf::new_empty(message.to, 0)
            }
            
          
        };

        // Update the recipient's account
        recipient_account.bal += message.amount;
        smt.update_account_with_token(recipient_account, message.token_id).map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to update recipient account".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;
    }

    // Generate a transaction hash
    let tx_hash = {
        let mut hasher = sha2::Sha256::new();
        hasher.update(&message_bytes);
        let result = hasher.finalize();
        hex::encode(result)
    };

    // Return the transaction hash
    Ok(serde_json::json!(tx_hash))
}


fn handle_send(
            params: &serde_json::Value,
            state: &RpcState,
        ) -> Result<serde_json::Value, JsonRpcError> {
            // Parse parameters
            let params = params
                .as_array()
                .ok_or_else(|| JsonRpcError {
                    code: -32602,
                    message: "Invalid params".to_string(),
                    data: None,
                })?;
        
            if params.len() != 6 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: format!("Expected 6 parameters, got {}", params.len()),
                    data: None,
                });
            }
        
            // Parse from address
            let from_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Invalid from address".to_string(),
                data: None,
            })?;
        
            let from_bytes = hex::decode(from_hex.trim_start_matches("0x")).map_err(|e| {
                JsonRpcError {
                    code: -32602,
                    message: "Invalid from address".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;
        
            if from_bytes.len() != 32 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Invalid from address length".to_string(),
                    data: None,
                });
            }
        
            let mut from = [0u8; 32];
            from.copy_from_slice(&from_bytes);
        
            // Parse to address
            let to_hex = params[1].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Invalid to address".to_string(),
                data: None,
            })?;
        
            let to_bytes = hex::decode(to_hex.trim_start_matches("0x")).map_err(|e| {
                JsonRpcError {
                    code: -32602,
                    message: "Invalid to address".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;
        
            if to_bytes.len() != 32 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Invalid to address length".to_string(),
                    data: None,
                });
            }
        
            let mut to = [0u8; 32];
            to.copy_from_slice(&to_bytes);
        
            // Parse token_id
            let token_id = params[2].as_u64().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Invalid token ID".to_string(),
                data: None,
            })?;
            
            // Parse amount
            let amount = params[3].as_u64().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Invalid amount".to_string(),
                data: None,
            })? as u128;
        
            // Parse nonce
            let nonce = params[4].as_u64().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Invalid nonce".to_string(),
                data: None,
            })?;
        
            // Parse signature
            let signature_hex = params[5].as_str().ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Invalid signature".to_string(),
                data: None,
            })?;
        
            let signature_bytes = hex::decode(signature_hex).map_err(|e| {
                JsonRpcError {
                    code: -32602,
                    message: "Invalid signature".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;
        
            if signature_bytes.len() != 64 {
                return Err(JsonRpcError {
                    code: -32602,
                    message: "Invalid signature length".to_string(),
                    data: None,
                });
            }
        
            let mut signature = [0u8; 64];
            signature.copy_from_slice(&signature_bytes);
        
            // Create the transaction message for signature verification
            let transaction = serde_json::json!({
                "from": from_hex,
                "to": to_hex.trim_start_matches("0x"),
                "token_id": token_id,
                "amount": amount,
                "nonce": nonce
            });
        
            // Serialize the transaction for signature verification
            let transaction_bytes = serde_json::to_vec(&transaction).map_err(|e| {
                JsonRpcError {
                    code: -32603,
                    message: "Failed to serialize transaction".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;
        
            // Verify the signature
            let public_key = ed25519_dalek::PublicKey::from_bytes(&from).map_err(|e| {
                JsonRpcError {
                    code: -32603,
                    message: "Invalid public key".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;
        
            let ed_signature = ed25519_dalek::Signature::from_bytes(&signature).map_err(|e| {
                JsonRpcError {
                    code: -32603,
                    message: "Invalid signature".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                }
            })?;
        
            if let Err(e) = public_key.verify(&transaction_bytes, &ed_signature) {
                return Err(JsonRpcError {
                    code: -32603,
                    message: "Invalid signature".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                });
            }
       
           
            
            /// Handles the set_full_state method.
        
            // Get the current root and generate proofs
            let (root, proof_from, proof_to) = {
                let smt = state.smt.lock().unwrap();
                let root = smt.root();
                
                // Generate proofs for both accounts
                let proof_from = smt.gen_proof_with_token(&from, token_id).map_err(|e| JsonRpcError {
                    code: -32603,
                    message: "Failed to generate sender proof".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                })?;
                
                let proof_to = smt.gen_proof_with_token(&to, token_id).map_err(|e| JsonRpcError {
                    code: -32603,
                    message: "Failed to generate recipient proof".to_string(),
                    data: Some(serde_json::to_value(e.to_string()).unwrap()),
                })?;
                
                (root, proof_from, proof_to)
            };
        
            // Update the SMT
            {
                let mut smt = state.smt.lock().unwrap();
                
                // Get the sender's account
                let mut sender_account = match smt.get_account_with_token(&from, token_id) {
                    Ok(account) => account,
                    Err(_) => {
                        return Err(JsonRpcError {
                            code: -32603,
                            message: format!("Sender account not found for token ID {}", token_id),
                            data: None,
                        });
                    }
                };
        
                // Check the nonce
                if sender_account.nonce != nonce {
                    return Err(JsonRpcError {
                        code: -32603,
                        message: format!("Invalid nonce: expected {}, got {}", sender_account.nonce, nonce),
                        data: None,
                    });
                }
        
                // Check the balance
                if sender_account.bal < amount {
                    return Err(JsonRpcError {
                        code: -32603,
                        message: format!("Insufficient balance: {} < {}", sender_account.bal, amount),
                        data: None,
                    });
                }
        
                // Update the sender's account
                sender_account.bal -= amount;
                sender_account.nonce += 1;
                smt.update_account_with_token(sender_account, token_id).map_err(|e| {
                    JsonRpcError {
                        code: -32603,
                        message: "Failed to update sender account".to_string(),
                        data: Some(serde_json::to_value(e.to_string()).unwrap()),
                    }
                })?;
        
                // Get or create the recipient's account
                let mut recipient_account = match smt.get_account_with_token(&to, token_id) {
                    Ok(account) => account,
                    Err(_) => {
                        // If the account doesn't exist, create a new one
                        core::types::AccountLeaf {
                            addr: to,
                            bal: 0,
                            nonce: 0,
                            token_id: token_id,
                        }
                    }
                };
        
                // Update the recipient's account
                recipient_account.bal += amount;
                smt.update_account_with_token(recipient_account, token_id).map_err(|e| {
                    JsonRpcError {
                        code: -32603,
                        message: "Failed to update recipient account".to_string(),
                        data: Some(serde_json::to_value(e.to_string()).unwrap()),
                    }
                })?;
            }

            // Create an UpdateMsg to broadcast to other nodes
            let update_msg = network::types::UpdateMsg {
                from,
                to,
                token_id,
                amount,
                root,
                post_root: root, // Using the same root as post_root for now
                proof_from,
                proof_to,
                nonce,
                signature: core::types::Signature(signature),
            };

            // Broadcast the update to other nodes using the update_tx channel
            if let Err(e) = state.update_tx.lock().unwrap().try_send(update_msg) {
                // Log the error but don't fail the transaction
                tracing::error!("Failed to broadcast update: {}", e);
            } else {
                tracing::info!("Successfully queued transaction update for broadcast");
            }
        
            // Generate a transaction hash
            let mut hasher = sha2::Sha256::new();
            hasher.update(&from);
            hasher.update(&to);
            hasher.update(&token_id.to_be_bytes());
            hasher.update(&amount.to_be_bytes());
            hasher.update(&nonce.to_be_bytes());
            hasher.update(&signature);
            let tx_hash = hasher.finalize();
            let tx_hash_hex = hex::encode(tx_hash);
        
            // Return the transaction hash
            Ok(serde_json::json!(tx_hash_hex))
        }



      
            // Using the FullState struct defined at the top of the file

            
fn handle_set_full_state(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;
    
    if params.len() != 1 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }
    
    // Parse the full state
    let full_state: FullState = serde_json::from_value(params[0].clone()).map_err(|e| JsonRpcError {
        code: -32602,
        message: "Invalid full state".to_string(),
        data: Some(serde_json::to_value(e.to_string()).unwrap()),
    })?;
    
    // Get the current root
    let current_root = {
        let smt = state.smt.lock().unwrap();
        smt.root()
    };
    
    // Only update if the new root is different from the current root
    // This prevents unnecessary updates and potential conflicts
    if current_root != [0u8; 32] && current_root != full_state.root {
        return Err(JsonRpcError {
            code: -32603,
            message: "Cannot update non-empty state with different root".to_string(),
            data: None,
        });
    }
    
    // Update the SMT with all accounts
    {
        let mut smt = state.smt.lock().unwrap();
        
        // Reset the SMT if it's not empty
        if current_root != [0u8; 32] {
            *smt = SMT::new_zero();
        }
        
        // Add all accounts to the SMT
        for account in &full_state.accounts {
            smt.update_account(account.clone()).map_err(|e| JsonRpcError {
                code: -32603,
                message: "Failed to update account".to_string(),
                data: Some(serde_json::to_value(e.to_string()).unwrap()),
            })?;
        }
        
        // Verify that the root matches
        let new_root = smt.root();
        if new_root != full_state.root {
            return Err(JsonRpcError {
                code: -32603,
                message: "Root mismatch after updating accounts".to_string(),
                data: None,
            });
        }
        
        // Generate and store proofs for all accounts
        for account in &full_state.accounts {
            let proof = smt.gen_proof(&account.addr).map_err(|e| JsonRpcError {
                code: -32603,
                message: "Failed to generate proof".to_string(),
                data: Some(serde_json::to_value(e.to_string()).unwrap()),
            })?;
            
            state.proof_store.put_proof(&account.addr, &new_root, &proof).map_err(|e| JsonRpcError {
                code: -32603,
                message: "Failed to store proof".to_string(),
                data: Some(serde_json::to_value(e.to_string()).unwrap()),
            })?;
        }
    }
    
    // Return success
    Ok(serde_json::json!(true))
}

/// Handles the get_full_state method.
fn handle_get_full_state(state: &RpcState) -> Result<serde_json::Value, JsonRpcError> {
    // Get all accounts from the SMT
    // Log the request for debugging
    info!("RPC: Getting full state");
    
    let (accounts, root) = {
        let mut smt = state.smt.lock().unwrap();
        
        // Get all accounts
        let accounts = smt.get_all_accounts().map_err(|e| {
            error!("RPC: Failed to get accounts: {}", e);
            JsonRpcError {
                code: -32603,
                message: "Failed to get accounts".to_string(),
                data: Some(serde_json::to_value(e.to_string()).unwrap()),
            }
        })?;
        
        // Get the current root
        let root = smt.root();
        
        info!("RPC: Retrieved {} accounts with root {:?}", accounts.len(), root);
        
        (accounts, root)
    };
    
    // Create the full state
    let full_state = FullState {
        accounts,
        root,
    };
    
    // Serialize the full state
    let full_state_json = serde_json::to_value(full_state).map_err(|e| JsonRpcError {
        code: -32603,
        message: "Failed to serialize full state".to_string(),
        data: Some(serde_json::to_value(e.to_string()).unwrap()),
    })?;
    
    Ok(full_state_json)
}

/// Handles the getBalanceWithToken method.
fn handle_get_balance_with_token(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 2 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let address_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid address".to_string(),
        data: None,
    })?;

    let token_id = params[1].as_u64().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid token ID".to_string(),
        data: None,
    })?;

    // Parse address
    let address_bytes = hex::decode(address_hex.trim_start_matches("0x")).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid address".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if address_bytes.len() != 32 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid address length".to_string(),
            data: None,
        });
    }

    let mut address = [0u8; 32];
    address.copy_from_slice(&address_bytes);

    // Get the account with the specified token
    let balance = {
        let mut smt = state.smt.lock().unwrap();
        
        // Log the request for debugging
        info!("RPC: Getting balance for address: {:?} with token ID: {}", address, token_id);
        
        // Try to get the account from the SMT
        match smt.get_account_with_token(&address, token_id) {
            Ok(account) => {
                info!("RPC: Found account with balance: {}", account.bal);
                account.bal
            },
            Err(e) => {
                // If the account doesn't exist, return a balance of 0
                // This is more user-friendly than returning an error
                warn!("RPC: Account not found: {}", e);
                0
            }
        }
    };

    // Convert the balance to u64 (the CLI expects a u64)
    let balance_u64 = if balance > u64::MAX as u128 {
        u64::MAX // Cap at u64::MAX if the balance is too large
    } else {
        balance as u64
    };

    // Return the balance as a JSON number
    Ok(serde_json::json!(balance_u64))
}

/// Handles the getAllBalances method.
fn handle_get_all_balances(
    params: &serde_json::Value,
    state: &RpcState,
) -> Result<serde_json::Value, JsonRpcError> {
    // Parse parameters
    let params = params
        .as_array()
        .ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        })?;

    if params.len() != 1 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        });
    }

    let address_hex = params[0].as_str().ok_or_else(|| JsonRpcError {
        code: -32602,
        message: "Invalid address".to_string(),
        data: None,
    })?;

    // Parse address
    let address_bytes = hex::decode(address_hex.trim_start_matches("0x")).map_err(|e| {
        JsonRpcError {
            code: -32602,
            message: "Invalid address".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        }
    })?;

    if address_bytes.len() != 32 {
        return Err(JsonRpcError {
            code: -32602,
            message: "Invalid address length".to_string(),
            data: None,
        });
    }

    let mut address = [0u8; 32];
    address.copy_from_slice(&address_bytes);

    // Get all accounts for this address
    let balances = {
        let smt = state.smt.lock().unwrap();
        
        // Log the request for debugging
        info!("RPC: Getting all balances for address: {:?}", address);
        
        // Get all accounts
        let accounts = smt.get_all_accounts().map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to get accounts".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;
        
        // Filter accounts for this address
        let mut balances = Vec::new();
        for account in accounts {
            if account.addr == address {
                balances.push(serde_json::json!({
                    "token_id": account.token_id,
                    "balance": account.bal,
                }));
            }
        }
        
        info!("RPC: Found {} token balances for address {:?}", balances.len(), address);
        
        balances
    };

    // Return the balances as a JSON array
    Ok(serde_json::json!(balances))
}

/// Handles the get_tokens method.
fn handle_get_tokens(state: &RpcState) -> Result<serde_json::Value, JsonRpcError> {
    // Get all tokens from the SMT
    let tokens = {
        let smt = state.smt.lock().unwrap();
        
        // Log the request for debugging
        info!("RPC: Getting all tokens");
        
        // Get all tokens
        let mut tokens = Vec::new();
        
        // Get all accounts to find all token IDs
        let accounts = smt.get_all_accounts().map_err(|e| JsonRpcError {
            code: -32603,
            message: "Failed to get accounts".to_string(),
            data: Some(serde_json::to_value(e.to_string()).unwrap()),
        })?;
        
        // Extract unique token IDs
        let mut token_ids = std::collections::HashSet::new();
        for account in &accounts {
            token_ids.insert(account.token_id);
        }
        
        // Get token info for each token ID
        for token_id in token_ids {
            match smt.get_token(token_id) {
                Ok(token_info) => {
                    tokens.push(serde_json::json!({
                        "token_id": token_info.token_id,
                        "issuer": hex::encode(token_info.issuer),
                        "metadata": token_info.metadata,
                        "total_supply": token_info.total_supply,
                    }));
                },
                Err(e) => {
                    warn!("RPC: Failed to get token info for token ID {}: {}", token_id, e);
                }
            }
        }
        
        info!("RPC: Found {} tokens", tokens.len());
        
        tokens
    };

    // Return the tokens as a JSON array
    Ok(serde_json::json!(tokens))
}