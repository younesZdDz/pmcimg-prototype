use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::crypto::{b64url_nopad_decode, ed25519_verify, sha256, sha256_hex};
use crate::error::{PmcImgError, PmcImgResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub mime: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crypto {
    pub cipher: String,               // "AES-256-GCM"
    pub nonce: String,                // base64url nopad
    pub key: String,                  // base64url nopad (v0: cleartext)
    pub ciphertext_sha256: String,    // lowercase hex
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub producer: String,
    pub device_id: String,
    pub created_at: String, // ISO 8601
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub alg: String,      // "Ed25519"
    pub pubkey: String,   // base64url
    pub sig: Option<String>, // base64url signature; None during canonicalization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>, // optional key identifier for signer (e.g., "midjourney-prod-v1")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String, // "pmcimg"
    pub version: u32,   // 1
    pub media: Media,
    pub crypto: Crypto,
    pub provenance: Provenance,
    pub signature: Signature,
}

impl Manifest {
    pub fn canonicalize_with_sig_null(&self) -> PmcImgResult<Vec<u8>> {
        // Convert to serde_json::Value using BTreeMap to ensure lexicographic key order.
        let mut root = BTreeMap::<String, Value>::new();
        root.insert("format".into(), Value::String(self.format.clone()));
        root.insert("version".into(), Value::Number(self.version.into()));

        // media
        let mut media = BTreeMap::<String, Value>::new();
        media.insert("mime".into(), Value::String(self.media.mime.clone()));
        media.insert("width".into(), Value::Number(self.media.width.into()));
        media.insert("height".into(), Value::Number(self.media.height.into()));
        root.insert("media".into(), Value::Object(media.into_iter().collect()));

        // crypto
        let mut crypto = BTreeMap::<String, Value>::new();
        crypto.insert("cipher".into(), Value::String(self.crypto.cipher.clone()));
        crypto.insert("nonce".into(), Value::String(self.crypto.nonce.clone()));
        crypto.insert("key".into(), Value::String(self.crypto.key.clone()));
        crypto.insert(
            "ciphertext_sha256".into(),
            Value::String(self.crypto.ciphertext_sha256.clone()),
        );
        root.insert("crypto".into(), Value::Object(crypto.into_iter().collect()));

        // provenance
        let mut prov = BTreeMap::<String, Value>::new();
        prov.insert("producer".into(), Value::String(self.provenance.producer.clone()));
        prov.insert("device_id".into(), Value::String(self.provenance.device_id.clone()));
        prov.insert("created_at".into(), Value::String(self.provenance.created_at.clone()));
        root.insert("provenance".into(), Value::Object(prov.into_iter().collect()));

        // signature with sig=null
        let mut sig = BTreeMap::<String, Value>::new();
        sig.insert("alg".into(), Value::String(self.signature.alg.clone()));
        sig.insert("pubkey".into(), Value::String(self.signature.pubkey.clone()));
        sig.insert("sig".into(), Value::Null);
        if let Some(key_id) = &self.signature.key_id {
            sig.insert("key_id".into(), Value::String(key_id.clone()));
        }
        root.insert("signature".into(), Value::Object(sig.into_iter().collect()));

        let canonical_json = serde_json::to_vec(&Value::Object(root.into_iter().collect()))
            .map_err(|e| PmcImgError::ManifestParse(format!("canonical encode: {e}")))?;
        Ok(canonical_json)
    }

    pub fn compute_to_sign(&self, ciphertext_sha256_hex: &str) -> PmcImgResult<[u8; 32]> {
        let mut bytes = self.canonicalize_with_sig_null()?;
        // append the ciphertext hash bytes (hex string should be encoded as UTF-8 per spec wording)
        bytes.extend_from_slice(ciphertext_sha256_hex.as_bytes());
        Ok(sha256(&bytes))
    }

    pub fn set_signature(&mut self, sig_b64: String) {
        self.signature.sig = Some(sig_b64);
    }

    pub fn verify(&self, ciphertext_full: &[u8]) -> PmcImgResult<()> {
        // 1) check ciphertext hash
        let local_hex = sha256_hex(ciphertext_full);
        if local_hex != self.crypto.ciphertext_sha256 {
            return Err(PmcImgError::CiphertextHashMismatch);
        }
        // 2) signature verification
        let to_verify = self.compute_to_sign(&local_hex)?;
        if self.signature.alg != "Ed25519" {
            return Err(PmcImgError::SignatureInvalid);
        }
        let pubkey_bytes = b64url_nopad_decode(&self.signature.pubkey)?;
        let vk = ed25519_dalek::VerifyingKey::try_from(pubkey_bytes.as_slice())
            .map_err(|e| PmcImgError::Crypto(format!("pubkey parse: {e}")))?;
        let sig_bytes = match &self.signature.sig {
            Some(s) => b64url_nopad_decode(s)?,
            None => return Err(PmcImgError::SignatureInvalid),
        };
        if ed25519_verify(&vk, &to_verify, &sig_bytes) {
            Ok(())
        } else {
            Err(PmcImgError::SignatureInvalid)
        }
    }
}

pub fn build_manifest(
    width: u32,
    height: u32,
    nonce_b64: String,
    key_b64: String,
    ciphertext_sha256_hex: String,
    producer: String,
    device_id: String,
    created_at: String,
    pubkey_b64: String,
) -> Manifest {
    Manifest {
        format: "pmcimg".into(),
        version: 1,
        media: Media {
            mime: "image/png".into(),
            width,
            height,
        },
        crypto: Crypto {
            cipher: "AES-256-GCM".into(),
            nonce: nonce_b64,
            key: key_b64,
            ciphertext_sha256: ciphertext_sha256_hex,
        },
        provenance: Provenance {
            producer,
            device_id,
            created_at,
        },
        signature: Signature {
            alg: "Ed25519".into(),
            pubkey: pubkey_b64,
            sig: None,
            key_id: None,
        },
    }
}


