# Unicodex 后续开发待办事项

> 来源：v0.1.0-alpha.1 / v0.1.0-alpha.2 / v0.2.0-alpha.1 / v0.3.0-alpha.1 测试报告
> 更新时间：2026-04-15
> 状态说明：Phase 3（签名与验证）已在 v0.3.0-alpha.1 中完成，所有已知 bug 已修复

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

暂无遗留问题。所有已知 bug 均已在 v0.3.0-alpha.1 第二轮测试后修复。

---

## Phase 3 扩展计划（签名体系增强）

以下功能已在 v0.3.0-alpha.1 实机测试中识别为未来扩展方向，不阻塞 Phase 4 开发：

| 编号 | 功能 | 描述 | 优先级 |
|------|------|------|--------|
| SIG-EXT-001 | CA 证书链验证 | 当前仅支持自签名证书；未来需支持 CA 签发的证书链逐级验证 | 高 |
| SIG-EXT-002 | ECDSA-P256 / RSA 算法支持 | 当前仅支持 Ed25519；规范允许多算法，需扩展 `keygen` 和签名/验证流程 | 中 |
| SIG-EXT-003 | 加密私钥（PBES2） | 当前私钥以明文 PEM 存储；需支持 PKCS#8 加密格式（passphrase 保护） | 中 |
| SIG-EXT-004 | 密钥轮换（Proof-of-Rotation） | 规范 Pair ID `0x55435820` 预留了密钥轮换数据块；需实现新旧密钥绑定链 | 低 |
| SIG-EXT-005 | 多签名者共享签名块 | 当前多签名者会导致前一签名者的 Layer 2 失效（NOTE-001）；需设计共享签名块方案 | 低 |

---

## Phase 4 计划 — 加密体系（Encryption）

目标版本：v0.4.0-alpha.x / v0.4.0-beta.x

规范依据：`docs/04-crypto-spec.md`

| 编号 | 功能 | 描述 |
|------|------|------|
| CRYPTO-001 | ucx-crypto 核心模块 | 实现 AES-256-GCM / ChaCha20-Poly1305 对称加密引擎 |
| CRYPTO-002 | UCXE 二进制格式 | 加密后的 UCX 文件格式（.ucxe），包含加密头和密文段 |
| CRYPTO-003 | 密钥派生 | Argon2id / PBKDF2 从用户口令派生加密密钥 |
| CRYPTO-004 | 章节级加密 | 按章节独立加密，允许部分章节免费阅读 |
| CRYPTO-005 | 段落级加密 | 更细粒度的加密控制（试读段落） |
| CRYPTO-006 | 多密钥获取方式 | 支持 direct / message / url / service 四种密钥获取途径 |
| CRYPTO-007 | `ucx encrypt` CLI | CLI 加密命令，支持指定加密范围和算法 |
| CRYPTO-008 | `ucx decrypt` CLI | CLI 解密命令，支持多种密钥输入方式 |

---

## Phase 5 计划 — 生态建设（Ecosystem）

目标版本：v1.0.0

规范依据：`docs/08-keys-identity.md`、`docs/09-official-services.md`

### 多语言 SDK

| 编号 | 功能 | 描述 |
|------|------|------|
| SDK-001 | go-ucx | Go 语言 SDK — UCX 文件读写、验证 |
| SDK-002 | py-ucx | Python 解析库 — 面向数据分析和脚本场景 |
| SDK-003 | js-ucx | JavaScript/TypeScript 库 — 面向 Web 阅读器和 Node.js |

### 官方服务

| 编号 | 功能 | 描述 |
|------|------|------|
| SVC-001 | 密钥分发服务 | 公钥注册与分发（类 keyserver） |
| SVC-002 | 身份验证 / CA 服务 | 签发和管理 UCX 签名证书 |
| SVC-003 | 包验证服务 | 在线验证 UCX 文件签名和完整性 |
| SVC-004 | 阅读器集成 API | 为第三方阅读器提供标准化接口 |

---

## 其他待办

| 编号 | 功能 | 描述 | 阶段 |
|------|------|------|------|
| MISC-001 | `ucx init --template` | 模板选择（小说、散文、诗歌等） | Phase 4+ |
| MISC-002 | 区块链溯源 | Merkle Tree + 多链锚定（规范 `06-blockchain-ext.md`） | 远期 |

---

*本文件于 v0.3.0-alpha.1 Phase 3 全部 bug 修复后更新（2026-04-15）。*
