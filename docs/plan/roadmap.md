# Unicodex 长期开发计划总览

> 版本：0.4.0
> 创建时间：2026-02-16
> 最后更新：2026-04-18
> 状态：Phase 0-4 已完成，Phase 5（生态建设）规划中

本文件是**前瞻计划**的单一真实来源（SSOT）：阶段划分、依赖关系、里程碑、技术栈、规范-模块映射。
- 逐版本编年史（“何时发布了什么”）见 [../memory/progress.md]
- 模块设计 / API / 实现状态见 [../memory/modules.md]
- 决策原因（ADR）见 [../memory/decisions.md]
- 未完成行动项见 [../TODO.md]

## 1. 目标

将 Unicodex 规范从文档设计推进到可用的参考实现，最终形成完整的工具链和生态系统。

## 2. 阶段总览

| 阶段 | 主题 | 状态 | 版本 |
|------|------|------|------|
| Phase 0 | 基础设施（Foundation） | ✅ 已完成 | v0.1.0-alpha.1 |
| Phase 1 | 核心读写（Core I/O） | ✅ 已完成 | v0.1.0-alpha.2 |
| Phase 2 | 完善与版本管理（Polish & Versioning） | ✅ 已完成 | v0.2.0-alpha.1 |
| Phase 3 | 签名与验证（Signature & Verification） | ✅ 已完成 | v0.3.0-alpha.1 |
| Phase 4 | 加密体系（Encryption） | ✅ 已完成 | v0.4.0-alpha.1 → alpha.2 |
| Phase 5 | 生态建设（Ecosystem） | 📋 规划中 | — |
| 区块链 | 区块链溯源（Blockchain Traceability） | 远期 | — |

> 各阶段“做了什么 / 何时做的”见 [../memory/progress.md]；模块当前 API 与状态见 [../memory/modules.md]。
> 区块链溯源作为独立扩展层设计，不与核心功能耦合，规范见 [../06-blockchain-ext.md]，将在核心工具链稳定后视需求推进。

## 3. 阶段依赖关系

```
Phase 0 ──→ Phase 1 ──→ Phase 2 ──→ Phase 3 ──→ Phase 4 ──→ Phase 5
  ✅          ✅          ✅          ✅          ✅          📋
                                                   ┆
                                          （远期）区块链溯源
```

**关键路径**：Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 4

- Phase 3（签名）与 Phase 4（加密）存在技术关联，可部分并行。
- Phase 5（生态建设）依赖完整的签名/加密体系，SDK 需覆盖全部功能后启动。
- 区块链溯源不在关键路径上，作为可选扩展独立演进。

## 4. 当前状态

| 阶段 | 状态 | 版本 | 说明 |
|------|------|------|------|
| 规范设计 | **已完成** | v0.0.0-alpha.1 | 10 份规范文档（00-09） |
| Phase 0 | **已完成** | v0.1.0-alpha.1 | Workspace + ucx-types + 模块脚手架 |
| Phase 1 | **已完成** | v0.1.0-alpha.2 | init/build/parse + CLI |
| Phase 2 | **已完成** | v0.2.0-alpha.1 | 版本管理 + CLI 增强 |
| Phase 3 | **已完成** | v0.3.0-alpha.1 | 双层签名 + 证书管理 + 完整验证 |
| Phase 4 | **已完成** | v0.4.0-alpha.1 → **alpha.2（安全加固轮，回归 26/28 FIXED）** | ucx-crypto 加密模块 + CLI 命令 + 构建/解析集成 |
| Phase 5 | 规划中 | — | 规范 08/09 已就绪 |
| 区块链 | 远期计划 | — | 规范 06-blockchain-ext.md 已就绪 |

> 测试总数、回归明细、各 crate 状态见 [../memory/progress.md] 与 [../memory/modules.md]。

## 5. 里程碑规划

| 里程碑 | 对应阶段 | 版本 | 交付物 |
|--------|---------|------|--------|
| M0 - 规范完成 | 规范设计 | v0.0.0-alpha.1 | 规范文档集 |
| M1 - 脚手架就绪 | Phase 0 | v0.1.0-alpha.1 | Workspace + ucx-types + 模块脚手架 |
| M2 - 基本读写 | Phase 1 | v0.1.0-alpha.2 | init → build → parse 完整工作流 |
| M3 - 完善工具链 | Phase 2 | v0.2.0-alpha.1 | 版本管理 + CLI 增强 + ucx check |
| M4 - 签名验证 | Phase 3 | v0.3.0-alpha.1 | 双层签名 + 证书链 + 完整验证 |
| M5 - 加密支持 | Phase 4 | v0.4.0-alpha.2（当前止于此） | 章节/段落加密 + 多密钥方式；进入 beta 待评估 |
| M6 - 1.0 发布 | Phase 5 | v1.0.0（全功能完成的终点，见下约束） | 稳定版本 + 多语言 SDK + 服务 |

