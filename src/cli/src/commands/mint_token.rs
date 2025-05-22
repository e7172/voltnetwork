//! Mint token command for the CLI wallet.

use crate::config::WalletConfig;
use crate::errors::WalletError;
use crate::wallet::Wallet;
use anyhow::Result;
use core::{proofs::Proof, types::Address};
use std::path::Path;
use tracing::{debug, info};

/// Runs the mint-token command.
pub async fn run<P: AsRef<Path>>(
    config: &WalletConfig,
    wallet_path: P,
    token_id: u64,
    to_hex: &str,
    amount: u128,
) -> Result<String, WalletError> {
    // Load the wallet
    let wallet = match Wallet::load(wallet_path) {
        Ok(wallet) => wallet,
        Err(e) => {
            return Err(WalletError::WalletError(format!(
                "Failed to load wallet: {}",
                e
            )));
        }
    };

    // Parse the recipient address
    let to_bytes = hex::decode(to_hex.trim_start_matches("0x")).map_err(|e| {
        WalletError::InvalidAddress(format!("Invalid recipient address: {}", e))
    })?;

    if to_bytes.len() != 32 {
        return Err(WalletError::InvalidAddress(format!(
            "Invalid recipient address length: {} (expected 32)",
            to_bytes.len()
        )));
    }

    let mut to = [0u8; 32];
    to.copy_from_slice(&to_bytes);

    // Get the issuer address (sender)
    let from = wallet.address()?;
    info!("Minting {} tokens with ID {} to {:?} from issuer {:?}", amount, token_id, to, from);

    // Get the current root from the node
    let root = get_root_from_node(&config.node).await?;
    debug!("Current root: {:?}", root);

    // Get the issuer's proof from the node
    let proof_from = get_proof_from_node(&config.node, &from, token_id).await?;
    debug!("Issuer proof: {:?}", proof_from);

    // Get the recipient's proof from the node
    let proof_to = get_proof_from_node(&config.node, &to, token_id).await?;
    debug!("Recipient proof: {:?}", proof_to);

    // Get the issuer's nonce from the node
    let nonce = get_nonce_from_node(&config.node, &from, token_id).await?;
    debug!("Issuer nonce: {}", nonce);

    // Create the mint message
    let message = core::types::SystemMsg::Mint {
        from,
        to,
        token_id,
        amount,
        nonce,
        signature: core::types::Signature([0u8; 64]), // Will be filled in later
    };

    // Serialize the message for signing
    let message_bytes = bincode::serialize(&message)
        .map_err(|e| WalletError::TransactionError(format!("Failed to serialize message: {}", e)))?;

    // Sign the message
    let signature = wallet.sign(&message_bytes)?;
    let mut signature_bytes = [0u8; 64];
    signature_bytes.copy_from_slice(&signature.to_bytes());

    // Create the final mint message with the signature
    let final_message = match message {
        core::types::SystemMsg::Mint { from, to, token_id, amount, nonce, signature: _ } => {
            core::types::SystemMsg::Mint {
                from,
                to,
                token_id,
                amount,
                nonce,
                signature: core::types::Signature(signature_bytes),
            }
        },
        _ => unreachable!(),
    };

    // Broadcast the mint message to the node
    let tx_hash = broadcast_mint_token_to_node(&config.node, &final_message).await?;
    debug!("Transaction hash: {}", tx_hash);

    Ok(format!("Successfully minted {} tokens with ID {} to {}. Transaction hash: {}", amount, token_id, to_hex, tx_hash))
}

/// Gets the current root from the node.
async fn get_root_from_node(node_url: &str) -> Result<[u8; 32], WalletError> {
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let response = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getRoot",
            "params": []
        }))
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to connect to node: {}", e)))?;

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = response_json.get("error") {
        return Err(WalletError::NetworkError(format!(
            "Node returned error: {}",
            error
        )));
    }

    let root_hex = response_json
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WalletError::NetworkError("Invalid response format".to_string()))?;

    let root_bytes = hex::decode(root_hex.trim_start_matches("0x"))
        .map_err(|e| WalletError::NetworkError(format!("Invalid root hash: {}", e)))?;

    if root_bytes.len() != 32 {
        return Err(WalletError::NetworkError(format!(
            "Invalid root hash length: {} (expected 32)",
            root_bytes.len()
        )));
    }

    let mut root = [0u8; 32];
    root.copy_from_slice(&root_bytes);
    Ok(root)
}

