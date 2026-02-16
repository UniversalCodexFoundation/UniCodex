# Unicodex 后续开发待办事项

> 来源：v0.1.0-alpha.1 测试报告 (`docs/test/problems/0.1.0-alpha.1.md`)
> 记录时间：2026-02-16
> 状态说明：以下问题超出 Phase 1 范围，纳入后续阶段计划

---

## Phase 2 计划项

### P-002 规范文档修复：Created-By 名称统一

**来源**：设计标准合规性问题 P-002
**说明**：MANIFEST.MF 中 `Created-By` 字段，代码使用 `unicodex`，规范文档示例使用 `unicodex-cli`。开发者确认代码正确，需要更新规范文档 `01-file-structure.md` 和 `07-versioning.md` 中的示例。
**优先级**：中等
**计划阶段**：Phase 2（规范文档修订）

### P-003 `ucx init` 自动初始化 Git 仓库

**来源**：设计标准合规性问题 P-003
**说明**：`ucx init` 应调用 `git2::Repository::init()` 自动初始化 Git 仓库，生成 `.gitignore`（至少包含 `dist/`），并提供 `--no-git` 选项跳过。
**优先级**：中等
**依赖**：ucx-version 模块（git2 集成）
**计划阶段**：Phase 2

### P-004 `ucx init` 目录结构优化

**来源**：设计标准合规性问题 P-004
**说明**：减少 `ucx init` 创建的冗余目录。最简初始化只创建 `content/`，`dist/` 由 `ucx build` 按需创建。可提供 `--full` 和 `--template` 选项。
**优先级**：中等
**计划阶段**：Phase 2

### P-007 codex.json 添加 `dates` 字段

**来源**：设计标准合规性问题 P-007
**说明**：`ucx init` 应自动填入 `dates.created`，`ucx build` 应自动更新 `dates.modified`。需要在 `ProjectConfig` 和 `Codex` 类型中添加 dates 相关字段。
**优先级**：低
**计划阶段**：Phase 2

### P-008 codex.json 添加 `file_version` 字段

**来源**：设计标准合规性问题 P-008
**说明**：codex.json 应包含 `file_version` 对象，需要 ucx-version 模块支持。至少在 init 时设置初始版本 `"0.0.0"`，build 时写入 `released_at`。
**优先级**：低
**依赖**：ucx-version 模块
**计划阶段**：Phase 2

### P-009 unicodex.toml 模板增强

**来源**：设计标准合规性问题 P-009
**说明**：`ucx init` 生成的模板应包含被注释掉的可选段示例（`[description]`、`[rights]`、`[cover]`、`[rating]`、`[dates]` 等），帮助用户了解所有可用配置项。
**优先级**：信息
**计划阶段**：Phase 2

### UX-002 BCP 47 语言标签验证

**来源**：用户体验问题 UX-002
**说明**：对 `--language` 参数进行基本的 BCP 47 格式验证。可使用 `icu` crate 或手动校验 `xx` / `xx-XX` 格式。
**优先级**：中等
**计划阶段**：Phase 2

### UX-003 `ucx init` 默认目录确认提示

**来源**：用户体验问题 UX-003
**说明**：当 `PATH` 为默认值 `.`（当前目录）且当前目录非空时，提示用户确认。或在初始化成功后显示清晰的提示。
**优先级**：中等
**计划阶段**：Phase 2

### UX-006 `ucx info` 输出增强

**来源**：用户体验问题 UX-006
**说明**：`ucx info` 添加文件大小、UCX 规范版本、构建工具版本、章节总数、字数统计、作品状态等信息。
**优先级**：低
**计划阶段**：Phase 2

### UX-007 `ucx build` 覆盖提示

**来源**：用户体验问题 UX-007
**说明**：覆盖已有 `.ucx` 文件时显示简短提示，或提供 `--force` 标志用于静默覆盖。
**优先级**：信息
**计划阶段**：Phase 2

### SEC-002 超长输入限制

**来源**：安全与健壮性问题 SEC-002
**说明**：对 `--name`、`--author` 等参数设置合理长度上限。开发者批注建议保留超长兼容性，通过 `--name-long` / `--author-long` 参数启用，需补充到标准文档。
**优先级**：中等
**计划阶段**：Phase 2（需同步更新规范文档）

### BUG-002 默认值本地化

**来源**：功能缺陷 BUG-002
**说明**：`ucx init` 的 `--name` 默认值 "Untitled" 和 `--author` 默认值 "Unknown" 考虑根据默认语言本地化。
**优先级**：低
**计划阶段**：Phase 2

---

## Phase 3+ 计划项

### SEC-004 签名机制实现

**来源**：安全与健壮性问题 SEC-004
**说明**：完整性保护当前仅依靠 MANIFEST.MF 哈希，签名机制（Layer 1 和 Layer 2）需在 Phase 3 的 ucx-sign 模块中实现。`ucx verify` 应在无签名时提示用户。
**优先级**：信息（Phase 1 已知限制）
**计划阶段**：Phase 3

---

## 建议性改进

### SUG-001 交互式 init 模式

`ucx init --interactive` — 交互式创建，引导用户输入书名、作者、语言、体裁等信息。
**计划阶段**：Phase 2

### SUG-002 `ucx init --from-existing`

从现有目录初始化 UCX 项目。
**计划阶段**：Phase 2

### SUG-003 `ucx info --json`

添加 `--json` 标志以机器可读格式输出信息。
**计划阶段**：Phase 2

### SUG-004 `ucx build --dry-run`

仅展示将要打包的文件列表和输出路径，不实际创建文件。
**计划阶段**：Phase 2

### SUG-005 `ucx verify` 输出增强

无签名时明确提示，显示验证耗时，支持 `--verbose` 显示详细哈希。
**计划阶段**：Phase 2-3

### SUG-006 `ucx check` 命令

独立的项目检查命令，验证 unicodex.toml 格式、struct.json 完整性、BCP 47 标签等。
**计划阶段**：Phase 2

### SUG-007 chapter-001.md 内容引导

生成的示例章节包含更多引导信息（格式说明、Markdown 语法提示等）。
**计划阶段**：Phase 2

---

*本文件由 v0.1.0-alpha.1 测试报告自动生成，随开发进度持续更新。*
