# 模块设计笔记

> 创建时间：2026-02-16
> 状态：持续更新

本文档记录各 Rust 模块的设计思路、接口草案和技术备忘。

## 1. unicodex-core（CLI 入口）

**定位**：仅作为 CLI 入口，不包含业务逻辑。编译产物为 `ucx` 命令。

**设计思路**：
- 使用 `clap` derive 宏定义子命令结构
- 每个子命令代理到对应模块的公开 API
- 错误处理使用 `anyhow`，将各模块的 `thiserror` 错误统一转换
- 日志初始化在 main.rs 中完成（`tracing-subscriber`）

**子命令映射**：
```
ucx init       → ucx-init
ucx build      → ucx-build
ucx pack       → ucx-build (别名)
ucx unpack     → ucx-parse
ucx info       → ucx-parse
ucx verify     → ucx-verify
ucx sign       → ucx-sign
ucx cert       → ucx-sign
ucx version    → ucx-version
ucx encrypt    → ucx-crypto
ucx decrypt    → ucx-crypto
ucx chain      → 预留（Phase 4）
```

---

## 2. ucx-parse（解析读取）

**定位**：UCX 文件的解析和内容提取，是阅读器集成的核心模块。

**核心类型草案**：
```rust
/// A parsed UCX archive ready for content extraction.
pub struct UcxArchive { ... }

/// Parsed codex.json metadata.
pub struct Codex { ... }

/// Parsed struct.json structure tree.
pub struct Structure { ... }

/// A single chapter entry with its content.
pub struct Chapter { ... }
```

**关键接口草案**：
```rust
/// Open and parse a UCX file.
pub fn open(path: &Path) -> Result<UcxArchive, ParseError>;

/// Extract all chapters from the archive.
pub fn chapters(archive: &UcxArchive) -> Result<Vec<Chapter>, ParseError>;

/// Get metadata from the archive.
pub fn metadata(archive: &UcxArchive) -> Result<Codex, ParseError>;
```

**设计考量**：
- `UcxArchive` 应持有 ZIP reader，支持按需读取（不一次性解压全部）
- MANIFEST.MF 解析需支持 RFC 822 风格的键值对格式
- 需处理加密文件的检测（Magic Number `UCXE`）

---

## 3. ucx-build（构建打包）

**定位**：将项目目录打包为 `.ucx` 文件。

**核心流程**：
1. 读取 `unicodex.toml` 项目配置
2. TOML 元数据 → JSON 转换（生成 codex.json、struct.json 等）
3. 收集所有内容文件和资源
4. 生成 MANIFEST.MF（计算所有文件的 BLAKE3 哈希）
5. 创建 ZIP 归档（mimetype 为第一个条目，不压缩）
6. 可选：自动执行 `ucx version auto`
7. 可选：自动签名（依赖 ucx-sign）

**设计考量**：
- mimetype 文件必须是 ZIP 归档的第一个条目，且不使用压缩
- MANIFEST.MF 格式遵循 JAR 规范（RFC 822 风格）
- 构建前需验证目录结构完整性

---

## 4. ucx-verify（验证）

**定位**：验证 UCX 文件的签名和完整性。

**验证层次**：
1. **文件完整性**：MANIFEST.MF 中的哈希 vs 实际文件哈希
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

**核心功能**：
- `detect_changes()`：对比 git tag 和当前 HEAD 的 diff
- `auto_version()`：根据变更类型自动生成 `卷.章.修订` 版本号
- 快照回退：无 Git 时使用 `.ucx-snapshot.json`

**设计考量**：
- Git tag 约定：`ucx-v{X.Y.Z}`
- 章节编号提取基于 `struct.json` 的叶子节点顺序计数
- 版本策略可配置（`unicodex.toml` 中的 `[version]` 段）

---

## 8. ucx-init（项目初始化）

**定位**：创建新的 UCX 项目目录结构。

**核心功能**：
- 生成 `unicodex.toml` 配置文件
- 创建标准目录结构（metadata/、content/、assets/ 等）
- 生成示例 codex.json 和 struct.json
- 分配 UCX ID（UUID v4）

**设计考量**：
- 支持交互式和非交互式两种模式
- 模板可自定义扩展

---

## 9. 公共类型问题

当前架构中，公共类型（如 `UcxId`、`ManifestEntry`、`Version`）存在归属问题：
- 多个模块可能需要引用相同的类型定义
- 如果放在某个功能模块中，会引入不必要的依赖

**待决定**：
- 是否需要抽取 `ucx-types` 或 `ucx-common` 公共类型 crate
- 暂时先在各模块中独立定义，待重复类型积累后再统一抽取

---

*后续开发中将持续更新各模块的设计细节*
