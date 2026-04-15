mod error;

pub use error::CryptoError;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Argon2id KDF parameters. Stored alongside encrypted data so decryption
/// can reproduce the same derived key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfParams {
    /// Base64-encoded 32-byte salt
    pub salt: String,
    /// Memory cost in KiB (default: 65536 = 64 MiB)
    pub memory_cost: u32,
    /// Number of iterations (default: 3)
    pub time_cost: u32,
    /// Degree of parallelism (default: 1)
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            salt: BASE64.encode(generate_salt()),
            memory_cost: 65_536,
            time_cost: 3,
            parallelism: 1,
        }
    }
}

/// Encrypted payload: 12-byte nonce + AES-256-GCM ciphertext (includes 16-byte auth tag).
/// Serialized to JSON with base64-encoded fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlob {
    /// Base64-encoded 12-byte nonce
    pub nonce: String,
    /// Base64-encoded ciphertext (plaintext + 16-byte GCM auth tag)
    pub ciphertext: String,
}

/// A 32-byte key that is zeroized when dropped.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DerivedKey([u8; 32]);

impl DerivedKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Generate a cryptographically random 32-byte salt.
pub fn generate_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    salt
}

/// Derive a 32-byte key from a password using Argon2id.
pub fn derive_key(password: &str, params: &KdfParams) -> Result<DerivedKey, CryptoError> {
    let salt_bytes = BASE64
        .decode(&params.salt)
        .map_err(|e| CryptoError::InvalidInput(format!("bad base64 salt: {e}")))?;

    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(params.memory_cost, params.time_cost, params.parallelism, Some(32))
            .map_err(|e| CryptoError::KdfError(e.to_string()))?,
    );

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), &salt_bytes, &mut key)
        .map_err(|e| CryptoError::KdfError(e.to_string()))?;

    Ok(DerivedKey(key))
}

/// Encrypt `plaintext` with AES-256-GCM using the provided 32-byte key.
/// Generates a random 12-byte nonce.
pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<EncryptedBlob, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::EncryptionError(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionError(e.to_string()))?;

    Ok(EncryptedBlob {
        nonce: BASE64.encode(nonce_bytes),
        ciphertext: BASE64.encode(ciphertext),
    })
}

/// Decrypt an `EncryptedBlob` with AES-256-GCM using the provided 32-byte key.
pub fn decrypt(blob: &EncryptedBlob, key: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::DecryptionError(e.to_string()))?;

    let nonce_bytes = BASE64
        .decode(&blob.nonce)
        .map_err(|e| CryptoError::InvalidInput(format!("bad base64 nonce: {e}")))?;
    let ciphertext_bytes = BASE64
        .decode(&blob.ciphertext)
        .map_err(|e| CryptoError::InvalidInput(format!("bad base64 ciphertext: {e}")))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext_bytes.as_ref())
        .map_err(|_| {
            CryptoError::DecryptionError(
                "decryption failed: wrong key or corrupted data".to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encrypt_decrypt() {
        let key = generate_salt(); // 32 random bytes as key
        let plaintext = b"hello world, this is a secret message";

        let blob = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&blob, &key).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_fails_decrypt() {
        let key1 = generate_salt();
        let key2 = generate_salt();
        let plaintext = b"secret data";

        let blob = encrypt(plaintext, &key1).unwrap();
        let result = decrypt(&blob, &key2);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CryptoError::DecryptionError(_)));
    }

    #[test]
    fn derive_key_deterministic() {
        let params = KdfParams {
            salt: BASE64.encode([42u8; 32]),
            memory_cost: 256, // low for test speed
            time_cost: 1,
            parallelism: 1,
        };

        let key1 = derive_key("my_password", &params).unwrap();
        let key2 = derive_key("my_password", &params).unwrap();

        assert_eq!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn derive_key_different_salt_differs() {
        let params1 = KdfParams {
            salt: BASE64.encode([1u8; 32]),
            memory_cost: 256,
            time_cost: 1,
            parallelism: 1,
        };
        let params2 = KdfParams {
            salt: BASE64.encode([2u8; 32]),
            memory_cost: 256,
            time_cost: 1,
            parallelism: 1,
        };

        let key1 = derive_key("same_password", &params1).unwrap();
        let key2 = derive_key("same_password", &params2).unwrap();

        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn derive_key_different_password_differs() {
        let params = KdfParams {
            salt: BASE64.encode([42u8; 32]),
            memory_cost: 256,
            time_cost: 1,
            parallelism: 1,
        };

        let key1 = derive_key("password_a", &params).unwrap();
        let key2 = derive_key("password_b", &params).unwrap();

        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn full_kdf_then_encrypt_decrypt() {
        let params = KdfParams {
            salt: BASE64.encode(generate_salt()),
            memory_cost: 256,
            time_cost: 1,
            parallelism: 1,
        };

        let derived = derive_key("test_passphrase", &params).unwrap();
        let plaintext = b"encrypted with derived key";

        let blob = encrypt(plaintext, derived.as_bytes()).unwrap();
        let decrypted = decrypt(&blob, derived.as_bytes()).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypted_blob_serialization() {
        let key = generate_salt();
        let blob = encrypt(b"test", &key).unwrap();

        let json = serde_json::to_string(&blob).unwrap();
        let deserialized: EncryptedBlob = serde_json::from_str(&json).unwrap();

        let decrypted = decrypt(&deserialized, &key).unwrap();
        assert_eq!(decrypted, b"test");
    }

    #[test]
    fn kdf_params_serialization() {
        let params = KdfParams::default();
        let json = serde_json::to_string(&params).unwrap();
        let deserialized: KdfParams = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.memory_cost, params.memory_cost);
        assert_eq!(deserialized.time_cost, params.time_cost);
        assert_eq!(deserialized.parallelism, params.parallelism);
        assert_eq!(deserialized.salt, params.salt);
    }
}
