use std::fs;
use std::path::PathBuf;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use clap::{Parser, Subcommand};
use pmcimg_core::{decode_and_verify_pmcimg, encode_png_to_pmcimg, EncodeOptions};

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
        /// Ed25519 private seed (32 bytes) as HEX or base64url (no padding)
        #[arg(long)]
        seed: String,
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

fn parse_seed_32(s: &str) -> Result<[u8; 32], String> {
    let bytes = if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(s).map_err(|e| format!("hex decode: {e}"))?
    } else {
        URL_SAFE_NO_PAD.decode(s.as_bytes()).map_err(|e| format!("base64url decode: {e}"))?
    };
    if bytes.len() != 32 {
        return Err(format!("seed must be 32 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
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
            seed,
            producer,
            device_id,
            created_at,
        } => {
            let png_bytes = fs::read(&input).map_err(|e| format!("read input: {e}"))?;
            let seed32 = parse_seed_32(&seed)?;
            let created_at = created_at.unwrap_or_else(now_iso8601);
            let res = encode_png_to_pmcimg(
                &png_bytes,
                &seed32,
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




