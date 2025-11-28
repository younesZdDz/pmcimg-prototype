use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use pmcimg_core::{decode_and_verify_pmcimg, encode_png_to_pmcimg_with_signing_key, EncodeOptions};
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::SigningKey;

#[derive(Parser)]
#[command(name = "pmcimg-cli", about = "PMC-Image v0 CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode input PNG into .pmcimg
    Encode {
        /// input PNG path
        #[arg(short, long)]
        input: PathBuf,
        /// output .pmcimg path
        #[arg(short, long)]
        output: PathBuf,
        /// Path to Ed25519 private key in PKCS#8 PEM format
        #[arg(long)]
        signing_key: PathBuf,
        /// Key identifier to embed in manifest.signature.key_id
        #[arg(long, value_name = "KEY_ID")]
        key_id: String,
        /// Producer string
        #[arg(long)]
        producer: String,
        /// Device id
        #[arg(long, default_value = "device-0000")]
        device_id: String,
        /// Created at (ISO 8601). If omitted, uses current UTC time.
        #[arg(long)]
        created_at: Option<String>,
    },
    /// Decode .pmcimg and verify; writes PNG output
    Decode {
        /// input .pmcimg path
        #[arg(short, long)]
        input: PathBuf,
        /// output PNG path
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn now_iso8601() -> String {
    match time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339) {
        Ok(s) => s,
        Err(_) => "1970-01-01T00:00:00Z".into(),
    }
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Encode {
            input,
            output,
            signing_key,
            key_id,
            producer,
            device_id,
            created_at,
        } => {
            let png_bytes = fs::read(&input).map_err(|e| format!("read input: {e}"))?;
            // Load Ed25519 signing key from PKCS#8 PEM -> DER
            let pem_bytes = fs::read(&signing_key).map_err(|e| format!("read signing key: {e}"))?;
            let (_label, der) = pem_rfc7468::decode_vec(&pem_bytes).map_err(|e| format!("PEM decode: {e}"))?;
            let sk = SigningKey::from_pkcs8_der(&der).map_err(|e| format!("parse signing key (PKCS#8 DER): {e}"))?;
            let created_at = created_at.unwrap_or_else(now_iso8601);
            let res = encode_png_to_pmcimg_with_signing_key(
                &png_bytes,
                &sk,
                Some(key_id),
                EncodeOptions {
                    producer,
                    device_id,
                    created_at,
                },
            )
            .map_err(|e| format!("{e}"))?;
            fs::write(&output, &res.pmcimg_bytes).map_err(|e| format!("write output: {e}"))?;
            println!("{}", serde_json::to_string_pretty(&res.manifest).unwrap_or_default());
            Ok(())
        }
        Commands::Decode { input, output } => {
            let pmc_bytes = fs::read(&input).map_err(|e| format!("read input: {e}"))?;
            let res = decode_and_verify_pmcimg(&pmc_bytes).map_err(|e| format!("{e}"))?;
            fs::write(&output, &res.png_bytes).map_err(|e| format!("write output: {e}"))?;
            println!("{}", serde_json::to_string_pretty(&res.manifest).unwrap_or_default());
            Ok(())
        }
    }
}




