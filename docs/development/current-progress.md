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

*后续开发进度将追加在此文档中*
