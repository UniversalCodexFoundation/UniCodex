# Unicodex 待办事项（未完成行动项）

> 更新时间：2026-05-31
> 来源：v0.4.0-alpha.2 回归测试报告（`test/problems/0.4.0-alpha.2.md`，commit 4db8c92）+ 后续收尾修复 259aecb / 05dd1f0 + 安全审查修复 5e47c67
> 范围：本文件**仅**记录未完成行动项。编年史见 [memory/progress.md](memory/progress.md)，决策原因见 [memory/decisions.md](memory/decisions.md)，模块状态见 [memory/modules.md](memory/modules.md)，前瞻计划见 [plan/roadmap.md](plan/roadmap.md)。

---

## 〇、安全审查修复（2026-05-31，commit 5e47c67，待回归）

> 全仓库安全审查 + 实机入侵测试发现并已修复，均有单元测试 + 真实 CLI 端到端再验证。需在下个版本回归后关闭。详见 [memory/decisions.md](memory/decisions.md) ADR-013、[memory/progress.md](memory/progress.md)。

| 编号 | 问题 | 严重度 | 修复 |
|------|------|--------|------|
| C-1 | `ucx unpack` Zip-Slip 任意文件写（盘符/反斜杠绝对路径绕过 extract_to 守卫） | Critical | 共享校验器 `ucx-types::path_safety`，生产/消费两侧统一委托 |
| H-1 | 零分块(`chunk_count==0`)UCXE 任意密钥下报告解密成功（未验证 AEAD tag） | High | `deserialize_chunks`/`decrypt_chunked` 双重拒绝 0 |
| M-1 | 证书 CN 含控制字符 -> 信任展示伪造（cert info / verify --show-signers） | Medium | 创建时校验 CN + 展示侧 `sanitize_for_display` |
| M-2 | `unicodex.toml` 未知/拼写错误字段静默接受（**闭合旧 NEW-R3-01**） | Medium | `ProjectConfig` 及各段加 `deny_unknown_fields` |
| M-3 | `struct.json` 无大小/节点/深度上限（解析放大 DoS，76MB→589MB） | Medium | 大小≤16MiB、节点≤100k、深度≤64 上限 |
| L-1 | 路径未显式拒绝 NUL/控制字符（**file 引用**部分；title 见下） | Low | 共享校验器覆盖 |
| L-2 | 空/纯空白证书 CN 被接受 | Low | 并入 M-1 校验 |

### 〇.2 第二轮（静态审查，commit 08e3b87，待回归）

> 精简静态审查（9 crate × 5 维度 → 对抗式验证，68 发现，7 High）修复全部 7 High + 多个 Medium，均有单元回归 + 真实 CLI 端到端再验证。详见 ADR-014。

| 类别 | 问题 | 严重度 | 修复 |
|------|------|--------|------|
| DoS | M-3 上界仅 build 生效，check/dry_run 绕过；ucx-version 无上界（**闭合 AUD-03**） | High | 上界下沉 `ucx-types::structure` 共享，强制于所有入口 |
| DoS | 解压炸弹：ucx-parse 解压路径无上界 | High | `read_entry_capped`（≤512MiB/条目，take 截断检出） |
| Panic | `find_signing_block` cd_offset 越界 OOB + `8+block_size` 溢出（含 verify() 同源 panic） | High | cd_offset≤len 守卫 + checked 算术 |
| 数据丢失 | CRLF 清单 `split("\n\n")` 丢全部条目 | High | 解析前 `\r\n→\n` 规范化 |
| 凭据 | 口令明文回显终端 + 注释谎称不回显 | High | TTY 感知：交互 rpassword 无回显、管道读 stdin |
| 数据丢失 | encrypt/decrypt 就地非原子覆盖，失败丢原稿 | High | `atomic_write`（temp+fsync+rename） |
| 认证 | 分块缺 chunk_count 绑定，尾块可静默截断 | Medium | 逐块 AAD 绑 `chunk_count`（**wire 变更，见 AUD-05**） |
| 规范 | `write_ucxe` 丢 raw_flags 保留位 | Medium | 写 `raw_flags` |
| 一致性 | build 缺 check 已有必填字段校验 | Medium | 补 title/creators 校验 |
| 一致性 | init [series] 模板 `total` 与 deny_unknown_fields 冲突；[dates] 重复 | Medium | 删 total、改 dates 注释 |
| 安全 | 口令/解密明文未 zeroize | Medium | `Zeroizing` 包装 |
| 一致性 | `version set` 静默降级 | Medium | 降级 warning + `UcxVersion` 派生 Ord |
| 性能 | tokio 未使用 | Low | 移除依赖 |
| 风格 | 多 crate 未过 cargo fmt | Low | 全工作区 cargo fmt |

