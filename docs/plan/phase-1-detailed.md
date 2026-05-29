# Phase 1 详细开发计划 — 核心读写（Core I/O）

> 版本：0.1.0
> 创建时间：2026-02-16
> 状态：✅ 已完成
> 目标版本：v0.2.0-alpha.1

## 1. 目标

完成 Phase 0 收尾（公共类型抽取）与 Phase 1 核心读写（ucx-build、ucx-parse、ucx-init），
实现「init → build → parse」完整工作流。

**里程碑 M2 交付物**：可构建/解析简单 UCX 文件。

## 2. 前置条件

- [x] Cargo Workspace 搭建完毕
- [x] 8 个 Rust 模块脚手架创建完毕
- [x] 10 份规范文档（v0.0.0-alpha.1）完成
- [x] `cargo check` 通过

## 3. 开发任务分解

### 3.1 Phase 0 收尾：ucx-types 公共类型 crate（前置，串行）

**目的**：解决多模块共享类型的归属问题（ADR-001 遗留），
所有模块共用的数据结构统一定义在 `ucx-types` 中。

**新增 crate**：`ucx-types`

**内容**：

| 类型 | 来源规范 | 说明 |
|------|---------|------|
| `UcxId` | 02-metadata §2.2 | UCX 唯一标识 `urn:ucx:{UUID}` |
| `Codex` | 02-metadata §2.1 | 作品核心元数据（codex.json 对应结构） |
| `Identifier` | 02-metadata §2.2 | 标识符（ucx_id, isbn, custom...） |
| `Title` | 02-metadata §2.2 | 标题（main, subtitle, original, short） |
| `Series` | 02-metadata §2.2 | 系列信息 |
| `Creator` | 02-metadata §2.2 | 创作者 |
| `Publisher` | 02-metadata §2.2 | 出版方 |
| `Description` | 02-metadata §2.2 | 作品简介 |
| `Rights` | 02-metadata §2.2 | 版权信息 |
| `Dates` | 02-metadata §2.2 | 日期信息 |
| `Rating` | 02-metadata §2.2 | 分级信息 |
| `Structure` | 01-file-structure §4.1 | 内容结构定义（struct.json 对应结构） |
| `StructureNode` | 01-file-structure §4.1 | 结构节点（支持无限嵌套） |
| `Encryption` | 01-file-structure §4.1.1 | 加密配置对象 |
| `KeyAccess` | 01-file-structure §4.1.1 | 密钥获取方式 |
| `Manifest` | 01-file-structure §3.2 | MANIFEST.MF 结构 |
| `ManifestEntry` | 01-file-structure §3.2 | 清单条目 |
| `HashAlgorithm` | 01-file-structure §7 | 哈希算法枚举 |
| `ProjectConfig` | 02-metadata §5.2 | unicodex.toml 配置结构 |

**新增依赖**：
- `uuid = { version = "1", features = ["v4"] }` — UUID v4 生成
- `base64 = "0.22"` — Base64 编码/解码（MANIFEST.MF 哈希值）

**实现要点**：
- 所有类型实现 `Serialize` / `Deserialize`
- `Codex` 的 JSON 序列化必须与 codex.json 规范完全匹配
- `Structure` 的 JSON 序列化必须与 struct.json 规范完全匹配
- `Manifest` 需实现自定义 RFC 822 风格序列化/反序列化
- `ProjectConfig` 用于解析 unicodex.toml

### 3.2 ucx-init 模块实现

**依赖**：ucx-types

**功能清单**：

1. **目录结构创建**
   - `content/` — 正文目录
   - `assets/` — 资源目录
   - `extras/` — 额外内容（不打包）
   - `dist/` — 输出目录

2. **文件生成**
   - `unicodex.toml` — 项目配置模板
   - `content/struct.json` — 初始结构（含一个示例章节）
   - `content/chapter-001.md` — 示例章节

3. **UCX ID 生成**
   - UUID v4 随机生成
   - 格式：`urn:ucx:{uuid}`

4. **API 设计**
   ```rust
   pub struct InitOptions {
       pub name: String,          // 项目名称
       pub author: String,        // 作者名称
       pub language: String,      // 语言（默认 zh-CN）
   }

   pub fn init(path: &Path, options: &InitOptions) -> Result<(), InitError>;
   ```

### 3.3 ucx-build 模块实现

**依赖**：ucx-types, blake3, zip, serde_json, toml

**功能清单**：

1. **unicodex.toml 解析**
   - 读取项目配置
   - 验证必需字段

