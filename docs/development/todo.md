# Unicodex 后续开发待办事项

> 来源：v0.1.0-alpha.1 / v0.1.0-alpha.2 测试报告
> 更新时间：2026-02-17
> 状态说明：已完成项在 v0.1.0-alpha.2 中修复并验证通过

---

## 已完成项（v0.1.0-alpha.2 已修复）

以下问题在 v0.1.0-alpha.2 中全部修复并通过验证，详见测试报告 `docs/test/problems/0.1.0-alpha.2.md`。

| 编号 | 描述 | 修复提交 |
|------|------|---------|
| NEW-001 | name/author 字段拒绝换行符（\n \r） | `fix(NEW-001)` |
| NEW-002 | 清理冗余 WARN 日志输出 | `fix(NEW-002)` |
| P-002 | 规范文档 Created-By 名称统一为 `unicodex` | `fix(P-002)` |
| P-003 | `ucx init` 自动初始化 Git 仓库 + `.gitignore`（`--no-git` 可跳过） | `fix(P-003)` |
| P-004 | `ucx init` 目录结构精简（默认仅 `content/`，`--full` 完整结构） | `fix(P-004)` |
| P-007 | codex.json `dates` 字段（init 自动 `created`，build 自动 `modified`） | `fix(P-007)` |
| P-009 | unicodex.toml 模板增强（注释示例段） | `fix(P-009)` |
| UX-002 | BCP 47 语言标签验证 | `fix(UX-002)` |
| UX-003 | `ucx init` 非空目录确认提示（`--yes` 跳过） | `fix(UX-003)` |
| UX-006 | `ucx info` 输出增强（文件大小、版本、章节数等） | `fix(UX-006)` |
| UX-007 | `ucx build` 覆盖文件确认提示（`--force` 跳过） | `fix(UX-007)` |
| SEC-002 | 输入长度限制 500 字符（`--allow-long-fields` 覆盖） | `fix(SEC-002)` |
| BUG-002 | 默认值本地化（"无标题"/"未知"） | `fix(BUG-002)` |
| SUG-001 | 交互式 init 模式（`--interactive` / `-i`） | `feat(SUG-001)` |
| SUG-002 | `ucx init --from-existing` 从现有目录初始化 | `feat(SUG-002)` |
| SUG-003 | `ucx info --json` 机器可读输出 | `feat(SUG-003)` |
| SUG-004 | `ucx build --dry-run` 预览打包内容 | `feat(SUG-004)` |
| SUG-005 | `ucx verify` 输出增强（计时、`--verbose`、无签名提示） | `feat(SUG-005)` |
| SUG-006 | `ucx check` 项目校验命令 | `feat(SUG-006)` |
| SUG-007 | chapter-001.md 引导内容增强 | `fix(SUG-007)` |

---

## 仍待开发项

### P-008 codex.json 添加 `file_version` 字段

**来源**：设计标准合规性问题 P-008
**说明**：codex.json 应包含 `file_version` 对象，需要 ucx-version 模块支持。至少在 init 时设置初始版本 `"0.0.0"`，build 时写入 `released_at`。
**优先级**：低
**依赖**：ucx-version 模块
**计划阶段**：Phase 2

---

### SEC-004 签名机制实现

**来源**：安全与健壮性问题 SEC-004
**说明**：完整性保护当前仅依靠 MANIFEST.MF 哈希，签名机制（Layer 1 和 Layer 2）需在 Phase 3 的 ucx-sign 模块中实现。`ucx verify` 已在无签名时提示用户（SUG-005 修复）。
**优先级**：信息（Phase 1 已知限制）
**计划阶段**：Phase 3

---

## Phase 2+ 新功能计划

以下为非 bug-fix 的新功能计划，待后续版本实施：

- `ucx unpack` — 解包 UCX 文件到项目目录
- `ucx version` — 基于 git2 的自动版本号管理
- `ucx init --template` — 模板选择（小说、散文、诗歌等）
- CI/CD 流水线配置

---

*本文件由测试报告生成，于 v0.1.0-alpha.2 修复周期后更新。*