**本轮新增的后续待办**（非阻塞，已确认但本批未改）：
- `AUD-01` `struct.json` 的 **`title` 字段** NUL/控制字符仍未校验（旧 NEW-R3-02 剩余；file 引用已覆盖）。Low。
- `AUD-02` `Codex`/`Structure`/`StructureNode` 的 `deny_unknown_fields`：需先核对 docs/02 字段完备性。Medium。
- `AUD-04` 输出父目录自动创建在各子命令间不一致。Low。
- `AUD-05` **【重要·破坏性】分块 AAD 绑定 `chunk_count` 是 wire 变更**：需同步全部 12 个语言 SDK 的分块加解密（含本会话刚发布的 **Cangjie**），否则分块文件跨实现验证失败。已更新 `sdk/UCX-FORMAT.md §7.7`。High（互操作）。
- `AUD-06` `ucx-parse::verify_hashes` 对清单声明但缺失的文件直接报错中止，应记为 invalid 继续核验。Medium。
- `AUD-07` `ucx-verify::verify()` 不内联 §5 逐文件完整性，Layer-1-only 归档篡改章节(保持 MANIFEST)仍判 ValidWithWarnings；至少修正模块文档表述或内联 §5。Medium。
- `AUD-08` `ucx-version` auto 路径 NewChapter 缺防降级（与手动 chapter 不一致）。Medium。
- `AUD-09` `find_eocd` 正向扫描取最低偏移 EOCD，与标准/文档反向扫描相悖（伪 EOCD 歧义）。Medium。
- `AUD-10` `ucx-build::create_ucx_archive`/`ucx-sign::sign` 整包驻留内存/重压缩（大归档峰值内存高），可流式优化。Medium（性能）。
- `AUD-11` `from_manifest_str` 缺必需字段的条目静默丢弃（应报错）。Medium。
- `AUD-12` init `--force` 覆盖 chapter-001.md/struct.json、文件名冲突静默丢失。Medium。
- `AUD-13` `--key` 经命令行泄露进程表/shell 历史：补 `--key-file`/env 或文档告警。Medium。
- `AUD-14` 杂项 Low/Info：dead code(`compute_blake3_hash`/`get_cd_size`/`decode_digest`)、过时注释('RSA/EC'/markdown)、`trim_end_matches(".md")`、`ext=="md"` 大小写、L2 未校验 `sig_algorithm_id==0x0001`、AES-CBC+--key 前置校验、open() 双开文件等。

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
| ~~NEW-R3-01~~ | R3 | ~~`unicodex.toml` 未知字段静默接受~~ → **已修复**（见 §〇 M-2，commit 5e47c67） | ~~Medium~~ |
| NEW-R3-02 | R3 | `struct.json` **title** 含 NUL 字节静默接受（file 引用已由 §〇 L-1 拒绝；title 仍待，见 AUD-01） | Low（已降级） |
| NEW-R1-02 | R1 | docs 承诺 `ucx sign` 自动读 toml `[signing]`，实测需显式 `-k`/`-c`（文档领先实现） | Minor |
| NEW-R1-03 | R1 | `docs/errors.md` 仍未创建 | Minor |
| NEW-R2-02 | R2 | argon2 参数越界与密码错误文案相同（反 oracle 设计权衡） | Low |
| NEW-R2-03 | R2 | 密文尾部追加字节被静默接受（不影响安全） | Low |
| NEW-R1-04 | R1 | `quickstart.md` 示例输出名与实际不符（复制粘贴失败） | Nit |
| NEW-R1-05 | R1 | `workflow.md` struct.json 表述可优化（理解歧义） | Nit |
| NEW-R2-04 | R2 | CBC 错误文案泄露内部函数名（信息泄露） | Nit |

---

## 三、v0.4.0-beta.1 候选必修

- **输入校验**：NEW-R3-01 ✅（toml 未知字段，已由 M-2 修复）；NEW-R3-02 剩余 title NUL（AUD-01）。
- **安全审查修复回归**：§〇 全部（C-1/H-1/M-1/M-2/M-3/L-1/L-2）需纳入下个版本回归测试报告。
- **文档一致性批**：NEW-R1-02、NEW-R1-03、NEW-R1-04、NEW-R1-05、NEW-R2-04。

---

## 四、运维待办

- **创建 `docs/errors.md`**：错误码对照表，关闭 NEW-R1-03。

> 已完成：git tag 回补（v0.0.0-alpha.1 → v0.4.0-alpha.2，7 个）；temp_test/ 已决定保留并 gitignore。

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

> 详见 [plan/roadmap.md](plan/roadmap.md) 与 [../sdk/README.md](../sdk/README.md)。

- **多语言 SDK**：✅ 首批 **14 种**只读阅读器 SDK 已就绪（对应 UCX 标准 0.4.x；首批 v0.4.0，Cangjie 已发补丁 **v0.4.1**）：Rust/Go/Python/TypeScript/Java/C#/Kotlin/C++/Ruby/PHP/Swift/Dart/ArkTS/Cangjie。原 SDK-001~003（go/py/js）已并入（无单独 js 版——TypeScript SDK 双产物 ESM/CJS 即 JS 可用）。
  - ✅ 已完成：Cangjie 补实现 分块(>64 MiB) UCXE 解密 + 证书有效期窗口校验（v0.4.1，真实 CLI ground truth + Python 端口逐位核验；见其 README）。
  - 待办：6 种无本机工具链的 SDK（Ruby/PHP/Swift/Dart/ArkTS/Cangjie）需在装有对应工具链的环境做真机构建 + T1–T10 回归（Cangjie 还需 `cjpm test` 复核 0.4.1 新增源码语法）。
  - 增强（远期）：各 SDK 可加写入/签名/加密能力（当前仅只读）。
- **官方服务**：SVC-001 密钥分发、SVC-002 身份验证 / CA、SVC-003 包验证、SVC-004 阅读器集成 API。

---

## 七、其他

- **MISC-001** `ucx init --template`：模板选择（小说、散文、诗歌等）。
- **MISC-002** 区块链溯源（远期）：Merkle Tree + 多链锚定，规范 `06-blockchain-ext.md`。
