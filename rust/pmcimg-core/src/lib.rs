mod container;
mod crypto;
mod error;
mod manifest;
mod png;

pub use crate::error::{PmcImgError, PmcImgResult};

use crate::container::{build_container, parse_container};
use crate::crypto::{
    b64url_nopad_encode, decrypt_aes_gcm, ed25519_keypair_from_seed, ed25519_sign, encrypt_aes_gcm, random_key,
    random_nonce, sha256_hex,
};
use crate::manifest::{build_manifest, Manifest};
use crate::png::parse_png_dimensions;
use serde::{Deserialize, Serialize};
use ed25519_dalek::SigningKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeOptions {
    pub producer: String,
    pub device_id: String,
    pub created_at: String, // ISO 8601
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeResult {
    pub pmcimg_bytes: Vec<u8>,
    pub manifest: Manifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodeResult {
    pub png_bytes: Vec<u8>,
    pub manifest: Manifest,
}

pub fn encode_png_to_pmcimg(
    png_bytes: &[u8],
    signing_seed32: &[u8; 32],
    opts: EncodeOptions,
) -> PmcImgResult<EncodeResult> {
    // Step 0 – parse PNG dims
    let (width, height) = parse_png_dimensions(png_bytes)?;
    // Step 1 – generate K, N
    let key = random_key();
    let nonce = random_nonce();
    // Step 2 – encrypt
    let ciphertext_full = encrypt_aes_gcm(&key, &nonce, png_bytes)?;
    let ciphertext_sha256_hex = sha256_hex(&ciphertext_full);
    // Step 3 – prepare manifest (without sig)
    let (sk, vk) = ed25519_keypair_from_seed(signing_seed32);
    let pubkey_b64 = b64url_nopad_encode(vk.as_bytes());
    let nonce_b64 = b64url_nopad_encode(&nonce);
    let key_b64 = b64url_nopad_encode(&key);
    let mut manifest = build_manifest(
        width,
        height,
        nonce_b64,
        key_b64,
        ciphertext_sha256_hex.clone(),
        opts.producer,
        opts.device_id,
        opts.created_at,
        pubkey_b64,
    );
    // Step 4 – sign
    let to_sign = manifest.compute_to_sign(&ciphertext_sha256_hex)?;
    let sig = ed25519_sign(&sk, &to_sign);
    manifest.set_signature(b64url_nopad_encode(&sig));
    // Step 5 – container
    let manifest_bytes = serde_json::to_vec(&manifest)
        .map_err(|e| PmcImgError::ManifestParse(format!("encode manifest: {e}")))?;
    let pmcimg_bytes = build_container(&manifest_bytes, &ciphertext_full);
    Ok(EncodeResult {
        pmcimg_bytes,
        manifest,
    })
}

/// Encode using an already-loaded Ed25519 signing key (e.g., from PEM/PKCS#8) and an optional key_id.
pub fn encode_png_to_pmcimg_with_signing_key(
    png_bytes: &[u8],
    signing_key: &SigningKey,
    key_id: Option<String>,
    opts: EncodeOptions,
) -> PmcImgResult<EncodeResult> {
    // Step 0 – parse PNG dims
    let (width, height) = parse_png_dimensions(png_bytes)?;
    // Step 1 – generate K, N
    let key = random_key();
    let nonce = random_nonce();
    // Step 2 – encrypt
    let ciphertext_full = encrypt_aes_gcm(&key, &nonce, png_bytes)?;
    let ciphertext_sha256_hex = sha256_hex(&ciphertext_full);
    // Step 3 – prepare manifest (without sig)
    let vk = signing_key.verifying_key();
    let pubkey_b64 = b64url_nopad_encode(vk.as_bytes());
    let nonce_b64 = b64url_nopad_encode(&nonce);
    let key_b64 = b64url_nopad_encode(&key);
    let mut manifest = build_manifest(
        width,
        height,
        nonce_b64,
        key_b64,
        ciphertext_sha256_hex.clone(),
        opts.producer,
        opts.device_id,
        opts.created_at,
        pubkey_b64,
    );
    if key_id.is_some() {
        manifest.signature.key_id = key_id;
    }
    // Step 4 – sign
    let to_sign = manifest.compute_to_sign(&ciphertext_sha256_hex)?;
    let sig = crate::crypto::ed25519_sign(signing_key, &to_sign);
    manifest.set_signature(b64url_nopad_encode(&sig));
    // Step 5 – container
    let manifest_bytes = serde_json::to_vec(&manifest)
        .map_err(|e| PmcImgError::ManifestParse(format!("encode manifest: {e}")))?;
    let pmcimg_bytes = build_container(&manifest_bytes, &ciphertext_full);
    Ok(EncodeResult {
        pmcimg_bytes,
        manifest,
    })
}

pub fn decode_and_verify_pmcimg(pmcimg_bytes: &[u8]) -> PmcImgResult<DecodeResult> {
    // Step 1 – parse container
    let parts = parse_container(pmcimg_bytes)?;
    // Step 2 – parse manifest
    let manifest: Manifest = serde_json::from_slice(&parts.manifest_bytes)
        .map_err(|e| PmcImgError::ManifestParse(format!("{e}")))?;
    // Step 2b – hash check already inside verify below
    // Step 3 – verify signature
    manifest.verify(&parts.ciphertext_full)?;
    // Step 4 – decrypt pixels
    let key_bytes = crate::crypto::b64url_nopad_decode(&manifest.crypto.key)?;
    let nonce_bytes = crate::crypto::b64url_nopad_decode(&manifest.crypto.nonce)?;
    if key_bytes.len() != 32 || nonce_bytes.len() != 12 {
        return Err(PmcImgError::Crypto("wrong key/nonce length".into()));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&key_bytes);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_bytes);
    let png_bytes = decrypt_aes_gcm(&key, &nonce, &parts.ciphertext_full)?;
    // Step 5 – return
    Ok(DecodeResult { png_bytes, manifest })
}

#[cfg(feature = "wasm")]
mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub struct WasmEncodeResult {
        pmcimg_bytes: Vec<u8>,
        manifest_json: String,
    }

    #[wasm_bindgen]
    impl WasmEncodeResult {
        #[wasm_bindgen(getter)]
        pub fn pmcimg_bytes(&self) -> Vec<u8> {
            self.pmcimg_bytes.clone()
        }
        #[wasm_bindgen(getter)]
        pub fn manifest_json(&self) -> String {
            self.manifest_json.clone()
        }
    }

    #[wasm_bindgen]
    pub fn wasm_encode_png_to_pmcimg(
        png_bytes: &[u8],
        signing_seed32: &[u8],
        producer: String,
        device_id: String,
        created_at: String,
    ) -> Result<WasmEncodeResult, JsValue> {
        if signing_seed32.len() != 32 {
            return Err(JsValue::from_str("signing_seed32 must be 32 bytes"));
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(signing_seed32);
        let res = encode_png_to_pmcimg(
            png_bytes,
            &seed,
            EncodeOptions {
                producer,
                device_id,
                created_at,
            },
        )
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;
        let manifest_json = serde_json::to_string(&res.manifest).map_err(|e| JsValue::from_str(&format!("{e}")))?;
        Ok(WasmEncodeResult {
            pmcimg_bytes: res.pmcimg_bytes,
            manifest_json,
        })
    }

    #[wasm_bindgen]
    pub fn wasm_decode_and_verify_pmcimg(pmcimg_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
        let res = decode_and_verify_pmcimg(pmcimg_bytes).map_err(|e| JsValue::from_str(&format!("{e}")))?;
        Ok(res.png_bytes)
    }
}




