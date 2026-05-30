# 进度编年史（Progress Chronicle）

> 本文档是 Unicodex 的**唯一编年史**（SSOT）：记录"何时发布了什么"。
> - 决策原因（为什么这么做）→ 见 [decisions.md](decisions.md)（ADR）
> - 模块设计/API/实现状态 → 见 [modules.md](modules.md)
> - 前瞻计划/阶段/里程碑 → 见 [../plan/roadmap.md](../plan/roadmap.md)
> - 未完成行动项 → 见 [../TODO.md](../TODO.md)
>
> 本文件只写"已发生的事实"，不复述上述文件的内容，仅相对链接交叉引用。
> 事实可追溯：尽量附 commit hash / 文件路径。

---

## 风险备注：tag 缺口（事实）

git tag 仅 3 个，且均打于 2026-02-16：`v0.0.0-alpha.1` / `v0.1.0-alpha.1` / `v0.1.0-alpha.2`。
`v0.2.0` / `v0.3.0` / `v0.4.0` 系列**从未打 tag**。因此后续不可用 `git log vX.Y.Z..HEAD` 作版本边界。
待办见 [../TODO.md](../TODO.md)。

---

## v0.0.0-alpha.1 — 规范奠基（2026-02-16）

- 完成 10 份规范文档 `docs/00-overview.md` ~ `docs/09-official-services.md`：文件结构、元数据、正文格式、加密、双层签名、区块链溯源、版本控制、密钥身份、官方服务。
- Git 仓库初始化（`.gitignore` + 首次提交），打 tag `v0.0.0-alpha.1`。
- 此阶段仅文档与规范，无功能代码、无功能测试。
- 关键决策：在规范期一次性确立全部架构基调（ADR-001 ~ ADR-005 均记于 2026-02-16），各自在对应 Phase 落地。详见 [decisions.md](decisions.md)。

## v0.1.0-alpha.1 — Phase 0：Workspace 与脚手架（2026-02-16）

- 搭建 Cargo Workspace，创建各 `ucx-*` 模块脚手架；`docs/plan/roadmap.md` 开发总览成形。
- `cargo check` 通过；此阶段为占位脚手架，尚无生产逻辑。
- 扩展模块（ucx-audio-book、ucx-interactive）暂注释，待后续阶段启用。
- 关键决策：core 依赖模块而非模块依赖 core，避免循环依赖。详见 [decisions.md](decisions.md) ADR-001。

## v0.1.0-alpha.2 — Phase 1：核心 I/O + 首轮修复（2026-02-16 ~ 02-17）

- 新建 `ucx-types` 公共类型 crate（Codex / Manifest / Structure / ProjectConfig / UcxId 等），落地类型归属问题。
- 实现核心读写三件套：`ucx-init`（初始化）、`ucx-build`（打包，BLAKE3 + ZIP，mimetype 首条目 STORED）、`ucx-parse`（解析校验）。
- CLI 接入 init/build/info/verify；新增 `ucx check` 等子命令与往返集成测试。
- 基于 alpha.1/alpha.2 测试报告完成 20 项修复（输入校验、本地化、目录精简、Git 自动初始化等）。
- 关键决策：`ucx-build`（写）与 `ucx-parse`（读）彻底分离，方便阅读器只引用读链路。详见 [decisions.md](decisions.md) ADR-002。
- 测试数：76（全部通过；Phase 1 核心 69，修复后 76）。

## v0.2.0-alpha.1 — Phase 2：版本管理（2026-03-25）

- `ucx-version` 从占位升级为生产实现：`UcxVersion`/`ChangeSet`、git2 变更检测、`.ucx-snapshot.json` 快照回退、自动/手动 bump。
- `ucx-types` 扩展 `FileVersion`/`VersionSection`；`ucx-build` 接入构建时自动版本解析。
- CLI 新增 `ucx version`（auto/patch/chapter/volume/set），版本状态持久化到 `.ucx-version.json`。
- 解决遗留项 P-008：codex.json 支持 `file_version` 字段。
- 关键决策：用 git2 而非系统 Git CLI、无仓库时快照回退；版本语义采用"卷.章.修订"。详见 [decisions.md](decisions.md) ADR-003、ADR-005。
- 测试数：122（全部通过）。

