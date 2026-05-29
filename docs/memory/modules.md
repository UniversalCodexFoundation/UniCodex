# 模块设计与实现状态（权威参考）

> 单一真实来源（SSOT）：**当前各模块的设计 / 公开 API / 实现状态**。
> 编年史见 [progress.md](./progress.md)；决策原因（ADR）见 [decisions.md](./decisions.md)；
> 前瞻计划 / 阶段 / 规范-模块映射见 [../plan/roadmap.md](../plan/roadmap.md)；未完成项见 [../TODO.md](../TODO.md)。
>
> 基线：git HEAD `05dd1f0`，版本 v0.4.0-alpha.2，edition 2024。
> 全部 9 个 crate 均为生产实现，无 placeholder。测试总数 284（280 单元 + 4 集成）。

工作区为 Cargo workspace，9 个 crate 平铺于仓库根（非 `crates/` 子目录）。
功能模块单向依赖 `ucx-types`，彼此不互相依赖（见 [decisions.md](./decisions.md) ADR-001）。

| crate | 角色 | 状态 | 测试 |
|---|---|---|---|
| ucx-types | 公共类型 | ✅ | 51 |
| unicodex-core | CLI 入口（bin=`ucx`） | ✅ | 4 集成 |
| ucx-parse | 解析读取 | ✅ | 15 |
| ucx-build | 构建打包 | ✅ | 24 |
| ucx-verify | 验证（Phase 3） | ✅ | 13 |
| ucx-sign | 双层签名（Phase 3） | ✅ | 42 |
| ucx-crypto | 加密解密（Phase 4） | ✅ | 82 |
| ucx-version | 版本管理（Phase 2） | ✅ | 40 |
| ucx-init | 项目初始化 | ✅ | 13 |

---

## 0. ucx-types（公共类型） ✅ 已实现（51 测试）

**定位**：所有 UCX 模块共享的数据结构，是 workspace 的依赖根。

**模块结构**：
```
ucx-types/src/
├── lib.rs         — 重导出所有公共类型
├── codex.rs       — Codex 元数据类型（对应 codex.json）
├── structure.rs   — Structure 内容结构类型（对应 struct.json）
├── manifest.rs    — Manifest 资源清单类型（对应 MANIFEST.MF）
├── project.rs     — ProjectConfig 项目配置类型（对应 unicodex.toml）
└── ucx_id.rs      — UcxId 唯一标识类型（urn:ucx:{UUID v4}）
```

**核心类型**：
```rust
pub struct Codex { schema, version, identifier, title, series, creators, publisher, language, genre, tags, status, word_count, description, rights, dates, cover, rating }
pub struct Structure { schema, version, structure: Vec<StructureNode> }
pub struct StructureNode { title, file, children, node_type, id, name, style, encryption }
pub struct Manifest { manifest_version, ucx_version, created_by, hash_algorithm, entries }
pub struct ManifestEntry { name, size, digest }
pub struct ProjectConfig { project, identifier, title, series, creators, publisher, book, description, rights, cover, rating, dates, build, signing }
pub struct UcxId(String)  // 格式：urn:ucx:{UUID v4}
pub enum HashAlgorithm { Blake3, Sha256, Sha512 }
```

**关键设计**：
- `Manifest::to_manifest_string()` / `from_manifest_str()` — RFC 822 格式自定义序列化，不依赖 serde
- `ManifestEntry::new()` 将原始哈希字节编码为 hex 字符串
- `StructureNode` 的 `file` 与 `children` 互斥（叶子 vs 容器节点）
- 所有可选字段使用 `#[serde(skip_serializing_if = "Option::is_none")]`

---

## 1. unicodex-core（CLI 入口） ✅ 已实现（4 集成测试）

**定位**：仅作为 CLI 入口，不包含业务逻辑；编译产物为 `ucx` 命令。

**实现状态**：已连接全部 Phase 1-4 子命令（12 个顶层子命令）。使用 `clap` derive 定义子命令，每个 arm 代理到对应模块的公开 API；错误用 `anyhow` 统一，日志用 `tracing-subscriber`（无时间戳、无 target）。

