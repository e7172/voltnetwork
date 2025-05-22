//! Send command for the CLI wallet.

use crate::config::WalletConfig;
use crate::errors::WalletError;
use crate::wallet::Wallet;
use anyhow::Result;
use core::{proofs::Proof, types::Address};
use network::types::UpdateMsg;
use std::path::Path;
use tracing::{debug, info};

/// Runs the send command.
pub async fn run<P: AsRef<Path>>(
    config: &WalletConfig,
    wallet_path: P,
    to_hex: &str,
    token_id: u64,
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

    // Get the sender address
    let from = wallet.address()?;
    let from_hex = hex::encode(&from);
    info!("Sending {} tokens with ID {} from {} to {}", amount, token_id, from_hex, to_hex);

    // Verify that the sender has enough balance
    let balance = get_balance_with_token_from_node(&config.node, &from, token_id).await?;
    if balance < amount {
        return Err(WalletError::InsufficientBalance(format!(
            "Insufficient balance: {} < {} for token ID {}",
            balance, amount, token_id
        )));
    }
    debug!("Sender balance for token {}: {}", token_id, balance);

    // Get the current nonce
    let nonce = get_nonce_with_token_from_node(&config.node, &from, token_id).await?;
    debug!("Sender nonce for token {}: {}", token_id, nonce);

    // Create a transaction message
    let transaction = serde_json::json!({
        "from": from_hex,
        "to": to_hex.trim_start_matches("0x"),
        "token_id": token_id,
        "amount": amount,
        "nonce": nonce
    });

    // Serialize the transaction for signing
    let transaction_bytes = serde_json::to_vec(&transaction)
        .map_err(|e| WalletError::TransactionError(format!("Failed to serialize transaction: {}", e)))?;

    // Sign the transaction
    let signature = wallet.sign(&transaction_bytes)?;
    let signature_hex = hex::encode(signature.to_bytes());

    // Make sure to append /rpc to the node URL
    let rpc_url = if config.node.ends_with("/rpc") {
        config.node.to_string()
    } else {
        format!("{}/rpc", config.node)
    };
    
    let client = reqwest::Client::new();
    
    // Call the send RPC method on the node
    let response = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "send",
            "params": [from_hex, to_hex, token_id, amount, nonce, signature_hex]
        }))
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to connect to node: {}", e)))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw send response: {}", response_text);
    
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

    // Get the transaction hash from the result
    let tx_hash = response_json
        .get("result")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    Ok(format!("Successfully sent {} tokens with ID {} to {}. Transaction hash: {}", amount, token_id, to_hex, tx_hash))
}

/// Gets the current root from the node.
async fn get_root_from_node(node_url: &str) -> Result<[u8; 32], WalletError> {
    // Create the JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getRoot",
        "params": [],
        "id": 1
    });

    // Send the request to the node
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let response = client
        .post(&rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(e.to_string()))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw root response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    // Check for errors
    if let Some(error) = response.get("error") {
        if !error.is_null() {
            return Err(WalletError::NodeRequestFailed(
                error.to_string(),
            ));
        }
    }

    // Get the root
    let root_hex = response
        .get("result")
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("No result in response: {}", response_text)))?
        .as_str()
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("Invalid root: {}", response_text)))?;

    // Parse the root
    let root_bytes = hex::decode(root_hex).map_err(|e| {
        WalletError::NodeRequestFailed(format!("Invalid root: {}", e))
    })?;

    if root_bytes.len() != 32 {
        return Err(WalletError::NodeRequestFailed(format!(
            "Invalid root length: {} (expected 32)",
            root_bytes.len()
        )));
    }

    let mut root = [0u8; 32];
    root.copy_from_slice(&root_bytes);

    Ok(root)
}

/// Gets a proof for an address from the node.
async fn get_proof_from_node(node_url: &str, address: &Address) -> Result<Proof, WalletError> {
    // Create the JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getProof",
        "params": [hex::encode(address)],
        "id": 1
    });

    // Send the request to the node
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let response = client
        .post(&rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(e.to_string()))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw proof response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    // Check for errors
    if let Some(error) = response.get("error") {
        if !error.is_null() {
            return Err(WalletError::NodeRequestFailed(
                error.to_string(),
            ));
        }
    }

    // Get the proof
    let proof_json = response
        .get("result")
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("No result in response: {}", response_text)))?;

    // Deserialize the proof
    let proof: Proof = serde_json::from_value(proof_json.clone()).map_err(|e| {
        WalletError::NodeRequestFailed(format!("Invalid proof: {}", e))
    })?;

    Ok(proof)
}

