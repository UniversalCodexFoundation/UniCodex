# Unicodex 待办事项（未完成行动项）

> 更新时间：2026-04-18
> 来源：v0.4.0-alpha.2 回归测试报告（`test/problems/0.4.0-alpha.2.md`，commit 4db8c92）+ 后续收尾修复 259aecb / 05dd1f0
> 范围：本文件**仅**记录未完成行动项。编年史见 [memory/progress.md](memory/progress.md)，决策原因见 [memory/decisions.md](memory/decisions.md)，模块状态见 [memory/modules.md](memory/modules.md)，前瞻计划见 [plan/roadmap.md](plan/roadmap.md)。

---

## 一、v0.4.0-alpha.2 已修复待回归

> 已在 HEAD（05dd1f0）修复，需在下个版本（v0.4.0-beta.1）回归验证后关闭。

| 编号 | 原回归标记 | 问题 | 严重度 | 修复 commit | 状态 |
|------|-----------|------|--------|-------------|------|
| NEW-R1-01 | D1-08b NOT-FIXED | `ucx init --force` 仍拒绝已有项目（与 `--help` 矛盾） | Major | 259aecb | 已修复待回归 |
| NEW-R2-01 | CRYPTO-2 PARTIAL | AEAD AAD 未覆盖 flags reserved bits 1-7 | Medium | 05dd1f0 | 已修复待回归（已加测试 `test_flags_reserved_bit_tampering_fails_decryption`） |

---

## 二、v0.4.0-alpha.2 待处理

> 截至 HEAD（05dd1f0）尚未修复，均为 Medium 及以下，无 Critical / High。

| 编号 | 组 | 问题 | 严重度 |
|------|----|------|--------|
| NEW-R3-01 | R3 | `unicodex.toml` 未知字段静默接受（拼写错误被吞） | Medium |
| NEW-R3-02 | R3 | `struct.json` title 含 NUL 字节静默接受（跨平台显示风险） | Medium |
| NEW-R1-02 | R1 | docs 承诺 `ucx sign` 自动读 toml `[signing]`，实测需显式 `-k`/`-c`（文档领先实现） | Minor |
| NEW-R1-03 | R1 | `docs/errors.md` 仍未创建 | Minor |
| NEW-R2-02 | R2 | argon2 参数越界与密码错误文案相同（反 oracle 设计权衡） | Low |
| NEW-R2-03 | R2 | 密文尾部追加字节被静默接受（不影响安全） | Low |
| NEW-R1-04 | R1 | `quickstart.md` 示例输出名与实际不符（复制粘贴失败） | Nit |
| NEW-R1-05 | R1 | `workflow.md` struct.json 表述可优化（理解歧义） | Nit |
| NEW-R2-04 | R2 | CBC 错误文案泄露内部函数名（信息泄露） | Nit |

---

## 三、v0.4.0-beta.1 候选必修

- **输入校验**：NEW-R3-01（toml 未知字段拒绝/告警）、NEW-R3-02（title NUL 字节拒绝）。
- **文档一致性批**：NEW-R1-02、NEW-R1-03、NEW-R1-04、NEW-R1-05、NEW-R2-04。

---

## 四、运维待办

- **回补缺失 git tag**：v0.4.0-alpha.1 → `58dc0d4`、v0.4.0-alpha.2 → `b56d1ec`，落实 CLAUDE.md「关键节点打 tag」要求（历史 v0.2.0/v0.3.0/v0.4.0 系列均未打 tag）。
- **评估清理 `temp_test/`**：145M 可再生 scratch 目录，已 gitignore，评估是否清理。
- **创建 `docs/errors.md`**：关闭 NEW-R1-03。

---

## 五、签名体系增强（未来，非阻塞）

| 编号 | 功能 | 优先级 |
|------|------|--------|
| SIG-EXT-001 | CA 证书链验证（当前仅自签名） | 高 |
| SIG-EXT-002 | ECDSA-P256 / RSA 算法支持（当前仅 Ed25519） | 中 |
| SIG-EXT-003 | 加密私钥 PBES2（PKCS#8，passphrase 保护，当前明文 PEM） | 中 |
| SIG-EXT-004 | 密钥轮换 Proof-of-Rotation（Pair ID `0x55435820` 预留块） | 低 |
| SIG-EXT-005 | 多签名者共享签名块（解决 NOTE-001 Layer 2 失效） | 低 |

---

## 六、Phase 5 生态建设（目标 v1.0.0）

> 详见 [plan/roadmap.md](plan/roadmap.md)。

- **多语言 SDK**：SDK-001 go-ucx、SDK-002 py-ucx、SDK-003 js-ucx。
- **官方服务**：SVC-001 密钥分发、SVC-002 身份验证 / CA、SVC-003 包验证、SVC-004 阅读器集成 API。

---

## 七、其他

- **MISC-001** `ucx init --template`：模板选择（小说、散文、诗歌等）。
- **MISC-002** 区块链溯源（远期）：Merkle Tree + 多链锚定，规范 `06-blockchain-ext.md`。
