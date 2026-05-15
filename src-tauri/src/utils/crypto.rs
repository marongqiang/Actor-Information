use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;

const NONCE_SIZE: usize = 12;

/// Encrypt plaintext using AES-256-GCM with a random nonce.
/// Returns base64(nonce + ciphertext).
pub fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("创建加密器失败: {}", e))?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("加密失败: {}", e))?;

    let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

/// Decrypt base64(nonce + ciphertext) using AES-256-GCM.
pub fn decrypt(encoded: &str, key: &[u8; 32]) -> Result<String, String> {
    let data = BASE64.decode(encoded).map_err(|e| format!("Base64解码失败: {}", e))?;

    if data.len() < NONCE_SIZE + 16 {
        return Err("数据太短，无法解密".to_string());
    }

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("创建加密器失败: {}", e))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("解密失败: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8解码失败: {}", e))
}

/// Derive a 32-byte AES key from a machine-specific identifier.
pub fn derive_machine_key() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let machine_id = format!(
        "{}_{}",
        whoami::hostname(),
        whoami::username()
    );
    let mut hasher = Sha256::new();
    hasher.update(machine_id.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

mod whoami {
    pub fn hostname() -> String {
        hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    pub fn username() -> String {
        std::env::var("USERNAME")
            .unwrap_or_else(|_| "unknown".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = derive_machine_key();
        let plaintext = "test_secret_data";
        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(plaintext, decrypted);
    }
}
