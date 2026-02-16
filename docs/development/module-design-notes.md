# 模块设计笔记

> 创建时间：2026-02-16
> 状态：持续更新

本文档记录各 Rust 模块的设计思路、接口草案和技术备忘。

## 0. ucx-types（公共类型） ✅ Phase 1 已实现

**定位**：所有 UCX 模块共享的数据结构，解决 ADR-001 中识别的类型归属问题。

**实现状态**：已完成（30 单元测试 + 1 doctest）

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
pub struct ProjectConfig { project, identifier, title, series, creators, publisher, book, description, rights, cover, rating, build, signing }
pub struct UcxId(String)  // format: urn:ucx:{UUID v4}
pub enum HashAlgorithm { Blake3, Sha256, Sha512 }
```

**关键设计决策**：
- `Manifest::to_manifest_string()` / `from_manifest_str()` — RFC 822 格式自定义序列化，不依赖 serde
- `ManifestEntry::new()` 将原始哈希字节编码为 hex 字符串
- `StructureNode` 的 `file` 和 `children` 互斥（叶子 vs 容器节点）
- 所有可选字段使用 `#[serde(skip_serializing_if = "Option::is_none")]`

---

## 1. unicodex-core（CLI 入口） ✅ Phase 1 已实现

**定位**：仅作为 CLI 入口，不包含业务逻辑。编译产物为 `ucx` 命令。

**实现状态**：Phase 1 命令已连接（init、build、info、verify）

**设计思路**：
- 使用 `clap` derive 宏定义子命令结构
- 每个子命令代理到对应模块的公开 API
- 错误处理使用 `anyhow`，将各模块的 `thiserror` 错误统一转换
- 日志初始化在 main.rs 中完成（`tracing-subscriber`）

**已实现子命令**：
```
ucx init <path> --name --author --language    → ucx_init::init()
ucx build <path> --output-dir --output-name   → ucx_build::build()
ucx info <file>                                → ucx_parse::open() + 格式化输出
ucx verify <file>                              → ucx_parse::open() + verify_hashes()
```

**待实现子命令**：
```
ucx unpack     → Phase 2
ucx version    → Phase 2（ucx-version）
ucx sign       → Phase 3（ucx-sign）
ucx encrypt    → Phase 4（ucx-crypto）
ucx decrypt    → Phase 4（ucx-crypto）
```

**集成测试**：4 个往返测试（unicodex-core/tests/round_trip.rs）

---

## 2. ucx-parse（解析读取） ✅ Phase 1 已实现

**定位**：UCX 文件的解析和内容提取，是阅读器集成的核心模块。

**实现状态**：已完成（6 单元测试）

**核心类型**：
```rust
pub struct UcxArchive {
    archive: ZipArchive<BufReader<File>>,  // 内部 ZIP reader
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
    pub fn read_chapter(&mut self, file: &str) -> Result<String, ParseError>;
    pub fn verify_hashes(&mut self) -> Result<Vec<HashVerifyResult>, ParseError>;
    pub fn list_files(&self) -> Vec<String>;
}
```

**解析流程**：
1. 打开文件 → `BufReader<File>` → `ZipArchive`
2. 验证 mimetype（第一个条目，值 = `application/vnd.unicodex+zip`）
3. 解析 `META-INF/MANIFEST.MF` → `Manifest::from_manifest_str()`
4. 解析 `metadata/codex.json` → `serde_json::from_str::<Codex>()`
5. 解析 `content/struct.json` → `serde_json::from_str::<Structure>()`

**设计考量**：
- `UcxArchive` 持有 ZIP reader，支持按需读取（不一次性解压全部）
- `read_chapter(file)` 自动添加 `content/` 前缀
- `verify_hashes()` 克隆 manifest entries 避免借用冲突
- 手动实现 `Debug`（因为 `ZipArchive` 未 derive Debug）

---

## 3. ucx-build（构建打包） ✅ Phase 1 已实现

**定位**：将项目目录打包为 `.ucx` 文件。

**实现状态**：已完成（12 单元测试 + 1 doctest）

**模块结构**：
```
ucx-build/src/
├── lib.rs         — 公开 API (build 函数、BuildOptions、BuildError)
├── archive.rs     — ZIP 归档创建（文件收集、哈希计算、MANIFEST.MF 生成）
└── convert.rs     — ProjectConfig → Codex 转换
```

**公开 API**：
```rust
pub fn build(project_path: &Path, options: &BuildOptions) -> Result<PathBuf, BuildError>;
pub fn config_to_codex(config: &ProjectConfig) -> Codex;

pub struct BuildOptions {
    pub output_dir: Option<PathBuf>,    // 覆盖输出目录
    pub output_name: Option<String>,    // 覆盖输出文件名
}

pub enum BuildError { ConfigNotFound, ConfigParse, InvalidStructure, Io, Zip, Serialization }
```

**构建流程**：
1. 读取 `unicodex.toml` → `ProjectConfig`
2. `config_to_codex()` → `Codex`
3. 读取 `content/struct.json` → `Structure`
4. `collect_project_files()` → 递归收集 content/ 和 assets/ 文件
5. 确定输出路径（优先级：options > config > "dist"）
6. `create_ucx_archive()` → 创建 ZIP：
   - mimetype（STORED，不压缩）
   - META-INF/MANIFEST.MF（DEFLATE）
   - metadata/codex.json（DEFLATE）
   - content/struct.json（DEFLATE）
   - 其他内容文件（根据扩展名选择压缩方式）

