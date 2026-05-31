<!-- SPDX-License-Identifier: MIT -->

[English](README_en.md) | 中文

# Unicodex

**统一标准的小说文件格式**

Unicodex 定义了 `.ucx` 小说容器格式，并提供完整的 Rust 参考实现（CLI 工具 `ucx`），
涵盖项目初始化、构建打包、解析读取、版本管理、双层签名验证、加密解密。

---

## 特性

- **ZIP 容器** -- 基于 ZIP 归档，内含标准化元数据（`codex.json`）、内容结构（`struct.json`）、资源清单（`MANIFEST.MF`）
- **BLAKE3 完整性校验** -- 对每个文件条目做 BLAKE3 哈希，写入 MANIFEST，消费侧逐条比对
- **Ed25519 双层签名** -- Layer 1（JAR 风格 SF/EC）+ Layer 2（APK v2 风格签名块），支持多签名者
- **UCXE 加密** -- AES-256-GCM / AES-256-CBC+HMAC / ChaCha20-Poly1305，KDF 支持 Argon2id / PBKDF2 / 直接密钥
- **版本管理** -- 基于 git2 的自动变更检测与语义化版本推进
- **14 种语言只读 SDK** -- Go / Python / TypeScript / Java / Kotlin / C# / C++ / Rust / Ruby / PHP / Swift / Dart / ArkTS / Cangjie（仓颉）

---

## 项目结构

```
unicodex/
├── unicodex-core/   CLI 入口（bin 名 ucx，12 个子命令）
├── ucx-types/       公共类型（Codex / Structure / Manifest / ProjectConfig / UcxId）
├── ucx-init/        项目初始化
├── ucx-build/       构建打包
├── ucx-parse/       解析读取
├── ucx-verify/      双层签名 + 完整性验证
├── ucx-sign/        Ed25519 密钥 / 证书 / 双层签名
├── ucx-version/     git2 变更检测 + 自动版本
├── ucx-crypto/      AES-GCM / ChaCha20 / AES-CBC + Argon2id / PBKDF2 + UCXE
├── sdk/             14 种语言只读阅读器 SDK
└── docs/            规范文档 + 开发文档
```

---

## 快速开始

### 环境要求

- Rust 1.85+（edition 2024）
- Git（用于 `ucx-version` 的变更检测）

### 构建

```bash
cargo build --release
```

### CLI 子命令

| 命令 | 功能 |
|------|------|
| `ucx init` | 初始化新项目 |
| `ucx build` | 构建打包为 `.ucx` 文件 |
| `ucx info` | 查看 UCX 文件元数据 |
| `ucx verify` | 验证签名与完整性 |
| `ucx check` | 检查项目结构 |
| `ucx unpack` | 解包 UCX 文件 |
| `ucx keygen` | 生成 Ed25519 密钥对 |
| `ucx cert` | 创建/查看证书（`create` / `info`） |
| `ucx sign` | 对 UCX 文件签名 |
| `ucx version` | 版本管理（`auto` / `patch` / `chapter` / `volume` / `set`） |
| `ucx encrypt` | 加密为 UCXE |
| `ucx decrypt` | 解密 UCXE |

### 基本工作流

```bash
# 1. 初始化项目
ucx init my-novel

# 2. 编辑内容（content/ 目录下的 Markdown 文件）

# 3. 构建打包
ucx build

# 4. 验证完整性
ucx verify my-novel.ucx

# 5. 签名
ucx keygen --output author.key
ucx cert create --key author.key --cn "Author Name" --output author.cert.pem
ucx sign my-novel.ucx --key author.key --cert author.cert.pem

# 6. 加密（可选）
ucx encrypt my-novel.ucx --passphrase
```

---

## 多语言 SDK

14 种语言的只读阅读器 SDK，全部达 Level 3（解析 + 验签 + 解密），详见 [sdk/README.md](sdk/README.md)。

| 语言 | 目录 | 状态 |
|------|------|------|
| Rust | [sdk/rust/](sdk/rust/) | L3 全功能 |
| Go | [sdk/go/](sdk/go/) | L3 全功能 |
| Python | [sdk/python/](sdk/python/) | L3 全功能 |
| TypeScript | [sdk/typescript/](sdk/typescript/) | L3 全功能 |
| Java | [sdk/java/](sdk/java/) | L3 全功能 |
| Kotlin | [sdk/kotlin/](sdk/kotlin/) | L3 全功能 |
| C# | [sdk/csharp/](sdk/csharp/) | L3 全功能 |
| C++ | [sdk/cpp/](sdk/cpp/) | L3 全功能 |
| Ruby | [sdk/ruby/](sdk/ruby/) | L3 全功能 |
| PHP | [sdk/php/](sdk/php/) | L3 全功能 |
| Swift | [sdk/swift/](sdk/swift/) | L3 全功能 |
| Dart | [sdk/dart/](sdk/dart/) | L3 全功能 |
| ArkTS | [sdk/arkts/](sdk/arkts/) | L3 全功能 |
| Cangjie | [sdk/cangjie/](sdk/cangjie/) | L3 全功能 |

---

## 规范文档

完整的 UCX 格式规范位于 [docs/](docs/) 目录：

- [00-overview.md](docs/00-overview.md) -- 规范概述
- [01-file-structure.md](docs/01-file-structure.md) -- 文件结构
- [02-metadata-spec.md](docs/02-metadata-spec.md) -- 元数据规范
- [03-content-format.md](docs/03-content-format.md) -- 正文内容格式
- [04-crypto-spec.md](docs/04-crypto-spec.md) -- 加密规范
- [05-signature-spec.md](docs/05-signature-spec.md) -- 签名机制
- [06-blockchain-ext.md](docs/06-blockchain-ext.md) -- 区块链溯源
- [07-versioning.md](docs/07-versioning.md) -- 版本控制
- [08-keys-identity.md](docs/08-keys-identity.md) -- 密钥与身份验证
- [09-official-services.md](docs/09-official-services.md) -- 官方服务

---

## 许可

本项目以 [MIT](LICENSE) 许可发布。

Copyright (c) 2026 UniversalCodexFoundation/MoYeRanQianZhi
