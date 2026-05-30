# UCX Wire-Format Specification (Language-Independent)

> **Status:** Authoritative, byte-level reference for SDK implementers.
> **Scope:** Parse + signature verification + decryption.
> **Source of truth:** This document is derived **directly from the reference Rust implementation**, not from prose specs. Every load-bearing fact cites `file:line`. Where the reference implementation disagrees with the human-readable docs in `docs/`, **the implementation wins** and the discrepancy is noted explicitly.
> **Audience:** Authors of the 12 language SDKs (Rust, Go, Python, TypeScript, Java, C#, Ruby, PHP, Swift, Kotlin, C++, Dart).

A `.ucx` file is a ZIP archive with a fixed internal layout, a JAR-style file manifest, a dual-layer digital signature system, and optional per-chapter encryption using a custom binary container called **UCXE**. This document specifies exactly enough to (a) parse the archive, (b) verify both signature layers, and (c) decrypt UCXE payloads, in any language.

---

## 1. Overview & file roles

| File type | Magic / detection | Purpose |
|-----------|-------------------|---------|
| `.ucx`    | ZIP local file header `50 4B 03 04` at **offset 0** | The novel container (ZIP). |
| **UCXE** payload | `55 43 58 45` ("UCXE") at offset 0 of a file *inside* the archive | An individually encrypted chapter/resource file, stored under its original name inside the `.ucx`. |

A `.ucx` is detected by requiring the ZIP Local File Header signature **at byte offset 0** — a trailing-EOCD-only ZIP (e.g. a payload prepended before a ZIP) is rejected. `ucx-parse/src/lib.rs:37,558-591`

A file inside the archive is detected as encrypted iff its first 4 bytes equal `UCXE`. `ucx-crypto/src/lib.rs:876-878`

The mimetype string is exactly `application/vnd.unicodex+zip`. `ucx-build/src/archive.rs:28`, validated in `ucx-parse/src/lib.rs:24,656`.

> **Encryption is independent of, and applied before, signing.** The MANIFEST and both signature layers cover the *ciphertext* bytes as they sit in the ZIP. A reader can therefore verify integrity and signatures **without** any decryption key. (See §5–§6.)

---

## 2. ZIP container layout

### 2.1 Entry order (as written by the builder)

The builder writes entries in this exact order. `ucx-build/src/archive.rs:344-380`

| # | Entry path | Compression | Notes |
|---|------------|-------------|-------|
| 1 | `mimetype` | **STORED** (no compression) | MUST be first. Content = `application/vnd.unicodex+zip`, no trailing newline (28 bytes). `archive.rs:28,346-350` |
| 2 | `META-INF/MANIFEST.MF` | **DEFLATE** | `archive.rs:354-358` |
| 3 | `metadata/codex.json` | **DEFLATE** | serialized pretty JSON **+ trailing `\n`**. `archive.rs:303-305,362-364` |
| 4 | `content/struct.json` | **DEFLATE** | serialized pretty JSON **+ trailing `\n`**. `archive.rs:306-308,366-370` |
| 5… | All other `content/` and `assets/` files, sorted by filename | per-extension (§2.3) | collected recursively, `sort_by_file_name`. `archive.rs:104,374-380` |

After signing, two more groups appear (see §6):
- Layer 1 files appended **at the end** of the central-directory entry list: `META-INF/signatures/{SIGNER}.SF`, `META-INF/signatures/{SIGNER}.EC`, `META-INF/certs/{SIGNER}.cert.pem`. `ucx-sign/src/zip_rewrite.rs:120-160`
- Layer 2 **UCX Signing Block**, a raw binary blob inserted **between the local file data and the Central Directory** (not a ZIP entry). `ucx-sign/src/zip_binary.rs:226-267`

> **Parser requirement:** the parser validates that ZIP entry **index 0** is named exactly `mimetype` and that its (trimmed) content equals the mimetype string. `ucx-parse/src/lib.rs:632-664`. The mimetype comparison trims trailing whitespace/newlines, so a trailing newline is tolerated on read even though the builder writes none.

### 2.2 Standard paths

| Path | Required | Meaning |
|------|----------|---------|
| `mimetype` | yes | fixed string, first entry, STORED |
| `META-INF/MANIFEST.MF` | yes | file hash manifest (§3) |
| `metadata/codex.json` | yes | work metadata (§4.1) |
| `content/struct.json` | yes | content tree (§4.2) |
| `content/*` | yes (≥ struct.json) | chapter files (plaintext or UCXE) |
| `assets/*` | optional | media |
| `META-INF/signatures/{SIGNER}.SF` | when signed | Layer 1 signature file (§6.1) |
| `META-INF/signatures/{SIGNER}.EC` | when signed | Layer 1 Ed25519 blob (§6.1) |
| `META-INF/certs/{SIGNER}.cert.pem` | when signed | signer cert (PEM); cross-checked against `.EC` (§6.1) |

Path rules: UTF-8, `/` separators, `META-INF/*` names uppercase. Extraction rejects entries containing `..` or starting with `/`. `ucx-parse/src/lib.rs:505-512`

### 2.3 Compression strategy (by extension, lowercased)

`ucx-build/src/archive.rs:227-258`

- **STORED** (already-compressed): `jpg jpeg png webp gif mp3 ogg mp4 pdf woff2 ttf otf`
- **DEFLATE**: `json md mdx ucxc txt typ tex toml xml html css js`, and **DEFLATE is the default** for any unknown extension.

> Compression choice is **not** integrity-relevant: all hashing and signing operate on **decompressed** entry bytes (the `zip` crate returns decompressed content). SDKs may write any valid compression and still produce a verifiable archive, **as long as decompressed bytes are identical**. Layer 2, however, covers the *raw on-disk ZIP bytes* — see §6.2.

---

## 3. MANIFEST.MF text format

`ucx-types/src/manifest.rs:160-300`

### 3.1 Grammar

RFC-822-ish. Sections separated by a blank line (`\n\n`). Lines are `Key: Value` (note: parser splits on the first `": "` — colon **followed by a space**). Every line is `\n`-terminated by the writer. `manifest.rs:160-189,203,216,255`

**Main section** (first section):
```
Manifest-Version: 1.0
UCX-Version: 1.0
Created-By: unicodex 0.4.0-alpha.2
Hash-Algorithm: BLAKE3
```
- `Manifest-Version` — required, always `1.0`. `manifest.rs:129,164`
- `UCX-Version` — required (e.g. `1.0`). Parser rejects archives whose **MAJOR** > 1. `manifest.rs:165`, `ucx-parse/src/lib.rs:50,604-627`
- `Created-By` — optional. `manifest.rs:166-168`
- `Hash-Algorithm` — required; one of `BLAKE3` | `SHA256` | `SHA512`. The builder always emits `BLAKE3`. `manifest.rs:36-90,185`, `archive.rs:185`

**Per-file sections** (one blank line, then):
```
Name: metadata/codex.json
Size: 2048
BLAKE3-Digest: <base64>
```
and optionally, for encrypted entries:
```
Encrypted: true
Original-Size: 15360
```
`manifest.rs:173-186`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `Name` | string | yes | archive-relative path. `manifest.rs:175` |
| `Size` | u64 | yes | size of the stored (ciphertext, if encrypted) bytes. `manifest.rs:176` |
| `{ALGO}-Digest` | string | yes | header name is `"{ALGO}-Digest"`, e.g. `BLAKE3-Digest`. `manifest.rs:68-70,177` |
| `Encrypted` | `true` | only when encrypted | emitted only if `true`. `manifest.rs:178-181` |
| `Original-Size` | u64 | only when encrypted | plaintext size. `manifest.rs:182-185` |

### 3.2 Digest encoding — **Base64, NOT hex** (load-bearing)

The digest value is **standard Base64 with padding** (RFC 4648 standard alphabet, `=` padding) of the **raw 32-byte BLAKE3 hash**. `ucx-types/src/manifest.rs:23,343-347` (`BASE64_STANDARD.encode(hash_bytes)`).

> ⚠️ **Discrepancy with docs:** `docs/01-file-structure.md` shows hex digests in its *example* (`af1349b9...`) but its field table says "Base64-encoded". The reference code uses **Base64**. Verification recomputes the BLAKE3 hash and compares its Base64 form. `ucx-parse/src/lib.rs:408,422-427`. **SDKs MUST use Base64-standard-with-padding.** A 32-byte BLAKE3 hash encodes to 44 Base64 chars (`...=`-padded to a length-44 string, e.g. ending in `=`).

### 3.3 Entries excluded from the manifest

The builder never adds manifest entries for: `mimetype`, `META-INF/MANIFEST.MF`, anything under `META-INF/signatures/`, anything under `META-INF/certs/`. `ucx-build/src/archive.rs:160-174`

Consequently a verifier that walks the manifest naturally only checks the data files (codex/struct/chapters/assets). `mimetype` is integrity-guaranteed by spec; the signature files protect themselves and the manifest via §6.

---

## 4. JSON schemas

Both JSON files are UTF-8 and parsed with permissive deserialization (unknown fields are ignored). `ucx-parse/src/lib.rs:700,719`

### 4.1 `metadata/codex.json` (Codex)

`ucx-types/src/codex.rs:26-345`. Required vs optional below. `$schema` uses the JSON key `$schema`; `node_type` etc. are JSON-renamed.

| JSON key | Type | Required | Source |
|----------|------|----------|--------|
| `$schema` | string | no | `codex.rs:30-31` |
| `version` | string | **yes** | metadata schema version, e.g. `"1.0"`. `codex.rs:35` |
| `identifier` | object | **yes** | see below. `codex.rs:39` |
| `title` | object | **yes** | see below. `codex.rs:43` |
| `series` | object | no | `{ name (req), index?:u32, total?:u32 }`. `codex.rs:48,189-204` |
| `creators` | array | **yes** | ≥1; see below. `codex.rs:52` |
| `publisher` | object | no | `{ name (req), imprint?, signature_ref? }`. `codex.rs:57,244-259` |
| `language` | string | **yes** | BCP-47, e.g. `"zh-CN"`. `codex.rs:61` |
| `genre` | string[] | no | `codex.rs:66` |
| `tags` | string[] | no | `codex.rs:71` |
| `status` | string | no | ongoing/completed/hiatus/abandoned/draft. `codex.rs:76` |
| `word_count` | u64 | no | `codex.rs:81` |
| `description` | object | no | `{ short?, long? }`. `codex.rs:86,268-279` |
| `rights` | object | no | `{ statement?, license? }`. `codex.rs:91,288-299` |
| `dates` | object | no | `{ created?, published?, modified? }` ISO-8601. `codex.rs:96,308-324` |
| `cover` | string | no | archive-relative path. `codex.rs:101` |
| `rating` | object | no | `{ system (req), value (req) }`. `codex.rs:106,336-345` |
| `file_version` | object | no | `{ version?, revision?:u64, released_at?, changelog? }`. `codex.rs:116,371-392` |

`identifier`: `{ ucx_id (req, string "urn:ucx:{UUIDv4}"), isbn?, issn?, doi?, custom?:map<string,string> }`. `codex.rs:126-151`
`title`: `{ main (req), subtitle?, original?, short? }`. `codex.rs:160-180`
`creators[]`: `{ name (req), role (req), signature_ref? }`. `signature_ref` links a creator to a `{SIGNER}` id in `META-INF/signatures/`. `codex.rs:221-235`

Optional fields are omitted entirely when absent (serde `skip_serializing_if`), so SDKs MUST treat missing keys as `null`/`None`, not error.

### 4.2 `content/struct.json` (Structure)

`ucx-types/src/structure.rs:25-240`

Root:
| JSON key | Type | Required |
|----------|------|----------|
| `$schema` | string | no |
| `version` | string | **yes** |
| `structure` | `StructureNode[]` | **yes** |

`StructureNode` (recursive):
| JSON key | Type | Required | Notes |
|----------|------|----------|-------|
| `title` | string | **yes** | the only required field. `structure.rs:65` |
| `file` | string | mutually exclusive with `children` | leaf; path **relative to `content/`**. `structure.rs:72-73` |
| `children` | `StructureNode[]` | mutually exclusive with `file` | container. `structure.rs:80-81` |
| `type` | string | no | JSON key is `type` (Rust field `node_type`). `structure.rs:85-86` |
| `id` | string | no | `structure.rs:90-91` |
| `name` | string | no | `structure.rs:95-96` |
| `style` | string | no | `structure.rs:100-101` |
| `encryption` | object | no | only meaningful on leaf nodes. `structure.rs:105-106` |

A leaf has `file` and no `children`; a container has `children` and no `file`. `structure.rs:113-122`. The actual ZIP entry for a leaf is `content/{file}`. `ucx-parse/src/lib.rs:348`.

`encryption` object (`structure.rs:151-166`):
- `algorithm` (req): `"AES-256-GCM"` | `"AES-256-CBC"` | `"ChaCha20-Poly1305"`.
- `key_access` (req): array (≥1) of `KeyAccess`.

`KeyAccess` (`structure.rs:178-240`) — `method` (req) plus method-specific optional fields:
- `direct`: `key` (base64), `iv?` (base64)
- `message`: `text`
- `url`: `url`, `auth_type` (`none|login|token|oauth|certificate`), `params?`
- `service`: `provider`, `service_type` (`purchase_verify|author_verify|subscription|public_key`), `params?`
- `extension`: `ext` (`"ext.{alias}"` or `"ext.{provider}.{id}@{version}"`), `params?`

`params` is `map<string,string>`. These objects only tell a reader **how to obtain the key**; they are not part of the UCXE binary container. The UCXE header alone is sufficient to decrypt once the 32-byte key (or passphrase) is in hand.

---

## 5. Integrity verification (BLAKE3 vs MANIFEST)

`ucx-parse/src/lib.rs:407-444`

For each entry in the parsed MANIFEST:
1. Read the entry named `entry.name` from the ZIP (decompressed bytes). `lib.rs:420`
2. Compute `BLAKE3(bytes)` → 32 raw bytes. `lib.rs:424`
3. Encode raw hash as **standard Base64 (with padding)**. `lib.rs:425`
4. Compare string-equal to `entry.digest`. Equal ⇒ valid. `lib.rs:427`

There is no per-entry salt, prefix, or length framing in this hash — it is a plain BLAKE3 over the exact decompressed file bytes (for encrypted files, that means over the **ciphertext UCXE bytes**, see §7/§6). For `codex.json`/`struct.json`, "exact bytes" includes the **trailing `\n`** the builder appends (§2.1).

> **MVP note:** §5 + §3 + §4 is the entire "minimal reader". No crypto-signature math is required to detect tampering of data files relative to the manifest. (But note: without verifying §6, an attacker could rewrite both a data file *and* its manifest entry. Manifest-only integrity is self-consistency, not authenticity.)

---

## 6. Dual-layer signatures

Both layers use **Ed25519** over BLAKE3-based digests. The algorithm id `0x0001` means "Ed25519 + BLAKE3" in both layers. `ucx-sign/src/layer1.rs:39`, `ucx-sign/src/layer2.rs:41`. RSA/ECDSA are **not** implemented despite being listed in docs.

Verification entry point and decision logic: `ucx-verify/src/lib.rs:223-276,898-936`.

**Status decision table** (`ucx-verify/src/lib.rs:898-936`):

| L1 present | L1 valid | L2 present | L2 valid | Status |
|---|---|---|---|---|
| no | – | no | – | `Unsigned` |
| yes | yes | yes | yes | `Valid` |
| yes | * | yes | * (any fail) | `Invalid` |
| yes | yes | no | – | `ValidWithWarnings` |
| yes | no | no | – | `Invalid` |
| no | – | yes | yes | `ValidWithWarnings` |
| no | – | yes | no | `Invalid` |

When **both** layers are present, **any** failure ⇒ `Invalid`. A single-layer-present-and-valid file is `ValidWithWarnings` (incomplete coverage).

### 6.1 Layer 1 — JAR-style SF / EC (per signer)

A signer is `{SIGNER}` — `[A-Z0-9_]{1,32}`. `ucx-sign/src/lib.rs:399-421`. Files produced: `ucx-sign/src/zip_rewrite.rs:120-160`.

**`{SIGNER}.SF`** (UTF-8 text, DEFLATE in ZIP). Generated by `generate_sf`. `ucx-sign/src/layer1.rs:115-146`:
```
Signature-Version: 1.0
UCX-Version: 1.0
Hash-Algorithm: BLAKE3
Created-By: unicodex {version}

BLAKE3-Digest-Manifest: <base64(BLAKE3(entire MANIFEST.MF bytes))>
BLAKE3-Digest-Manifest-Main-Attr: <base64(BLAKE3(main-attributes section))>
```
- `BLAKE3-Digest-Manifest` = Base64 of BLAKE3 over the **complete** `MANIFEST.MF` bytes. `layer1.rs:118-119`
- `BLAKE3-Digest-Manifest-Main-Attr` = Base64 of BLAKE3 over the **main-attributes section only** = everything **up to and including** the line terminator before the first blank line. The extractor includes the first `\r\n` (Windows) or first `\n` (Unix) of the blank-line separator, i.e. `content[..pos+2]` for `\r\n\r\n` or `content[..pos+1]` for `\n\n`; if there's no blank line, the whole content. `layer1.rs:127,426-446`

**`{SIGNER}.EC`** (binary, STORED in ZIP). Custom blob (**not** PKCS#7, contrary to docs). Built by `sign_sf`. `ucx-sign/src/layer1.rs:188-228`:

| Field | Offset | Size | Encoding | Value |
|-------|--------|------|----------|-------|
| `signature_algorithm_id` | 0 | 4 | u32 LE | `0x00000001` (Ed25519+BLAKE3) |
| `signature_length` | 4 | 4 | u32 LE | `64` |
| `signature` | 8 | 64 | raw | Ed25519 signature over the **raw `.SF` file bytes** |
| `cert_length` | 72 | 4 | u32 LE | length of cert DER |
| `cert_der` | 76 | `cert_length` | raw | DER-encoded X.509 certificate (Ed25519 SPKI) |

The **signed message is the exact `.SF` content bytes** (no hashing wrapper; Ed25519 internally hashes). `layer1.rs:195`. Verification: parse blob, extract the cert, read the Ed25519 public key from the cert's SubjectPublicKeyInfo (32-byte raw key, OID `1.3.101.112`), `verify(sf_content, signature)`. `layer1.rs:261-357,468-503`.

**`{SIGNER}.cert.pem`** — PEM (`-----BEGIN CERTIFICATE-----`, LF line endings) of the same cert. `ucx-sign/src/zip_rewrite.rs:149-155`.

**Layer 1 verification per signer** (`ucx-verify/src/lib.rs:572-842`):
1. Find all `META-INF/signatures/*.SF`. `signer_id` = filename stem. `lib.rs:587-632`
2. Read `MANIFEST.MF`; compute `Base64(BLAKE3(manifest))`. Compare to the SF's `BLAKE3-Digest-Manifest`. `lib.rs:682-685` → `digest_matches`.
3. Verify the `.EC` Ed25519 signature over the `.SF` bytes ⇒ `sig_valid` + embedded `cert_der`. `lib.rs:689-699`
4. Check cert validity window (`notBefore`/`notAfter` vs now). Expired/not-yet-valid ⇒ invalid. `lib.rs:706-731`
5. **Cert-PEM cross-check:** if `META-INF/certs/{SIGNER}.cert.pem` exists, decode it (PEM label MUST be `CERTIFICATE`) and require its DER to byte-equal the cert embedded in `.EC`; mismatch ⇒ invalid. Missing PEM is allowed (no cross-check). `lib.rs:746-798`
6. Signer is valid iff `digest_matches && sig_valid && cert_time_valid && cert_pem_match`. `lib.rs:800`
7. Layer 1 overall valid iff **all** signers valid. `lib.rs:619,835`

> **SF does NOT independently re-verify each file's hash.** Layer 1 chains: `EC signs SF` → `SF digests MANIFEST` → `MANIFEST digests each file`. A complete reader should therefore ALSO run §5 to bind individual files to the manifest. The reference `verify()` does not re-run §5 inside Layer 1; the manifest-digest link plus §5 (run separately via `ucx-parse`) together close the chain.

> The SF also carries `BLAKE3-Digest-Manifest-Main-Attr`; the reference verifier parses it but does not separately assert it (the full-manifest digest already covers it). SDKs MAY recompute it for defense-in-depth.

### 6.2 Layer 2 — APK-v2-style UCX Signing Block

A single raw binary blob placed **between the last local file entry and the Central Directory**. It is **not** a ZIP entry. `ucx-sign/src/zip_binary.rs:226-267`.

**Block structure** (`ucx-sign/src/layer2.rs:288-339`). All integers little-endian.
```
[ size_of_block : u64 LE ]            ; leading copy
[ pair_size     : u64 LE ]            ; = 4 + len(signers_data)
[ pair_id       : u32 LE ]            ; = 0x55435801  (PAIR_ID_UCX_SIG_V1)
[ signers_data  : pair_size-4 bytes ] ; one signer blob (see below)
[ size_of_block : u64 LE ]            ; trailing copy (== leading)
[ magic         : 16 bytes ]          ; "UCX Sig Block 1\0"
```
- `size_of_block` = `len(pair_block) + 8 (trailing size) + 16 (magic)`, where `pair_block = pair_size(8) + pair_id... ` — i.e. it counts everything **after** the leading u64. `layer2.rs:327`
- Magic constant: `UCX_SIGNING_BLOCK_MAGIC = b"UCX Sig Block 1\0"` (16 bytes; note trailing NUL). `ucx-sign/src/zip_binary.rs:25`
- Pair id constant: `0x55435801`. `zip_binary.rs:29`

**Signer data blob** (`signers_data`; `ucx-sign/src/layer2.rs:230-282`):
```
[ signed_data_length    : u32 LE ]
[ signed_data : signed_data_length bytes ]:
    [ digest_algorithm_id : u32 LE ]   ; 0x00000001
    [ digest              : 32 bytes ] ; the protected-content digest (§6.2.2)
    [ cert_length         : u32 LE ]
    [ cert_der            : cert_length bytes ]
[ signature_algorithm_id : u32 LE ]    ; 0x00000001
[ signature_length       : u32 LE ]    ; 64
[ signature              : 64 bytes ]  ; Ed25519 over the `signed_data` bytes
[ public_key_length      : u32 LE ]    ; 32
[ public_key             : 32 bytes ]  ; raw Ed25519 public key
```
The **Ed25519 signature is over the `signed_data` byte block** (algorithm_id ‖ digest ‖ cert_length ‖ cert_der), **not** over the raw digest. `layer2.rs:248`. Verification reconstructs `signed_data` identically. `ucx-verify/src/lib.rs:504-536`.

#### 6.2.1 Locating the block (verifier)

`ucx-sign/src/zip_binary.rs:296-393`:
1. Find EOCD (scan backward for `50 4B 05 06`, validate the comment-length field is consistent with file end). `zip_binary.rs:33,68-99`
2. Read CD offset from EOCD+16 (u32 LE). `zip_binary.rs:121-139`
3. The 16 bytes immediately before CD must equal the magic; else no Layer 2. `zip_binary.rs:314-319`
4. Read the trailing `size_of_block` (u64 LE at `magic_start-8`). `block_start = cd_offset - 8 - size_of_block`. `zip_binary.rs:321-354`
5. Leading `size_of_block` at `block_start` must equal the trailing one. `zip_binary.rs:363-386`
6. The whole block = `data[block_start .. cd_offset]`. `zip_binary.rs:390`

#### 6.2.2 Protected-content digest

The signature covers the original ZIP **with the signing block removed**, reconstituted as three concatenated sections, then hashed with a chunked BLAKE3 scheme.

**Reconstruction** (`ucx-verify/src/lib.rs:449-495`):
- `entries = file[0 .. block_offset]` (local file data, up to where the block was inserted)
- `cd_and_eocd = file[cd_offset .. end]`
- `original_zip = entries ‖ cd_and_eocd`
- Then **patch the EOCD's CD-offset field** in `original_zip` back to `cd_offset - block_size` (u32 LE at the new EOCD+16), because removing the block shifts the CD. `lib.rs:478-482`, `zip_binary.rs:152-155`.

**Chunked digest** over `protected = section1(entries) ‖ section3(CD) ‖ section4(EOCD)` of the reconstructed ZIP (`ucx-sign/src/layer2.rs:70-168`):
- Split `protected` into 1 MiB chunks (`CHUNK_SIZE = 1048576`); empty input ⇒ one empty chunk. `layer2.rs:29,70-79`
- `chunk_digest[i] = BLAKE3( 0xA5 ‖ u32_le(len(chunk_i)) ‖ chunk_i )`. Prefix byte `0xA5`. `layer2.rs:33,87-94`
- `top_digest = BLAKE3( 0x5A ‖ u32_le(chunk_count) ‖ chunk_digest[0] ‖ chunk_digest[1] ‖ … )`. Prefix byte `0x5A`. `layer2.rs:37,99-106`
- The 32-byte `top_digest` is the value placed in / compared against `signed_data.digest`. `layer2.rs:140-167`

**Per-signer Layer 2 validity** (`ucx-verify/src/lib.rs:365-424`): `digest_matches (stored == recomputed) && sig_valid (Ed25519 over signed_data) && cert_time_valid`. Layer 2 overall valid iff all signer entries valid and ≥1 entry present. `lib.rs:328-336,399-401`.

### 6.3 Certificates & keys (PEM / DER)

- **Private key** PEM: PKCS#8, `-----BEGIN PRIVATE KEY-----`, LF. `ucx-sign/src/keys.rs:79-105`
- **Public key** PEM: SPKI, `-----BEGIN PUBLIC KEY-----`, LF. `keys.rs:283-295`
- **Certificate** PEM: `-----BEGIN CERTIFICATE-----`, LF, DER inside. `ucx-sign/src/cert.rs:206-217`
- Self-signed X.509 v3, Ed25519 (SPKI OID `1.3.101.112`): KeyUsage=digitalSignature, ExtKeyUsage=codeSigning (`1.3.6.1.5.5.7.3.3`), BasicConstraints CA:FALSE. `cert.rs:96-180`
- Public key is read from the cert's `subjectPublicKeyInfo.subjectPublicKey` as a **32-byte raw Ed25519 key**. `ucx-sign/src/layer1.rs:468-502`
- Cert fingerprint (used to match L1/L2 signers): lowercase-hex `BLAKE3(cert_der)`, 64 hex chars. `cert.rs:282-287`, `ucx-verify/src/lib.rs:951-995`
- Cert validity check compares `notBefore`/`notAfter` against `SystemTime::now()`. `cert.rs:473-511`
- "self-signed" vs "CA-issued" is decided by comparing subject-CN to issuer-CN. `ucx-verify/src/lib.rs:1027-1046`

---

## 7. UCXE encryption format

A UCXE file is the **on-disk replacement** of a plaintext chapter/resource: same filename inside the ZIP, content replaced by the UCXE binary. `docs/04-crypto-spec.md`; container code in `ucx-crypto/src/format.rs`.

### 7.1 Constants

`ucx-crypto/src/lib.rs:80-84`
- Magic: `UCXE_MAGIC = 55 43 58 45` ("UCXE", 4 bytes).
- Format version: `UCXE_FORMAT_VERSION = 0x01`.

### 7.2 Binary header & layout (field-by-field)

Serialized by `write_ucxe`, parsed by `parse_ucxe_bounded`. `ucx-crypto/src/format.rs:311-373,413-618`.

| # | Field | Offset | Size | Endian | Value / notes |
|---|-------|--------|------|--------|---------------|
| 1 | Magic | 0 | 4 | — | `55 43 58 45` ("UCXE"). `format.rs:314,467-473` |
| 2 | Format version | 4 | 1 | — | `0x01`; parser rejects others. `format.rs:321,483-488` |
| 3 | Algorithm ID | 5 | 1 | — | §7.3 table. `format.rs:322,489-491` |
| 4 | KDF ID | 6 | 1 | — | §7.4 table. `format.rs:323,492-494` |
| 5 | Flags | 7 | 1 | — | bit0 = chunked; bits1-7 reserved (MUST be 0 on write). The **raw byte** is preserved & bound into AAD. `format.rs:320,481,610` |
| 6 | KDF params | 8 | 0 / 4 / 12 | LE | Present only if KDF≠None; layout per §7.4. `format.rs:330-351,498-514` |
| 7 | Salt length | after #6 | 2 | u16 LE | `format.rs:355,522-523` |
| 8 | Salt | — | `salt_len` | — | cap 1024 bytes on parse. `format.rs:356,525-531` |
| 9 | IV/Nonce length | — | 2 | u16 LE | `format.rs:360,535-536` |
| 10 | IV/Nonce | — | `iv_len` | — | cap 256 bytes; 12 for AEAD, 16 for CBC. `format.rs:361,538-544` |
| 11 | Ciphertext length | — | 8 | u64 LE | cap 16 GiB on parse. `format.rs:365,548-555` |
| 12 | Ciphertext | — | `ct_len` | — | for chunked mode this is the **serialized chunk stream** (§7.7). `format.rs:366,577-586` |
| 13 | Auth Tag | — | algo-determined | — | **no length prefix**; length from algorithm (§7.5). `format.rs:370,588-597` |

> **Field-order note (vs docs):** the on-disk order is **header(8B) → KDF params → salt → IV → ciphertext → tag**. `docs/04-crypto-spec.md` §4.1's diagram lists Salt before the KDF-params note; the implementation writes **KDF params first, then salt**. Follow the table above. `format.rs:330-356`.

Parser bounds (reject, don't crash): salt ≤ 1024, IV ≤ 256, ciphertext ≤ 16 GiB and `ct_len + tag_len ≤ remaining`. KDF params are validated immediately after decode (§8). `format.rs:39-48,516-518,547-597`.

### 7.3 Algorithm ID

`ucx-crypto/src/lib.rs:168-218`

| ID | Algorithm | Tag len | IV/Nonce len |
|----|-----------|---------|--------------|
| `0x01` | AES-256-GCM | 16 | 12 |
| `0x02` | AES-256-CBC + HMAC-SHA256 (Encrypt-then-MAC) | 32 | 16 |
| `0x03` | ChaCha20-Poly1305 | 16 | 12 |

Unknown ⇒ reject. `format.rs:489-491`.

### 7.4 KDF ID & parameter block

`ucx-crypto/src/lib.rs:230-280`, `ucx-crypto/src/format.rs:128-196,498-514`

| ID | KDF | Param block (LE), inserted at offset 8 |
|----|-----|----------------------------------------|
| `0x00` | None (key supplied directly) | *(none, 0 bytes)* |
| `0x01` | Argon2id | `memory_cost_kib:u32 ‖ time_cost:u32 ‖ parallelism:u32` (12 B) |
| `0x02` | PBKDF2-HMAC-SHA256 | `iterations:u32` (4 B) |

Unknown ⇒ reject. KDF-id/param-variant mismatch ⇒ reject. `format.rs:492-494`, `ucx-crypto/src/kdf.rs:92-161`.

### 7.5 Auth-tag semantics

`ucx-crypto/src/format.rs:280-294`
- AES-256-GCM: 16-byte GCM tag.
- AES-256-CBC: 32-byte `HMAC-SHA256` (Encrypt-then-MAC).
- ChaCha20-Poly1305: 16-byte Poly1305 tag.

For GCM/ChaCha20 the library splits ciphertext and tag (tag stored separately in field #13). `ucx-crypto/src/aes_gcm.rs:46-75`, `chacha20.rs`.

### 7.6 AAD (Associated Authenticated Data) — load-bearing

> ⚠️ **Discrepancy with docs:** `docs/04-crypto-spec.md` §4.1 says AAD is "fixed 8 bytes". The implementation uses a **longer AAD for AEAD modes**. SDKs MUST follow the code.

**AEAD modes (AES-GCM, ChaCha20-Poly1305)** — `build_aead_aad`. `ucx-crypto/src/format.rs:224-237`, used at `ucx-crypto/src/lib.rs:667,782`:
```
AAD = header_bytes(8) ‖ kdf_params_bytes(0|4|12) ‖ salt(salt_len)
```
where:
- `header_bytes(8)` = `magic(4) ‖ format_version(1) ‖ algorithm_id(1) ‖ kdf_id(1) ‖ raw_flags(1)`. The **raw flags byte from disk** is used (including reserved bits), so flipping any reserved bit breaks the tag. `format.rs:110-122`
- `kdf_params_bytes` = exact LE encoding of the KDF param block (0/4/12 bytes), identical to field #6. `format.rs:179-195`
- `salt` = the raw salt bytes.

So for **AES-GCM/ChaCha20**, the AAD covers the full fixed header **plus** KDF params **plus** salt. Decryption recomputes the same AAD from parsed fields; any header/KDF/salt tamper ⇒ `AuthenticationFailed`. `ucx-crypto/src/lib.rs:768-827`. In **chunked mode**, this same AAD is passed to **every** chunk's AEAD call. `ucx-crypto/src/lib.rs:667-688,796-797`, `ucx-crypto/src/chunked.rs:270-316`.

**AES-256-CBC mode** — the HMAC covers `aad ‖ iv ‖ ciphertext`, where **`aad = header_bytes(8)` only** (just `header.to_bytes()`, NOT kdf_params, NOT salt). `ucx-crypto/src/lib.rs:750,859`, HMAC computed over `aad ‖ iv ‖ ciphertext` in `ucx-crypto/src/aes_cbc.rs:109-124,162-173`.

> SDKs MUST implement two AAD constructions: the **8+kdf+salt** form for AEAD, and the **8-byte header only** form for CBC's HMAC. Mixing them up will fail authentication.

### 7.7 Chunked encryption (large files)

`ucx-crypto/src/chunked.rs`, gated by `ucx-crypto/src/lib.rs:332-334,669-688`.

- **Threshold:** chunking is used when plaintext length **> 64 MiB** (`CHUNKED_THRESHOLD = 67108864`); `>` is strict. `chunked.rs:47`, `lib.rs:334`.
- **Chunk size:** 1 MiB (`CHUNK_SIZE = 1048576`); last chunk may be smaller. `chunked.rs:43,287`.
- **Only AEAD** (GCM, ChaCha20) supported; **CBC is never chunked.** `chunked.rs:279-283`, `lib.rs:496-497`.
- **Header IV field (#10)** stores the 12-byte **base nonce** (random per file). `lib.rs:672-684`.
- **Header tag field (#13)** is present but a placeholder of zero bytes of `tag_length` — each chunk carries its own tag; the top-level tag is unused in chunked mode. `lib.rs:685-688`.

**Per-chunk nonce (concatenation, NOT XOR):** `chunked.rs:103-112`
```
nonce_i = base_nonce[0..8]  ‖  u32_be(chunk_index)      ; 12 bytes total
```
High 8 bytes = first 8 bytes of the base nonce; low 4 bytes = the chunk index as **big-endian u32**. Distinct indices ⇒ distinct nonces (collision-free). Supports up to 2³² chunks. (The old XOR scheme is forbidden.)

**Serialized chunk stream** (this is field #12 when chunked). `chunked.rs:399-421`, parser `chunked.rs:437-524`:
```
[ chunk_count : u32 LE ]
repeat chunk_count times:
    [ chunk_ciphertext_size : u32 LE ]
    [ ciphertext            : chunk_ciphertext_size bytes ]   ; without tag
    [ tag                   : 16 bytes ]                      ; per-chunk AEAD tag
```
Each chunk is `AEAD.encrypt(key, nonce_i, plaintext_chunk, aad)` with the **same file-level AAD** (§7.6); decryption verifies each chunk's tag and concatenates. `chunked.rs:270-377`.

### 7.8 Key handling, passphrase NFC, direct keys

- **Direct-key (KDF=None):** key is a 32-byte value supplied out-of-band. AES-CBC is **rejected** in direct-key mode (needs 64 bytes = enc+mac). `ucx-crypto/src/lib.rs:313-346`.
- **Passphrase mode (KDF≠None):**
  - The passphrase is **NFC-normalized** (`unicode_normalization::nfc`) **before** being passed (as UTF-8 bytes) to the KDF, on both encrypt and decrypt. SDKs MUST NFC-normalize identically or passphrases with combining characters will fail. `ucx-crypto/src/lib.rs:23,36-38,473-486,587-597`.
  - Output length: 32 bytes for AEAD; **64 bytes for AES-CBC** (`enc_key = derived[0..32]`, `mac_key = derived[32..64]`). `ucx-crypto/src/lib.rs:470,575,718-737`, `ucx-crypto/src/kdf.rs:347-430`.
  - Salt MUST be exactly 16 bytes on the decrypt path. `ucx-crypto/src/lib.rs:579-585`.

### 7.9 Decryption algorithm (per algorithm)

`ucx-crypto/src/lib.rs:768-861`:
- Recompute AAD (§7.6) from the parsed header/KDF/salt.
- **GCM/ChaCha20, non-chunked:** require 12-byte nonce + 16-byte tag; `AEAD.decrypt(key, nonce, ciphertext, tag, aad)`. `lib.rs:801-819`, `aes_gcm.rs:85-117`.
- **GCM/ChaCha20, chunked:** deserialize chunk stream, derive each nonce (§7.7), decrypt each chunk with the file AAD, concatenate. `lib.rs:784-797`.
- **AES-CBC:** require 16-byte IV + 32-byte HMAC tag; **verify HMAC over `header(8) ‖ iv ‖ ciphertext` first** (constant-time), then AES-CBC decrypt + PKCS#7 unpad. `lib.rs:832-861`, `aes_cbc.rs:154-180`.

Public decrypt APIs collapse all crypto/parse errors into one opaque "decryption failed" to avoid oracles; I/O errors pass through. `ucx-crypto/src/lib.rs:138-152,371-415`.

---

## 8. KDF details

`ucx-crypto/src/kdf.rs`

### 8.1 Argon2id

- Variant **Argon2id**, version **0x13 (v19)**, output length 32 (AEAD) or 64 (CBC). `kdf.rs:238-262,363-401`.
- Defaults: `memory = 65536 KiB (64 MiB)`, `time_cost = 3`, `parallelism = 4`. `kdf.rs:21-29,624-628`.
- Parse-time bounds (reject out-of-range): memory `19456…4194304` KiB, time `2…100`, parallelism `≥1`. `kdf.rs:50-62,107-141`.

### 8.2 PBKDF2-HMAC-SHA256

- `pbkdf2_hmac::<Sha256>`; output 32 or 64 bytes. `kdf.rs:288-307,405-428`.
- Default iterations: `600000`. `kdf.rs:31-33,629-631`.
- Parse-time bounds: iterations `100000…10000000`. `kdf.rs:64-69,142-154`.

### 8.3 Salt & derivation

- Salt: 16 bytes (`SALT_SIZE = 16`), generated via OS CSPRNG. `kdf.rs:35-37,179-187`.
- Empty passphrase rejected. `kdf.rs:230-234,295-299`.
- Same passphrase+salt+params ⇒ deterministic key. KDF params stored in the UCXE header (§7.4) so a passphrase alone reproduces the key.

> Implementations MUST validate KDF params on parse and refuse out-of-range values **before** attempting decryption (`validate_kdf_params`). `ucx-crypto/src/format.rs:516-518`, `kdf.rs:92-161`.

---

## 9. Minimum-viable reader (MVP) subset

For languages with weak crypto support, a useful read-only viewer needs only:

1. **ZIP read** with the offset-0 `PK\x03\x04` check. (§1)
2. **mimetype** check: first entry named `mimetype`, content (trimmed) == `application/vnd.unicodex+zip`. (§2.1)
3. **MANIFEST.MF** parse (§3) — only `Name`, `Size`, `BLAKE3-Digest`, plus `UCX-Version` MAJOR ≤ 1.
4. **BLAKE3** + **Base64** integrity check vs manifest (§5).
5. **codex.json / struct.json** JSON parse (§4) to render the table of contents and read plaintext chapters.
6. Detect UCXE magic on chapter bytes; if encrypted, surface "encrypted" rather than mis-decoding. `ucx-parse/src/lib.rs:355-361`.

MVP needs **only BLAKE3 + Base64 + ZIP + JSON** — **no** Ed25519, AEAD, Argon2, or X.509. Signature verification (§6) and decryption (§7) are independent, additive capabilities a stronger SDK can layer on.

---

## 10. Library suggestions per ecosystem

Capabilities needed for full support: **ZIP**, **BLAKE3**, **Base64**, **JSON**, **Ed25519**, **X.509/DER parse (Ed25519 SPKI)**, **AES-256-GCM**, **AES-256-CBC + HMAC-SHA256**, **ChaCha20-Poly1305**, **Argon2id**, **PBKDF2-HMAC-SHA256**, **PEM (RFC 7468)**, **Unicode NFC**.

| Lang | ZIP | BLAKE3 | AEAD (GCM/ChaCha) | Argon2 / PBKDF2 | Ed25519 / X.509 |
|------|-----|--------|-------------------|------------------|------------------|
| **Rust** (reference) | `zip` | `blake3` | `aes-gcm`, `chacha20poly1305`, `aes`+`cbc`+`hmac` | `argon2`, `pbkdf2` | `ed25519-dalek`, `x509-cert`, `rcgen`, `pem-rfc7468` |
| **Go** | `archive/zip` | `lukechampine.com/blake3` | `crypto/cipher` (GCM), `golang.org/x/crypto/chacha20poly1305` | `golang.org/x/crypto/argon2`, `golang.org/x/crypto/pbkdf2` | `crypto/ed25519`, `crypto/x509` |
| **Python** | `zipfile` | `blake3` (PyPI) | `cryptography` (AESGCM, ChaCha20Poly1305, CBC+HMAC) | `argon2-cffi`, `hashlib.pbkdf2_hmac` | `cryptography` (ed25519, x509) |
| **TypeScript / Node** | `jszip` / `fflate` | `@noble/hashes/blake3` | `@noble/ciphers` (gcm, chacha, cbc) or WebCrypto | `@noble/hashes/argon2`, WebCrypto PBKDF2 | `@noble/curves/ed25519`, `@peculiar/x509` |
| **Java** | `java.util.zip` | BouncyCastle `Blake3Digest` | JCE `AES/GCM`, BouncyCastle ChaCha20Poly1305, `AES/CBC`+`HmacSHA256` | BouncyCastle Argon2, `PBKDF2WithHmacSHA256` | BouncyCastle / `java.security` Ed25519, `java.security.cert.X509Certificate` |
| **C#/.NET** | `System.IO.Compression` | `Blake3` (NuGet) | `AesGcm`, `ChaCha20Poly1305`, `AesCbc`+`HMACSHA256` | `Konscious.Security.Cryptography.Argon2`, `Rfc2898DeriveBytes` | `System.Security.Cryptography` (Ed25519 via BouncyCastle), `X509Certificate2` |
| **Ruby** | `rubyzip` | `blake3` gem | `openssl` (GCM, CBC+HMAC), `RbNaCl` (ChaCha20Poly1305) | `argon2` gem, `OpenSSL::PKCS5` | `RbNaCl`/`ed25519` gem, `OpenSSL::X509` |
| **PHP** | `ZipArchive` | `BlakeOne/blake3` or ext | `sodium_crypto_aead_*`, `openssl_*` | `sodium_crypto_pwhash` (Argon2id), `hash_pbkdf2` | `sodium_crypto_sign_verify_detached`, `phpseclib` X.509 |
| **Swift** | `ZIPFoundation` | `blake3-swift` / via CryptoKit-adjacent | CryptoKit `AES.GCM`, `ChaChaPoly`; CommonCrypto for CBC+HMAC | `swift-argon2`, CommonCrypto PBKDF2 | CryptoKit `Curve25519.Signing`, `SwiftASN1`/`X509` |
| **Kotlin** | `java.util.zip` | BouncyCastle | (same as Java) | BouncyCastle / JCE | BouncyCastle / `java.security` |
| **C++** | `libzip` / `minizip` | official `BLAKE3` C lib | OpenSSL (`EVP_aead`-style or GCM/CBC), libsodium (ChaCha20Poly1305) | libsodium Argon2id, OpenSSL PBKDF2 | libsodium Ed25519, OpenSSL X.509 |
| **Dart** | `archive` | `blake3` (pub) | `cryptography` pkg (AesGcm, Chacha20.poly1305Aead, AesCbc+Hmac) | `cryptography` (Argon2id, Pbkdf2) | `cryptography` (Ed25519), `basic_utils` / `pointycastle` X.509 |

Notes:
- **X.509 needs only**: parse DER, read `subjectPublicKeyInfo` (Ed25519 OID `1.3.101.112`, 32-byte raw key), and `validity` (`notBefore`/`notAfter`). No CA-chain/path validation is performed by the reference verifier. The reference treats certs as self-signed and trusts the embedded public key; trust policy is left to the application.
- **Ed25519 X.509 generation** (signing side) is rarer in some ecosystems; SDKs that only **verify** never generate certs.
- **BLAKE3** is the single hard dependency even for the MVP; pick the maintained binding for each platform.

---

## Appendix A — Constant quick-reference (all source-verified)

| Constant | Value | Source |
|----------|-------|--------|
| ZIP magic @ offset 0 | `50 4B 03 04` | `ucx-parse/src/lib.rs:37` |
| mimetype | `application/vnd.unicodex+zip` | `ucx-build/src/archive.rs:28` |
| UCXE magic | `55 43 58 45` ("UCXE") | `ucx-crypto/src/lib.rs:80` |
| UCXE version | `0x01` | `ucx-crypto/src/lib.rs:84` |
| Algo IDs | GCM `0x01`, CBC `0x02`, ChaCha `0x03` | `ucx-crypto/src/lib.rs:172-179` |
| KDF IDs | None `0x00`, Argon2id `0x01`, PBKDF2 `0x02` | `ucx-crypto/src/lib.rs:234-241` |
| Tag lens | GCM 16, CBC 32, ChaCha 16 | `ucx-crypto/src/format.rs:280-294` |
| AEAD nonce | 12 bytes | `ucx-crypto/src/aes_gcm.rs:22` |
| CBC IV | 16 bytes | `ucx-crypto/src/aes_cbc.rs:47` |
| Salt | 16 bytes | `ucx-crypto/src/kdf.rs:37` |
| Chunk threshold | > 64 MiB (`67108864`) | `ucx-crypto/src/chunked.rs:47` |
| Chunk size | 1 MiB (`1048576`) | `ucx-crypto/src/chunked.rs:43` |
| Chunk nonce | `base[0..8] ‖ u32_be(idx)` | `ucx-crypto/src/chunked.rs:103-112` |
| Argon2id defaults | m=65536 KiB, t=3, p=4, v=0x13 | `ucx-crypto/src/kdf.rs:21-29,246-251` |
| PBKDF2 default | 600000 iters, HMAC-SHA256 | `ucx-crypto/src/kdf.rs:33,304` |
| Manifest digest encoding | **Base64-standard (padded)** of raw hash | `ucx-types/src/manifest.rs:343-347` |
| L1/L2 algo id | `0x0001` (Ed25519+BLAKE3) | `ucx-sign/src/layer1.rs:39`, `layer2.rs:41` |
| Ed25519 sig len | 64 bytes | `ucx-sign/src/layer1.rs:43` |
| L2 block magic | `UCX Sig Block 1\0` (16 B) | `ucx-sign/src/zip_binary.rs:25` |
| L2 pair id | `0x55435801` | `ucx-sign/src/zip_binary.rs:29` |
| L2 chunk prefixes | chunk `0xA5`, top `0x5A`; chunk size 1 MiB | `ucx-sign/src/layer2.rs:29-37` |
| Ed25519 cert SPKI OID | `1.3.101.112` | `ucx-sign/src/cert.rs:555`, `layer1.rs:489-491` |

## Appendix B — Known doc-vs-code discrepancies (implementers, take note)

1. **Manifest digest is Base64, not hex.** Docs example uses hex; code uses Base64-standard-padded. (§3.2)
2. **UCXE AEAD AAD is `header(8) ‖ kdf_params ‖ salt`, not a fixed 8 bytes.** CBC's HMAC AAD is the 8-byte header only. (§7.6)
3. **UCXE field order is header → KDF params → salt → IV → ct → tag.** Docs §4.1 diagram is ambiguous about KDF-params vs salt order; code writes KDF params first. (§7.2)
4. **`.EC` is a custom binary blob (algo/sig/cert), not PKCS#7 SignedData.** (§6.1)
5. **Only Ed25519 is implemented** for both signature layers; RSA/ECDSA in docs are not implemented. (§6)
6. **Chunked sub-nonce is concatenation `base[0..8] ‖ u32_be(idx)`**, explicitly NOT the older XOR scheme. (§7.7)
7. **Chunking threshold is `> 64 MiB` (strict)**, and chunked-mode top-level tag field is a zero placeholder. (§7.7)
