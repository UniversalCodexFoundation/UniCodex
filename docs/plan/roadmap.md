# Unicodex 长期开发计划总览

> 版本：0.2.0
> 创建时间：2026-02-16
> 最后更新：2026-02-16
> 状态：Phase 1 已完成，Phase 2 规划中

## 1. 目标

将 Unicodex 规范从文档设计推进到可用的参考实现，最终形成完整的工具链和生态系统。

## 2. 开发阶段总览

```
┌─────────────────────────────────────────────────────────────────┐
│                   Unicodex 开发阶段总览                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Phase 0 ─ 基础设施（Foundation）                  ✅ 已完成    │
│  ├── Cargo Workspace 搭建（resolver v3, edition 2024）         │
│  ├── 9 个 Rust 模块脚手架                                      │
│  ├── ucx-types 公共类型 crate（Codex, Structure, Manifest 等）  │
│  └── 10 份规范文档（v0.0.0-alpha.1）                           │
│                                                                 │
│  Phase 1 ─ 核心读写（Core I/O）                    ✅ 已完成    │
│  ├── ucx-init：项目初始化                                      │
│  │   ├── 目录结构创建 + UCX ID 生成                            │
│  │   ├── unicodex.toml 模板生成                                │
│  │   ├── 路径遍历防护 + 输入验证                               │
│  │   └── struct.json / chapter-001.md 脚手架                   │
│  ├── ucx-build：打包构建 UCX 文件                              │
│  │   ├── TOML → JSON 元数据转换（codex.json）                  │
│  │   ├── MANIFEST.MF 生成（BLAKE3 + Base64）                   │
│  │   ├── ZIP 归档创建（mimetype STORED + DEFLATE 压缩策略）    │
│  │   └── struct.json 校验（互斥规则 + 文件引用验证）           │
│  ├── ucx-parse：解析读取 UCX 文件                              │
│  │   ├── mimetype 验证 + MANIFEST.MF 解析                      │
│  │   ├── codex.json / struct.json 反序列化                     │
│  │   ├── 章节内容提取                                          │
│  │   └── BLAKE3 哈希完整性验证                                 │
│  └── unicodex-core：CLI 基础集成                               │
│      ├── ucx init / build / info / verify 命令                 │
│      └── 默认 WARN 日志级别 + 未实现命令友好提示               │
│                                                                 │
│  Phase 2 ─ 完善与版本管理（Polish & Versioning）    📋 规划中   │
│  ├── ucx-version：版本管理模块                                 │
│  │   ├── git2 集成与变更检测                                   │
│  │   ├── 自动版本号生成（卷.章.修订）                          │
│  │   └── file_version / dates 字段支持                         │
│  ├── ucx-init 增强                                             │
│  │   ├── Git 仓库自动初始化 + .gitignore 生成                  │
│  │   ├── 目录结构精简（--full / --template 选项）              │
│  │   ├── 交互式模式（--interactive）                           │
│  │   ├── 从现有目录初始化（--from-existing）                   │
│  │   ├── BCP 47 语言标签验证                                   │
│  │   └── 默认值本地化 + 超长输入处理                           │
│  ├── ucx-build / ucx-parse 增强                                │
│  │   ├── ucx check 命令（独立项目校验）                        │
│  │   ├── ucx build --dry-run                                   │
│  │   ├── ucx build 覆盖提示 / --force                          │
│  │   ├── ucx info --json + 输出增强                            │
│  │   ├── ucx unpack 解包功能                                   │
│  │   └── dates / file_version 元数据完整支持                   │
│  ├── CLI 体验优化                                              │
│  │   ├── 当前目录非空时确认提示                                │
│  │   ├── unicodex.toml 模板增强（注释示例）                    │
│  │   └── chapter-001.md 引导内容                               │
│  └── 规范文档修订                                              │
│      └── Created-By 名称统一等勘误                             │
│                                                                 │
│  Phase 3 ─ 签名与验证（Signature & Verification）   📋 规划中   │
│  ├── ucx-sign：签名机制                                        │
│  │   ├── Layer 1 — JAR 式签名（MF → SF → RSA/EC）             │
│  │   ├── Layer 2 — APK v2 式全局签名                           │
│  │   ├── 多签名支持（作者、编辑、出版方）                      │
│  │   └── 证书链管理                                            │
│  └── ucx-verify：完整验证                                      │
│      ├── 双层签名验证                                          │
│      ├── 证书链验证                                            │
│      ├── 无签名时明确提示                                      │
│      └── --verbose 详细输出                                    │
│                                                                 │
│  Phase 4 ─ 加密体系（Encryption）                   📋 规划中   │
│  └── ucx-crypto：加密模块                                      │
│      ├── AES-256-GCM / ChaCha20-Poly1305 章节加密              │
│      ├── UCXE 二进制格式处理                                   │
│      ├── 密钥派生（Argon2id / PBKDF2）                         │
│      ├── 章节级 + 段落级加密                                   │
│      └── 多种密钥获取方式（direct/message/url/service）        │
│                                                                 │
│  Phase 5 ─ 生态建设（Ecosystem）                    📋 规划中   │
│  ├── 多语言 SDK                                                │
│  │   ├── go-ucx：Go 语言 SDK                                  │
│  │   ├── py-ucx：Python 解析库                                 │
│  │   └── js-ucx：JavaScript/TypeScript 库                      │
│  ├── 官方服务                                                  │
│  │   ├── 密钥分发服务                                          │
│  │   ├── 身份验证 / CA 服务                                    │
│  │   ├── 包验证服务                                            │
│  │   └── 公钥注册服务                                          │
│  └── 阅读器集成 API                                            │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  未来计划 ─ 区块链溯源（Blockchain Traceability）               │
│  ├── Merkle Tree 构建                                          │
│  ├── genesis.proof 生成                                        │
│  ├── 多链锚定（Ethereum / Bitcoin / Polygon / ...）            │
│  └── 链上验证                                                  │
│                                                                 │
│  说明：区块链溯源作为独立扩展层设计，不与核心功能耦合。        │
│  规范见 06-blockchain-ext.md，将在核心工具链稳定后视需求推进。  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 3. 阶段依赖关系

```
Phase 0 ──→ Phase 1 ──→ Phase 2 ──┬──→ Phase 3 ──→ Phase 4
  ✅           ✅         📋       │       📋           📋
                                   │
                                   └──→ Phase 5（Phase 3 完成后可启动）
                                            📋

                          ·  ·  ·  ·  ·  ·  ·  ·  ·  ·  ·  ·

                                   未来计划：区块链溯源
                                   （核心工具链稳定后独立推进）