**子命令与 dispatch 行号**（`unicodex-core/src/main.rs`）：

| 子命令 | match arm 行号 | 代理目标 |
|---|---|---|
| `init` | main.rs:416 | `ucx_init::init()` / `init_from_existing()` |
| `build` | main.rs:504 | `ucx_build::build()` / `dry_run()` |
| `info` | main.rs:573 | `ucx_parse::open()` + 格式化输出 |
| `verify` | main.rs:702 | `ucx_verify::verify()`（+ 哈希校验） |
| `check` | main.rs:821 | `ucx_build::check()` |
| `unpack` | main.rs:856 | `ucx_parse` 解压到目录 |
| `keygen` | main.rs:914 | `ucx_sign::keygen()` |
| `cert`（create/info） | main.rs:927 | `ucx_sign::create_cert()` |
| `sign` | main.rs:973 | `ucx_sign::sign()` |
| `version`（auto/patch/chapter/volume/set） | main.rs:980 → `handle_version_command()` main.rs:1259 | `ucx_version::*` |
| `encrypt` | main.rs:983 | `ucx_crypto::encrypt` / `encrypt_with_passphrase` |
| `decrypt` | main.rs:1039 | `ucx_crypto::decrypt` / `decrypt_with_passphrase` |

**辅助函数**：`prompt_with_default()`（交互式提示）、`print_structure_tree()`（递归打印结构树）、`format_file_size()`（人类可读文件大小）。

**集成测试**：4 个往返测试（`unicodex-core/tests/round_trip.rs`）。

---

## 2. ucx-parse（解析读取） ✅ 已实现（15 测试）

**定位**：UCX 文件的解析与内容提取，是阅读器集成的核心模块。

**核心类型**：
```rust
pub struct UcxArchive {
    archive: ZipArchive<BufReader<File>>,  // 内部 ZIP reader
    file_path: PathBuf,                     // 原始文件路径
    codex: Codex,                           // 缓存的元数据
    structure: Structure,                   // 缓存的结构
    manifest: Manifest,                     // 缓存的清单
}
pub struct HashVerifyResult { name, expected, actual, valid }
pub enum ParseError { InvalidFormat, MissingFile, MetadataParse, ManifestParse, Io, Zip, Encoding }
```

**公开 API**：
```rust
pub fn open(path: &Path) -> Result<UcxArchive, ParseError>;
impl UcxArchive {
    pub fn codex(&self) -> &Codex;
    pub fn structure(&self) -> &Structure;
    pub fn manifest(&self) -> &Manifest;
    pub fn file_size(&self) -> u64;
    pub fn chapter_count(&self) -> usize;
    pub fn read_chapter(&mut self, file: &str) -> Result<String, ParseError>;
    pub fn verify_hashes(&mut self) -> Result<Vec<HashVerifyResult>, ParseError>;
    pub fn list_files(&self) -> Vec<String>;
}
```

**解析流程**：打开 → `BufReader<File>` → `ZipArchive` → 校验 mimetype（首条目，值 = `application/vnd.unicodex+zip`）→ 解析 `META-INF/MANIFEST.MF` / `metadata/codex.json` / `content/struct.json`。

**设计考量**：持有 ZIP reader 支持按需读取（不一次性解压）；`read_chapter(file)` 自动加 `content/` 前缀；`verify_hashes()` 克隆 manifest entries 避免借用冲突；手动实现 `Debug`（`ZipArchive` 未 derive）。

---

## 3. ucx-build（构建打包） ✅ 已实现（24 测试）

**定位**：将项目目录打包为 `.ucx` 文件。

**模块结构**：
```
ucx-build/src/
├── lib.rs        — 公开 API（build / dry_run / check / resolve_output_path / config_to_codex / BuildOptions / BuildError）
├── archive.rs    — ZIP 归档创建（文件收集、哈希计算、MANIFEST.MF 生成）
└── convert.rs    — ProjectConfig → Codex 转换（含 dates 自动填充）
```