**压缩策略**：
- STORED（不压缩）：jpg/jpeg/png/webp/gif/mp3/ogg/mp4/pdf/woff2/ttf/otf
- DEFLATE（压缩）：json/md/mdx/ucxc/txt/typ/tex/toml/xml/html/css/js + 未知格式

**MANIFEST.MF 排除列表**：
- mimetype、META-INF/MANIFEST.MF、META-INF/signatures/*、META-INF/certs/*

---

## 4. ucx-verify（验证）

**定位**：验证 UCX 文件的签名和完整性。

**当前状态**：placeholder（Phase 1 中哈希验证功能已在 ucx-parse 中实现）

**验证层次**（待实现）：
1. **文件完整性**：✅ 已在 ucx-parse 的 `verify_hashes()` 中实现
2. **Layer 1 签名**：SF 文件对 MANIFEST.MF 的摘要 → RSA/EC 文件对 SF 的签名
3. **Layer 2 签名**：APK v2 风格的全局签名块
4. **证书链**：签名者证书的信任链验证

**设计考量**：
- 验证应尽最大努力（best-effort），不轻易拒绝
- MAJOR 版本不匹配时警告但仍尝试解析
- 验证结果应返回详细报告（每个文件、每层签名的状态）

---

## 5. ucx-sign（签名）

**定位**：为 UCX 文件生成双层签名。

**当前状态**：placeholder

**Layer 1（JAR 式）**：
- 生成 `{SIGNER}.SF`：对 MANIFEST.MF 各段落的摘要
- 生成 `{SIGNER}.RSA` 或 `{SIGNER}.EC`：对 SF 文件的数字签名 + 证书

**Layer 2（APK v2 式）**：
- 在 ZIP 文件的 Central Directory 之前插入签名块
- 签名覆盖整个 ZIP 内容（防止非 META-INF 文件的篡改）

**设计考量**：
- 支持多签名者（作者 + 出版方 + 平台）
- 签名算法优先 Ed25519，兼容 RSA

---

## 6. ucx-crypto（加密解密）

**定位**：章节级/资源级/段落级加密支持。

**当前状态**：placeholder

**核心功能**：
- UCXE 二进制格式读写
- AES-256-GCM 加密/解密（推荐）
- ChaCha20-Poly1305 加密/解密
- AES-256-CBC + HMAC-SHA256 (Encrypt-then-MAC)
- Argon2id / PBKDF2 密钥派生
- 大文件分块加密（>64 MiB）

**设计考量**：
- IV/Nonce 必须使用 CSPRNG 生成
- 解密后明文应安全清零
- 错误信息不泄露具体失败原因（防 Oracle 攻击）

---

## 7. ucx-version（版本管理）

**定位**：基于 git2 的自动版本号管理。

**当前状态**：placeholder

**核心功能**：
- `detect_changes()`：对比 git tag 和当前 HEAD 的 diff
- `auto_version()`：根据变更类型自动生成 `卷.章.修订` 版本号
- 快照回退：无 Git 时使用 `.ucx-snapshot.json`

**设计考量**：
- Git tag 约定：`ucx-v{X.Y.Z}`
- 章节编号提取基于 `struct.json` 的叶子节点顺序计数
- 版本策略可配置（`unicodex.toml` 中的 `[version]` 段）

---

## 8. ucx-init（项目初始化） ✅ Phase 1 已实现

**定位**：创建新的 UCX 项目目录结构。

**实现状态**：已完成（4 单元测试 + 1 doctest）

**公开 API**：
```rust
pub fn init(path: &Path, options: &InitOptions) -> Result<(), InitError>;

pub struct InitOptions { pub name: String, pub author: String, pub language: String }
pub enum InitError { AlreadyExists, Io, TomlSerialize, JsonSerialize }
```

**初始化流程**：
1. 检查 `unicodex.toml` 是否已存在（已存在则返回 AlreadyExists）
2. 创建项目根目录（如果不存在）
3. 创建标准子目录：content/、assets/、extras/、dist/
4. 生成 UCX ID（UUID v4）
5. 生成 `unicodex.toml`（通过 ProjectConfig 序列化）
6. 生成 `content/struct.json`（一个示例章节节点）
7. 生成 `content/chapter-001.md`（起始章节模板）

**设计考量**：
- 幂等性检查：已存在 unicodex.toml 时立即返回错误，不修改任何文件
- `create_dir_all` 是幂等的，目录已存在时安全
- 默认 build 输出目录为 "dist"

---

## ~~9. 公共类型问题~~（已解决）

~~当前架构中，公共类型（如 `UcxId`、`ManifestEntry`、`Version`）存在归属问题。~~

**已解决**：通过创建 `ucx-types` 公共类型 crate 解决（见 §0）。所有功能模块依赖 `ucx-types`，不互相依赖。

---

*后续开发中将持续更新各模块的设计细节*
