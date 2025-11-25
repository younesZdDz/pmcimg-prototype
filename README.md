# PMC-Image v0 Prototype

Implements the `.pmcimg` container format per [`spec/pmcimg-v0.md`](./spec/pmcimg-v0.md)(**We strongly suggest reading the spec first**):
- AES-256-GCM encryption of the original PNG
- Manifest with crypto params and provenance, signed with Ed25519
- Canonicalized JSON for signing; container with header + manifest + ciphertext

## Layout
```
pmcimg-prototype/
  spec/pmcimg-v0.md
  rust/pmcimg-core/        # Rust library (optionally compiled to WASM)
  cli/pmcimg-cli/          # Rust CLI: encode/decode
```

## Build (Rust)
Workspace build:
```
cargo build --release
```

### CLI usage
Encode:
```
cargo run --package pmcimg-cli -- encode \
  --input input.png \
  --output output.pmcimg \
  --seed 6f6e6c79796f75726f776e33326279746573656564686578 \
  --producer "demo-producer" \
  --device-id "device-1234" \
  --created-at "2025-11-22T18:00:00Z"
```

Decode:
```
cargo run --package pmcimg-cli -- decode \
  --input output.pmcimg \
  --output restored.png
```

Seed can be hex (64 chars) or base64url (no padding).

## WASM (scaffold)
Build the core with `--features wasm` via wasm-pack:
```sh
cargo install wasm-pack

cd rust/pmcimg-core

wasm-pack build --target web --features wasm
```
Then wire `js/pmcimg-wasm-wrapper` to use the generated package and `js/example-web-demo/demo.js` to import it.

## Security notes
- v0 stores the symmetric key in cleartext in the manifest by design (to enforce “manifest required”).
- Future versions should wrap the key to a device or platform public key and/or enforce stronger provenance policies.




