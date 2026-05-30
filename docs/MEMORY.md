# Unicodex 实时关键记忆（MEMORY）

> 本文件是所有贡献者 Agent 的**共享长期记忆入口**，每次会话优先阅读。
> 维护原则：只放"当前快照 + 索引 + 关键约定/风险"，详细历史放 `docs/memory/`。
> 最后核验：**2026-04-18**（基于 git HEAD `05dd1f0` 实机核验，可追溯）。

---

## 1. 项目一句话

Unicodex 是统一标准的小说文件标准（UCX 格式），用 Rust 实现完整工具链（CLI 名 `ucx`），
覆盖项目初始化、构建打包、解析读取、版本管理、双层签名验证、加密解密。

---

## 2. 当前真实基线（务必以此为准，勿凭旧文档猜测）

| 项 | 值 | 核验来源 |
|----|----|---------|
| 当前版本 | **v0.4.0-alpha.2** | 根 `Cargo.toml` `[workspace.package] version` |
| git HEAD | `05dd1f0`（在 alpha.2 升级后又有 2 个 fix commit） | `git log` |
| Rust edition | **2024** | 根 `Cargo.toml` |
| rust-version | **1.85** | 根 `Cargo.toml`（仅 ucx-types 显式继承，其余成员未声明） |
| resolver | 3 | 根 `Cargo.toml` |
| license | MIT OR Apache-2.0 | 根 `Cargo.toml` |
| 成员 crate | **9 个** | 见下 §4 |
| 测试总数 | **284**（280 单元 + 4 集成 round_trip） | `grep -rc '#\[test\]' */src` + `tests/` |
| 顶层 CLI 子命令 | **12 个** | `unicodex-core/src/main.rs` `enum Commands` |
| 编译状态 | `cargo build` 通过，无 error/warning | 2026-04-18 实测 |

**Phase 进度**：Phase 0–4 **全部已完成**；Phase 5（生态建设）**进行中**——多语言只读 SDK 首批 **14 种**已就绪（见 [../sdk/README.md](../sdk/README.md)，对应 UCX 标准 0.4.x；首批 v0.4.0，Cangjie 已发补丁 **v0.4.1**：补全分块 UCXE 解密 + 证书有效期窗口校验）。
**⚠️ 无任何 placeholder crate**——Phase 2/3/4 的 ucx-version/sign/verify/crypto 均为生产实现。

---

## 3. 记忆地图（去哪里找什么）

| 路径 | 内容 | 角色 |
|------|------|------|
| `docs/MEMORY.md`（本文件） | 当前快照 + 索引 + 关键约定 | 实时关键记忆（CLAUDE.md 指定） |
| `docs/memory/` | 长期大量记忆（CLAUDE.md 指定） | 详细历史 |
| `docs/memory/progress.md` | 逐版本开发编年史 | 进度史 |
| `docs/TODO.md` | 待办与遗留问题（CLAUDE.md 指定） | 待办 |
| `docs/memory/decisions.md` | 架构决策记录（ADR-001 ~ ADR-012） | 决策史 |
| `docs/memory/modules.md` | 各 crate 设计/API/实现状态 | 模块笔记 |
| `docs/plan/roadmap.md` | 长期路线图 + 阶段总览 | 计划 |
| `docs/plan/phase-1-detailed.md` | Phase 1 详细计划（Phase 2-4 无独立详细计划文档） | 计划 |
| `docs/00-overview.md` ~ `09-official-services.md` | 10 份 UCX 规范文档 | 规范 |
| `docs/quickstart.md` / `docs/workflow.md` | 用户向上手/工作流文档 | 用户文档 |
| `docs/test/problems/X.Y.Z.md` | **每版本一份**回归测试报告（共 6 份） | 测试史 |
| `sdk/README.md` + `sdk/{UCX-FORMAT,SDK-API}.md` + `sdk/testdata/` | 多语言 SDK 索引 + wire-format/API 契约 + 夹具（Phase 5；各 `sdk/<lang>/` 为独立子仓库） | SDK |

> **结构说明**：已对齐 CLAUDE.md 规范——`docs/MEMORY.md`（实时关键记忆）+ `docs/memory/`（长期记忆）
> + `docs/TODO.md`（待办）+ `docs/plan/`（计划）。旧 `docs/development/` 已于本次整理迁移内容后删除。

---

## 4. Workspace 模块（9 crate，全部 version=0.4.0-alpha.2）

```
unicodex/
├── unicodex-core/  ← CLI 入口（bin 名 ucx，main.rs ~1384 LOC，0 单元测试 + 4 集成测试）
├── ucx-types/      ← 公共类型 Codex/Structure/Manifest/ProjectConfig/UcxId（51 测试）
├── ucx-init/       ← 项目初始化（13 测试）
├── ucx-build/      ← 构建打包（24 测试）
├── ucx-parse/      ← 解析读取（15 测试）
├── ucx-verify/     ← 双层签名 + 完整性验证（13 测试）✅ Phase 3
├── ucx-sign/       ← Ed25519 密钥/证书/双层签名（42 测试，7 模块）✅ Phase 3
├── ucx-version/    ← git2 变更检测 + 自动版本（40 测试）✅ Phase 2
└── ucx-crypto/     ← AES-GCM/ChaCha20/AES-CBC + Argon2id/PBKDF2 + UCXE（82 测试，8 模块）✅ Phase 4
```

