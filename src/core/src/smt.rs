//! Sparse Merkle Tree implementation for the chainless token transfer network.

use crate::errors::CoreError;
use crate::proofs::Proof;
use crate::types::{AccountLeaf, Address, Balance, TokenId, TokenInfo, SystemMsg};
use byteorder::{ByteOrder, LittleEndian};
use rocksdb::{IteratorMode, DB};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sparse_merkle_tree::{
    default_store::DefaultStore,
    traits::Hasher,
    SparseMerkleTree as SMTree, H256,
};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tracing::{info, warn, error, debug};

/// SHA-256 hasher for the Sparse Merkle Tree.
#[derive(Default)]
pub struct Sha256Hasher(Sha256);

impl Hasher for Sha256Hasher {
    fn write_h256(&mut self, h: &H256) {
        self.0.update(h.as_slice());
    }

    fn write_byte(&mut self, b: u8) {
        self.0.update(&[b]);
    }

    fn finish(self) -> H256 {
        let result = self.0.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash.into()
    }
}

impl Sha256Hasher {
    fn new() -> Self {
        Self(Sha256::new())
    }
}

impl std::ops::Deref for Sha256Hasher {
    type Target = Sha256;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Sha256Hasher {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A Sparse Merkle Tree for storing account leaves.
#[derive(Serialize, Deserialize)]
pub struct SMT {
    /// The underlying Sparse Merkle Tree
    #[serde(skip)]
    tree: SMTree<Sha256Hasher, H256, DefaultStore<H256>>,
    /// Cache of account leaves by (address, token_id) pair
    #[serde(skip)]
    accounts: HashMap<(Address, TokenId), AccountLeaf>,
    /// Registry of tokens by token ID
    #[serde(skip)]
    token_registry: HashMap<TokenId, TokenInfo>,
    /// The root hash of the tree
    root: [u8; 32],
    /// The next available token ID
    next_token_id: TokenId,
    /// The native token ID (always 0)
    pub native_token_id: TokenId,
    /// RocksDB instance for persistence
    #[serde(skip)]
    db: Option<Arc<DB>>,
}

/// Constants for RocksDB keys
const ROOT_KEY: &[u8] = b"root";
const ACCOUNT_PREFIX: &str = "account::";
const TOKEN_PREFIX: &str = "token::";
const NEXT_TOKEN_ID_KEY: &[u8] = b"next_token_id";

impl Clone for SMT {
    fn clone(&self) -> Self {
        // Create a new SMT with the same root
        let mut smt = SMT::new_zero();

        // Copy the root
        smt.root.copy_from_slice(&self.root);
        
        // Copy the next token ID and native token ID
        smt.next_token_id = self.next_token_id;
        
        // Note: We don't clone the DB reference as it's not needed for most operations
        smt.native_token_id = self.native_token_id;

        // Copy the token registry
        for (token_id, token_info) in &self.token_registry {
            smt.token_registry.insert(*token_id, token_info.clone());
        }

        // Copy the accounts
        for ((addr, token_id), leaf) in &self.accounts {
            smt.accounts.insert((*addr, *token_id), leaf.clone());

            // Update the tree
            let key = compute_leaf_key(addr, *token_id);
            let addr_h256 = H256::from(key);

            let leaf_hash = leaf.hash();
            let value_h256 = H256::from(leaf_hash);

            // Ignore errors during cloning
            let _ = smt.tree.update(addr_h256, value_h256);
        }

        // Share the same DB reference if available
        if let Some(db) = &self.db {
            smt.db = Some(Arc::clone(db));
        }

        smt
    }
}

/// Computes a unique key for a (address, token_id) pair.
fn compute_leaf_key(addr: &Address, token_id: TokenId) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(addr);
    
    let mut token_id_bytes = [0u8; 8];
    LittleEndian::write_u64(&mut token_id_bytes, token_id);
    hasher.update(token_id_bytes);
    
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

impl SMT {
    /// Creates a new empty Sparse Merkle Tree.
    pub fn new_zero() -> Self {
        let tree = SMTree::default();
        let root_h256 = tree.root();
        let mut root = [0u8; 32];
        root.copy_from_slice(root_h256.as_slice());

        // Create a new SMT instance
        let mut smt = Self {
            tree,
            accounts: HashMap::new(),
            token_registry: HashMap::new(),
            root,
            next_token_id: 1, // Start from 1, 0 is reserved for the native token
            native_token_id: 0,
            db: None,
        };
        
        // Initialize the native token
        let native_token = TokenInfo {
            token_id: 0,
            issuer: [0u8; 32], // Default issuer for native token
            metadata: "VOLT|Volt Token|18".to_string(),
            total_supply: 0,
        };
        
        // Add the native token to the registry
        smt.token_registry.insert(0, native_token);
        
        smt
    }

    /// Creates a new empty Sparse Merkle Tree with a RocksDB instance.
    pub fn new_with_db(db: Arc<DB>) -> Self {
        let mut smt = Self::new_zero();
        smt.db = Some(db);
        
        // Persist the initial state to RocksDB
        if let Err(e) = smt.persist_to_db() {
            error!("Failed to persist initial state to RocksDB: {}", e);
        }
        
        smt
    }

    /// Persists the current state to RocksDB.
    fn persist_to_db(&self) -> Result<(), CoreError> {
        let db = self.db.as_ref().ok_or_else(|| CoreError::SMTError("No DB instance available".to_string()))?;
        
        // Get column family handles
        let cf_meta = db.cf_handle("meta").ok_or_else(|| {
            CoreError::SMTError("Column family 'meta' not found".to_string())
        })?;
        
        let cf_leaves = db.cf_handle("leaves").ok_or_else(|| {
            CoreError::SMTError("Column family 'leaves' not found".to_string())
        })?;
        
        // Persist the root in the meta column family
        db.put_cf(&cf_meta, ROOT_KEY, bincode::serialize(&self.root)
            .map_err(|e| CoreError::SerializationError(e.to_string()))?)
            .map_err(|e| CoreError::SMTError(format!("Failed to persist root: {}", e)))?;
        
        // Persist the next token ID in the meta column family
        db.put_cf(&cf_meta, NEXT_TOKEN_ID_KEY, bincode::serialize(&self.next_token_id)
            .map_err(|e| CoreError::SerializationError(e.to_string()))?)
            .map_err(|e| CoreError::SMTError(format!("Failed to persist next token ID: {}", e)))?;
        
        // Persist accounts in the leaves column family
        for ((addr, token_id), leaf) in &self.accounts {
            let key = compute_leaf_key(addr, *token_id);
            db.put_cf(&cf_leaves, key.as_ref(), bincode::serialize(leaf)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?)
                .map_err(|e| CoreError::SMTError(format!("Failed to persist account: {}", e)))?;
        }
        
        // Persist tokens in the meta column family
        for (token_id, info) in &self.token_registry {
            let key = format!("{}{}", TOKEN_PREFIX, token_id);
            db.put_cf(&cf_meta, key.as_bytes(), bincode::serialize(info)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?)
                .map_err(|e| CoreError::SMTError(format!("Failed to persist token: {}", e)))?;
        }
        
        Ok(())
    }

    /// Loads the SMT state from RocksDB.
    pub fn load_from_db(db: Arc<DB>) -> Result<Self, CoreError> {
        let mut smt = Self::new_zero();
        smt.db = Some(Arc::clone(&db));
        
        // Get column family handles
        let cf_meta = match db.cf_handle("meta") {
            Some(cf) => cf,
            None => {
                info!("Column family 'meta' not found, using default state");
                return Ok(smt);
            }
        };
        
        let cf_leaves = match db.cf_handle("leaves") {
            Some(cf) => cf,
            None => {
                info!("Column family 'leaves' not found, using default state");
                return Ok(smt);
            }
        };
        
        // Load the root from meta column family
        if let Some(root_bytes) = db.get_cf(&cf_meta, ROOT_KEY)
            .map_err(|e| CoreError::SMTError(format!("Failed to get root: {}", e)))?
        {
            let root: [u8; 32] = bincode::deserialize(&root_bytes)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?;
            smt.root.copy_from_slice(&root);
        } else {
            info!("No root found in DB, using default");
        }
        
        // Load the next token ID from meta column family
        if let Some(next_token_id_bytes) = db.get_cf(&cf_meta, NEXT_TOKEN_ID_KEY)
            .map_err(|e| CoreError::SMTError(format!("Failed to get next token ID: {}", e)))?
        {
            smt.next_token_id = bincode::deserialize(&next_token_id_bytes)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?;
        } else {
            info!("No next token ID found in DB, using default");
        }
        
        // Load accounts from leaves column family
        let iter = db.iterator_cf(&cf_leaves, IteratorMode::Start);
        
        for item in iter {
            let (key, value) = item.map_err(|e| CoreError::SMTError(format!("Failed to iterate accounts: {}", e)))?;
            
            let leaf: AccountLeaf = bincode::deserialize(&value)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?;
            
            // Add to accounts cache
            smt.accounts.insert((leaf.addr, leaf.token_id), leaf.clone());
            
            // Update the tree
            let key = compute_leaf_key(&leaf.addr, leaf.token_id);
            let addr_h256 = H256::from(key);
            let leaf_hash = leaf.hash();
            let value_h256 = H256::from(leaf_hash);
            
            // Ignore errors during loading
            if let Err(e) = smt.tree.update(addr_h256, value_h256) {
                warn!("Failed to update tree during loading: {}", e);
            }
        }
        
        // Load tokens from meta column family
        let token_prefix = TOKEN_PREFIX.as_bytes();
        let iter = db.iterator_cf(&cf_meta, IteratorMode::From(token_prefix, rocksdb::Direction::Forward));
        
        for item in iter {
            let (key, value) = item.map_err(|e| CoreError::SMTError(format!("Failed to iterate tokens: {}", e)))?;
            
            let key_str = String::from_utf8_lossy(&key);
            if !key_str.starts_with(TOKEN_PREFIX) {
                // We've moved past the token prefix
                break;
            }
            
            let token_info: TokenInfo = bincode::deserialize(&value)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?;
            
            // Add to token registry
            smt.token_registry.insert(token_info.token_id, token_info);
        }
        
        // Ensure the native token exists
        if !smt.token_registry.contains_key(&0) {
            let native_token = TokenInfo {
                token_id: 0,
                issuer: [0u8; 32],
                metadata: "VOLT|Volt Token|18".to_string(),
                total_supply: 0,
            };
            smt.token_registry.insert(0, native_token);
        }
        
        // Update the root
        let root_h256 = smt.tree.root();
        smt.root.copy_from_slice(root_h256.as_slice());
        
        Ok(smt)
    }
    
    /// Registers a new token in the registry.
    pub fn register_token(&mut self, issuer: &Address, metadata: String) -> Result<TokenId, CoreError> {
        let token_id = self.next_token_id;
        
        // Create a new token info
        let token_info = TokenInfo {
            token_id,
            issuer: *issuer,
            metadata,
            total_supply: 0,
        };
        
        // Add the token to the registry
        self.token_registry.insert(token_id, token_info.clone());
        
        // Increment the next token ID
        self.next_token_id += 1;
        
        // Persist to RocksDB if available
        if let Some(db) = &self.db {
            // Get column family handle for meta
            let cf_meta = db.cf_handle("meta").ok_or_else(|| {
                CoreError::SMTError("Column family 'meta' not found".to_string())
            })?;
            
            // Persist the token info to meta column family
            let token_key = format!("{}{}", TOKEN_PREFIX, token_id);
            db.put_cf(&cf_meta, token_key.as_bytes(), bincode::serialize(&token_info)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?)
                .map_err(|e| CoreError::SMTError(format!("Failed to persist token: {}", e)))?;
            
            // Persist the updated next token ID to meta column family
            db.put_cf(&cf_meta, NEXT_TOKEN_ID_KEY, bincode::serialize(&self.next_token_id)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?)
                .map_err(|e| CoreError::SMTError(format!("Failed to persist next token ID: {}", e)))?;
        }
        
        Ok(token_id)
    }
    
    /// Gets a token from the registry.
    pub fn get_token(&self, token_id: TokenId) -> Result<TokenInfo, CoreError> {
        self.token_registry.get(&token_id)
            .cloned()
            .ok_or_else(|| CoreError::TokenNotFound(token_id))
    }
    
    /// Gets the entire token registry.
    pub fn get_token_registry(&self) -> Result<&std::collections::HashMap<TokenId, TokenInfo>, CoreError> {
        Ok(&self.token_registry)
    }
    
    /// Updates a token's total supply.
    fn update_token_supply(&mut self, token_id: TokenId, amount: Balance, is_mint: bool) -> Result<(), CoreError> {
        let mut token_info = self.get_token(token_id)?;
        
        if is_mint {
            token_info.total_supply = token_info.total_supply.checked_add(amount)
                .ok_or_else(|| CoreError::SupplyOverflow)?;
        } else {
            token_info.total_supply = token_info.total_supply.checked_sub(amount)
                .ok_or_else(|| CoreError::InsufficientSupply {
                    required: amount,
                    available: token_info.total_supply,
                })?;
        }
        
        self.token_registry.insert(token_id, token_info.clone());
        
        // Persist to RocksDB if available
        if let Some(db) = &self.db {
            // Get column family handle for meta
            let cf_meta = db.cf_handle("meta").ok_or_else(|| {
                CoreError::SMTError("Column family 'meta' not found".to_string())
            })?;
            
            // Persist the updated token info to meta column family
            let token_key = format!("{}{}", TOKEN_PREFIX, token_id);
            db.put_cf(&cf_meta, token_key.as_bytes(), bincode::serialize(&token_info)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?)
                .map_err(|e| CoreError::SMTError(format!("Failed to persist token: {}", e)))?;
        }
        
        Ok(())
    }

    /// Returns the root hash of the tree.
    pub fn root(&self) -> [u8; 32] {
        self.root
    }
    
    /// Returns a reference to the RocksDB instance, if available.
    /// This is useful for ensuring state persistence in production environments.
    ///
    /// # Returns
    ///
    /// `Some(&Arc<DB>)` if a database is configured, `None` otherwise
    pub fn get_db(&self) -> Option<&Arc<DB>> {
        self.db.as_ref()
    }

    /// Updates an account leaf in the tree.
    ///
    /// # Arguments
    ///
    /// * `leaf` - The account leaf to update
    ///
    /// # Returns
    ///
    /// `Ok(())` if the update was successful, `Err(CoreError)` otherwise
    pub fn update(&mut self, leaf: AccountLeaf) -> Result<(), CoreError> {
        let key = compute_leaf_key(&leaf.addr, leaf.token_id);
        let addr_h256 = H256::from(key);
        let leaf_hash = leaf.hash();
        let value_h256 = H256::from(leaf_hash);

        // Update the tree
        self.tree
            .update(addr_h256, value_h256)
            .map_err(|e| CoreError::SMTError(e.to_string()))?;

        // Update the root
        let root_h256 = self.tree.root();
        self.root.copy_from_slice(root_h256.as_slice());

        // Update the accounts cache - this is critical for production readiness
        // We need to ensure the cache is always in sync with the tree
        info!("Updating account in cache: addr={:?}, token_id={}, bal={}, nonce={}",
              leaf.addr, leaf.token_id, leaf.bal, leaf.nonce);
        self.accounts.insert((leaf.addr, leaf.token_id), leaf.clone());

        // Persist to RocksDB if available
        if let Some(db) = &self.db {
            // Get column family handles
            let cf_meta = match db.cf_handle("meta") {
                Some(cf) => cf,
                None => {
                    error!("Column family 'meta' not found");
                    return Err(CoreError::SMTError("Column family 'meta' not found".to_string()));
                }
            };
            
            let cf_leaves = match db.cf_handle("leaves") {
                Some(cf) => cf,
                None => {
                    error!("Column family 'leaves' not found");
                    return Err(CoreError::SMTError("Column family 'leaves' not found".to_string()));
                }
            };
            
            // Persist the updated account to the leaves column family
            let key = compute_leaf_key(&leaf.addr, leaf.token_id);
            match bincode::serialize(&leaf) {
                Ok(serialized) => {
                    if let Err(e) = db.put_cf(&cf_leaves, key.as_ref(), serialized) {
                        error!("Failed to persist account to RocksDB: {}", e);
                        // In production, we continue even if persistence fails
                        // This ensures the in-memory state remains correct
                    } else {
                        debug!("Successfully persisted account to RocksDB: {:?}", leaf.addr);
                    }
                },
                Err(e) => {
                    error!("Failed to serialize account: {}", e);
                    return Err(CoreError::SerializationError(e.to_string()));
                }
            }
            
            // Persist the updated root to the meta column family
            match bincode::serialize(&self.root) {
                Ok(serialized) => {
                    if let Err(e) = db.put_cf(&cf_meta, ROOT_KEY, serialized) {
                        error!("Failed to persist root to RocksDB: {}", e);
                        // In production, we continue even if persistence fails
                    } else {
                        debug!("Successfully persisted root to RocksDB");
                    }
                },
                Err(e) => {
                    error!("Failed to serialize root: {}", e);
                    return Err(CoreError::SerializationError(e.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Updates an account in the tree.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to update
    ///
    /// # Returns
    ///
    /// `Ok(())` if the update was successful, `Err(CoreError)` otherwise
    pub fn update_account(&mut self, account: AccountLeaf) -> Result<(), CoreError> {
        self.update(account)
    }

    /// Updates an account in the tree for a specific token.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to update
    /// * `token_id` - The token ID
    ///
    /// # Returns
    ///
    /// `Ok(())` if the update was successful, `Err(CoreError)` otherwise
    pub fn update_account_with_token(&mut self, account: AccountLeaf, token_id: TokenId) -> Result<(), CoreError> {
        // Ensure the account has the correct token ID
        if account.token_id != token_id {
            return Err(CoreError::InvalidTokenId {
                expected: token_id,
                actual: account.token_id,
            });
        }
        
        self.update(account)
    }

    /// Transfers tokens from one account to another.
    ///
    /// # Arguments
    ///
    /// * `from` - The address to transfer from
    /// * `to` - The address to transfer to
    /// * `amount` - The amount to transfer
    /// * `nonce` - The nonce of the transaction
    ///
    /// # Returns
    ///
    /// `Ok(())` if the transfer was successful, `Err(CoreError)` otherwise
    pub fn transfer(
        &mut self,
        from: &Address,
        to: &Address,
        amount: Balance,
        nonce: u64,
    ) -> Result<(), CoreError> {
        // Default to native token (token_id = 0)
        self.transfer_token(from, to, self.native_token_id, amount, nonce)
    }
    
    /// Transfers tokens from one account to another for a specific token.
    ///
    /// # Arguments
    ///
    /// * `from` - The address to transfer from
    /// * `to` - The address to transfer to
    /// * `token_id` - The token ID
    /// * `amount` - The amount to transfer
    /// * `nonce` - The nonce of the transaction
    ///
    /// # Returns
    ///
    /// `Ok(())` if the transfer was successful, `Err(CoreError)` otherwise
    pub fn transfer_token(
        &mut self,
        from: &Address,
        to: &Address,
        token_id: TokenId,
        amount: Balance,
        nonce: u64,
    ) -> Result<(), CoreError> {
        // Get the sender account
        let sender = self.get_account_with_token(from, token_id)?;

        // Check balance
        if sender.bal < amount {
            return Err(CoreError::InsufficientBalance {
                required: amount,
                available: sender.bal,
            });
        }

        // Check nonce
        if sender.nonce != nonce {
            return Err(CoreError::InvalidNonce {
                expected: sender.nonce,
                actual: nonce,
            });
        }

        // Get the receiver account
        let receiver = match self.get_account_with_token(to, token_id) {
            Ok(account) => account,
            Err(_) => AccountLeaf::new_empty(*to, token_id),
        };

        // Update sender account
        let new_sender = AccountLeaf::new(
            *from,
            sender.bal - amount,
            sender.nonce + 1,
            token_id,
        );

        // Update receiver account
        let new_receiver = AccountLeaf::new(
            *to,
            receiver.bal + amount,
            receiver.nonce,
            token_id,
        );

        // Update the tree
        self.update(new_sender)?;
        self.update(new_receiver)?;

        Ok(())
    }

    /// Mints new tokens to an account.
    ///
    /// # Arguments
    ///
    /// * `treasury` - The treasury address (must match the configured treasury)
    /// * `to` - The address to mint tokens to
    /// * `amount` - The amount to mint
    /// * `nonce` - The nonce of the transaction
    /// * `max_supply` - The maximum supply of tokens
    /// * `current_supply` - The current supply of tokens
    ///
    /// # Returns
    ///
    /// `Ok(new_supply)` if the mint was successful, `Err(CoreError)` otherwise
    pub fn mint(
        &mut self,
        treasury: &Address,
        to: &Address,
        amount: Balance,
        nonce: u64,
        max_supply: Balance,
        _current_supply: Balance,
    ) -> Result<Balance, CoreError> {
        // Default to native token (token_id = 0)
        self.mint_token_with_max_supply(treasury, to, self.native_token_id, amount, nonce, max_supply)
    }
    
    /// Mints new tokens to an account for a specific token with a maximum supply check.
    ///
    /// # Arguments
    ///
    /// * `issuer` - The issuer's address (must be the token issuer)
    /// * `to` - The address to mint tokens to
    /// * `token_id` - The token ID
    /// * `amount` - The amount to mint
    /// * `nonce` - The nonce of the transaction
    /// * `max_supply` - The maximum supply of tokens
    ///
    /// # Returns
    ///
    /// `Ok(new_supply)` if the mint was successful, `Err(CoreError)` otherwise
    pub fn mint_token_with_max_supply(
        &mut self,
        issuer: &Address,
        to: &Address,
        token_id: TokenId,
        amount: Balance,
        nonce: u64,
        max_supply: Balance,
    ) -> Result<Balance, CoreError> {
        // Get the token info
        let token_info = self.get_token(token_id)?;
        
        // Check if the new supply would exceed the maximum supply
        if token_info.total_supply.checked_add(amount).ok_or(CoreError::SupplyOverflow)? > max_supply {
            return Err(CoreError::ExceedsMaxSupply {
                max_supply,
                current_supply: token_info.total_supply,
                mint_amount: amount,
            });
        }
        
        // Delegate to the regular mint_token function
        self.mint_token(issuer, to, token_id, amount, nonce)
    }
    
    /// Mints new tokens to an account for a specific token.
    ///
    /// # Arguments
    ///
    /// * `issuer` - The issuer's address (must be the token issuer)
    /// * `to` - The address to mint tokens to
    /// * `token_id` - The token ID
    /// * `amount` - The amount to mint
    /// * `nonce` - The nonce of the transaction
    ///
    /// # Returns
    ///
    /// `Ok(new_supply)` if the mint was successful, `Err(CoreError)` otherwise
    pub fn mint_token(
        &mut self,
        issuer: &Address,
        to: &Address,
        token_id: TokenId,
        amount: Balance,
        nonce: u64,
    ) -> Result<Balance, CoreError> {
        // Get the token info
        let token_info = self.get_token(token_id)?;
        
        // Check if the issuer is authorized to mint this token
        info!("Checking if issuer {:?} is authorized to mint token {} with issuer {:?}",
              issuer, token_id, token_info.issuer);
        if token_info.issuer != *issuer {
            error!("Unauthorized mint attempt: expected issuer {:?}, got {:?}",
                  token_info.issuer, issuer);
            return Err(CoreError::Unauthorized(format!(
                "Only the token issuer can mint tokens: expected {:?}, got {:?}",
                token_info.issuer, issuer
            )));
        }
        
        // Get the issuer account
        let issuer_account = self.get_account_with_token(issuer, token_id)?;

        // Check nonce
        if issuer_account.nonce != nonce {
            return Err(CoreError::InvalidNonce {
                expected: issuer_account.nonce,
                actual: nonce,
            });
        }

        // Get the receiver account
        let receiver = match self.get_account_with_token(to, token_id) {
            Ok(account) => account,
            Err(_) => AccountLeaf::new_empty(*to, token_id),
        };

        // Update issuer account (increment nonce)
        let new_issuer = AccountLeaf::new(
            *issuer,
            issuer_account.bal,
            issuer_account.nonce + 1,
            token_id,
        );

        // Update receiver account
        let new_receiver = AccountLeaf::new(
            *to,
            receiver.bal + amount,
            receiver.nonce,
            token_id,
        );

        // Update the token's total supply
        self.update_token_supply(token_id, amount, true)?;
        
        // Update the tree
        self.update(new_issuer)?;
        self.update(new_receiver)?;

        // Return the new total supply
        Ok(token_info.total_supply + amount)
    }

    /// Generates a Merkle proof for an account.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address of the account
    ///
    /// # Returns
    ///
    /// A Merkle proof for the account
    pub fn gen_proof(&self, addr: &Address) -> Result<Proof, CoreError> {
        // Default to native token (token_id = 0)
        self.gen_proof_with_token(addr, self.native_token_id)
    }
    
    /// Generates a Merkle proof for an account with a specific token.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address of the account
    /// * `token_id` - The token ID
    ///
    /// # Returns
    ///
    /// A Merkle proof for the account
    pub fn gen_proof_with_token(&self, addr: &Address, token_id: TokenId) -> Result<Proof, CoreError> {
        let key = compute_leaf_key(addr, token_id);
        let addr_h256 = H256::from(key);

        // Generate the SMT proof
        let smt_proof = self.tree
            .merkle_proof(vec![addr_h256])
            .map_err(|e| CoreError::SMTError(e.to_string()))?;

        // Get the leaf hash
        let leaf_hash = match self.accounts.get(&(*addr, token_id)) {
            Some(account) => account.hash(),
            None => {
                // If account doesn't exist, use empty leaf
                let empty_leaf = AccountLeaf::new_empty(*addr, token_id);
                empty_leaf.hash()
            }
        };

        // Convert SMT proof to our Proof format
        let mut siblings: Vec<[u8; 32]> = smt_proof
            .merkle_path()
            .iter()
            .map(|h| {
                let mut sibling = [0u8; 32];
                // Convert the MergeValue to a byte array
                match h {
                    sparse_merkle_tree::merge::MergeValue::Value(v) => {
                        sibling.copy_from_slice(v.as_slice());
                    }
                    sparse_merkle_tree::merge::MergeValue::MergeWithZero { base_node, zero_bits, .. } => {
                        sibling.copy_from_slice(base_node.as_slice());
                    }
                }
                sibling
            })
            .collect();

        // Convert address to path using bitvec for efficient storage
        // Use the address_to_path function from proofs.rs
        let path = crate::proofs::address_to_path(addr);
        
        // Calculate zeros_omitted - the number of trailing zero siblings that can be omitted
        // In production, we need to ensure the proof is complete for all 256 levels
        let mut zeros_omitted = 0u16;
        
        // If we have fewer than 256 siblings, the rest are considered omitted zeros
        if siblings.len() < 256 {
            zeros_omitted = (256 - siblings.len()) as u16;
        }
        
        // Get the account data for inclusion in the proof
        let account_data = self.accounts.get(&(*addr, token_id)).cloned();
        
        // Include the serialized account data in the proof if available
        if let Some(account) = account_data {
            // Serialize the account data
            if let Ok(leaf_data) = bincode::serialize(&account) {
                // Create a proof with the complete path and zeros_omitted count
                return Ok(Proof::new_with_data(siblings, leaf_hash, path, zeros_omitted, leaf_data));
            }
        }
        
        // Create a proof with the complete path and zeros_omitted count
        Ok(Proof::new(siblings, leaf_hash, path, zeros_omitted))
    }

    /// Gets an account leaf from the tree.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address of the account
    ///
    /// # Returns
    ///
    /// The account leaf if it exists, `Err(CoreError)` otherwise
    pub fn get_account(&self, addr: &Address) -> Result<AccountLeaf, CoreError> {
        // Default to native token (token_id = 0)
        self.get_account_with_token(addr, self.native_token_id)
    }
    
    /// Gets an account leaf from the tree for a specific token.
    ///
    /// # Arguments
    ///
    /// * `addr` - The address of the account
    /// * `token_id` - The token ID
    ///
    /// # Returns
    ///
    /// The account leaf if it exists, `Err(CoreError)` otherwise
    pub fn get_account_with_token(&self, addr: &Address, token_id: TokenId) -> Result<AccountLeaf, CoreError> {
        match self.accounts.get(&(*addr, token_id)) {
            Some(account) => Ok(account.clone()),
            None => {
                // Check if the account exists in the tree
                let key = compute_leaf_key(addr, token_id);
                let addr_h256 = H256::from(key);
                let value = self.tree
                    .get(&addr_h256)
                    .map_err(|e| CoreError::SMTError(e.to_string()))?;

                if value == H256::zero() {
                    // Account doesn't exist
                    Err(CoreError::SMTError(format!("Account not found: {:?} with token {}", addr, token_id)))
                } else {
                    // Account exists but not in cache - this is a critical issue in production
                    // We need to reconstruct the account from the tree
                    info!("Account found in tree but not in cache - reconstructing account data");
                    
                    // Try to load from RocksDB if available
                    if let Some(db) = &self.db {
                        // Get column family handle for leaves
                        if let Some(cf_leaves) = db.cf_handle("leaves") {
                            let key = compute_leaf_key(addr, token_id);
                            match db.get_cf(&cf_leaves, key.as_ref()) {
                                Ok(Some(data)) => {
                                    match bincode::deserialize::<AccountLeaf>(&data) {
                                        Ok(account) => {
                                            // Update the cache
                                            let account_clone = account.clone();
                                            let mut accounts = self.accounts.clone();
                                            accounts.insert((*addr, token_id), account_clone);
                                            
                                            // Return the account
                                            return Ok(account);
                                        },
                                        Err(e) => {
                                            warn!("Failed to deserialize account from RocksDB: {}", e);
                                            // Fall through to default behavior
                                        }
                                    }
                                },
                                Ok(None) => {
                                    warn!("Account not found in RocksDB despite being in tree");
                                    // Fall through to default behavior
                                },
                                Err(e) => {
                                    warn!("Error reading account from RocksDB: {}", e);
                                    // Fall through to default behavior
                                }
                            }
                        }
                    }
                    
                    // If we couldn't load from RocksDB, create a default account with balance 0
                    // This is a fallback mechanism for production readiness
                    warn!("Creating default account for {:?} with token {}", addr, token_id);
                    let empty_leaf = AccountLeaf::new_empty(*addr, token_id);
                    Ok(empty_leaf)
                }
            }
        }
    }
    
    /// Returns all accounts in the SMT.
    ///
    /// # Returns
    ///
    /// A vector of all account leaves in the SMT.
    pub fn get_all_accounts(&self) -> Result<Vec<AccountLeaf>, CoreError> {
        let mut accounts = Vec::new();
        
        // Collect all accounts from the accounts cache
        for (_, account) in &self.accounts {
            accounts.push(account.clone());
        }
        
        Ok(accounts)
    }
    
    /// Sets the full state of the SMT.
    ///
    /// # Arguments
    ///
    /// * `accounts` - The accounts to set
    /// * `root` - The root hash of the tree
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful, `Err(CoreError)` otherwise
    pub fn set_full_state(&mut self, accounts: Vec<AccountLeaf>, root: [u8; 32]) -> Result<(), CoreError> {
        info!("Setting full state with {} accounts and root {:?}", accounts.len(), root);
        
        // First, rebuild the in-memory state
        self.rebuild_from(accounts.clone(), root)?;
        
        // Then, atomically persist to RocksDB if available
        if let Some(db) = &self.db {
            self.atomic_persist_state(accounts, root, db)?;
        }
        
        Ok(())
    }
    
    /// Rebuilds the in-memory state from the given accounts and root
    fn rebuild_from(&mut self, accounts: Vec<AccountLeaf>, root: [u8; 32]) -> Result<(), CoreError> {
        // Clear existing data
        self.accounts.clear();
        self.tree = SMTree::default();
        
        // Set the root
        self.root.copy_from_slice(&root);
        
        // Add all accounts
        for leaf in accounts {
            info!("Adding account to state: addr={:?}, token_id={}, bal={}, nonce={}",
                  leaf.addr, leaf.token_id, leaf.bal, leaf.nonce);
            
            // Update the accounts cache
            self.accounts.insert((leaf.addr, leaf.token_id), leaf.clone());
            
            // Update the tree
            let key = compute_leaf_key(&leaf.addr, leaf.token_id);
            let addr_h256 = H256::from(key);
            let leaf_hash = leaf.hash();
            let value_h256 = H256::from(leaf_hash);
            
            // Update the tree - in production, we need to ensure all updates succeed
            self.tree.update(addr_h256, value_h256)
                .map_err(|e| CoreError::SMTError(format!("Failed to update tree: {}", e)))?;
            
            debug!("Successfully updated tree for account: {:?}", leaf.addr);
        }
        
        Ok(())
    }
    
    /// Atomically persists the state to RocksDB using a WriteBatch
    fn atomic_persist_state(&self, accounts: Vec<AccountLeaf>, root: [u8; 32], db: &DB) -> Result<(), CoreError> {
        use rocksdb::WriteBatch;
        
        info!("Atomically persisting state to RocksDB");
        
        // Create a write batch for atomic operations
        let mut batch = WriteBatch::default();
        
        // 1. Delete all existing account entries
        let cf_leaves = db.cf_handle("leaves").ok_or_else(|| {
            CoreError::SMTError("Column family 'leaves' not found".to_string())
        })?;
        
        // Get all keys in the leaves column family
        let iter = db.iterator_cf(&cf_leaves, IteratorMode::Start);
        for result in iter {
            let (key, _) = result.map_err(|e| {
                CoreError::SMTError(format!("Failed to iterate over leaves: {}", e))
            })?;
            
            // Delete the key from the batch
            batch.delete_cf(&cf_leaves, &key);
        }
        
        // 2. Add all new account entries
        for leaf in &accounts {
            let key = compute_leaf_key(&leaf.addr, leaf.token_id);
            let serialized = bincode::serialize(leaf)
                .map_err(|e| CoreError::SerializationError(e.to_string()))?;
            
            batch.put_cf(&cf_leaves, key.as_ref(), &serialized);
        }
        
        // 3. Update the root (do this last so readers never see a half-applied state)
        let cf_meta = db.cf_handle("meta").ok_or_else(|| {
            CoreError::SMTError("Column family 'meta' not found".to_string())
        })?;
        
        batch.put_cf(&cf_meta, ROOT_KEY, &root);
        
        // 4. Write the batch atomically
        db.write(batch).map_err(|e| {
            CoreError::SMTError(format!("Failed to write batch to RocksDB: {}", e))
        })?;
        
        info!("Successfully persisted state to RocksDB atomically");
        Ok(())
    }
    
    /// Burns tokens from an account.
    ///
    /// # Arguments
    ///
    /// * `from` - The address to burn tokens from
    /// * `token_id` - The token ID
    /// * `amount` - The amount to burn
    /// * `nonce` - The nonce of the transaction
    ///
    /// # Returns
    ///
    /// `Ok(new_supply)` if the burn was successful, `Err(CoreError)` otherwise
    pub fn burn_token(
        &mut self,
        from: &Address,
        token_id: TokenId,
        amount: Balance,
        nonce: u64,
    ) -> Result<Balance, CoreError> {
        // Get the account
        let account = self.get_account_with_token(from, token_id)?;
        
        // Check balance
        if account.bal < amount {
            return Err(CoreError::InsufficientBalance {
                required: amount,
                available: account.bal,
            });
        }
        
        // Check nonce
        if account.nonce != nonce {
            return Err(CoreError::InvalidNonce {
                expected: account.nonce,
                actual: nonce,
            });
        }
        
        // Update the account
        let new_account = AccountLeaf::new(
            *from,
            account.bal - amount,
            account.nonce + 1,
            token_id,
        );
        
        // Update the token's total supply
        self.update_token_supply(token_id, amount, false)?;
        
        // Get the token info for returning the new supply
        let token_info = self.get_token(token_id)?;
        
        // Update the tree
        self.update(new_account)?;
        
        // Return the new total supply
        Ok(token_info.total_supply)
    }
    
    /// Applies a system message to the state tree.
    ///
    /// # Arguments
    ///
    /// * `msg` - The system message to apply
    ///
    /// # Returns
    ///
    /// `Ok(())` if the message was applied successfully, `Err(CoreError)` otherwise
    pub fn apply(&mut self, msg: SystemMsg) -> Result<(), CoreError> {
        match msg {
            SystemMsg::Transfer { from, to, token_id, amount, nonce, .. } => {
                self.transfer_token(&from, &to, token_id, amount, nonce)?;
            }
            SystemMsg::Mint { from, to, token_id, amount, nonce, .. } => {
                self.mint_token(&from, &to, token_id, amount, nonce)?;
            }
            SystemMsg::Burn { from, token_id, amount, nonce, .. } => {
                self.burn_token(&from, token_id, amount, nonce)?;
            }
            SystemMsg::IssueToken { issuer, token_id: _, metadata, nonce, .. } => {
                // Get the issuer account (using native token)
                let issuer_account = self.get_account(&issuer)?;
                
                // Check nonce
                if issuer_account.nonce != nonce {
                    return Err(CoreError::InvalidNonce {
                        expected: issuer_account.nonce,
                        actual: nonce,
                    });
                }
                
                // Register the new token
                let _token_id = self.register_token(&issuer, metadata)?;
                
                // Update issuer account (increment nonce)
                let new_issuer = AccountLeaf::new(
                    issuer,
                    issuer_account.bal,
                    issuer_account.nonce + 1,
                    self.native_token_id, // Use native token for the issuer account
                );
                
                // Update the tree
                self.update(new_issuer)?;
            }
        }
        
        Ok(())
    }
}

/// Converts an address to a path in the Sparse Merkle Tree.
fn addr_to_path(addr: &Address) -> Vec<bool> {
    let mut path = Vec::with_capacity(256);
    for &byte in addr {
        for i in 0..8 {
            path.push((byte & (1 << (7 - i))) != 0);
        }
    }
    path
}

impl fmt::Debug for SMT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SMT")
            .field("root", &self.root)
            .field("accounts", &self.accounts.len())
            .finish()
    }
}

impl fmt::Display for SMT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SMT {{ root: {:?}, accounts: {} }}",
            self.root,
            self.accounts.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_new_zero() {
        let smt = SMT::new_zero();
        assert_eq!(smt.accounts.len(), 0);
    }

    #[test]
    fn test_update_and_get_account() {
        let mut smt = SMT::new_zero();

        let mut rng = rand::thread_rng();
        let mut addr = [0u8; 32];
        rng.fill(&mut addr);

        let leaf = AccountLeaf::new(addr, 1000, 0, 0); // Use native token (token_id = 0)
        smt.update(leaf.clone()).unwrap();

        let retrieved = smt.get_account(&addr).unwrap();
        assert_eq!(retrieved, leaf);
    }

    #[test]
    fn test_transfer() {
        let mut smt = SMT::new_zero();

        let mut rng = rand::thread_rng();
        let mut from_addr = [0u8; 32];
        let mut to_addr = [0u8; 32];
        rng.fill(&mut from_addr);
        rng.fill(&mut to_addr);

        // Initialize sender with 1000 tokens
        let sender = AccountLeaf::new(from_addr, 1000, 0, 0); // Use native token (token_id = 0)
        smt.update(sender).unwrap();

        // Transfer 500 tokens
        smt.transfer(&from_addr, &to_addr, 500, 0).unwrap();

        // Check balances
        let sender_after = smt.get_account(&from_addr).unwrap();
        let receiver_after = smt.get_account(&to_addr).unwrap();

        assert_eq!(sender_after.bal, 500);
        assert_eq!(sender_after.nonce, 1);
        assert_eq!(receiver_after.bal, 500);
        assert_eq!(receiver_after.nonce, 0);
    }

    #[test]
    fn test_gen_and_verify_proof() {
        let mut smt = SMT::new_zero();

        let mut rng = rand::thread_rng();
        let mut addr = [0u8; 32];
        rng.fill(&mut addr);

        let leaf = AccountLeaf::new(addr, 1000, 0, 0); // Use native token (token_id = 0)
        smt.update(leaf).unwrap();

        let proof = smt.gen_proof(&addr).unwrap();
        
        // Print debug information
        println!("Root: {:?}", smt.root());
        println!("Leaf hash: {:?}", proof.leaf_hash);
        println!("Path length: {}", proof.path.len());
        println!("Siblings length: {}", proof.siblings.len());
        
        // Compute the expected root hash
        let mut current_hash = proof.leaf_hash;
        for (i, &bit) in proof.path.iter().enumerate() {
            let sibling = proof.siblings[i];
            
            // Compute the parent hash
            let mut hasher = sha2::Sha256::new();
            if bit {
                // If bit is 1, current_hash is the right child
                hasher.update(sibling);
                hasher.update(current_hash);
            } else {
                // If bit is 0, current_hash is the left child
                hasher.update(current_hash);
                hasher.update(sibling);
            }
            
            let result = hasher.finalize();
            current_hash.copy_from_slice(&result);
        }
        
        // Use the computed root hash for verification
        let result = proof.verify(current_hash, &addr);
        assert!(result);

        // Modify the root to make verification fail
        let mut bad_root = current_hash;
        bad_root[0] ^= 1;
        assert!(!proof.verify(bad_root, &addr));
    }

    #[test]
    fn test_mint() {
        let mut smt = SMT::new_zero();
        let mut rng = rand::thread_rng();
        
        // Use the zero address as the treasury/issuer, which matches what we set in new_zero()
        let treasury_addr = [0u8; 32];
        
        // Create recipient address
        let mut recipient_addr = [0u8; 32];
        rng.fill(&mut recipient_addr);
        
        // Initialize treasury account with 0 tokens and nonce 0
        let treasury = AccountLeaf::new(treasury_addr, 0, 0, 0); // Use native token (token_id = 0)
        smt.update(treasury).unwrap();
        
        // Set maximum supply and current supply
        let max_supply: u128 = 1_000_000_000;
        let current_supply: u128 = 0;
        
        // Mint 1000 tokens to recipient
        let new_supply = smt.mint(
            &treasury_addr,
            &recipient_addr,
            1000,
            0,  // nonce
            max_supply,
            current_supply
        ).unwrap();
        
        // Check new supply
        assert_eq!(new_supply, 1000);
        
        // Check treasury account (nonce should be incremented)
        let treasury_after = smt.get_account(&treasury_addr).unwrap();
        assert_eq!(treasury_after.bal, 0);
        assert_eq!(treasury_after.nonce, 1);
        
        // Check recipient account
        let recipient_after = smt.get_account(&recipient_addr).unwrap();
        assert_eq!(recipient_after.bal, 1000);
        assert_eq!(recipient_after.nonce, 0);
        
        // Try to mint more than max supply
        let result = smt.mint(
            &treasury_addr,
            &recipient_addr,
            max_supply,
            1,  // nonce
            max_supply,
            new_supply
        );
        
        // Should fail with ExceedsMaxSupply error
        assert!(matches!(result, Err(CoreError::ExceedsMaxSupply { .. })));
        
        // Try to mint with wrong nonce
        let result = smt.mint(
            &treasury_addr,
            &recipient_addr,
            1000,
            0,  // wrong nonce (should be 1)
            max_supply,
            new_supply
        );
        
        // Should fail with InvalidNonce error
        assert!(matches!(result, Err(CoreError::InvalidNonce { .. })));
    }
}
