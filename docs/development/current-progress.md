# 开发进度记录

> 创建时间：2026-02-16
> 状态：持续更新

本文档记录 Unicodex 项目的开发进度和关键事件，确保项目状态可追溯。

## 时间线

### 2026-02-16 — 项目启动

**完成事项**：

1. **规范文档完成**（v0.0.0-alpha.1）
   - `00-overview.md` — 项目概述 + 开发规范
   - `01-file-structure.md` — 文件结构规范
   - `02-metadata-spec.md` — 元数据规范
   - `03-content-format.md` — 正文内容格式（.ucxc/.mdx/.md/.txt/.typ）
   - `04-crypto-spec.md` — 加密规范（AES-256-GCM / ChaCha20-Poly1305）
   - `05-signature-spec.md` — 签名机制规范（双层签名体系）
   - `06-blockchain-ext.md` — 区块链溯源规范
   - `07-versioning.md` — 版本控制规范（git2 + 卷.章.修订）
   - `08-keys-identity.md` — 密钥与身份验证规范
   - `09-official-services.md` — 官方服务与服务商规范

2. **Git 仓库初始化**
   - 创建 `.gitignore`
   - 首次提交并标记 `v0.0.0-alpha.1`

3. **项目基础设施搭建**
   - Cargo Workspace 配置
   - 8 个 Rust 模块脚手架创建
   - 开发计划总览（`docs/plan/roadmap.md`）
   - 开发文档目录（`docs/development/`）

**已知问题**：
- ~~`01-file-structure.md` 和 `05-signature-spec.md` 中可能存在 `metadata/book.json` 的旧引用~~ — 已确认不存在
- ~~`07-versioning.md` 和 `00-overview.md` 中残留 `ucx-cli` 引用~~ — 已修复
- ~~公共类型（UCX ID、Manifest 等）尚未确定归属模块~~ — 已通过 ucx-types crate 解决（ADR-001）
- 扩展模块（ucx-audio-book、ucx-interactive）暂时注释，待后续阶段启用

**下一步**：
- ~~Phase 0 收尾：确保 `cargo check` 通过，设置 CI~~ — 已完成
- ~~开始 Phase 1：核心 I/O 模块开发~~ — 已完成

---

### 2026-02-16 — Phase 1 完成：核心 I/O 模块

**完成事项**：

1. **ucx-types 公共类型 crate**（新建）
   - 解决 ADR-001 中识别的类型归属问题
   - 5 个子模块：`codex`、`manifest`、`structure`、`project`、`ucx_id`
   - 核心类型：`Codex`、`Manifest`、`ManifestEntry`、`HashAlgorithm`、`Structure`、`StructureNode`、`ProjectConfig`、`UcxId`
   - MANIFEST.MF 的 RFC 822 格式序列化/反序列化
   - 30 个单元测试 + 1 个 doctest

2. **ucx-init 项目初始化模块**（实现）
   - `init(path, options)` → 创建标准目录结构 + 生成 unicodex.toml + struct.json + 示例章节
   - `InitOptions`：name、author、language 参数
   - UCX ID 自动生成（UUID v4）
   - 4 个单元测试 + 1 个 doctest

3. **ucx-build 构建打包模块**（实现）
   - `build(project_path, options)` → 读取 unicodex.toml → 转换 Codex → 收集文件 → 计算 BLAKE3 哈希 → 生成 MANIFEST.MF → 创建 ZIP
   - `BuildOptions`：output_dir、output_name 覆盖
   - `config_to_codex()` — ProjectConfig → Codex 映射转换
   - 智能压缩策略（已压缩格式 STORED，文本格式 DEFLATE）
   - ZIP 条目顺序：mimetype(STORED) → MANIFEST.MF → codex.json → struct.json → 内容文件
   - 12 个单元测试 + 1 个 doctest

4. **ucx-parse 解析读取模块**（实现）
   - `open(path)` → 打开 ZIP → 验证 mimetype → 解析 MANIFEST.MF/codex.json/struct.json
   - `UcxArchive` 结构体：codex()、structure()、manifest()、read_chapter()、verify_hashes()、list_files()
   - BLAKE3 哈希完整性验证
   - 6 个单元测试

5. **CLI 集成**（unicodex-core/src/main.rs）
   - `ucx init <path> --name --author --language` — 调用 ucx-init
   - `ucx build <path> --output-dir --output-name` — 调用 ucx-build
   - `ucx info <file>` — 显示元数据、结构树、清单摘要
   - `ucx verify <file>` — BLAKE3 哈希完整性验证

