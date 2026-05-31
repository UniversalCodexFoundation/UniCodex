English | [中文](README.md)

# Unicodex

**A Unified Novel File Standard**

Unicodex defines the `.ucx` novel container format and provides a complete Rust reference
implementation (CLI tool `ucx`) covering project initialization, build/packaging, parsing,
version management, dual-layer signature verification, and encryption/decryption.

---

## Features

- **ZIP Container** -- ZIP-based archive with standardized metadata (`codex.json`), content structure (`struct.json`), and resource manifest (`MANIFEST.MF`)
- **BLAKE3 Integrity** -- BLAKE3 hash for every file entry recorded in MANIFEST, verified entry-by-entry on the consumer side
- **Ed25519 Dual-Layer Signatures** -- Layer 1 (JAR-style SF/EC) + Layer 2 (APK v2-style signing block), with multi-signer support
- **UCXE Encryption** -- AES-256-GCM / AES-256-CBC+HMAC / ChaCha20-Poly1305, KDF via Argon2id / PBKDF2 / direct key
- **Version Management** -- Git2-based automatic change detection and semantic version bumping
- **14-Language Read-Only SDKs** -- Go / Python / TypeScript / Java / Kotlin / C# / C++ / Rust / Ruby / PHP / Swift / Dart / ArkTS / Cangjie

---

## Project Structure

```
unicodex/
├── unicodex-core/   CLI entry point (binary name: ucx, 12 subcommands)
├── ucx-types/       Shared types (Codex / Structure / Manifest / ProjectConfig / UcxId)
├── ucx-init/        Project initialization
├── ucx-build/       Build and packaging
├── ucx-parse/       Parsing and reading
├── ucx-verify/      Dual-layer signature + integrity verification
├── ucx-sign/        Ed25519 key / certificate / dual-layer signing
├── ucx-version/     Git2 change detection + automatic versioning
├── ucx-crypto/      AES-GCM / ChaCha20 / AES-CBC + Argon2id / PBKDF2 + UCXE
├── sdk/             14-language read-only reader SDKs
└── docs/            Specification and development documentation
```

---

## Quick Start

### Clone the Repository

This repo references 14 SDK sub-repos via git submodules. Use `--recurse-submodules` when cloning:

```bash
git clone --recurse-submodules https://github.com/UniversalCodexFoundation/UniCodex.git
```

If already cloned with empty SDK directories, fetch them with:

```bash
git submodule update --init --recursive
```

### Requirements

- Rust 1.85+ (edition 2024)
- Git (for `ucx-version` change detection)

### Build

```bash
cargo build --release
```

### CLI Subcommands

| Command | Description |
|---------|-------------|
| `ucx init` | Initialize a new project |
| `ucx build` | Build and package into a `.ucx` file |
| `ucx info` | View UCX file metadata |
| `ucx verify` | Verify signatures and integrity |
| `ucx check` | Check project structure |
| `ucx unpack` | Unpack a UCX file |
| `ucx keygen` | Generate an Ed25519 key pair |
| `ucx cert` | Create/view certificates (`create` / `info`) |
| `ucx sign` | Sign a UCX file |
| `ucx version` | Version management (`auto` / `patch` / `chapter` / `volume` / `set`) |
| `ucx encrypt` | Encrypt to UCXE |
| `ucx decrypt` | Decrypt UCXE |

### Basic Workflow

```bash
# 1. Initialize a project
ucx init my-novel

# 2. Edit content (Markdown files under content/)

# 3. Build and package
ucx build

# 4. Verify integrity
ucx verify my-novel.ucx

# 5. Sign
ucx keygen --output author.key
ucx cert create --key author.key --cn "Author Name" --output author.cert.pem
ucx sign my-novel.ucx --key author.key --cert author.cert.pem

# 6. Encrypt (optional)
ucx encrypt my-novel.ucx --passphrase
```

---

## Multi-Language SDKs

14 read-only reader SDKs, all at Level 3 (parse + verify + decrypt). See [sdk/README.md](sdk/README.md) for details.

| Language | Directory | Status |
|----------|-----------|--------|
| Rust | [sdk/rust/](sdk/rust/) | L3 full |
| Go | [sdk/go/](sdk/go/) | L3 full |
| Python | [sdk/python/](sdk/python/) | L3 full |
| TypeScript | [sdk/typescript/](sdk/typescript/) | L3 full |
| Java | [sdk/java/](sdk/java/) | L3 full |
| Kotlin | [sdk/kotlin/](sdk/kotlin/) | L3 full |
| C# | [sdk/csharp/](sdk/csharp/) | L3 full |
| C++ | [sdk/cpp/](sdk/cpp/) | L3 full |
| Ruby | [sdk/ruby/](sdk/ruby/) | L3 full |
| PHP | [sdk/php/](sdk/php/) | L3 full |
| Swift | [sdk/swift/](sdk/swift/) | L3 full |
| Dart | [sdk/dart/](sdk/dart/) | L3 full |
| ArkTS | [sdk/arkts/](sdk/arkts/) | L3 full |
| Cangjie | [sdk/cangjie/](sdk/cangjie/) | L3 full |

---

## Specification

The full UCX format specification is in the [docs/](docs/) directory:

- [00-overview.md](docs/00-overview.md) -- Overview
- [01-file-structure.md](docs/01-file-structure.md) -- File Structure
- [02-metadata-spec.md](docs/02-metadata-spec.md) -- Metadata Specification
- [03-content-format.md](docs/03-content-format.md) -- Content Format
- [04-crypto-spec.md](docs/04-crypto-spec.md) -- Encryption Specification
- [05-signature-spec.md](docs/05-signature-spec.md) -- Signature Mechanism
- [06-blockchain-ext.md](docs/06-blockchain-ext.md) -- Blockchain Provenance
- [07-versioning.md](docs/07-versioning.md) -- Version Control
- [08-keys-identity.md](docs/08-keys-identity.md) -- Keys and Identity
- [09-official-services.md](docs/09-official-services.md) -- Official Services

---

## License

This project is released under the [MIT](LICENSE) license.

Copyright (c) 2026 UniversalCodexFoundation/MoYeRanQianZhi