> 注：git tag 已回补完整（v0.0.0-alpha.1 → v0.4.0-alpha.2，共 7 个 annotated）。
> **版本约束**：全功能彻底完成前版本号一律 `v0.x.x`，**禁止 `v1.0.0`**（见 [../memory/decisions.md] ADR-011）；M6 是唯一进入 v1.0.0 的节点。

## 6. 规范文档与模块对照

| 规范文档 | 涉及模块 | 阶段 |
|---------|---------|------|
| 00-overview.md | — | 总览 |
| 01-file-structure.md | ucx-build, ucx-parse | Phase 1 ✅ |
| 02-metadata-spec.md | ucx-types, ucx-init | Phase 1 ✅ / Phase 2 补全 |
| 03-content-format.md | ucx-build, ucx-parse | Phase 1 ✅ / Phase 2 增强 |
| 04-crypto-spec.md | ucx-crypto | Phase 4 ✅ |
| 05-signature-spec.md | ucx-sign, ucx-verify | Phase 3 ✅ |
| 06-blockchain-ext.md | （未来扩展） | 远期计划 |
| 07-versioning.md | ucx-version | Phase 2 ✅ |
| 08-keys-identity.md | ucx-sign, ucx-crypto | Phase 3-4 ✅ |
| 09-official-services.md | （服务端） | Phase 5 |

## 7. 技术栈

| 领域 | 技术选型 | 说明 |
|------|---------|------|
| 主要语言 | Rust (edition 2024) | 所有核心模块，rust-version 1.85，resolver 3 |
| 哈希算法 | BLAKE3 (`blake3`) | 高性能，推荐默认，摘要 Base64 编码 |
| 签名算法 | Ed25519 (`ed25519-dalek`) | 推荐默认 |
| 加密算法 | AES-256-GCM / ChaCha20-Poly1305 | 章节/段落级 AEAD（Phase 4） |
| 密钥派生 | Argon2id / PBKDF2 | passphrase 进入 KDF 前统一 NFC 归一化 |
| ZIP 操作 | `zip` crate | UCX 容器格式 |
| 版本管理 | `git2` (libgit2) | 变更检测与自动版本号 |
| 序列化 | `serde` + `serde_json` + `toml` | 元数据处理 |
| 错误处理 | `thiserror` (库) / `anyhow` (CLI) | 分层错误模型 |
| CLI 框架 | `clap` (derive) | 命令行解析 |
| 日志 | `tracing` + `tracing-subscriber` | 结构化日志，默认 WARN 级别 |
| 编码 | `base64` | MANIFEST.MF 摘要编码 |

## 8. Workspace 模块架构（9 crate）

```
unicodex/
├── unicodex-core/    ← CLI 入口（bin = ucx，12 个顶层子命令）
├── ucx-types/        ← 公共类型（Codex, Structure, Manifest, ProjectConfig 等）
├── ucx-init/         ← 项目初始化（Phase 1-2）
├── ucx-build/        ← 构建打包（Phase 1-2）
├── ucx-parse/        ← 解析读取（Phase 1-2）
├── ucx-version/      ← 版本管理（Phase 2）
├── ucx-sign/         ← 签名与证书管理（Phase 3）
├── ucx-verify/       ← 签名与完整性验证（Phase 3）
└── ucx-crypto/       ← 加密与解密（Phase 4）
```

> 各 crate 的 API、测试数与实现细节见 [../memory/modules.md]。

## 9. 备注

- Phase 1 详细计划见 [phase-1-detailed.md]（Phase 2-4 进度直接记录于 [../memory/progress.md]）。
- 开发过程中的思路与决策记录见 [../memory/decisions.md]（ADR）。
- 待办事项见 [../TODO.md]。
- 所有代码变更通过 Git 持续管理，关键节点设置 Tag。

---

*根据 v0.4.0-alpha.2 开发进度更新*
