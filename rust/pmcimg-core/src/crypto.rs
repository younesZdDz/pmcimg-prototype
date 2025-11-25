use aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::error::{PmcImgError, PmcImgResult};

pub const AAD: &[u8] = b"PMCIMGv1";

pub fn random_key() -> [u8; 32] {
    let mut k = [0u8; 32];
    getrandom::getrandom(&mut k).expect("randomness available");
    k
}

pub fn random_nonce() -> [u8; 12] {
    let mut n = [0u8; 12];
    getrandom::getrandom(&mut n).expect("randomness available");
    n
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(sha256(data))
}

pub fn encrypt_aes_gcm(key: &[u8; 32], nonce: &[u8; 12], plaintext: &[u8]) -> PmcImgResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| PmcImgError::Crypto(format!("{e}")))?;
    let nonce = Nonce::from_slice(nonce);
    let ct = cipher
        .encrypt(nonce, aead::Payload { msg: plaintext, aad: AAD })
        .map_err(|e| PmcImgError::Crypto(format!("{e}")))?;
    Ok(ct)
}

pub fn decrypt_aes_gcm(key: &[u8; 32], nonce: &[u8; 12], ciphertext_and_tag: &[u8]) -> PmcImgResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| PmcImgError::Crypto(format!("{e}")))?;
    let nonce = Nonce::from_slice(nonce);
    let pt = cipher
        .decrypt(nonce, aead::Payload { msg: ciphertext_and_tag, aad: AAD })
        .map_err(|e| PmcImgError::Crypto(format!("{e}")))?;
    Ok(pt)
}

pub fn ed25519_keypair_from_seed(seed32: &[u8; 32]) -> (SigningKey, VerifyingKey) {
    // Deterministic signing key from 32-byte seed
    let sk = SigningKey::from_bytes(seed32);
    let vk = sk.verifying_key();
    (sk, vk)
}

pub fn ed25519_sign(sk: &SigningKey, payload32: &[u8; 32]) -> Vec<u8> {
    sk.sign(payload32).to_bytes().to_vec()
}

pub fn ed25519_verify(vk: &VerifyingKey, payload32: &[u8; 32], sig: &[u8]) -> bool {
    if let Ok(sig) = ed25519_dalek::Signature::from_slice(sig) {
        vk.verify(payload32, &sig).is_ok()
    } else {
        false
    }
}

pub fn b64url_nopad_encode(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

pub fn b64url_nopad_decode(s: &str) -> PmcImgResult<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(s.as_bytes())
        .map_err(|e| PmcImgError::Crypto(format!("base64url decode: {e}")))
}