## v0.3.0-alpha.1 — Phase 3：双层签名与验证（2026-04-12）

- `ucx-sign` 生产实现：Ed25519 密钥对、自签名 X.509 v3 证书、Layer 1（JAR 式 MANIFEST→SF→EC）、Layer 2（APK v2 式全局签名块，1 MiB 分块 BLAKE3）、ZIP 二进制注入/重写。
- `ucx-verify` 生产实现：双层验证 + 签名者信息提取，四级状态 VERIFIED / PARTIAL / UNSIGNED / INVALID。
- CLI 新增 `ucx keygen`、`ucx cert create|info`、`ucx sign`，并升级 `ucx verify --show-signers`。
- 解决遗留项 SEC-004：完整双层签名体系落地。
- 关键决策：加密在签名之前（Encrypt-then-Sign），密文受签名保护、无需解密即可验完整性。详见 [decisions.md](decisions.md) ADR-004。
- 测试数：实机回归 43 项 40 通过（93%），见 `docs/test/problems/0.3.0-alpha.1.md`；各 crate 单元测试现状（ucx-sign 42 / ucx-verify 13）见 [modules.md](modules.md)。

## v0.4.0-alpha.1 — Phase 4：加密体系（2026-04-16）

- `ucx-crypto` 生产实现：AES-256-GCM / ChaCha20-Poly1305 / AES-256-CBC 三算法，Argon2id / PBKDF2 两 KDF，UCXE 二进制格式。
- 加密粒度：章节级（整文件 UCXE）、段落级（Base64 嵌 `<encrypted>`）、大文件分块（>64 MiB 按 1 MiB 分块，nonce 派生）。
- CLI 新增 `ucx encrypt` / `ucx decrypt`（直接密钥 + 口令两模式）；ManifestEntry 支持 `Encrypted` / `Original-Size`。
- 关键决策：默认 AES-256-GCM（无 AES-NI 场景用 ChaCha20-Poly1305），推荐 Argon2id KDF。详见 [decisions.md](decisions.md)（沿用 ADR-004 加解密-签名顺序）。
- 测试数：218+（全部通过，ucx-crypto 新增 58；clippy 零警告）。

## v0.4.0-alpha.2 — 安全加固与回归测试（2026-04-18）

主题：对 alpha.1 做系统性安全加固并完成回归测试。按子系统归类（附 commit）：

**加密（ucx-crypto）**
- UCXE 长度边界校验，防解析越界 panic（`10905a8`）。
- chunked 子 nonce 由 XOR 改为拼接派生，消除 nonce 碰撞（`9ad7a36`）。
- AES-CBC 的 HMAC 覆盖范围扩展至 `aad || iv || ct`（`8b29351`）。
- AEAD 的 AAD 覆盖 UCXE 头部及 flags 字节（`5288720`，并由 `05dd1f0` 用原始 flags 字节覆盖 reserved bits 收尾）。
- KDF 参数上下限校验（拒绝弱参数与 time=MAX 类 DoS）；passphrase 入 KDF 前统一 NFC 归一化（`e985b25`）。
- decrypt 失败统一返回 `DecryptionFailed`，避免错误文案侧信道（`07da9e2`）；密钥用 zeroize 清零（`6ad307a`）。

**签名 / 验证（ucx-sign / ucx-verify）**
- 证书 notBefore / notAfter 有效期校验（过期/未生效 → INVALID）（`5c65811`）。
- 私钥文件权限加固，仅当前用户可读写（`f26bc04`）。
- 任一层签名失败即判 FAIL，不再降级为 PARTIAL（`e70d28e`）。
- 强制外部 certs 与 EC 内嵌证书一致，不一致 → INVALID（`9e0bfe0`）。

