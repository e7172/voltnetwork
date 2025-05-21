//! Mint command for the CLI wallet.

use crate::config::WalletConfig;
use crate::errors::WalletError;
use crate::wallet::Wallet;
use anyhow::Result;
use core::{proofs::Proof, types::Address};
use network::types::MintMsg;
use std::path::Path;
use tracing::{debug, info};

/// Runs the mint command.
pub async fn run<P: AsRef<Path>>(
    config: &WalletConfig,
    wallet_path: P,
    to_hex: &str,
    amount: u128,
) -> Result<String, WalletError> {
    info!("Minting {} tokens to {}", amount, to_hex);

    // Load the wallet to get the private key for signing
    let wallet = Wallet::load(&wallet_path)
        .map_err(|e| WalletError::WalletError(format!("Failed to load wallet: {}", e)))?;

    // Get the wallet's address as hex (this should be the treasury address)
    let from_hex = hex::encode(wallet.address()?);

    // Create the message to sign
    let message = format!("mint:{}:{}", to_hex, amount);
    
    // Sign the message
    let signature = wallet.sign(message.as_bytes())
        .map_err(|e| WalletError::TransactionError(format!("Failed to sign message: {}", e)))?;
    
    // Convert the signature to hex
    let signature_hex = hex::encode(signature.to_bytes());

    // Make sure to append /rpc to the node URL
    let rpc_url = if config.node.ends_with("/rpc") {
        config.node.to_string()
    } else {
        format!("{}/rpc", config.node)
    };
    
    let client = reqwest::Client::new();
    
    // Call the mint RPC method with the new parameters
    let response = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "mint",
            "params": [from_hex, signature_hex, to_hex, amount]
        }))
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to connect to node: {}", e)))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw mint response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = response_json.get("error") {
        if !error.is_null() {
            return Err(WalletError::NetworkError(format!(
                "Node returned error: {}",
                error
            )));
        }
    }

    // Check if the mint was successful
    let success = response_json
        .get("result")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if success {
        Ok(format!("Successfully minted {} tokens to {}", amount, to_hex))
    } else {
        Err(WalletError::NetworkError("Failed to mint tokens".to_string()))
    }
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
            "method": "get_root",
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

/// Gets a proof from the node.
async fn get_proof_from_node(node_url: &str, address: &Address) -> Result<Proof, WalletError> {
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
            "method": "get_proof",
            "params": [address_hex]
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

    Ok(Proof::new(siblings, leaf_hash, path))
}

/// Gets the nonce for an address from the node.
async fn get_nonce_from_node(node_url: &str, address: &Address) -> Result<u64, WalletError> {
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
            "method": "get_nonce",
            "params": [address_hex]
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

/// Gets the current total supply from the node.
async fn get_total_supply_from_node(node_url: &str) -> Result<u128, WalletError> {
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
            "method": "get_total_supply",
            "params": []
        }))
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to connect to node: {}", e)))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw total supply response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = response_json.get("error") {
        if !error.is_null() {
            return Err(WalletError::NetworkError(format!(
                "Node returned error: {}",
                error
            )));
        }
    }

    let total_supply_str = response_json
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WalletError::NetworkError("Invalid response format".to_string()))?;

    let total_supply = total_supply_str
        .parse::<u128>()
        .map_err(|e| WalletError::NetworkError(format!("Invalid total supply: {}", e)))?;

    Ok(total_supply)
}

/// Gets the maximum supply from the node.
async fn get_max_supply_from_node(node_url: &str) -> Result<u128, WalletError> {
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
            "method": "get_max_supply",
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

    let max_supply_str = response_json
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WalletError::NetworkError("Invalid response format".to_string()))?;

    let max_supply = max_supply_str
        .parse::<u128>()
        .map_err(|e| WalletError::NetworkError(format!("Invalid max supply: {}", e)))?;

    Ok(max_supply)
}

/// Broadcasts a mint message to the node.
async fn broadcast_mint_to_node(node_url: &str, message: &MintMsg) -> Result<String, WalletError> {
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    
    // Serialize the mint message to a hex string
    let message_bytes = bincode::serialize(message)
        .map_err(|e| WalletError::TransactionError(format!("Failed to serialize message: {}", e)))?;
    
    let message_hex = hex::encode(&message_bytes);
    
    let response = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "broadcast_mint",
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