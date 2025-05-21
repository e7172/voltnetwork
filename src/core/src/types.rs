//! Core types for the chainless token transfer network.

use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// A 32-byte address, typically derived from a public key.
pub type Address = [u8; 32];

/// Token balance, represented as a 128-bit unsigned integer.
pub type Balance = u128;

/// Transaction nonce, used to prevent replay attacks.
pub type Nonce = u64;

/// Token ID, used to identify different tokens in the system.
pub type TokenId = u64;

/// Signature, represented as a 64-byte array.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

// Implement serialization for Signature
impl serde::Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as a byte array
        serializer.serialize_bytes(&self.0)
    }
}

// Implement deserialization for Signature
impl<'de> serde::Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SignatureVisitor;

        impl<'de> serde::de::Visitor<'de> for SignatureVisitor {
            type Value = Signature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a 64-byte signature")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 64 {
                    return Err(E::custom(format!(
                        "invalid signature length: {} (expected 64)",
                        v.len()
                    )));
                }

                let mut signature = [0u8; 64];
                signature.copy_from_slice(v);
                Ok(Signature(signature))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut signature = [0u8; 64];
                for i in 0..64 {
                    signature[i] = seq.next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(Signature(signature))
            }
        }

        deserializer.deserialize_bytes(SignatureVisitor)
    }
}

/// System message types for the token transfer network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SystemMsg {
    /// Transfer tokens from one account to another.
    Transfer {
        /// The sender's address
        from: Address,
        /// The recipient's address
        to: Address,
        /// The token ID
        token_id: TokenId,
        /// The amount to transfer
        amount: Balance,
        /// The nonce for this transaction
        nonce: Nonce,
        /// The signature of the sender
        signature: Signature,
    },
    
    /// Mint new tokens (can only be sent by the token issuer).
    Mint {
        /// The issuer's address
        from: Address,
        /// The recipient's address
        to: Address,
        /// The token ID
        token_id: TokenId,
        /// The amount to mint
        amount: Balance,
        /// The nonce for this transaction
        nonce: Nonce,
        /// The signature of the issuer
        signature: Signature,
    },
    
    /// Burn tokens (can only be sent by the token holder).
    Burn {
        /// The holder's address
        from: Address,
        /// The token ID
        token_id: TokenId,
        /// The amount to burn
        amount: Balance,
        /// The nonce for this transaction
        nonce: Nonce,
        /// The signature of the holder
        signature: Signature,
    },
    
    /// Issue a new token (registers a new token ID).
    IssueToken {
        /// The issuer's address
        issuer: Address,
        /// The token ID (assigned by the system)
        token_id: TokenId,
        /// Token metadata (name, symbol, decimals, etc.)
        metadata: String,
        /// The nonce for this transaction
        nonce: Nonce,
        /// The signature of the issuer
        signature: Signature,
    },
}

/// Represents an account leaf in the Sparse Merkle Tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountLeaf {
    /// The account address
    pub addr: Address,
    /// The account balance
    pub bal: Balance,
    /// The account nonce
    pub nonce: Nonce,
    /// The token ID (0 for the native token)
    pub token_id: TokenId,
}

impl AccountLeaf {
    /// Creates a new account leaf with the given address, balance, nonce, and token ID.
    pub fn new(addr: Address, bal: Balance, nonce: Nonce, token_id: TokenId) -> Self {
        Self { addr, bal, nonce, token_id }
    }

    /// Creates a new account leaf with zero balance and nonce.
    pub fn new_empty(addr: Address, token_id: TokenId) -> Self {
        Self {
            addr,
            bal: 0,
            nonce: 0,
            token_id,
        }
    }

    /// Computes the hash of this account leaf.
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.addr);
        
        let mut bal_bytes = [0u8; 16];
        LittleEndian::write_u128(&mut bal_bytes, self.bal);
        hasher.update(bal_bytes);
        
        let mut nonce_bytes = [0u8; 8];
        LittleEndian::write_u64(&mut nonce_bytes, self.nonce);
        hasher.update(nonce_bytes);
        
        let mut token_id_bytes = [0u8; 8];
        LittleEndian::write_u64(&mut token_id_bytes, self.token_id);
        hasher.update(token_id_bytes);
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

/// Represents a token in the registry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenInfo {
    /// The token ID
    pub token_id: TokenId,
    /// The issuer's address
    pub issuer: Address,
    /// Token metadata (name, symbol, decimals, etc.)
    pub metadata: String,
    /// The total supply of the token
    pub total_supply: Balance,
}

impl fmt::Display for AccountLeaf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Account {{ addr: {:?}, bal: {}, nonce: {}, token_id: {} }}",
            self.addr, self.bal, self.nonce, self.token_id
        )
    }
}

impl fmt::Display for TokenInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Token {{ id: {}, issuer: {:?}, metadata: {}, total_supply: {} }}",
            self.token_id, self.issuer, self.metadata, self.total_supply
        )
    }
}

impl fmt::Display for SystemMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemMsg::Transfer { from, to, token_id, amount, nonce, .. } => {
                write!(
                    f,
                    "Transfer {{ from: {:?}, to: {:?}, token_id: {}, amount: {}, nonce: {} }}",
                    from, to, token_id, amount, nonce
                )
            }
            SystemMsg::Mint { from, to, token_id, amount, nonce, .. } => {
                write!(
                    f,
                    "Mint {{ from: {:?}, to: {:?}, token_id: {}, amount: {}, nonce: {} }}",
                    from, to, token_id, amount, nonce
                )
            }
            SystemMsg::Burn { from, token_id, amount, nonce, .. } => {
                write!(
                    f,
                    "Burn {{ from: {:?}, token_id: {}, amount: {}, nonce: {} }}",
                    from, token_id, amount, nonce
                )
            }
            SystemMsg::IssueToken { issuer, token_id, metadata, nonce, .. } => {
                write!(
                    f,
                    "IssueToken {{ issuer: {:?}, token_id: {}, metadata: {}, nonce: {} }}",
                    issuer, token_id, metadata, nonce
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_account_leaf_hash() {
        let mut rng = rand::thread_rng();
        let mut addr = [0u8; 32];
        rng.fill(&mut addr);
        
        let bal = 1000;
        let nonce = 5;
        let token_id = 0;
        
        let leaf = AccountLeaf::new(addr, bal, nonce, token_id);
        let hash = leaf.hash();
        
        // Hash should be deterministic
        assert_eq!(hash, leaf.hash());
        
        // Different leaves should have different hashes
        let leaf2 = AccountLeaf::new(addr, bal + 1, nonce, token_id);
        assert_ne!(hash, leaf2.hash());
        
        // Different token IDs should have different hashes
        let leaf3 = AccountLeaf::new(addr, bal, nonce, token_id + 1);
        assert_ne!(hash, leaf3.hash());
    }

    #[test]
    fn test_new_empty_account() {
        let mut rng = rand::thread_rng();
        let mut addr = [0u8; 32];
        rng.fill(&mut addr);
        
        let token_id = 1;
        let leaf = AccountLeaf::new_empty(addr, token_id);
        assert_eq!(leaf.bal, 0);
        assert_eq!(leaf.nonce, 0);
        assert_eq!(leaf.addr, addr);
        assert_eq!(leaf.token_id, token_id);
    }
}