**解析 / 构建 / CLI（ucx-parse / ucx-build / unicodex-core）**
- 校验 ZIP 起始签名 + UCX-Version MAJOR（MAJOR>1 拒绝）（`71ade05`）。
- struct.json 施加路径约束（拒绝 `../`、绝对路径、Windows 保留名）+ 版本格式校验（`61fe040`）。
- `ucx init` 默认拒绝已有项目并提供 `--force`（`85908b8`，`--force` 分支收尾修复见 `259aecb`）。
- 口令最小 8 字符校验，`--allow-weak` 可放行（`fe3401b`）。

**文档**
- UCXE 头部 Reserved byte 重定义为 Flags byte（`af5ad6d`）。

**回归结论**
- 28 项回归：26 FIXED（92.9%）/ 1 NOT-FIXED（NEW-R1-01 `init --force`）/ 1 PARTIAL（NEW-R2-01 AAD reserved bits）；新发现 11 项（0 Critical / 0 High，3 Medium）。报告见 commit `4db8c92`、`docs/test/problems/0.4.0-alpha.2.md`，结论"建议进入 beta"。
- **收尾闭环**：报告遗留的 1 NOT-FIXED 与 1 PARTIAL 已分别由 `259aecb`（`--force`）与 `05dd1f0`（AAD reserved bits）修复。
- 测试数：284（280 单元 + 4 集成；`cargo build` 通过无警告）。

---

## 2026-05-31 — Phase 5 启动：多语言 SDK（14 种语言）

生态建设第一步：为 UCX 标准建立多语言**只读阅读器 SDK**（解析 + BLAKE3 完整性 + Ed25519 双层签名验证 + UCXE 解密），位于 `sdk/`。

- **14 种语言**：Rust、Go、Python、TypeScript、Java、C#、Kotlin、C++、Ruby、PHP、Swift、Dart、**ArkTS**(HarmonyOS)、**Cangjie**(仓颉)。全部达 Level 3（parse+verify+decrypt，3 算法 + 2 KDF）。
- **统一契约**：`sdk/UCX-FORMAT.md`（字节级 wire-format，源码核验，含 7 处 doc-vs-code 纠错）+ `sdk/SDK-API.md`（统一 API/数据模型/错误/常量/命名映射/能力分级）+ `sdk/testdata/`（共享夹具 + `expected.json`，一致性测试 T1–T10）。
- **验证**：8 种本机工具链构建并跑 T1–T10 全过（Rust/Go/Python/TS/Java/C#/Kotlin/C++）；6 种无工具链者经逐位外部核验（Ruby/PHP/Swift/Dart/ArkTS/Cangjie），fingerprint `c7eda2f7…` 精确匹配、三种解密往返、篡改拒绝。ArkTS、Cangjie 实现中各发现并修复 2 个 Argon2id bug。
- **TypeScript→JS**：tsup 双产物（ESM + CommonJS，ESM-only 依赖内联），`import`/`require` 均可用且带 `.d.ts` 类型声明，无需单独 JS 版。
- **版本号方案**（ADR-012）：SDK 版本 `X.Y.Z`，`X.Y`=所支持 UCX 标准、`Z`=补丁；全部初始 **0.4.0**。
- **仓库结构**：每个 `sdk/<lang>/` 为独立 git 子仓库（可单独开源），父仓库 `.gitignore` 忽略 `/sdk/*/`，仅保留 `sdk/README.md` 索引 + 规范 + 夹具。

> 已知缺口：6 种无工具链 SDK 待真机构建回归；Cangjie 的分块(>64MiB)解密与证书有效期窗口校验未实现（见其 README）。详见 [../TODO.md](../TODO.md)。

---

## 2026-05-31 — 全仓库安全审查 + 实机入侵测试 + 修复