2. **TOML → JSON 转换**
   - `unicodex.toml` → `metadata/codex.json`
   - 收集 content/ 目录生成 `content/struct.json`

3. **MANIFEST.MF 生成**
   - 遍历所有待打包文件
   - 计算 BLAKE3 哈希
   - 生成 RFC 822 风格的 MANIFEST.MF

4. **ZIP 归档创建**
   - `mimetype` 为第一个条目（STORED，不压缩）
   - 文本文件使用 DEFLATE 压缩
   - 已压缩格式（jpg/png/mp3/pdf）使用 STORED
   - 遵循压缩策略表（01-file-structure §10）

5. **API 设计**
   ```rust
   pub struct BuildOptions {
       pub output_dir: Option<PathBuf>,    // 输出目录（默认 dist/）
       pub output_name: Option<String>,     // 输出文件名
   }

   pub fn build(project_path: &Path, options: &BuildOptions) -> Result<PathBuf, BuildError>;
   ```

### 3.4 ucx-parse 模块实现

**依赖**：ucx-types, blake3, zip, serde_json

**功能清单**：

1. **ZIP 归档打开**
   - 使用 `zip` crate 打开文件
   - 基础格式验证

2. **mimetype 验证**
   - 检查第一个条目为 `mimetype`
   - 内容为 `application/vnd.unicodex+zip`

3. **MANIFEST.MF 解析**
   - RFC 822 风格键值对解析
   - 提取所有条目的文件名、大小、哈希值

4. **元数据解析**
   - `metadata/codex.json` → `Codex` 结构
   - 验证必需字段

5. **结构解析**
   - `content/struct.json` → `Structure` 结构

6. **章节提取**
   - 根据 struct.json 中的文件路径提取内容
   - 返回章节原始文本

7. **哈希验证**
   - 对每个文件计算 BLAKE3 哈希
   - 与 MANIFEST.MF 中的摘要对比

8. **API 设计**
   ```rust
   pub struct UcxArchive { ... }

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

### 3.5 CLI 集成

**涉及模块**：unicodex-core

- `ucx init <path>` → 调用 ucx-init
- `ucx build [--output <dir>]` → 调用 ucx-build
- `ucx info <file>` → 调用 ucx-parse，显示元数据
- `ucx unpack <file> [--output <dir>]` → 调用 ucx-parse，解包文件

### 3.6 集成测试

- **round-trip 测试**：`init` → 编辑 → `build` → `parse` → 验证内容一致
- **MANIFEST.MF 哈希验证**：构建后解析，验证所有哈希匹配
- **结构一致性**：build 输出的 struct.json 与 parse 读取的一致
- **压缩策略验证**：检查 mimetype 为 STORED，文本文件为 DEFLATED

## 4. 任务依赖关系

```
ucx-types ──┬──→ ucx-init ──────────────────┐
            ├──→ ucx-build ──────────────────┤──→ CLI 集成 ──→ 集成测试
            └──→ ucx-parse ──────────────────┘
```

**关键路径**：ucx-types → ucx-build/ucx-parse → CLI 集成 → 集成测试

## 5. 新增依赖

| crate | 版本 | 用途 | 使用模块 |
|-------|------|------|---------|
| `uuid` | 1 (features: v4) | UUID 生成 | ucx-types, ucx-init |
| `base64` | 0.22 | Base64 编解码 | ucx-types |
| `walkdir` | 2 | 递归遍历目录 | ucx-build |

## 6. 蜂群分工

| 角色 | 职责 | 并行阶段 |
|------|------|---------|
| Lead | ucx-types 开发 + 协调 + 集成 | 串行（Phase 0 收尾） |
| init-dev | ucx-init 完整实现 | 并行 |
| build-dev | ucx-build 完整实现 | 并行 |
| parse-dev | ucx-parse 完整实现 | 并行 |

## 7. 验收标准

- [x] `cargo check` 通过（所有模块无编译错误）
- [x] `cargo test` 通过（所有单元测试 + 集成测试）— 69 tests 全部通过
- [x] `ucx init test-project` 生成正确的目录结构和配置文件
- [x] `ucx build` 从项目目录生成有效的 `.ucx` 文件
- [x] `ucx info <file>.ucx` 正确显示元数据
- [x] round-trip 测试通过（init → build → parse 内容一致）
- [x] MANIFEST.MF 哈希验证通过
- [x] 代码覆盖：每个公开函数至少有一个测试
- [x] 所有代码符合项目注释规范（中英双语）

---

*本计划由蜂群团队执行，执行过程中持续更新 `docs/memory/progress.md`*