**公开 API**：
```rust
pub fn build(project_path: &Path, options: &BuildOptions) -> Result<PathBuf, BuildError>;
pub fn dry_run(project_path: &Path, options: &BuildOptions) -> Result<DryRunResult, BuildError>;
pub fn check(project_path: &Path) -> Result<CheckResult, BuildError>;
pub fn resolve_output_path(project_path: &Path, options: &BuildOptions) -> Result<PathBuf, BuildError>;
pub fn config_to_codex(config: &ProjectConfig) -> Codex;
pub struct BuildOptions { output_dir: Option<PathBuf>, output_name: Option<String>, dry_run: bool }
pub struct DryRunResult { output_path, files: Vec<DryRunFile>, total_size }
pub enum BuildError { ConfigNotFound, ConfigParse, InvalidStructure, Io, Zip, Serialization }
```

**构建流程**：读 `unicodex.toml` → `config_to_codex()` → 读 `content/struct.json` → 递归收集 content/、assets/ → 确定输出路径（options > config > "dist"）→ 创建 ZIP（mimetype STORED 在首，其余按扩展名选择压缩）。

**压缩策略**：STORED（不压缩）jpg/jpeg/png/webp/gif/mp3/ogg/mp4/pdf/woff2/ttf/otf；DEFLATE 其余文本与未知格式。**MANIFEST 排除**：mimetype、`META-INF/MANIFEST.MF`、`META-INF/signatures/*`、`META-INF/certs/*`。

---

## 4. ucx-verify（验证） ✅ 已实现（Phase 3，13 测试）

**定位**：验证 UCX 文件的完整性与签名。

**模块结构**：单文件 `ucx-verify/src/lib.rs`。

**公开 API**：
```rust
pub fn verify(path: &Path) -> Result<VerifyReport, VerifyError>;  // lib.rs:223
```

**验证层次**：
- **Layer 1**：SF 文件对 `MANIFEST.MF` 各段摘要 + RSA/EC 文件对 SF 的签名校验。
- **Layer 2**：签名块的 Ed25519 全局签名校验（覆盖整个 ZIP）。
- 文件级哈希完整性由 `ucx-parse::verify_hashes()` 提供，`ucx verify` CLI 一并调用。

**设计考量**：best-effort 验证，不轻易拒绝；MAJOR 版本不匹配时告警但仍尝试解析；`VerifyReport` 返回各文件、各层签名的详细状态。

---

## 5. ucx-sign（签名） ✅ 已实现（Phase 3，42 测试）

**定位**：为 UCX 文件生成双层签名，并提供密钥 / 证书工具。

**模块结构**：
```
ucx-sign/src/
├── lib.rs         — 公开 API（sign / keygen / create_cert）
├── keys.rs        — Ed25519 密钥生成与读写
├── cert.rs        — 自签名 X.509 证书创建 / 解析
├── layer1.rs      — JAR 式：SF 生成 + 对 SF 的签名
├── layer2.rs      — APK v2 式：全局签名块构建 / 解析
├── zip_binary.rs  — ZIP 二进制结构读取（定位 Central Directory）
└── zip_rewrite.rs — 将签名块插入 / 重写进 ZIP
```

**公开 API**：
```rust
pub fn sign(...) -> Result<(), SignError>;        // lib.rs:123
pub fn keygen(output_path: &Path) -> Result<(), SignError>;  // lib.rs:285
pub fn create_cert(...) -> Result<(), SignError>; // lib.rs:356
```

**Layer 1（JAR 式）**：生成 `{SIGNER}.SF`（对 MANIFEST.MF 各段落的摘要）+ `{SIGNER}.EC`/`.RSA`（对 SF 的数字签名 + 证书）。
**Layer 2（APK v2 式）**：在 ZIP 的 Central Directory 之前插入签名块，覆盖整个 ZIP 内容以防非 META-INF 文件被篡改。

**设计考量**：支持多签名者（作者 + 出版方 + 平台）；签名算法首选 Ed25519。

---

## 6. ucx-crypto（加密解密） ✅ 已实现（Phase 4，82 测试）

**定位**：章节级 / 资源级 / 段落级加密，UCXE 二进制格式读写。