对 Rust 工作区（9 crate / ~22.5k LOC）做了一轮授权的大规模安全审查（workflow 多 agent：侦察 + 多维度审查 + 对抗式验证 + 8 类实机入侵测试 + 综合）。实机测试用 debug(带溢出检查)/release 二进制对真实 `ucx` 命令做畸形输入/签名篡改/oracle/资源耗尽攻击，产物隔离于 gitignored `temp_test/audit/`。

**发现并修复（commit `5e47c67`，均经单元测试 + 真实 CLI 端到端再验证）**：
- **[C-1 Critical]** `ucx unpack` Zip-Slip 任意文件写：`ucx-parse::extract_to` 旧守卫只查 `..`/`/` 开头，放过 Windows 盘符/反斜杠绝对路径（实机已写入 `C:\Windows\Temp`）。修复：抽取共享校验器 `ucx-types::path_safety`，生产/消费两侧统一委托。
- **[H-1 High]** 零分块(`chunk_count==0`)UCXE 在任意密钥下被报告解密成功（从不验证 AEAD tag）。修复：双重拒绝 `chunk_count==0`。
- **[M-1/L-2]** 证书 CN 控制字符注入（信任展示伪造）+ 空 CN：创建时校验 + 展示侧 `sanitize_for_display`。
- **[M-2]** `unicodex.toml` 静默接受未知/拼写错误字段：`ProjectConfig` 及各段加 `deny_unknown_fields`（闭合旧 `NEW-R3-01`）。
- **[M-3]** `struct.json` 解析放大 DoS：加大小(16MiB)/节点(100k)/深度(64)上限。
- **[L-1]** 路径未显式拒绝 NUL/控制字符：共享校验器覆盖（闭合旧 `NEW-R3-02`）。

**未触发/确认稳健**（实机）：无 panic/挂起/OOM/错误退出码；crypto 长度字段与 `chunk_count` OOM 上限、防 oracle 统一文案、`signer_id` 白名单、CLI 边界（12 子命令 ~92 用例）均稳健。误报：`main.rs` Base64 密钥 `try_into().unwrap()` 前已校验 `len()==32`。

详见 [decisions.md](./decisions.md) ADR-013、[../TODO.md](../TODO.md)。

### 第二轮（静态审查，commit `08e3b87`）

精简可靠的全仓库静态审查（9 crate × 5 维度 → 对抗式验证，68 条发现，7 High）补齐了第一轮入侵阶段未覆盖的深度，并修复全部 7 High + 多个 Medium：
- **DoS 上界对称化**：M-3 struct.json 上界下沉 `ucx-types::structure` 共享，强制于 build/dry_run/check 与 ucx-version 全部入口（旧版仅 build）；`ucx-parse` 解压路径新增解压炸弹上界。
- **panic 清除**：`ucx-sign::find_signing_block` 加 cd_offset 守卫 + checked 算术（根除 verify() 同源 panic）。
- **分块抗截断**：逐块 AAD 绑入 chunk_count（wire 变更，需同步 SDK）。
- **凭据**：口令 TTY 感知无回显（管道兼容）、口令/明文 zeroize、encrypt/decrypt 原子写。
- **其他**：CRLF 清单数据丢失、build 必填字段、init 模板与 deny_unknown_fields 冲突、version set 降级告警、移除 tokio、全工作区 cargo fmt。

详见 [decisions.md](./decisions.md) ADR-014。

---

## 当前状态 / 下一步

- UCX 标准/工具链：**v0.4.0-alpha.2**，Phase 0–4 完成；git tag 已回补完整（`v0.0.0-alpha.1` → `v0.4.0-alpha.2`）；其后含安全加固 commit `5e47c67`（待下一版本回归）。
- 生态：Phase 5 多语言 SDK 首批 **14 种**已就绪（均 `v0.4.0`，对应 UCX 标准 0.4.x）。
- 待办（beta.1 必修、SDK 真机工具链构建验证、官方服务等）见 [../TODO.md](../TODO.md)；阶段与里程碑见 [../plan/roadmap.md](../plan/roadmap.md)。