/// Gets a proof from the node for a specific token.
async fn get_proof_from_node(node_url: &str, address: &Address, token_id: u64) -> Result<Proof, WalletError> {
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let address_hex = hex::encode(address);
    
    let response = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "get_proof_with_token",
            "params": [address_hex, token_id]
        }))
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to connect to node: {}", e)))?;

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = response_json.get("error") {
        return Err(WalletError::NetworkError(format!(
            "Node returned error: {}",
            error
        )));
    }

    let proof_json = response_json
        .get("result")
        .ok_or_else(|| WalletError::NetworkError("Invalid response format".to_string()))?;

    // Parse siblings
    let siblings_json = proof_json
        .get("siblings")
        .and_then(|v| v.as_array())
        .ok_or_else(|| WalletError::NetworkError("Invalid proof format".to_string()))?;

    let mut siblings = Vec::with_capacity(siblings_json.len());
    for sibling_json in siblings_json {
        let sibling_hex = sibling_json
            .as_str()
            .ok_or_else(|| WalletError::NetworkError("Invalid sibling format".to_string()))?;

        let sibling_bytes = hex::decode(sibling_hex.trim_start_matches("0x"))
            .map_err(|e| WalletError::NetworkError(format!("Invalid sibling hash: {}", e)))?;

        if sibling_bytes.len() != 32 {
            return Err(WalletError::NetworkError(format!(
                "Invalid sibling hash length: {} (expected 32)",
                sibling_bytes.len()
            )));
        }

        let mut sibling = [0u8; 32];
        sibling.copy_from_slice(&sibling_bytes);
        siblings.push(sibling);
    }

    // Parse leaf hash
    let leaf_hash_hex = proof_json
        .get("leaf_hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WalletError::NetworkError("Invalid proof format".to_string()))?;

    let leaf_hash_bytes = hex::decode(leaf_hash_hex.trim_start_matches("0x"))
        .map_err(|e| WalletError::NetworkError(format!("Invalid leaf hash: {}", e)))?;

    if leaf_hash_bytes.len() != 32 {
        return Err(WalletError::NetworkError(format!(
            "Invalid leaf hash length: {} (expected 32)",
            leaf_hash_bytes.len()
        )));
    }

    let mut leaf_hash = [0u8; 32];
    leaf_hash.copy_from_slice(&leaf_hash_bytes);

    // Parse path
    let path_json = proof_json
        .get("path")
        .and_then(|v| v.as_array())
        .ok_or_else(|| WalletError::NetworkError("Invalid proof format".to_string()))?;

    let mut path = Vec::with_capacity(path_json.len());
    for bit_json in path_json {
        let bit = bit_json
            .as_bool()
            .ok_or_else(|| WalletError::NetworkError("Invalid path format".to_string()))?;
        path.push(bit);
    }

    Ok(Proof::new(siblings, leaf_hash, path, 0)) // Adding zeros_omitted parameter with default value 0
}

/// Gets the nonce for an address and token from the node.
async fn get_nonce_from_node(node_url: &str, address: &Address, token_id: u64) -> Result<u64, WalletError> {
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let address_hex = hex::encode(address);
    
    let response = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "get_nonce_with_token",
            "params": [address_hex, token_id]
        }))
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to connect to node: {}", e)))?;

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = response_json.get("error") {
        return Err(WalletError::NetworkError(format!(
            "Node returned error: {}",
            error
        )));
    }

    let nonce = response_json
        .get("result")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| WalletError::NetworkError("Invalid response format".to_string()))?;

    Ok(nonce)
}

/// Broadcasts a mint token message to the node.
async fn broadcast_mint_token_to_node(
    node_url: &str,
    message: &core::types::SystemMsg,
) -> Result<String, WalletError> {
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    
    // Serialize the message to a hex string
    let message_bytes = bincode::serialize(message)
        .map_err(|e| WalletError::TransactionError(format!("Failed to serialize message: {}", e)))?;
    
    let message_hex = hex::encode(&message_bytes);
    
    let response = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "p3p_mintToken",
            "params": [message_hex]
        }))
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to connect to node: {}", e)))?;

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = response_json.get("error") {
        return Err(WalletError::NetworkError(format!(
            "Node returned error: {}",
            error
        )));
    }

    let tx_hash = response_json
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WalletError::NetworkError("Invalid response format".to_string()))?
        .to_string();

    Ok(tx_hash)
}