6. **集成测试**（unicodex-core/tests/round_trip.rs）
   - 完整往返测试：init → build → parse → read_chapter → verify_hashes
   - 自定义输出选项测试
   - 多章节项目测试（3 章）
   - 二次初始化幂等性测试
   - 4 个集成测试

**测试汇总**：69 tests（62 unit + 4 integration + 3 doctests），全部通过

**关键技术决策**：
- ADR-001 落地：ucx-types 作为共享类型 crate，所有模块依赖它而非互相依赖
- ADR-002 落地：ucx-build（写）和 ucx-parse（读）完全分离
- MANIFEST.MF 使用 BLAKE3 哈希（规范推荐），hex 编码
- ZIP 中 mimetype 必须为第一个条目且使用 STORED 压缩

**下一步**：
- Phase 2：ucx-verify（签名验证）、ucx-version（版本管理）
- Phase 3：ucx-sign（双层签名）、ucx-crypto（加密解密）
- 设置 CI/CD 流水线

---

### 2026-02-17 — v0.1.0-alpha.2 修复：20 项问题修复

**完成事项**：

基于 v0.1.0-alpha.1 和 v0.1.0-alpha.2 测试报告，完成了 20 项修复（2 项新发现 + 18 项遗留问题）。
排除 P-008（依赖 ucx-version 模块）和 SEC-004（依赖签名机制）。

1. **ucx-init 模块增强**（10 项修复）
   - NEW-001：name/author 字段拒绝换行符（`\n`/`\r`），仅保留 TAB 例外
   - BUG-002：默认值本地化（"Untitled"→"无标题"、"Unknown"→"未知"）
   - SEC-002：输入长度限制 500 字符，`--allow-long-fields` 可覆盖
   - UX-002：BCP 47 语言标签基本格式验证
   - P-004：目录结构精简（默认仅 `content/`，`--full` 创建完整结构）
   - SUG-007：chapter-001.md 引导内容增强（UCX 提示注释块）
   - P-009：unicodex.toml 模板增强（注释示例段）
   - P-007：codex.json `dates` 字段（init 自动 `created`，build 自动 `modified`）
   - P-003：Git 仓库自动初始化 + `.gitignore` 生成（`--no-git` 可跳过）
   - `InitOptions` 从 3 个字段扩展为 6 个（`allow_long_fields`、`full`、`no_git`）
   - 新增 `init_from_existing()` 公开函数（SUG-002）
   - 新增 `validate_language_tag()` 内部函数

2. **ucx-build 模块增强**（4 项修复）
   - NEW-002：清理 `validate_file_references()` 中冗余 WARN 日志
   - SUG-004：`dry_run()` 公开函数 + `DryRunResult`/`DryRunFile` 类型
   - SUG-006：`check()` 公开函数 + `CheckResult`/`CheckItem` 类型（8 项校验）
   - UX-007：`resolve_output_path()` 公开函数（用于 CLI 覆盖检查）
   - `BuildOptions` 新增 `dry_run: bool` 字段
   - 新增 `is_valid_bcp47()` 内部辅助函数

3. **ucx-parse 模块增强**（1 项修复）
   - UX-006：`UcxArchive` 新增 `file_path` 字段、`file_size()` 和 `chapter_count()` 方法

4. **CLI 增强**（7 项修复）
   - UX-003：非空目录确认提示，`--yes`/`-y` 跳过
   - SUG-001：`--interactive`/`-i` 交互式 init 模式
   - SUG-002：`--from-existing` 从现有 .md 文件初始化
   - UX-007：build 覆盖确认，`--force`/`-f` 跳过
   - SUG-003：`ucx info --json` 机器可读 JSON 输出
   - SUG-005：`ucx verify --verbose` 详细哈希 + 计时 + 无签名提示
   - SUG-006：`ucx check` 新子命令
   - tracing subscriber 添加 `.without_time().with_target(false)`

5. **规范文档修复**（1 项修复）
   - P-002：`01-file-structure.md` 和 `05-signature-spec.md` 中 `unicodex-cli` → `unicodex`

6. **新增依赖**
   - `chrono`：日期格式化（ucx-init、ucx-build）
   - `git2`：Git 仓库初始化（ucx-init）
   - `serde_json`：CLI JSON 输出（unicodex-core 运行时依赖）

**测试汇总**：76 tests（全部通过），release 构建成功

**遗留问题**：
- P-008：codex.json `file_version` 字段（依赖 ucx-version 模块，Phase 2）
- SEC-004：签名机制（Layer 1 + Layer 2，Phase 3）

**下一步**：
- Phase 2：ucx-version（版本管理）、ucx-verify（签名验证逻辑）、P-008
- Phase 3：ucx-sign（双层签名）、SEC-004
- CI/CD 流水线配置

---

*后续开发进度将追加在此文档中*
