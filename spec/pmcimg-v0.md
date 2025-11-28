## 1. Scope: “PMC-Image v0” format
- File extension: `.pmcimg`
- For now supports: `PNG` as payload
- Container: very simple custom binary format
- Manifest: `JSON` (maybe `CBOR` in future versions), signed with `Ed25519`
- Pixels: the original `PNG` file, encrypted with `AES-GCM`
- Manifest contains:
    - Info about the image (width, height, mime)
    - The hash of the encrypted pixel block
    - The symmetric key + nonce (or key wrapped in future versions)
    - The signature over the manifest + ciphertext hash

**Key property:**
<pre>
If you delete or corrupt the manifest, you lose the symmetric key → you cannot decrypt the pixel block → the image is unreadable.
</pre>
This hits the PMC requirement: **manifest is required to decode.**

## 2. File format design

Let’s define a container layout:

| Field                  | Size          | Description                              |
|------------------------|---------------|------------------------------------------|
| **Magic**              | 4 bytes       | File signature (`PMCI`)            |
| **Version**            | 1 byte        | Format version                            |
| **ManifestLen**        | 4 bytes       | Length of manifest section (bytes)        |
| **CiphertextLen**      | 8 bytes       | Length of encrypted media bytes           |
| **Manifest**           | ManifestLen   | `JSON` or `CBOR` manifest bytes               |
| **Ciphertext (PNG)**   | CiphertextLen | Encrypted `PNG` bytes           |



**Concretely:**
- Magic: `ASCII` `"PMCI"` (Protected Media Container - Image)
- Version: `0x01`
- ManifestLen: `uint32` big-endian
- CiphertextLen: `uint64` big-endian
- Then:
    - ManifestLen bytes of manifest (`UTF-8` `JSON`, or `CBOR`)
    - CiphertextLen bytes of encrypted `PNG`

**Manifest structure (simplified JSON version)**

For `v0`, something like:
```json
{
  "format": "pmcimg",
  "version": 1,

  "media": {
    "mime": "image/png",
    "width": 1024,
    "height": 768
  },

  // Crypto over the pixel block
  "crypto": {
    "cipher": "AES-256-GCM",
    "nonce": "BASE64URL_NONCE",
    "key": "BASE64URL_SYMM_KEY",        // v0: stored in cleartext, just to enforce “manifest required”
    "ciphertext_sha256": "HEX_HASH"     // hash of ciphertext bytes (encrypted PNG)
  },

  // Provenance bits (minimal)
  "provenance": {
    "producer": "DALL-E-foo",
    "device_id": "device-1234",
    "created_at": "2025-11-22T18:00:00Z",
  },

  // Signing key and signature
  "signature": {
    "alg": "Ed25519",
    "pubkey": "BASE64URL_PUBKEY",
    "sig": "BASE64URL_SIGNATURE",
    "signed_payload": "sha256(manifest_without_signature + ciphertext_sha256)"
  }
}
```

**Notes:**

The symmetric key is inside the manifest (for `v0`).

If someone strips the manifest, the ciphertext is useless.

In a future version we’d wrap the key using a device or platform public key, but `v0` can keep it simple.

## 3. Encoding flow (creating a .pmcimg)

Assume we start from `input.png` and we have a producer `Ed25519` keypair.

### Step 0 – read the PNG
```js
png_bytes = read("input.png")
(width, height) = parse_png_dimensions(png_bytes) // via a PNG lib
````

### Step 1 – generate crypto parameters

Generate random `32-byte` symmetric key K

Generate `12-byte` random nonce N for `AES-GCM`

```js
K = random_bytes(32)
N = random_bytes(12)
```
### Step 2 – encrypt the PNG

Use `AES-256-GCM`:

```js
ciphertext, auth_tag = AES_GCM_encrypt(key=K, nonce=N, plaintext=pngBytes, AAD="PMCIMGv1")
````

Store `ciphertext || auth_tag` as the encrypted pixel block. (We can also store tag separately, but concatenating is easy.)

```js
ciphertext = ENC(png_bytes)
ciphertext_full = ciphertext || auth_tag
ciphertext_sha256 = SHA256(ciphertext_full)
```
### Step 3 – prepare manifest (without signature)

Construct manifest object:

```json
{
  "format": "pmcimg",
  "version": 1,
  "media": {
    "mime": "image/png",
    "width": 1024,
    "height": 768
  },
  "crypto": {
    "cipher": "AES-256-GCM",
    "nonce": "BASE64URL(N)",
    "key": "BASE64URL(K)",                // v0 simple variant
    "ciphertext_sha256": "HEX(ciphertext_sha256)"
  },
  "provenance": {
    "producer": "DALL-E-foo",
    "device_id": "device-1234",
    "created_at": "2025-11-22T18:00:00Z",
  },
  "signature": {
    "alg": "Ed25519",
    "pubkey": "BASE64URL(pubkey)",        // public key of the signer
    "sig": null                           // filled in next step
  }
}
```
### Step 4 – sign the manifest + ciphertext hash

We need to define a canonical representation for signing (very important, otherwise verification breaks). For `v0` we will:

- Take the `JSON`, remove `"sig"` or set it to `null`, sort keys in a stable order, and encode `UTF-8` → `manifest_canonical_bytes`.

- Compute the payload to be signed, e.g.:
```js
to_sign = SHA256( manifest_canonical_bytes || ciphertext_sha256 )
sig = Ed25519_sign(private_key, to_sign)
```
- Put `sig` in the manifest:
```json
"signature": {
  "alg": "Ed25519",
  "pubkey": "BASE64URL(pubkey)",
  "sig": "BASE64URL(sig)"
}
```

- Now we encode manifest to bytes:
```js
manifest_bytes = UTF8(JSON.stringify(manifest_canonical))
manifest_len = len(manifest_bytes)
ciphertext_len = len(ciphertext_full)
````

### Step 5 – build the .pmcimg file

Binary layout:
```js
write_bytes("output.pmcimg", [
  "PMCI",               // Magic (4 bytes)
  0x01,                 // Version
  uint32_be(manifest_len),
  uint64_be(ciphertext_len),
  manifest_bytes,
  ciphertext_full
])
```

At this point we have a valid `PMC-Image` `v0`.

## 4. Decoding / verifying flow

Given a `.pmcimg` file:

### Step 1 – parse container

1. Read first 4 bytes → must be `"PMCI"`
2. Read version byte → must be `0x01`
3. Read `manifest_len` `(4B)`
4. Read `ciphertext_len` `(8B)`
5. Read `manifest_bytes` (`manifest_len`)
6. Read `ciphertext_full` (`ciphertext_len`)

If any offset/length is inconsistent → fail.

### Step 2 – parse manifest

- Parse manifest_bytes as `JSON` → manifest.
- Extract:
    - `crypto.nonce`, `crypto.key`, `crypto.ciphertext_sha256`
    - `signature.alg`, `signature.pubkey`, `signature.sig`
- Compute:
```js
ciphertext_sha256_local = SHA256(ciphertext_full)
```
- Check:
```js
if HEX(ciphertext_sha256_local) != manifest.crypto.ciphertext_sha256:
    fail("pixel block tampered")
```

### Step 3 – verify signature

Reconstruct the canonical manifest used for signing:
- Clone manifest
- Set `signature.sig = null` (or remove field, depending on how we specified the canonicalization)
- Encode to canonical `JSON` → `manifest_canonical_bytes`

Compute:
```js
to_verify = SHA256( manifest_canonical_bytes || ciphertext_sha256_local )
```

Verify:
```js
sig_ok = Ed25519_verify(pubkey, to_verify, sig)
if !sig_ok: fail("invalid signature")
```

If this passes:
- We know: manifest + ciphertext link hasn’t been altered, and manifest came from whoever owns that public key (assuming we trust that key).

### Step 4 – decrypt pixels

Extract:
```js
K = BASE64URL_DECODE(manifest.crypto.key)
N = BASE64URL_DECODE(manifest.crypto.nonce)
```

Split `ciphertext_full` into ciphertext and auth_tag depending on our `AES-GCM` implementation.

Decrypt:
```js
png_bytes = AES_GCM_decrypt(key=K, nonce=N, ciphertext, auth_tag, AAD="PMCIMGv1")
```

If decryption fails (`GCM` tag mismatch) → fail: pixels tampered or wrong key.

If ok → we now have a normal `PNG` byte buffer.

### Step 5 – render

Feed `png_bytes` to any standard `PNG` decoder (browser, image lib, etc.) and display.