**模块结构**：
```
ucx-crypto/src/
├── lib.rs        — 公开 API（encrypt / decrypt / *_with_passphrase / read_ucxe / write_ucxe）
├── format.rs     — UCXE 二进制格式（header / flags / AEAD AAD）读写
├── aes_gcm.rs    — AES-256-GCM（推荐）
├── chacha20.rs   — ChaCha20-Poly1305
├── aes_cbc.rs    — AES-256-CBC + HMAC-SHA256（Encrypt-then-MAC）
├── kdf.rs        — Argon2id / PBKDF2 密钥派生
├── chunked.rs    — 大文件分块加密（>64 MiB）
└── paragraph.rs  — 段落级加密
```

**公开 API**：
```rust
pub fn encrypt(...) -> ...;                  // lib.rs:313
pub fn decrypt(...) -> ...;                  // lib.rs:371
pub fn encrypt_with_passphrase(...) -> ...;  // lib.rs:440
pub fn decrypt_with_passphrase(...) -> ...;  // lib.rs:541
// 另含：write_ucxe / read_ucxe、derive_argon2id / derive_pbkdf2、encrypt_chunked、encrypt_paragraph
```

**设计考量**：IV/Nonce 必须 CSPRNG 生成；passphrase 进入 KDF 前统一 NFC 归一化（commit e985b25）；AEAD AAD 使用原始 flags 字节、覆盖 reserved bits（commit 05dd1f0）；解密错误不泄露具体失败原因（防 Oracle 攻击）。

---

## 7. ucx-version（版本管理） ✅ 已实现（Phase 2，40 测试）

**定位**：基于 git2 的自动版本号管理，无 Git 时回退快照。

**模块结构**：
```
ucx-version/src/
├── lib.rs       — 公开 API（detect_changes / auto_version / classify_changes / compute_next_version）
├── git.rs       — git2 集成（detect_changes_git / create_version_tag）
├── snapshot.rs  — 无 Git 回退（Snapshot::{load, save, compute_current, diff}）
└── bump.rs      — 版本号递增（卷.章.修订）规则
```

**公开 API**：
```rust
pub fn detect_changes(...) -> ...;   // lib.rs:223
pub fn auto_version(...) -> ...;     // lib.rs:296
// 另含：classify_changes、compute_next_version、detect_changes_git、create_version_tag、
//       Snapshot::{load, save, compute_current, diff}
```

**设计考量**：Git tag 约定 `ucx-v{X.Y.Z}`；章节编号基于 `struct.json` 叶子节点顺序计数；版本策略可在 `unicodex.toml` 的 `[version]` 段配置。

---

## 8. ucx-init（项目初始化） ✅ 已实现（13 测试）

**定位**：创建新的 UCX 项目目录结构。

**公开 API**：
```rust
pub fn init(path: &Path, options: &InitOptions) -> Result<(), InitError>;
pub fn init_from_existing(path: &Path, options: &InitOptions) -> Result<(), InitError>;
pub struct InitOptions { name, author, language, allow_long_fields, full, no_git, force }
pub enum InitError { AlreadyExists, Io, TomlSerialize, JsonSerialize, InvalidInput }
```

**`init` 流程**：检查 `unicodex.toml` 是否已存在（已存在且非 `--force` 则 `AlreadyExists`，不修改任何文件；`--force` 跳过此检查，commit 259aecb）→ 创建根目录与子目录（默认仅 `content/`，`--full` 追加 `assets/`、`extras/`）→ 生成 UCX ID → 写 `unicodex.toml` / `content/struct.json` / `content/chapter-001.md` → 初始化 Git + `.gitignore`（`--no-git` 跳过）。

**`init_from_existing` 流程**：扫描 .md → 移入 `content/` → 自动生成 `struct.json` → 执行标准 init 后续步骤。

**输入验证**：`validate_input()`（空串 / 控制字符含 `\n`\`\r` / 长度 ≤500）；`validate_language_tag()`（BCP 47 基本格式）。默认值本地化为中文（"无标题"/"未知"），`dates.created` 自动填入 init 日期。

---

## 9. 历史注记

早期曾讨论公共类型（`UcxId`、`ManifestEntry` 等）的归属问题，已通过创建 `ucx-types` crate 解决（§0，详见 [decisions.md](./decisions.md) ADR-001）。
