use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
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