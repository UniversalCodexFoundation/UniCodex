# Unicodex 后续开发待办事项

> 来源：v0.1.0-alpha.1 / v0.1.0-alpha.2 / v0.2.0-alpha.1 / v0.3.0-alpha.1 测试报告
> 更新时间：2026-04-12
> 状态说明：Phase 3（签名与验证）已在 v0.3.0-alpha.1 中完成

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

## Phase 3 完成项（v0.3.0-alpha.1）

| 编号/功能 | 描述 | 状态 |
|-----------|------|------|
| SEC-004 | 双层签名机制（Layer 1 + Layer 2）完整实现 | ✅ 已完成 |
| ucx-sign | Ed25519 密钥生成、自签名证书、JAR 式 + APK v2 式签名 | ✅ 已完成 |
| ucx-verify | 双层签名验证 + 签名者信息提取 | ✅ 已完成 |
| ucx keygen | CLI 密钥对生成命令 | ✅ 已完成 |
| ucx cert | CLI 证书管理命令（create / info） | ✅ 已完成 |
| ucx sign | CLI 双层签名命令（替换 placeholder） | ✅ 已完成 |
| ucx verify --show-signers | CLI 签名验证升级 | ✅ 已完成 |

---

## 仍待开发项

暂无遗留问题。

---

## Phase 4+ 新功能计划

- `ucx init --template` — 模板选择（小说、散文、诗歌等）
- `ucx encrypt` / `ucx decrypt` — AES-256-GCM / ChaCha20-Poly1305 加密（Phase 4）
- 多语言 SDK — Go, Python, JavaScript（Phase 5）

---

*本文件于 v0.3.0-alpha.1 Phase 3 完成后更新。*
