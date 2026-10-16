use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};

pub fn encrypt_bytes(data: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    let key = derive_key(passphrase);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("Key: {}", e))?;
    let nonce_bytes = rand_bytes(12);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("Encrypt: {}", e))?;
    let mut out = Vec::new();
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn decrypt_bytes(data: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    if data.len() < 12 {
        return Err("Data too short".into());
    }
    let key = derive_key(passphrase);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("Key: {}", e))?;
    let (nonce_bytes, ciphertext) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decrypt: {}", e))
}

fn derive_key(passphrase: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(passphrase.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn rand_bytes(len: usize) -> Vec<u8> {
    use aes_gcm::aead::rand_core::RngCore;
    let mut buf = vec![0u8; len];
    aes_gcm::aead::OsRng.fill_bytes(&mut buf);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_recovers_plaintext() {
        let plaintext = b"super secret partition image bytes";
        let enc = encrypt_bytes(plaintext, "hunter2").expect("encrypt");
        assert_ne!(&enc[12..], &plaintext[..]); // ciphertext differs
        let dec = decrypt_bytes(&enc, "hunter2").expect("decrypt");
        assert_eq!(dec, plaintext);
    }

    #[test]
    fn wrong_passphrase_fails() {
        let enc = encrypt_bytes(b"data", "right").expect("encrypt");
        assert!(decrypt_bytes(&enc, "wrong").is_err());
    }

    #[test]
    fn nonces_differ_between_calls() {
        let a = encrypt_bytes(b"data", "k").unwrap();
        let b = encrypt_bytes(b"data", "k").unwrap();
        assert_ne!(a[..12], b[..12]); // random nonce prefix
    }

    #[test]
    fn short_input_rejected() {
        assert!(decrypt_bytes(&[0u8; 4], "k").is_err());
    }
}
