# Unicodex 规范概述

> 版本：0.1.0-draft
> 状态：设计阶段
> 文件后缀：`.ucx`

## 1. 项目定位

Unicodex 是一种**统一的小说文件标准格式**，面向全领域（网文平台、个人作者、出版社、分发平台），旨在解决以下问题：

| 问题领域 | 现状痛点 | Unicodex 方案 |
|---------|---------|--------------|
| 格式碎片化 | txt/epub/mobi 等格式各异，元数据丢失 | 统一 ZIP 容器 + 标准化元数据 |
| 版权保护 | 内容易被篡改、盗版难追溯 | JAR 式三层签名（MF → SF → RSA/EC） |
| 内容完整性 | 无法验证内容是否被修改 | MANIFEST.MF 全文件哈希校验 |
| 来源追溯 | 作者身份难以验证 | 多签名 + 证书链 + 区块链存证 |
| 多方协作 | 合著/出版/分发各方权责不清 | 多签名机制（作者/出版方/平台独立签名） |

## 2. 设计原则

1. **开放标准** - 规范完全公开，任何人可实现兼容工具
2. **安全优先** - 签名、哈希验证贯穿整个设计，加密为可选扩展
3. **向后兼容** - 版本升级保持旧版本文件可读
4. **最小依赖** - 核心功能不依赖外部服务（区块链为可选扩展）
5. **Rust 实现** - 参考实现使用 Rust，保证内存安全与性能

## 3. 核心架构

```
.ucx (ZIP Archive)
├── mimetype           固定声明
├── META-INF/          【核心安全区】签名 + 校验
├── metadata/          【元数据区】作品信息
├── content/           【正文区】章节内容
├── assets/            【资源区】图片/字体（可选）
├── extensions/        【扩展区】第三方扩展（可选）
└── chain/             【溯源区】区块链存证（可选）
```

## 4. 文档索引

| 文档 | 描述 | 状态 |
|-----|------|-----|
| [01-file-structure.md](./01-file-structure.md) | 文件结构规范 | 设计中 |
| [02-metadata-spec.md](./02-metadata-spec.md) | 元数据规范 | 设计中 |
| [03-content-format.md](./03-content-format.md) | 正文内容格式（.ucxc/.mdx/.md/.txt/.typ） | 设计中 |
| [04-crypto-spec.md](./04-crypto-spec.md) | 加密规范（可选功能） | 设计中 |
| [05-signature-spec.md](./05-signature-spec.md) | 签名机制规范（双层签名体系） | 设计中 |
| [06-blockchain-ext.md](./06-blockchain-ext.md) | 区块链溯源规范 | 设计中 |
| [07-versioning.md](./07-versioning.md) | 版本控制规范 | 设计中 |
| [08-keys-identity.md](./08-keys-identity.md) | 密钥与身份验证规范 | 设计中 |
| [09-official-services.md](./09-official-services.md) | 官方服务与服务商规范（密钥分发、身份验证、包验证等） | 设计中 |

## 5. 术语定义

| 术语 | 英文 | 定义 |
|-----|------|-----|
| UCX 文件 | UCX File | 符合 Unicodex 规范的 `.ucx` 文件 |
| MANIFEST.MF | Manifest | 资源清单，记录所有文件的哈希值 |
| SF 文件 | Signature File | 签名文件，对 MANIFEST.MF 的摘要 |
| RSA/EC 文件 | Certificate Block | 证书块，包含公钥和对 SF 的签名 |
| struct.json | Structure | 内容结构定义文件 |
| genesis.proof | Genesis Proof | 区块链锚定文件 |
| Certificate Chain | Certificate Chain | 证书链，验证签名者身份 |

## 6. 已确定的设计决策

