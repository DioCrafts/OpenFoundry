use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};

pub fn random_token(byte_len: usize) -> String {
    let mut bytes = vec![0_u8; byte_len];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn random_base32_secret(byte_len: usize) -> String {
    let mut bytes = vec![0_u8; byte_len];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    base32::encode(base32::Alphabet::RFC4648 { padding: false }, &bytes)
}

pub fn hash_token(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

pub fn hash_content(content: &str, salt: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    if let Some(salt) = salt {
        hasher.update(salt.as_bytes());
    }
    hasher.update(content.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

pub fn sign_content(content: &str, key_material: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key_material.as_bytes())
        .expect("hmac accepts arbitrary key sizes");
    mac.update(content.as_bytes());
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

pub fn verify_signature(content: &str, key_material: &str, signature: &str) -> bool {
    sign_content(content, key_material) == signature
}

pub fn generate_recovery_codes(count: usize) -> Vec<String> {
    (0..count)
        .map(|_| {
            random_token(6)
                .chars()
                .take(10)
                .collect::<String>()
                .to_uppercase()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_signatures_roundtrip() {
        let signature = sign_content("payload", "secret");
        assert!(verify_signature("payload", "secret", &signature));
        assert!(!verify_signature("payload", "wrong", &signature));
    }
}
