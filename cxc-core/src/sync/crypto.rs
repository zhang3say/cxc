use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::prelude::*;
use rand::RngCore;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptError(String),
    #[error("Decryption failed: incorrect password or corrupted data")]
    DecryptError,
    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("Invalid nonce length: expected 12 bytes, got {0}")]
    InvalidNonceLength(usize),
}

/// Derive a 32-byte AES-256 key from a passphrase using SHA-256.
fn derive_key(passphrase: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Encrypt plaintext bytes using AES-256-GCM.
/// Returns (base64_ciphertext, base64_nonce).
pub fn encrypt_aes_gcm(passphrase: &str, plaintext: &[u8]) -> Result<(String, String), CryptoError> {
    let key_bytes = derive_key(passphrase);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| CryptoError::EncryptError(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::EncryptError(e.to_string()))?;

    let b64_ciphertext = BASE64_STANDARD.encode(ciphertext);
    let b64_nonce = BASE64_STANDARD.encode(nonce_bytes);

    Ok((b64_ciphertext, b64_nonce))
}

/// Decrypt ciphertext bytes using AES-256-GCM with the given passphrase and nonce.
pub fn decrypt_aes_gcm(passphrase: &str, b64_ciphertext: &str, b64_nonce: &str) -> Result<Vec<u8>, CryptoError> {
    let key_bytes = derive_key(passphrase);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| CryptoError::EncryptError(e.to_string()))?;

    let nonce_bytes = BASE64_STANDARD.decode(b64_nonce)?;
    if nonce_bytes.len() != 12 {
        return Err(CryptoError::InvalidNonceLength(nonce_bytes.len()));
    }
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = BASE64_STANDARD.decode(b64_ciphertext)?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| CryptoError::DecryptError)?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let password = "super_secret_password_123";
        let message = b"Hello, Code Cross-Connect!";

        let (b64_cipher, b64_nonce) = encrypt_aes_gcm(password, message).unwrap();
        let decrypted = decrypt_aes_gcm(password, &b64_cipher, &b64_nonce).unwrap();

        assert_eq!(message, decrypted.as_slice());
    }

    #[test]
    fn test_decrypt_with_wrong_password_fails() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let message = b"Confidential API keys";

        let (b64_cipher, b64_nonce) = encrypt_aes_gcm(password, message).unwrap();
        let result = decrypt_aes_gcm(wrong_password, &b64_cipher, &b64_nonce);

        assert!(matches!(result, Err(CryptoError::DecryptError)));
    }
}