| 决策项 | 结论 |
|-------|------|
| 容器格式 | ZIP-based |
| 安全区命名 | META-INF（JAR 风格，大写文件名） |
| 签名体系 | JAR 三层 + APK v2 全局签名块（双层） |
| 签名验证 | 高自由度：警告但允许强制打开 |
| 多签名 | 支持，各签名者独立签名 |
| 推荐哈希 | BLAKE3 |
| 推荐签名 | Ed25519 |
| 加密功能 | 可选，后续设计 |
| 章节格式 | 支持 .ucxc / .mdx / .md / .txt / .typ / .tex |
| 扩展功能 | extensions/ 目录，命名空间隔离 + 多版本共存（org.unicodex/{id}/{version}/） |
| 区块链存证 | chain/genesis.proof（可选） |
| 实现语言 | Rust |
| 章节/资源级加密 | 可选，每个章节和资源独立配置，支持多种密钥获取方式 |
| 官方服务 | 密钥分发、身份验证、CA、包验证、公钥注册、时间戳，均可替换为第三方 |

## 7. 开发规范

### 7.1 语言与技术栈

| 项目 | 语言 | 说明 |
|------|------|------|
| 核心实现 | **Rust** | 主要开发语言，所有核心模块必须用 Rust 实现 |
| 版本管理 | **Rust** (git2) | 使用 libgit2 的 Rust 绑定 `git2` crate 实现项目版本管理 |
| 其他语言绑定 | Go / Python / JavaScript / ... | 可选，需添加语言前缀 |

### 7.2 项目结构

```
Unicodex/                           # 项目根目录
├── docs/                           # 规范文档（本目录）
│   ├── 00-overview.md
│   ├── 01-file-structure.md
│   └── ...
│
├── unicodex-core/                  # [Rust] 核心库 + CLI 入口
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                  # 库入口
│       ├── main.rs                 # CLI 入口（ucx 命令）
│       ├── cli/                    # CLI 命令实现
│       ├── archive/                # ZIP 操作
│       ├── signature/              # 签名验证
│       ├── crypto/                 # 加密解密
│       ├── manifest/               # MANIFEST.MF 处理
│       └── ...
│
├── ucx-init/                       # [Rust] 项目初始化模块
│   ├── Cargo.toml                  # 依赖 unicodex-core
│   └── src/
│       └── lib.rs
│
├── ucx-build/                      # [Rust] 打包构建模块
│   ├── Cargo.toml                  # 依赖 unicodex-core
│   └── src/
│       └── lib.rs
│
├── ucx-parse/                      # [Rust] 解析读取模块
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
│
├── ucx-verify/                     # [Rust] 验证模块
│   ├── Cargo.toml                  # 依赖 unicodex-core
│   └── src/
│       └── lib.rs
│
├── ucx-sign/                       # [Rust] 签名模块
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
│
├── ucx-version/                    # [Rust] 版本管理模块（基于 git2）
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
│
├── go-ucx/                         # [Go] Go 语言绑定/SDK
│   ├── go.mod
│   └── ...
│
├── py-ucx/                         # [Python] Python 库（读取/解析）
│   ├── pyproject.toml
│   └── ...
│
├── py-ucx-sdk/                     # [Python] Python SDK（完整功能）
│   ├── pyproject.toml
│   └── ...
│
├── js-ucx/                         # [JavaScript/TypeScript] JS/TS 库
│   ├── package.json
│   └── ...
│
├── dotnet-ucx/                     # [C#/.NET] .NET 库
│   ├── Unicodex.csproj
│   └── ...
│
└── java-ucx/                       # [Java/Kotlin] JVM 库
    ├── build.gradle.kts
    └── ...
```

### 7.3 模块命名规范

**Rust 模块**（主要开发语言，无前缀）：

| 命名模式 | 说明 | 示例 |
|---------|------|------|
| `unicodex-core` | CLI 入口（依赖所有 `ucx-*` 功能模块，编译为 `ucx` 命令） | `unicodex-core` |
| `ucx-{功能}` | 功能模块（独立库，可被第三方阅读器/工具单独引用） | `ucx-init`、`ucx-build`、`ucx-verify`、`ucx-sign` |
| `ucx-{扩展名}` | 扩展模块 | `ucx-audio-book`、`ucx-interactive` |

**其他语言**（需添加语言前缀）：

