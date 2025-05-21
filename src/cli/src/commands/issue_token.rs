//! Issue token command for the CLI wallet.

use crate::config::WalletConfig;
use crate::errors::WalletError;
use crate::wallet::Wallet;
use anyhow::Result;
use core::types::Address;
use std::path::Path;
use tracing::{debug, info};

/// Runs the issue-token command.
pub async fn run<P: AsRef<Path>>(
    config: &WalletConfig,
    wallet_path: P,
    metadata: &str,
    collateral: Option<u128>,
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

    // Get the issuer address
    let issuer = wallet.address()?;
    info!("Issuing token with metadata '{}' from issuer {:?}", metadata, issuer);

    // Get the current nonce from the node
    let nonce = get_nonce_from_node(&config.node, &issuer).await?;
    debug!("Issuer nonce: {}", nonce);

    // Create the issue token message
    let message = core::types::SystemMsg::IssueToken {
        issuer,
        token_id: 0, // Will be assigned by the system
        metadata: metadata.to_string(),
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

    // Create the final message with the signature
    let final_message = match message {
        core::types::SystemMsg::IssueToken { issuer, token_id: _, metadata, nonce, signature: _ } => {
            core::types::SystemMsg::IssueToken {
                issuer,
                token_id: 0, // Will be assigned by the system
                metadata,
                nonce,
                signature: core::types::Signature(signature_bytes),
            }
        },
        _ => unreachable!(),
    };

    // Broadcast the message to the node
    let token_id = broadcast_issue_token_to_node(&config.node, &final_message).await?;
    debug!("Token ID: {}", token_id);

    Ok(format!("Successfully issued token with ID {}. Metadata: '{}'", token_id, metadata))
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

    let nonce = response_json
        .get("result")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| WalletError::NetworkError(format!("Invalid response format: {}", response_text)))?;

    Ok(nonce)
}

/// Broadcasts an issue token message to the node.
async fn broadcast_issue_token_to_node(
    node_url: &str,
    message: &core::types::SystemMsg,
) -> Result<u64, WalletError> {
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
            "method": "p3p_issueToken",
            "params": [message_hex]
        }))
        .send()
        .await
        .map_err(|e| WalletError::NetworkError(format!("Failed to connect to node: {}", e)))?;

    // Get the raw response text for debugging
    let response_text = response.text().await
        .map_err(|e| WalletError::NetworkError(format!("Failed to get response text: {}", e)))?;
    
    // Print the raw response for debugging
    println!("Raw issue token response: {}", response_text);
    
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

    let token_id = response_json
        .get("result")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| WalletError::NetworkError(format!("Invalid response format: {}", response_text)))?;

    Ok(token_id)
}