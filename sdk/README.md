# Unicodex SDKs — 多语言阅读器 SDK 索引

本目录是 **Unicodex / UCX** 小说容器格式的多语言 **只读阅读器 SDK** 集合。
每个 SDK 实现统一的四项能力：**解析(parse) + 完整性校验(integrity) + 双层签名验证(verify) + UCXE 解密(decrypt)**，不含写入/签名/加密。

所有 SDK 严格对齐两份权威契约：

- [`UCX-FORMAT.md`](UCX-FORMAT.md) —— **字节级 wire-format 规范**（ZIP 容器、MANIFEST、双层 Ed25519 签名、UCXE 加密、KDF、分块；每条事实标注参考实现 `源文件:行号`）。
- [`SDK-API.md`](SDK-API.md) —— **统一 API 契约**（公开操作、数据模型、错误模型、量化常量、各语言命名映射、能力分级、一致性测试 T1–T10）。
- [`testdata/`](testdata/) —— 共享一致性夹具（`sample.ucx` / `sample-signed.ucx` / `plain-{aesgcm,chacha,pass}.ucxe` / `author.cert.pem`）+ [`testdata/expected.json`](testdata/expected.json) 期望值。每个 SDK 自带一份副本以便独立开源。

---

## 能力矩阵（14 种语言）

全部 SDK 均达 **Level 3（parse + verify + decrypt 全功能只读阅读器）**，支持算法 AES-256-GCM / AES-256-CBC+HMAC / ChaCha20-Poly1305，KDF Argon2id / PBKDF2。

| 语言 | 目录 | 能力 | 一致性测试 (T1–T10) | 本机构建 | 主要实现 |
|------|------|------|---------------------|----------|----------|
| Rust | [`rust/`](rust/) | L3 | ✅ 10/10 | ✅ cargo | facade 复用 `ucx-parse`/`ucx-verify`/`ucx-crypto` |
| Go | [`go/`](go/) | L3 | ✅ 10/10 | ✅ go 1.25 | stdlib + `lukechampine/blake3` + `x/crypto` |
| Python | [`python/`](python/) | L3 | ✅ 10/10 | ✅ 3.12 | `cryptography` + `blake3` + `argon2-cffi` |
| TypeScript | [`typescript/`](typescript/) | L3 | ✅ 10/10 | ✅ node 24 | `@noble/*` + `fflate`；**双 ESM/CJS、JS 直接可用** |
| Java | [`java/`](java/) | L3 | ✅ 10/10 | ✅ JDK 21 | BouncyCastle + JCE |
| C# | [`csharp/`](csharp/) | L3 | ✅ 10/10 | ✅ .NET 8 | `Blake3` + BouncyCastle + Konscious Argon2 |
| Kotlin | [`kotlin/`](kotlin/) | L3 | ✅ 10/10 | ✅ JDK21+Gradle | BouncyCastle + JCE |
| C++ | [`cpp/`](cpp/) | L3 | ✅ 10/10 | ✅ cmake | BLAKE3 C + libsodium + mbedtls/OpenSSL |
| Ruby | [`ruby/`](ruby/) | L3 | ✅ 外部核验¹ | ⏸ 无工具链 | OpenSSL + `argon2` gem + 纯 Ruby BLAKE3/ZIP |
| PHP | [`php/`](php/) | L3 | ✅ 外部核验¹ | ⏸ 无工具链 | ext-sodium/openssl + 纯 PHP BLAKE3/Argon2 |
| Swift | [`swift/`](swift/) | L3 | ✅ 外部核验¹ | ⏸ 无工具链 | swift-crypto + 纯 Swift BLAKE3/Argon2/DEFLATE |
| Dart | [`dart/`](dart/) | L3 | ✅ 外部核验¹ | ⏸ 无工具链 | `cryptography` 包 + 纯 Dart BLAKE3/DER |
| ArkTS | [`arkts/`](arkts/) | L3 | ✅ 外部核验² | ⏸ 无工具链 | **100% 纯 ArkTS**（HarmonyOS，无原生 crypto 依赖） |
| Cangjie (仓颉) | [`cangjie/`](cangjie/) | L3 | ✅ 外部核验¹ | ⏸ 无工具链 | 纯仓颉 + stdx.crypto.digest(SHA256/512)（v0.4.1，含分块解密 + 证书时效） |

**构建/测试状态说明**：

- **本机构建 ✅**：宿主机装有该工具链，已实际 `build` + 跑 T1–T10 一致性测试全过（8 种）。
- **⏸ 无工具链**：宿主 Windows 环境无该语言工具链，未实跑；但所有 load-bearing 算法均经**逐位外部核验**——
  - ¹ 将该 SDK 的算法逻辑 1:1 移植到 Python/Node，对照参考库与真实夹具验证（fingerprint `c7eda2f7…44219d0`、三种解密往返、篡改拒绝）。
  - ² ArkTS 提供 8 个 Node(.cjs) 逐行对照脚本（BLAKE3/SHA-512/BLAKE2b/Argon2id/Ed25519/AES-GCM/ChaCha20/Layer1+Layer2 KAT），全部通过。

> 无工具链的 SDK 代码完整、可被装有对应工具链的环境直接构建；任何编译错误应视为待修缺陷。

---

## 版本号方案（ADR-012）

所有 SDK 统一采用 `X.Y.Z` 版本号，均对应 UCX 标准 **`0.4.x`**（首批 `0.4.0`；Cangjie 已发补丁 **`0.4.1`**）：

- **`X.Y`**（前两位）= 所支持的 **UCX 标准版本**（major.minor）。**前两位相同 ⇒ 支持同一 UCX 标准、对外 API 相同**。
- **`Z`**（末位）= 该 SDK 自身的补丁号（修 bug、不改对外 API）。各 SDK 的 `Z` 互相独立递增。

`0.4.x` 对应 UCX 标准 **0.4.x**。UCX 标准升级到下一 minor（如 0.5）时新增 `0.5.x` SDK 线，旧 `0.4.x` 线**持续发布补丁、不被废弃**（类似 Python 多版本系列并行维护）。详见 [`../docs/memory/decisions.md`](../docs/memory/decisions.md) ADR-012。

---

## 仓库结构：独立子仓库 + 父仓库索引

每个 `sdk/<lang>/` 是一个**独立的 git 子仓库**，可单独开源（各自带 `LICENSE`、`README.md`、包清单、`testdata/` 副本与测试）。
父仓库（Unicodex 主仓库）通过 `.gitignore` 忽略 `/sdk/*/`，**只跟踪本目录的索引与共享资产**：`README.md`（本文件）、`UCX-FORMAT.md`、`SDK-API.md`、`testdata/`。

要在父仓库中以 git submodule 形式登记各子仓库（在各 SDK 拥有远程仓库后）：

```bash
# 示例：为某语言 SDK 添加远程后登记为 submodule
git submodule add <remote-url-of-sdk> sdk/<lang>
```

许可：全部 SDK 采用 **MIT**（与主项目一致）。

---

## 一致性测试（Conformance）

每个 SDK 实现 [`SDK-API.md`](SDK-API.md) §8 的 **T1–T10**，断言全部取自 [`testdata/expected.json`](testdata/expected.json)：
解析元数据/章节、`readChapterText`、BLAKE3 完整性、双层签名 `VERIFIED` 与签名者指纹、AES-GCM/ChaCha20 直接密钥解密、Argon2id 口令解密、篡改拒绝。

新增/修改 SDK 时，请同时运行其本地 T1–T10，确保与契约和夹具保持一致。
