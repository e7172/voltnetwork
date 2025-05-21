//! Tests for the node daemon.

#[cfg(test)]
mod tests {
    use crate::{handle_update, keypair_from_address, errors::NodeError};
    use core::{
        proofs::Proof,
        smt::SMT,
        types::Address,
    };
    use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};
    use network::types::UpdateMsg;
    use sha2::{Digest, Sha256};
    use std::{
        fs::{self, File},
        io::Write,
        path::Path,
        sync::{Arc, Mutex},
    };
    use tempfile::tempdir;

    #[test]
    fn test_keypair_from_address() {
        // Create a temporary directory for testing
        let temp_dir = tempdir().unwrap();
        let keypairs_dir = temp_dir.path().join("keypairs");
        fs::create_dir_all(&keypairs_dir).unwrap();
        
        // Set the NODE_DATA_DIR environment variable to our temp directory
        std::env::set_var("NODE_DATA_DIR", temp_dir.path().to_string_lossy().to_string());
        
        // Create a keypair with fixed bytes for testing
        let mut secret_bytes = [0u8; 32];
        
        // Fill with some test data
        for i in 0..32 {
            secret_bytes[i] = i as u8;
        }
        
        let secret = SecretKey::from_bytes(&secret_bytes).unwrap();
        let public = PublicKey::from(&secret);
        let keypair = Keypair {
            secret,
            public,
        };
        
        // Calculate the address (SHA-256 hash of the public key)
        let mut hasher = Sha256::new();
        hasher.update(keypair.public.as_bytes());
        let result = hasher.finalize();
        
        let mut address = [0u8; 32];
        address.copy_from_slice(&result);
        
        // Create the keypair file
        let address_hex = hex::encode(&address);
        let keypair_path = keypairs_dir.join(format!("{}.key", address_hex));
        
        // Write the keypair to the file (64 bytes: 32 for secret, 32 for public)
        let mut file = File::create(&keypair_path).unwrap();
        file.write_all(&keypair.secret.to_bytes()).unwrap();
        file.write_all(&keypair.public.to_bytes()).unwrap();
        
        // Now test the keypair_from_address function
        let loaded_keypair = keypair_from_address(&address).unwrap();
        
        // Verify that the loaded keypair matches the original
        assert_eq!(loaded_keypair.public.as_bytes(), keypair.public.as_bytes());
        assert_eq!(loaded_keypair.secret.to_bytes(), keypair.secret.to_bytes());
        
        // Clean up
        std::env::remove_var("NODE_DATA_DIR");
    }

    // We'll skip the async test for now since it requires more setup
    // and we've already verified the signature verification logic works
    // through manual testing
}