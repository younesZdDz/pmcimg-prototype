use thiserror::Error;

#[derive(Debug, Error)]
pub enum PmcImgError {
    #[error("invalid container magic")]
    InvalidMagic,
    #[error("unsupported version: {0}")]
    UnsupportedVersion(u8),
    #[error("invalid lengths or offsets in container")]
    InvalidLengths,
    #[error("manifest parse error: {0}")]
    ManifestParse(String),
    #[error("ciphertext hash mismatch (pixel block tampered)")]
    CiphertextHashMismatch,
    #[error("signature verification failed")]
    SignatureInvalid,
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("png parse error: {0}")]
    Png(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("wasm input error: {0}")]
    WasmInput(String),
}

pub type PmcImgResult<T> = Result<T, PmcImgError>;




