//! Balance command for the CLI wallet.

use crate::config::WalletConfig;
use crate::errors::WalletError;
use crate::wallet::Wallet;
use anyhow::Result;
use core::types::Address;
use std::path::Path;
use tracing::{debug, info};

/// Runs the balance command.
pub async fn run<P: AsRef<Path>>(
    config: &WalletConfig,
    wallet_path: P,
) -> Result<u128, WalletError> {
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

    // Get the address
    let address = wallet.address()?;
    info!("Getting balance for address: {:?}", address);
    
    // Print the address for the user
    println!("Wallet address: 0x{}", hex::encode(&address));

    // Get the balance from the node
    let balance = get_balance_from_node(&config.node, &address).await?;
    debug!("Balance: {}", balance);

    Ok(balance)
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
    println!("Raw response: {}", response_text);
    
    // If the response is empty, return an error
    if response_text.is_empty() {
        return Err(WalletError::NetworkError("Empty response from node".to_string()));
    }
    
    // Parse the response
    let response: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| WalletError::NetworkError(format!("Failed to parse response: {}", e)))?;

    // Check for errors in the response
    if let Some(error) = response.get("error") {
        // Only return an error if the error is not null
        if !error.is_null() {
            return Err(WalletError::NodeRequestFailed(
                error.to_string(),
            ));
        }
    }

    // Get the balance
    let result = response.get("result")
        .ok_or_else(|| WalletError::NodeRequestFailed(format!("No result in response: {}", response_text)))?;
    
    // Handle the case where result might be a number or a string
    let balance = if result.is_u64() {
        result.as_u64().unwrap()
    } else if result.is_string() {
        result.as_str().unwrap().parse::<u64>()
            .map_err(|e| WalletError::NodeRequestFailed(format!("Invalid balance string: {}", e)))?
    } else if result.is_null() {
        // If result is null, return 0 as the balance
        0
    } else {
        return Err(WalletError::NodeRequestFailed(format!("Invalid balance format: {}", result)));
    };

    Ok(balance as u128)
}