/// Gets the nonce for an address from the node.
async fn get_nonce_from_node(node_url: &str, address: &Address) -> Result<u64, WalletError> {
    // Create the JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getNonce",
        "params": [hex::encode(address)],
        "id": 1
    });

    // Send the request to the node
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let response = client
        .post(&rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(e.to_string()))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw nonce response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    // Check for errors
    if let Some(error) = response.get("error") {
        if !error.is_null() {
            return Err(WalletError::NodeRequestFailed(
                error.to_string(),
            ));
        }
    }

    // Get the nonce
    let nonce = response
        .get("result")
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("No result in response: {}", response_text)))?
        .as_u64()
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("Invalid nonce: {}", response_text)))?;

    Ok(nonce)
}

/// Broadcasts an update message to the node.
async fn broadcast_update_to_node(
    node_url: &str,
    message: &UpdateMsg,
) -> Result<String, WalletError> {
    // Create the JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "broadcastUpdate",
        "params": [serde_json::to_value(message).unwrap()],
        "id": 1
    });

    // Send the request to the node
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let response = client
        .post(&rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(e.to_string()))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw broadcast response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    // Check for errors
    if let Some(error) = response.get("error") {
        if !error.is_null() {
            return Err(WalletError::NodeRequestFailed(
                error.to_string(),
            ));
        }
    }

    // Get the transaction hash
    let tx_hash = response
        .get("result")
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("No result in response: {}", response_text)))?
        .as_str()
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("Invalid transaction hash: {}", response_text)))?
        .to_string();

    Ok(tx_hash)
}

/// Gets the balance for an address from the node.
async fn get_balance_from_node(node_url: &str, address: &Address) -> Result<u128, WalletError> {
    // Create the JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getBalance",
        "params": [hex::encode(address)],
        "id": 1
    });

    // Send the request to the node
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let response = client
        .post(&rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(e.to_string()))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw balance response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    // Check for errors
    if let Some(error) = response.get("error") {
        if !error.is_null() {
            return Err(WalletError::NodeRequestFailed(
                error.to_string(),
            ));
        }
    }

    // Get the balance
    let balance = response
        .get("result")
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("No result in response: {}", response_text)))?;
    
    // Handle the case where result might be a number or a string
    let balance_u128 = if balance.is_u64() {
        balance.as_u64().unwrap() as u128
    } else if balance.is_string() {
        balance.as_str().unwrap().parse::<u128>()
            .map_err(|e| WalletError::NodeRequestFailed(format!("Invalid balance string: {}", e)))?
    } else if balance.is_null() {
        // If result is null, return 0 as the balance
        0
    } else {
        return Err(WalletError::NodeRequestFailed(format!("Invalid balance format: {}", balance)));
    };

    Ok(balance_u128)
}

/// Gets the balance for an address and token from the node.
async fn get_balance_with_token_from_node(node_url: &str, address: &Address, token_id: u64) -> Result<u128, WalletError> {
    // Create the JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getBalanceWithToken",
        "params": [hex::encode(address), token_id],
        "id": 1
    });

    // Send the request to the node
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let response = client
        .post(&rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(e.to_string()))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw balance response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    // Check for errors
    if let Some(error) = response.get("error") {
        if !error.is_null() {
            return Err(WalletError::NodeRequestFailed(
                error.to_string(),
            ));
        }
    }

    // Get the balance
    let balance = response
        .get("result")
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("No result in response: {}", response_text)))?;
    
    // Handle the case where result might be a number or a string
    let balance_u128 = if balance.is_u64() {
        balance.as_u64().unwrap() as u128
    } else if balance.is_string() {
        balance.as_str().unwrap().parse::<u128>()
            .map_err(|e| WalletError::NodeRequestFailed(format!("Invalid balance string: {}", e)))?
    } else if balance.is_null() {
        // If result is null, return 0 as the balance
        0
    } else {
        return Err(WalletError::NodeRequestFailed(format!("Invalid balance format: {}", balance)));
    };

    Ok(balance_u128)
}

/// Gets the nonce for an address and token from the node.
async fn get_nonce_with_token_from_node(node_url: &str, address: &Address, token_id: u64) -> Result<u64, WalletError> {
    // Create the JSON-RPC request
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "get_nonce_with_token",
        "params": [hex::encode(address), token_id],
        "id": 1
    });

    // Send the request to the node
    // Make sure to append /rpc to the node URL
    let rpc_url = if node_url.ends_with("/rpc") {
        node_url.to_string()
    } else {
        format!("{}/rpc", node_url)
    };
    
    let client = reqwest::Client::new();
    let response = client
        .post(&rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(e.to_string()))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw nonce response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    // Check for errors
    if let Some(error) = response.get("error") {
        if !error.is_null() {
            return Err(WalletError::NodeRequestFailed(
                error.to_string(),
            ));
        }
    }

    // Get the nonce
    let nonce = response
        .get("result")
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("No result in response: {}", response_text)))?
        .as_u64()
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("Invalid nonce: {}", response_text)))?;

    Ok(nonce)
}
