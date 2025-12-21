use rand::Rng;
use sha2::{Digest, Sha256};

/// Generate a random API key with the format: pk_live_{random_hex}
///
/// The key consists of:
/// - Prefix: "pk_live_" (8 characters)
/// - Random hex string: 32 characters (128 bits of entropy)
///
/// Total length: 40 characters
///
/// # Returns
/// A tuple containing:
/// - The plain-text API key (to be shown to user once)
/// - The SHA-256 hash of the API key (to be stored in database)
/// - The key prefix (first 8 characters for identification)
///
/// # Example
/// ```
/// let (api_key, hash, prefix) = generate_api_key();
/// // api_key: "pk_live_a1b2c3d4e5f6..."
/// // hash: "sha256_hash_of_the_key"
/// // prefix: "pk_live_"
/// ```
pub fn generate_api_key(prod_flag: bool) -> (String, String, String) {
    let prod_prefix = "pk_live_";
    let test_prefix = "pk_test_";

    // Generate 16 random bytes (128 bits)
    let mut rng = rand::thread_rng();
    let random_bytes: [u8; 16] = rng.r#gen();
    
    // Convert to hex string (32 characters)
    let random_hex = hex::encode(random_bytes);
    
    // Create the full API key with prefix
    let api_key = if prod_flag {
        format!("{}{}", prod_prefix, random_hex)
    } else {
        format!("{}{}", test_prefix, random_hex)
    };

    let prefix: &str = if prod_flag { prod_prefix } else { test_prefix };

    // Hash the API key using SHA-256
    let hash = hash_api_key(&api_key);
    
    (api_key, hash, prefix.to_string())
}

/// Hash an API key using SHA-256
///
/// # Arguments
/// * `api_key` - The plain-text API key to hash
///
/// # Returns
/// The hexadecimal representation of the SHA-256 hash
pub fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Verify if a plain-text API key matches a stored hash
///
/// # Arguments
/// * `api_key` - The plain-text API key to verify
/// * `stored_hash` - The stored SHA-256 hash to compare against
///
/// # Returns
/// `true` if the API key matches the hash, `false` otherwise
pub fn verify_api_key(api_key: &str, stored_hash: &str) -> bool {
    let computed_hash = hash_api_key(api_key);
    computed_hash == stored_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key_format() {
        let (api_key, hash, prefix) = generate_api_key(true);
        
        println!("API Key: {}", api_key);
        println!("Hash: {}", hash);
        println!("Prefix: {}", prefix);
        
        // Check prefix
        assert_eq!(prefix, "pk_live_");
        assert!(api_key.starts_with("pk_live_"));

        // Check total length (pk_live_ = 8 chars + 32 hex chars = 40 total)
        assert_eq!(api_key.len(), 40);
        
        // Check hash is 64 characters (SHA-256 produces 32 bytes = 64 hex chars)
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_generate_api_key_uniqueness() {
        let (key1, _, _) = generate_api_key(true);
        let (key2, _, _) = generate_api_key(true);
        println!("key1: {}", key1);
        println!("key2: {}", key2);
        // Two generated keys should be different
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_hash_api_key_consistency() {
        let test_key = "pk_live_test123456789abcdef0123456";
        let hash1 = hash_api_key(test_key);
        let hash2 = hash_api_key(test_key);
        println!("hash1: {}", hash1);
        println!("hash2: {}", hash2);
        // Same input should produce same hash
        assert_eq!(hash1, hash2);
        
        // Hash should be 64 characters
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_verify_api_key() {
        let (api_key, hash, _) = generate_api_key(true);
        println!("api_key: {}", api_key);
        println!("hash: {}", hash);
        // Correct key should verify
        assert!(verify_api_key(&api_key, &hash));
        
        // Wrong key should not verify
        assert!(!verify_api_key("pk_live_wrongkey", &hash));
    }

    #[test]
    fn test_different_keys_different_hashes() {
        let key1 = "pk_live_key1";
        let key2 = "pk_live_key2";
        println!("key1: {}", key1);
        println!("key2: {}", key2);
        let hash1 = hash_api_key(key1);
        let hash2 = hash_api_key(key2);
        println!("hash1: {}", hash1);
        println!("hash2: {}", hash2);
        // Different keys should produce different hashes
        assert_ne!(hash1, hash2);
    }
}