```

**关键路径**：Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 4

- Phase 3（签名）和 Phase 4（加密）存在技术关联但可部分并行
- Phase 5（生态建设）在签名体系完成后启动，SDK 需要覆盖完整功能
- 区块链溯源不在关键路径上，作为可选扩展独立演进

## 4. 当前状态

| 阶段 | 状态 | 版本 | 说明 |
|------|------|------|------|
| 规范设计 | **已完成** | v0.0.0-alpha.1 | 10 份规范文档（00-09） |
| Phase 0 | **已完成** | v0.1.0-alpha.1 | Workspace + ucx-types + 模块脚手架 |
| Phase 1 | **已完成** | v0.1.0-alpha.2 | init/build/parse + CLI + 75 tests |
| Phase 2 | 规划中 | — | 19 项待办已记录于 todo.md |
| Phase 3 | 规划中 | — | 规范 05-signature-spec.md 已就绪 |
| Phase 4 | 规划中 | — | 规范 04-crypto-spec.md 已就绪 |
| Phase 5 | 规划中 | — | 规范 08/09 已就绪 |
| 区块链 | 远期计划 | — | 规范 06-blockchain-ext.md 已就绪 |

## 5. 里程碑规划

| 里程碑 | 对应阶段 | 版本标签 | 交付物 |
|--------|---------|---------|--------|
| M0 - 规范完成 | 规范设计 | v0.0.0-alpha.1 | 规范文档集 |
| M1 - 脚手架就绪 | Phase 0 | v0.1.0-alpha.1 | Workspace + ucx-types + 模块脚手架 |
| M2 - 基本读写 | Phase 1 | v0.1.0-alpha.2 | init → build → parse 完整工作流 |
| M3 - 完善工具链 | Phase 2 | v0.2.0-alpha.x | 版本管理 + CLI 增强 + ucx check |
| M4 - 签名验证 | Phase 3 | v0.3.0-alpha.x | 双层签名 + 证书链 + 完整验证 |
| M5 - 加密支持 | Phase 4 | v0.4.0-beta.x | 章节/段落加密 + 多密钥方式 |
| M6 - 1.0 发布 | Phase 5 | v1.0.0 | 稳定版本 + 多语言 SDK + 服务 |

## 6. 规范文档与模块对照

| 规范文档 | 涉及模块 | 阶段 |
|---------|---------|------|
| 00-overview.md | — | 总览 |
| 01-file-structure.md | ucx-build, ucx-parse | Phase 1 ✅ |
| 02-metadata-spec.md | ucx-types, ucx-init | Phase 1 ✅ / Phase 2 补全 |
| 03-content-format.md | ucx-build, ucx-parse | Phase 1 ✅ / Phase 2 增强 |
| 04-crypto-spec.md | ucx-crypto | Phase 4 |
| 05-signature-spec.md | ucx-sign, ucx-verify | Phase 3 |
| 06-blockchain-ext.md | （未来扩展） | 远期计划 |
| 07-versioning.md | ucx-version | Phase 2 |
| 08-keys-identity.md | ucx-sign, ucx-crypto | Phase 3-4 |
| 09-official-services.md | （服务端） | Phase 5 |

## 7. 技术栈

| 领域 | 技术选型 | 说明 |
|------|---------|------|
| 主要语言 | Rust (edition 2024) | 所有核心模块，rust-version 1.85 |
| 哈希算法 | BLAKE3 (`blake3`) | 高性能，推荐默认，摘要 Base64 编码 |
| 签名算法 | Ed25519 (`ed25519-dalek`) | 推荐默认 |
| ZIP 操作 | `zip` crate | UCX 容器格式 |
| 版本管理 | `git2` (libgit2) | 变更检测与自动版本号 |
| 序列化 | `serde` + `serde_json` + `toml` | 元数据处理 |
| 错误处理 | `thiserror` (库) / `anyhow` (CLI) | 分层错误模型 |
| CLI 框架 | `clap` (derive) | 命令行解析 |
| 日志 | `tracing` + `tracing-subscriber` | 结构化日志，默认 WARN 级别 |
| 异步运行时 | `tokio` | 网络相关操作 |
| 编码 | `base64` | MANIFEST.MF 摘要编码 |

## 8. Workspace 模块架构

```
unicodex/
├── unicodex-core/    ← CLI 入口（ucx 命令）
├── ucx-types/        ← 公共类型（Codex, Structure, Manifest, ProjectConfig 等）
├── ucx-init/         ← 项目初始化
├── ucx-build/        ← 构建打包
├── ucx-parse/        ← 解析读取
├── ucx-verify/       ← 签名与完整性验证（Phase 3）
├── ucx-sign/         ← 签名与证书管理（Phase 3）
├── ucx-version/      ← 版本管理（Phase 2）
└── ucx-crypto/       ← 加密与解密（Phase 4）
```

## 9. 备注

- 各阶段的详细开发计划见 `docs/plan/phase-N-*.md`
- 开发过程中的思路和决策记录在 `docs/development/` 目录下
- Phase 1 测试报告及问题追踪见 `docs/test/problems/`
- Phase 2 待办事项见 `docs/development/todo.md`
- 所有代码变更通过 Git 持续管理，关键节点设置 Tag

---

*根据 v0.1.0-alpha.2 开发进度更新*
