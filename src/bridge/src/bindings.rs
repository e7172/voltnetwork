/// Ethereum contract bindings for the bridge.
use ethers::{prelude::*, abi::Contract as EthersContract};
use std::sync::Arc;

/// The ETHBridge contract interface
pub struct ETHBridgeContract<M: Middleware> {
    contract: Contract<M>,
}

impl<M: Middleware> ETHBridgeContract<M> {
    /// Creates a new instance of the contract
    pub fn new(address: Address, client: impl Into<Arc<M>>) -> Self {
        // Define the contract ABI
        let abi = include_str!("../contracts/ETHBridge.abi");
        let contract = Contract::new(address, serde_json::from_str::<EthersContract>(abi).expect("Invalid ABI"), client.into());
        Self { contract }
    }

    /// Locks tokens in the contract
    pub fn lock(&self, to: H256) -> ContractCall<M, ()> {
        self.contract.method("lock", (to,)).expect("Method not found")
    }

    /// Unlocks tokens from the contract
    pub fn unlock(
        &self,
        to: Address,
        amount: U256,
        proof_siblings: Vec<[u8; 32]>,
        proof_path: Vec<bool>,
        from: H256,
    ) -> ContractCall<M, ()> {
        self.contract
            .method("unlock", (to, amount, proof_siblings, proof_path, from))
            .expect("Method not found")
    }

    /// Updates the root of the Merkle tree
    pub fn update_root(&self, new_root: H256) -> ContractCall<M, ()> {
        self.contract.method("updateRoot", (new_root,)).expect("Method not found")
    }

    /// Gets the balance of the contract
    pub fn get_balance(&self) -> ContractCall<M, U256> {
        self.contract.method("getBalance", ()).expect("Method not found")
    }

    /// Gets the current root of the Merkle tree
    pub fn current_root(&self) -> ContractCall<M, H256> {
        self.contract.method("currentRoot", ()).expect("Method not found")
    }

    /// Checks if a proof has been used
    pub fn is_proof_used(&self, proof_id: H256) -> ContractCall<M, bool> {
        self.contract.method("isProofUsed", (proof_id,)).expect("Method not found")
    }
}
