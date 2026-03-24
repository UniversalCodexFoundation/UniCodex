# Unicodex 后续开发待办事项

> 来源：v0.1.0-alpha.1 / v0.1.0-alpha.2 / v0.2.0-alpha.1 测试报告
> 更新时间：2026-03-25
> 状态说明：Phase 2（版本管理）已在 v0.2.0-alpha.1 中完成

---

## Phase 2 完成项（v0.2.0-alpha.1）

| 编号/功能 | 描述 | 状态 |
|-----------|------|------|
| P-008 | codex.json `file_version` 字段支持 | ✅ 已完成 |
| ucx-version | 基于 git2 的变更检测 + snapshot 回退 | ✅ 已完成 |
| ucx version auto | 自动检测变更并升级版本号 | ✅ 已完成 |
| ucx version patch/chapter/volume/set | 手动版本管理命令 | ✅ 已完成 |
| auto_on_build | 构建时自动版本检测（[version] 配置） | ✅ 已完成 |
| .ucx-snapshot.json | 构建后自动更新文件快照 | ✅ 已完成 |

---

## 仍待开发项

### SEC-004 签名机制实现

**来源**：安全与健壮性问题 SEC-004
**说明**：完整性保护当前仅依靠 MANIFEST.MF 哈希，签名机制（Layer 1 和 Layer 2）需在 Phase 3 的 ucx-sign 模块中实现。`ucx verify` 已在无签名时提示用户（SUG-005 修复）。
**优先级**：信息（Phase 1 已知限制）
**计划阶段**：Phase 3

---

## Phase 3+ 新功能计划

- `ucx unpack` — 解包 UCX 文件到项目目录（Phase 2 延后）
- `ucx init --template` — 模板选择（小说、散文、诗歌等）
- `ucx sign` / `ucx verify` 签名 — JAR 风格三层签名 + APK v2（Phase 3）
- `ucx encrypt` / `ucx decrypt` — AES-256-GCM / ChaCha20-Poly1305 加密（Phase 4）
- 多语言 SDK — Go, Python, JavaScript（Phase 5）

---

*本文件于 v0.2.0-alpha.1 Phase 2 完成后更新。*