**12 个顶层 CLI 子命令**：`init` `build` `info` `verify` `check` `unpack` `keygen` `cert`(create/info) `sign` `version`(auto/patch/chapter/volume/set) `encrypt` `decrypt`。

---

## 5. 关键约定与风险（避免后续 Agent 踩坑）

1. **版本封顶 `v0.x.x`（强约束）**：在**全部功能彻底完成前**，版本号一律保持 `v0.x.x`，**禁止设为 `v1.0.0`**。
   `v1.0.0` 仅作为"全功能完成"的终点里程碑 M6。详见 [memory/decisions.md](memory/decisions.md) ADR-011。
2. **git tag**：已回补完整（`v0.0.0-alpha.1` → `v0.4.0-alpha.2`，共 7 个 annotated）。
   今后**每次版本发布必须打 tag**（CLAUDE.md "关键节点设置 tag"），且 tag 与 `Cargo.toml` 版本一致。
3. **`temp_test/` 是未跟踪的测试 scratch**（145M：`ucx.exe` + 922 文件 + 50 个测试私钥），
   已加入 `.gitignore`。正式测试报告在 `docs/test/problems/`，**勿把 temp_test 当正式产物**。
4. **加密在签名之前**（Encrypt-then-Sign，ADR-004）：签名保护密文，无需解密即可验签。
5. **错误文案防 oracle**：解密失败统一返回 `decryption failed`，不泄露具体原因。
6. **路径与输入安全（共享校验器）**：ZIP 条目名与 struct.json `file` 引用统一经 `ucx-types::path_safety::validate_safe_relative_path` 校验——拒绝 `../`/反斜杠/绝对(含 Windows 盘符)/`Path::is_absolute`/Windows 保留名/NUL 与控制字符/尾随点空格。生产侧（`ucx-build`）与消费侧（`ucx-parse::extract_to`）**共用同一函数**，禁止再各写一份（防漂移，曾导致 Zip-Slip C-1）。详见 ADR-013/014。`title` 字段 NUL 仍待校验（AUD-01）。
9. **两轮安全审查已完成**（commit `5e47c67` + `08e3b87`，ADR-013/014）。要点：
   - **struct.json DoS 上界**唯一真值在 `ucx-types::structure`（`MAX_STRUCT_JSON_BYTES=16MiB`/`MAX_STRUCT_NODES=100k`/`MAX_STRUCT_DEPTH=64` + `enforce_structure_limits`），`ucx-build`(build/dry_run/check) 与 `ucx-version` 共用；`ucx-parse` 解压有 512MiB/条目上界（解压炸弹）。
   - **分块 AEAD 的逐块 AAD = `file_aad ‖ u32_le(chunk_count)`**（抗截断，**interop 关键**：所有 SDK 必须同绑 chunk_count，否则分块文件跨实现验证失败；旧分块文件与新版不兼容）。
   - **口令读取 TTY 感知**：交互无回显(rpassword)，管道/CI 读 stdin（勿改回纯 rpassword——Windows 会挂起）；口令/解密明文 `zeroize`；encrypt/decrypt **原子写**（temp+fsync+rename）。
   - `find_signing_block` 等对攻击者可控偏移/长度一律 checked 算术 + `<=len` 守卫（绝不 panic）。
7. **commit 节奏**：每完成一部分即 commit，conventional commits（`feat/fix/docs/chore` + scope）。
8. **多语言 SDK 版本号方案**（Phase 5）：SDK 版本 `X.Y.Z` 中 `X.Y` = 所支持的 UCX 标准版本（**前两位相同 ⇒ 对外 API 相同**），`Z` = SDK 自身补丁号；旧标准线持续发补丁、不废弃（类 Python 多版本并行）。当前全部 SDK 初始版本 = **v0.4.0**（对应 UCX 标准 0.4.x）。详见 [memory/decisions.md](memory/decisions.md) ADR-012。

---

## 6. 下一步（详见 `docs/TODO.md`）

**v0.4.0-beta.1 候选必修**（截至 HEAD 仍未处理）：
- `NEW-R3-01` unicodex.toml 未知字段静默接受（Medium，输入校验）
- `NEW-R3-02` struct.json title 含 NUL 字节静默接受（Medium，输入校验）
- 文档/一致性批：`NEW-R1-02`（sign 自动读 `[signing]`）、`NEW-R1-03`（创建 `docs/errors.md`）、`NEW-R1-04/05`、`NEW-R2-04`

**已修复待回归**（修复 commit 晚于 alpha.2 报告，需在下个版本回归验证）：
- `NEW-R1-01` ucx init --force → `259aecb`
- `NEW-R2-01` AEAD AAD reserved bits → `05dd1f0`

**运维待办**：
- 回补缺失 git tag（v0.4.0-alpha.1→`58dc0d4`、v0.4.0-alpha.2→`b56d1ec`）
- 评估 `temp_test/` 清理（可回收 145M；原始报告确认无独有结论后可删）

**Phase 5 — 生态建设**（目标 v1.0.0）：多语言 SDK（go/py/js-ucx）+ 官方服务（密钥分发/CA/包验证/阅读器 API）。
区块链溯源（规范 `06`）为远期独立扩展，不在关键路径。
