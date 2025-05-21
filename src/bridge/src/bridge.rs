//! Bridge implementation for the Ethereum bridge.

use crate::bindings::ETHBridgeContract;
use crate::errors::BridgeError;
use core::{proofs::Proof, types::Address};
use ethers::{
    core::types::{Address as EthAddress, TransactionReceipt, U256},
    middleware::{Middleware, SignerMiddleware},
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
};
use std::str::FromStr;
use std::sync::Arc;

/// A bridge for transferring tokens between Ethereum and the stateless token network.
pub struct Bridge<M: Middleware> {
    /// The ETHBridge contract
    contract: ETHBridgeContract<M>,
    /// The provider for the Ethereum network
    provider: Arc<M>,
}

impl<S: ethers::signers::Signer + 'static> Bridge<SignerMiddleware<Provider<Http>, S>> {
    /// Creates a new bridge.
    pub async fn new(
        provider: Arc<SignerMiddleware<Provider<Http>, S>>,
        contract_address: &str,
    ) -> Result<Self, BridgeError> {
        let contract_address = EthAddress::from_str(contract_address).map_err(|e| {
            BridgeError::InvalidAddress(format!("Invalid contract address: {}", e))
        })?;

        let contract = ETHBridgeContract::new(contract_address, provider.clone());

        Ok(Self {
            contract,
            provider,
        })
    }

    /// Locks ETH in the contract and emits a Locked event.
    pub async fn lock(
        &self,
        to: &Address,
        amount: u128,
    ) -> Result<TransactionReceipt, BridgeError> {
        // Convert the address to bytes32
        let mut to_bytes32 = [0u8; 32];
        to_bytes32.copy_from_slice(to);

        // Convert the amount to U256
        let amount = U256::from(amount);

        // Call the lock function
        let tx = self
            .contract
            .lock(to_bytes32.into())
            .value(amount);

        let pending_tx = tx.send()
            .await
            .map_err(|e| BridgeError::ContractError(format!("Failed to lock tokens: {}", e)))?;

        // Wait for the transaction to be mined
        let receipt = pending_tx
            .await
            .map_err(|e| BridgeError::TransactionError(format!("Transaction failed: {}", e)))?
            .ok_or_else(|| BridgeError::TransactionError("Transaction receipt not found".to_string()))?;

        Ok(receipt)
    }

    /// Unlocks ETH from the contract and sends it to the specified address.
    pub async fn unlock(
        &self,
        to: &str,
        amount: u128,
        proof: &Proof,
        from: &Address,
    ) -> Result<TransactionReceipt, BridgeError> {
        // Convert the address to EthAddress
        let to = EthAddress::from_str(to).map_err(|e| {
            BridgeError::InvalidAddress(format!("Invalid recipient address: {}", e))
        })?;

        // Convert the amount to U256
        let amount = U256::from(amount);

        // Convert the proof to the format expected by the contract
        let proof_bytes32: Vec<[u8; 32]> = proof.siblings.clone();
        let proof_path: Vec<bool> = proof.path.clone();

        // Convert the from address to bytes32
        let mut from_bytes32 = [0u8; 32];
        from_bytes32.copy_from_slice(from);

        // Call the unlock function
        let tx = self
            .contract
            .unlock(to, amount, proof_bytes32, proof_path, from_bytes32.into());

        let pending_tx = tx.send()
            .await
            .map_err(|e| BridgeError::ContractError(format!("Failed to unlock tokens: {}", e)))?;

        // Wait for the transaction to be mined
        let receipt = pending_tx
            .await
            .map_err(|e| BridgeError::TransactionError(format!("Transaction failed: {}", e)))?
            .ok_or_else(|| BridgeError::TransactionError("Transaction receipt not found".to_string()))?;

        Ok(receipt)
    }

    /// Updates the current root of the stateless token network.
    pub async fn update_root(
        &self,
        new_root: &[u8; 32],
    ) -> Result<TransactionReceipt, BridgeError> {
        // Convert the root to bytes32
        let mut root_bytes32 = [0u8; 32];
        root_bytes32.copy_from_slice(new_root);

        // Call the updateRoot function
        let tx = self
            .contract
            .update_root(root_bytes32.into());

        let pending_tx = tx.send()
            .await
            .map_err(|e| BridgeError::ContractError(format!("Failed to update root: {}", e)))?;

        // Wait for the transaction to be mined
        let receipt = pending_tx
            .await
            .map_err(|e| BridgeError::TransactionError(format!("Transaction failed: {}", e)))?
            .ok_or_else(|| BridgeError::TransactionError("Transaction receipt not found".to_string()))?;

        Ok(receipt)
    }

    /// Returns the balance of the contract.
    pub async fn get_balance(&self) -> Result<u128, BridgeError> {
        let balance = self
            .contract
            .get_balance()
            .call()
            .await
            .map_err(|e| BridgeError::ContractError(format!("Failed to get balance: {}", e)))?;

        Ok(balance.as_u128())
    }

    /// Returns the current root of the stateless token network.
    pub async fn get_current_root(&self) -> Result<[u8; 32], BridgeError> {
        let root = self
            .contract
            .current_root()
            .call()
            .await
            .map_err(|e| BridgeError::ContractError(format!("Failed to get current root: {}", e)))?;

        let mut root_bytes = [0u8; 32];
        root_bytes.copy_from_slice(root.as_ref());

        Ok(root_bytes)
    }

    /// Checks if a proof has been used.
    pub async fn is_proof_used(&self, proof_id: &[u8; 32]) -> Result<bool, BridgeError> {
        let mut proof_id_bytes32 = [0u8; 32];
        proof_id_bytes32.copy_from_slice(proof_id);

        let used = self
            .contract
            .is_proof_used(proof_id_bytes32.into())
            .call()
            .await
            .map_err(|e| BridgeError::ContractError(format!("Failed to check if proof is used: {}", e)))?;

        Ok(used)
    }
}

/// Creates a new bridge with a local wallet.
pub async fn new_bridge_with_wallet(
    rpc_url: &str,
    contract_address: &str,
    private_key: &str,
) -> Result<Bridge<SignerMiddleware<Provider<Http>, LocalWallet>>, BridgeError> {
    // Create a provider
    let provider = Provider::<Http>::try_from(rpc_url).map_err(|e| {
        BridgeError::EthereumError(format!("Failed to create provider: {}", e))
    })?;

    // Create a wallet
    let wallet = private_key
        .parse::<LocalWallet>()
        .map_err(|e| BridgeError::SignatureError(format!("Invalid private key: {}", e)))?;

    // Create a signer
    let signer = SignerMiddleware::new(provider, wallet);

    // Create a bridge
    Bridge::new(Arc::new(signer), contract_address).await
}
