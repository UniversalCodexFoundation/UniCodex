# Unicodex 长期开发计划总览

> 版本：0.1.0-draft
> 创建时间：2026-02-16
> 状态：规划中

## 1. 目标

将 Unicodex 规范从文档设计推进到可用的参考实现，最终形成完整的工具链和生态系统。

## 2. 开发阶段总览

```
┌─────────────────────────────────────────────────────────────────┐
│                   Unicodex 开发阶段总览                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Phase 0 ─ 基础设施（Foundation）                               │
│  ├── Cargo Workspace 搭建                                      │
│  ├── 公共类型定义（UCX ID、版本号、错误类型等）                │
│  ├── CI/CD 流水线                                              │
│  └── 开发规范与贡献指南                                        │
│                                                                 │
│  Phase 1 ─ 核心读写（Core I/O）                                │
│  ├── ucx-build：打包构建 UCX 文件                              │
│  │   ├── TOML → JSON 元数据转换                                │
│  │   ├── MANIFEST.MF 生成                                      │
│  │   ├── ZIP 归档创建                                          │
│  │   └── mimetype 固定声明写入                                 │
│  ├── ucx-parse：解析读取 UCX 文件                              │
│  │   ├── ZIP 归档解压                                          │
│  │   ├── MANIFEST.MF 解析与校验                                │
│  │   ├── codex.json / struct.json 解析                         │
│  │   └── 章节内容提取                                          │
│  └── ucx-init：项目初始化                                      │
│      ├── unicodex.toml 模板生成                                │
│      ├── 目录结构创建                                          │
│      └── 交互式配置向导                                        │
│                                                                 │
│  Phase 2 ─ 安全体系（Security）                                │
│  ├── ucx-sign：签名机制                                        │
│  │   ├── Layer 1 JAR 式签名（MF → SF → RSA/EC）               │
│  │   ├── Layer 2 APK v2 式全局签名                             │
│  │   ├── 多签名支持                                            │
│  │   └── 证书链管理                                            │
│  ├── ucx-verify：验证模块                                      │
│  │   ├── 双层签名验证                                          │
│  │   ├── MANIFEST.MF 完整性校验                                │
│  │   └── 证书链验证                                            │
│  └── ucx-crypto：加密模块                                      │
│      ├── AES-256-GCM / ChaCha20-Poly1305 实现                  │
│      ├── UCXE 二进制格式处理                                   │
│      ├── 密钥派生（Argon2id / PBKDF2）                         │
│      ├── 章节级加密                                            │
│      └── 段落级加密                                            │
│                                                                 │
│  Phase 3 ─ 工具集成（Tooling）                                 │
│  ├── ucx-version：版本管理                                     │
│  │   ├── git2 变更检测                                         │
│  │   ├── 自动版本号生成（卷.章.修订）                          │
│  │   └── 快照回退（无 Git 环境）                               │
│  └── unicodex-core：CLI 集成                                   │
│      ├── 统一命令行入口（ucx）                                 │
│      ├── 所有子命令集成                                        │
│      └── 交互式界面与进度条                                    │
│                                                                 │
│  Phase 4 ─ 区块链溯源（Blockchain）                            │
│  ├── Merkle Tree 构建                                          │
│  ├── genesis.proof 生成                                        │
│  ├── 多链锚定（Ethereum / Bitcoin / Polygon / ...）            │
│  └── 链上验证                                                  │
│                                                                 │
│  Phase 5 ─ 多语言 SDK（Ecosystem）                             │
│  ├── go-ucx：Go 语言 SDK                                      │
│  ├── py-ucx：Python 解析库                                     │
│  ├── js-ucx：JavaScript/TypeScript 库                          │
│  └── 其他语言绑定按需扩展                                      │
│                                                                 │
│  Phase 6 ─ 官方服务（Services）                                │
│  ├── 密钥分发服务                                              │
│  ├── 身份验证 / CA 服务                                        │
│  ├── 包验证服务                                                │
│  └── 公钥注册服务                                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 3. 阶段依赖关系

```
Phase 0 ──→ Phase 1 ──→ Phase 2 ──→ Phase 3
                │              │           │
                │              │           └──→ Phase 4
                │              │
                │              └──→ Phase 5（可与 Phase 3 并行）
                │
                └──→ Phase 6（可在 Phase 2 之后启动）
```

**关键路径**：Phase 0 → Phase 1 → Phase 2 → Phase 3

Phase 5（多语言 SDK）和 Phase 6（官方服务）可在安全体系完成后独立推进。

## 4. 当前状态

| 阶段 | 状态 | 说明 |
|------|------|------|
| 规范设计 | **已完成** | 10 份规范文档（00-09），标记 v0.0.0-alpha.1 |
| Phase 0 | **进行中** | Workspace 搭建、模块脚手架 |
| Phase 1 | 未开始 | — |
| Phase 2 | 未开始 | — |
| Phase 3 | 未开始 | — |
| Phase 4 | 未开始 | — |
| Phase 5 | 未开始 | — |
| Phase 6 | 未开始 | — |

## 5. 里程碑规划

| 里程碑 | 对应阶段 | 版本标签 | 交付物 |
|--------|---------|---------|--------|
| M0 - 规范完成 | 规范设计 | v0.0.0-alpha.1 | 规范文档集 |
| M1 - 脚手架就绪 | Phase 0 | v0.1.0-alpha.1 | Workspace + 空模块 + CI |
| M2 - 基本读写 | Phase 1 | v0.2.0-alpha.x | 可构建/解析简单 UCX 文件 |
| M3 - 签名验证 | Phase 2 | v0.3.0-alpha.x | 签名 + 验证 + 基本加密 |
| M4 - CLI 完整 | Phase 3 | v0.4.0-beta.x | 完整 CLI 工具链 |
| M5 - 区块链支持 | Phase 4 | v0.5.0-beta.x | 多链存证功能 |
| M6 - 1.0 发布 | Phase 5-6 | v1.0.0 | 稳定版本 + 多语言 SDK |

## 6. 技术栈确认

| 领域 | 技术选型 | 说明 |
|------|---------|------|
| 主要语言 | Rust (edition 2021) | 所有核心模块 |
| 哈希算法 | BLAKE3 (`blake3` crate) | 高性能，推荐默认 |
| 签名算法 | Ed25519 (`ed25519-dalek`) | 推荐默认 |
| ZIP 操作 | `zip` crate | UCX 容器格式 |
| 版本管理 | `git2` crate (libgit2) | 变更检测 |
| 序列化 | `serde` + `serde_json` + `toml` | 元数据处理 |
| 错误处理 | `thiserror` (库) / `anyhow` (CLI) | 分层错误模型 |
| CLI 框架 | `clap` (derive) | 命令行解析 |
| 异步运行时 | `tokio` | 网络相关操作 |

## 7. 备注

- 本文档仅为**总览规划**，各阶段的详细设计将在对应的 `docs/plan/phase-N-*.md` 中展开
- 开发过程中的思路和决策记录在 `docs/development/` 目录下
- 所有代码变更通过 Git 持续管理，关键节点设置 Tag

---

*后续将根据开发进展更新各阶段状态*