| 语言 | 前缀 | 示例 |
|------|------|------|
| Go | `go-` | `go-ucx`、`go-ucx-server` |
| Python | `py-` | `py-ucx`、`py-ucx-sdk`、`py-ucx-reader` |
| JavaScript/TypeScript | `js-` | `js-ucx`、`js-ucx-viewer` |
| C#/.NET | `dotnet-` | `dotnet-ucx`、`dotnet-ucx-reader` |
| Java/Kotlin | `java-` | `java-ucx`、`java-ucx-android` |
| Swift | `swift-` | `swift-ucx`、`swift-ucx-ios` |
| C/C++ | `c-` / `cpp-` | `c-ucx`、`cpp-ucx` |

### 7.4 Rust 项目依赖关系

```
┌─────────────────────────────────────────────────────────────────┐
│                   Rust 模块依赖关系                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│   │ ucx-init │ │ucx-build │ │ucx-parse │ │ucx-verify│  ...     │
│   └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
│        ▲            ▲            ▲            ▲                 │
│        │            │            │            │                 │
│        │            │            │            │                 │
│        └────────────┴─────┬──────┴────────────┘                 │
│                           │                                     │
│                           │ 依赖                                │
│                           │                                     │
│                    ┌──────┴──────┐                              │
│                    │unicodex-core│  ← CLI 入口                  │
│                    │  (ucx 命令) │     依赖所有 ucx-* 模块       │
│                    └─────────────┘                              │
│                                                                 │
│   功能模块可被第三方单独引用：                                    │
│   ┌─────────────────────────────────────────────┐               │
│   │  第三方阅读器                                │               │
│   │  ├── 依赖 ucx-parse   → 获得解包/读取能力   │               │
│   │  ├── 依赖 ucx-verify  → 获得验证能力        │               │
│   │  └── 依赖 ucx-crypto  → 获得解密能力        │               │
│   └─────────────────────────────────────────────┘               │
│                                                                 │
│   功能模块之间也可互相依赖：                                      │
│   ucx-build 可依赖 ucx-sign（构建时自动签名）                    │
│   ucx-verify 可依赖 ucx-crypto（解密后校验）                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 7.5 Workspace 配置

项目根目录使用 Cargo Workspace 管理所有 Rust 模块：

```toml
# Cargo.toml (根目录)
[workspace]
resolver = "2"
members = [
    "unicodex-core",
    "ucx-init",
    "ucx-build",
    "ucx-parse",
    "ucx-verify",
    "ucx-sign",
    "ucx-version",
    "ucx-crypto",
    # 扩展模块
    "ucx-audio-book",
    "ucx-interactive",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/unicodex/unicodex"
authors = ["Unicodex Contributors"]

[workspace.dependencies]
# 内部依赖
unicodex-core = { path = "unicodex-core" }
ucx-verify = { path = "ucx-verify" }
ucx-sign = { path = "ucx-sign" }
ucx-version = { path = "ucx-version" }
ucx-crypto = { path = "ucx-crypto" }

# 外部依赖（统一版本管理）
blake3 = "1.5"
ed25519-dalek = "2.1"
zip = "0.6"
git2 = "0.19"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
thiserror = "1.0"
anyhow = "1.0"
clap = { version = "4.4", features = ["derive"] }
tokio = { version = "1.35", features = ["full"] }
```

### 7.6 模块职责划分

| 模块 | 职责 | CLI 命令 | 可被第三方引用 |
|------|------|---------|---------------|
| `unicodex-core` | CLI 入口，集成所有功能模块 | `ucx <command>` | 否（仅作为 CLI） |
| `ucx-init` | 创建新项目、生成 `unicodex.toml` | `ucx init` | 是 |
| `ucx-build` | 打包构建 UCX 文件、TOML→JSON 转换 | `ucx build`、`ucx pack` | 是 |
| `ucx-parse` | 解析读取 UCX 文件、提取内容 | `ucx unpack`、`ucx info` | 是（阅读器解包） |
| `ucx-verify` | 本地验证、在线验证 | `ucx verify` | 是（阅读器验证） |
| `ucx-sign` | 签名、证书管理 | `ucx sign`、`ucx cert` | 是 |
| `ucx-version` | 版本管理、变更检测（基于 git2） | `ucx version` | 是 |
| `ucx-crypto` | 加密解密、密钥管理 | `ucx encrypt`、`ucx decrypt` | 是（阅读器解密） |

### 7.7 代码规范

#### 7.7.1 通用规范

- **注释语言**：代码注释使用**英文**（便于国际化协作）
- **文档注释**：公开 API 必须有 `///` 文档注释
- **错误处理**：使用 `thiserror` 定义错误类型，避免 `unwrap()` / `expect()` 在库代码中出现
- **日志**：使用 `tracing` 或 `log` crate，禁止直接 `println!`（除 CLI 输出外）

#### 7.7.2 Rust 代码风格

```rust
// 文件头部：模块说明
//! UCX Signature Verification Module
//!
//! This module provides signature verification for UCX files,
//! supporting both Layer 1 (JAR-style) and Layer 2 (APK v2-style) signatures.

use unicodex_core::{UcxArchive, ManifestFile};
use thiserror::Error;

/// Errors that can occur during signature verification.
///
/// 签名验证过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum VerifyError {
    /// Signature block not found in UCX archive.
    #[error("signing block not found")]
    SigningBlockNotFound,

    /// Signature verification failed.
    #[error("signature verification failed: {0}")]
    SignatureInvalid(String),

    /// Certificate chain validation failed.
    #[error("certificate chain validation failed: {0}")]
    CertificateInvalid(String),
}

/// Verification result for a UCX file.
///
/// UCX 文件的验证结果。
#[derive(Debug, Clone)]
pub struct VerifyResult {
    /// Overall verification status.
    pub status: VerifyStatus,
    /// Layer 2 (signing block) verification result.
    pub layer2: Option<Layer2Result>,
    /// Layer 1 (META-INF) verification result.
    pub layer1: Option<Layer1Result>,
    /// File integrity check result.
    pub file_integrity: FileIntegrityResult,
}

/// Verify a UCX file.
///
/// 验证 UCX 文件的完整性和签名。
///
/// # Arguments
///
/// * `archive` - The UCX archive to verify.
///
/// # Returns
///
/// Returns `VerifyResult` containing detailed verification information.
///
/// # Example
///
/// ```rust
/// use ucx_verify::verify;
/// use unicodex_core::UcxArchive;
///
/// let archive = UcxArchive::open("novel.ucx")?;
/// let result = verify(&archive)?;
/// println!("Status: {:?}", result.status);
/// ```
pub fn verify(archive: &UcxArchive) -> Result<VerifyResult, VerifyError> {
    // Implementation...
    todo!()
}
```

#### 7.7.3 目录结构规范

```
ucx-verify/
├── Cargo.toml
├── README.md                       # 模块说明文档
├── src/
│   ├── lib.rs                      # 库入口，导出公开 API
│   ├── main.rs                     # CLI 入口（可选）
│   ├── error.rs                    # 错误定义
│   ├── layer1.rs                   # Layer 1 验证逻辑
│   ├── layer2.rs                   # Layer 2 验证逻辑
│   ├── integrity.rs                # 文件完整性检查
│   └── result.rs                   # 结果类型定义
├── tests/
│   ├── integration_test.rs         # 集成测试
│   └── fixtures/                   # 测试用 UCX 文件
│       ├── valid.ucx
│       ├── unsigned.ucx
│       └── tampered.ucx
└── benches/                        # 性能测试（可选）
    └── verify_bench.rs
```

### 7.8 版本管理

- 所有 Rust 模块使用**统一版本号**（通过 Workspace 管理）
- 遵循 [Semantic Versioning 2.0.0](https://semver.org/)
- 版本号格式：`MAJOR.MINOR.PATCH`
  - `MAJOR`：不兼容的 API 变更
  - `MINOR`：向后兼容的功能新增
  - `PATCH`：向后兼容的问题修复

### 7.9 发布与分发

| 分发渠道 | 包名 | 说明 |
|---------|------|------|
| crates.io | `unicodex-core`、`ucx-cli` | Rust 包 |
| PyPI | `ucx`、`ucx-sdk` | Python 包 |
| npm | `@unicodex/ucx` | JavaScript 包 |
| Homebrew | `ucx` | macOS CLI |
| apt/yum | `ucx` | Linux CLI |
| GitHub Releases | `ucx-{version}-{platform}` | 预编译二进制 |

---

*请指出需要修改或扩展的方向*
