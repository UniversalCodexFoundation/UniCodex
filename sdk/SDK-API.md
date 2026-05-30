# Unicodex SDK — Unified API Contract (统一 API 契约)

> **目的**：保证 12 种语言的 SDK 暴露**同一套概念接口**（操作、数据模型、错误、常量、行为），
> 仅命名大小写按各语言习惯本地化。任何 SDK 都必须遵守本契约 + 字节级 [UCX-FORMAT.md](UCX-FORMAT.md)。
> **范围**：只读阅读器 = 解析(parse) + 完整性校验(integrity) + 签名验证(verify) + 解密(decrypt)。**不含写入/签名/加密**。
> **一致性基线**：所有 SDK 必须通过 [§8 一致性测试](#8-一致性测试-conformance) 对 `testdata/expected.json` 的断言。

---

## 1. 设计原则

1. **概念统一，命名本地化**：同一操作在所有 SDK 中语义、参数、返回结构一致；标识符大小写遵循各语言惯例（§2）。
2. **解析永不失败于未知字段**：JSON 用宽松反序列化，未知字段忽略；缺失可选字段返回 null/None，不报错。
3. **能力分级 + 诚实声明**（§7）：若某语言生态无法实现某层（如缺 Argon2/Ed25519 库），该 SDK 实现到力所能及的等级，并在 README 与 `capabilities()` 中**如实声明**，禁止伪造。
4. **量化常量统一暴露**（§6）：每个 SDK 暴露同名常量（mimetype、magic、算法/KDF ID、长度、默认参数），数值来自 UCX-FORMAT.md。
5. **可独立开源**：每个 SDK 自包含（含本地 `testdata/` 副本与测试），可单独成库。

---

## 2. 命名规范映射

| 语言 | 方法/函数 | 类型 | 字段 | 常量 | 包/命名空间 |
|------|-----------|------|------|------|-------------|
| Rust | `snake_case` | `PascalCase` | `snake_case` | `SCREAMING_SNAKE` | crate `unicodex` |
| Go | `PascalCase`(导出) | `PascalCase` | `PascalCase` | `PascalCase` | module `.../go-ucx`, pkg `ucx` |
| Python | `snake_case` | `PascalCase` | `snake_case` | `SCREAMING_SNAKE` | pkg `unicodex` |
| TypeScript | `camelCase` | `PascalCase` | `camelCase` | `SCREAMING_SNAKE` | `@unicodex/ucx` |
| Java | `camelCase` | `PascalCase` | `camelCase` | `SCREAMING_SNAKE` | `org.unicodex.ucx` |
| C# | `PascalCase` | `PascalCase` | `PascalCase` | `PascalCase` | `Unicodex.Ucx` |
| Ruby | `snake_case` | `PascalCase` | `snake_case` | `SCREAMING_SNAKE` | gem `unicodex` |
| PHP | `camelCase` | `PascalCase` | `camelCase` | `SCREAMING_SNAKE` | `Unicodex\Ucx` |
| Swift | `camelCase` | `PascalCase` | `camelCase` | `camelCase` | module `Unicodex` |
| Kotlin | `camelCase` | `PascalCase` | `camelCase` | `SCREAMING_SNAKE` | `org.unicodex.ucx` |
| C++ | `snake_case` | `PascalCase` | `snake_case` | `SCREAMING_SNAKE` | namespace `unicodex` |
| Dart | `camelCase` | `PascalCase` | `camelCase` | `SCREAMING_SNAKE` | pkg `unicodex` |

> 下文用**语言中立的 `camelCase`** 书写规范名；实现时按上表转换。主归档类型统一命名 **`UcxArchive`**。

---

## 3. 数据模型（Data Model）

所有结构提供一个 `raw` 字段（原始已解析 JSON / map），便于前向兼容未建模字段。可选字段标 `?`。

```
UcxArchive            // open() 返回的句柄，持有解析后的 codex/structure/manifest，可按需读取条目
  codex      -> Codex
  structure  -> Structure
  manifest   -> Manifest
  filePath?  -> string

Codex {
  ucxId: string                 // = identifier.ucx_id，形如 "urn:ucx:{uuidv4}"
  title: Title
  creators: Creator[]           // ≥1
  language: string              // BCP-47
  series?: Series
  publisher?: Publisher
  genre?: string[]
  tags?: string[]
  status?: string               // ongoing|completed|hiatus|abandoned|draft
  wordCount?: u64
  description?: { short?: string, long?: string }
  rights?: { statement?: string, license?: string }
  dates?: { created?: string, published?: string, modified?: string }   // ISO-8601
  cover?: string
  rating?: { system: string, value: string }
  fileVersion?: { version?: string, revision?: u64, releasedAt?: string, changelog?: string }
  identifier: Identifier
  schemaVersion: string         // = codex.json "version"
  raw: object
}
Title { main: string, subtitle?, original?, short? }
Creator { name: string, role: string, signatureRef?: string }
Identifier { ucxId: string, isbn?, issn?, doi?, custom?: map<string,string> }
Series { name: string, index?: u32, total?: u32 }
Publisher { name: string, imprint?, signatureRef?: string }

Structure {
  schemaVersion: string         // = struct.json "version"
  nodes: StructureNode[]
  raw: object
}
StructureNode {                 // 递归：leaf 有 file 无 children；container 有 children 无 file
  title: string                 // 唯一必填
  file?: string                 // 相对 content/ 的路径
  children?: StructureNode[]
  type?: string                 // JSON key "type"
  id?: string
  name?: string
  style?: string
  encryption?: object           // 仅 leaf 有意义；见 UCX-FORMAT §4.2
}

Chapter {                       // chapters() 扁平化后的叶子
  title: string
  file: string                  // 相对 content/，如 "chapter-001.md"
  path: string                  // 归档内绝对路径 "content/{file}"
}

Manifest {
  manifestVersion: string       // "1.0"
  ucxVersion: string            // "1.0"
  createdBy?: string
  hashAlgorithm: string         // "BLAKE3"|"SHA256"|"SHA512"（参考实现只产出 BLAKE3）
  entries: ManifestEntry[]
}
ManifestEntry {
  name: string                  // 归档相对路径
  size: u64                     // 存储(密文)字节数
  digest: string                // Base64-standard(padded) 的原始哈希；NOT hex
  encrypted: bool               // 默认 false
  originalSize?: u64            // 仅加密条目
}

IntegrityResult {
  valid: bool                   // 所有条目均通过
  entries: IntegrityEntry[]
}
IntegrityEntry { name: string, expected: string, actual: string, valid: bool }

SignatureResult {
  status: SignatureStatus       // 枚举，见下
  layer1Present: bool, layer1Valid: bool
  layer2Present: bool, layer2Valid: bool
  signers: Signer[]
}
enum SignatureStatus { UNSIGNED, VERIFIED, VALID_WITH_WARNINGS, INVALID }
Signer {
  signerId: string              // 来自 META-INF/signatures/{SIGNER}.SF 的文件名 stem
  subjectCn?: string
  fingerprint?: string          // lowercase-hex BLAKE3(cert_der)，64 hex
  certType?: string             // "self-signed" | "ca-issued"
  layer1Valid: bool
  layer2Valid: bool
}
```

> **SignatureStatus 判定**严格遵循 UCX-FORMAT §6 状态表：两层都在→任一失败即 `INVALID`；
> 仅一层在且有效→`VALID_WITH_WARNINGS`；都不在→`UNSIGNED`。

---

## 4. 公开操作（Operations）

每个 SDK 必须提供以下操作（命名按 §2 本地化）。`bytes` = 字节序列，`key` = 32 字节。

### 4.1 打开与解析
```
open(path: string) -> UcxArchive            // 读文件→校验 ZIP magic@0→校验 mimetype→解析 codex/struct/manifest
openBytes(data: bytes) -> UcxArchive         // 同上，输入为内存字节（推荐提供）
```
失败抛 `InvalidFormat`（非 ZIP / mimetype 不符 / MAJOR>1）或 `Io`。

### 4.2 元数据与结构（同步访问，open 时已解析）
```
archive.codex -> Codex
archive.structure -> Structure
archive.manifest -> Manifest
archive.chapters() -> Chapter[]              // 深度优先扁平化所有 leaf，按文档顺序
archive.listFiles() -> string[]              // 归档内全部条目名
```

### 4.3 章节读取
```
archive.readChapter(file: string) -> bytes   // 读 content/{file} 的原始字节（可能是 UCXE 密文）
archive.readChapterText(file: string) -> string   // 上者的 UTF-8 解码便捷方法
archive.isChapterEncrypted(file: string) -> bool  // content/{file} 前 4 字节是否 == UCXE magic
```

### 4.4 完整性校验
```
archive.verifyIntegrity() -> IntegrityResult // 对每个 manifest 条目：BLAKE3(解压字节)→Base64→比对 digest
```

### 4.5 签名验证
```
archive.verifySignatures() -> SignatureResult // Layer1(SF/EC Ed25519) + Layer2(签名块 Ed25519)，按 UCX-FORMAT §6
```

### 4.6 解密（模块级函数，作用于 UCXE 字节）
```
isUcxe(data: bytes) -> bool                              // 前 4 字节 == UCXE magic
decryptWithKey(ucxe: bytes, key: bytes/*32*/) -> bytes   // KDF=None 直接密钥；AES-CBC 在此模式被拒
decryptWithPassphrase(ucxe: bytes, passphrase: string) -> bytes  // 口令模式：NFC 归一化→KDF→AEAD/CBC
```
便捷（可选）：`archive.readChapterDecryptedWithKey(file, key)` / `...WithPassphrase(file, passphrase)`。

### 4.7 能力查询
```
capabilities() -> Capabilities   // 见 §7
```

---

## 5. 错误模型（Error Model）

统一**类别**，各语言映射到惯用机制（Rust `Result`/enum，Go `error` 哨兵，异常语言用异常类层级）：

| 类别 | 触发 | 备注 |
|------|------|------|
| `InvalidFormat` | 非 ZIP、mimetype 不符、UCX MAJOR>1、UCXE magic/version 非法 | |
| `NotFound` | 请求的条目/章节不存在 | |
| `ParseError` | JSON / MANIFEST 解析失败 | |
| `Unsupported` | 该 SDK 未实现的算法/KDF/能力 | 对应 §7 未实现项 |
| `DecryptionError` | 任何解密失败 | **不透明**：统一文案，不泄露具体原因（防 oracle，对齐参考实现） |
| `IoError` | 文件读写失败 | I/O 错误可透传 |

> 解密失败必须折叠为单一不透明错误（参考实现 `ucx-crypto/src/lib.rs:138-152`）。

---

## 6. 量化常量（每个 SDK 暴露同名常量）

数值取自 [UCX-FORMAT.md](UCX-FORMAT.md) 与 Appendix A（已源码核验）。

```
MIMETYPE                    = "application/vnd.unicodex+zip"   // 28 字节，无尾随换行
ZIP_MAGIC                   = [0x50,0x4B,0x03,0x04]            // 必须在 offset 0
UCXE_MAGIC                  = [0x55,0x43,0x58,0x45]            // "UCXE"
UCXE_FORMAT_VERSION         = 0x01
SUPPORTED_UCX_MAJOR         = 1                                // 解析拒绝 MAJOR>1

// 对称算法 ID（UCXE 头部第 6 字节）
ALGO_AES_256_GCM            = 0x01   // nonce 12, tag 16
ALGO_AES_256_CBC            = 0x02   // iv 16,   mac 32 (HMAC-SHA256, Encrypt-then-MAC)
ALGO_CHACHA20_POLY1305      = 0x03   // nonce 12, tag 16

// KDF ID（UCXE 头部第 7 字节）
KDF_NONE                    = 0x00   // 直接密钥（无参数块）
KDF_ARGON2ID                = 0x01   // 参数块 12B: mem_kib:u32 ‖ time:u32 ‖ par:u32 (LE)
KDF_PBKDF2_HMAC_SHA256      = 0x02   // 参数块 4B:  iterations:u32 (LE)

// 长度
AEAD_NONCE_LEN              = 12
CBC_IV_LEN                  = 16
GCM_TAG_LEN                 = 16
CHACHA_TAG_LEN             = 16
CBC_MAC_LEN                 = 32
SALT_LEN                    = 16     // 解密路径要求恰好 16

// KDF 默认与边界
ARGON2_VERSION             = 0x13    // v19
ARGON2_DEFAULT_MEM_KIB     = 65536   // 64 MiB；边界 19456..4194304
ARGON2_DEFAULT_TIME        = 3       // 边界 2..100
ARGON2_DEFAULT_PARALLELISM = 4       // ≥1
PBKDF2_DEFAULT_ITERS       = 600000  // 边界 100000..10000000

// 分块（大文件）
CHUNK_THRESHOLD            = 67108864  // 明文 > 64 MiB 才分块（严格 >）
CHUNK_SIZE                 = 1048576   // 1 MiB
// 子 nonce = base_nonce[0..8] ‖ u32_be(chunk_index)

// 签名
SIG_ALGO_ED25519_BLAKE3    = 0x0001
ED25519_SIG_LEN            = 64
ED25519_PUBKEY_LEN         = 32
ED25519_SPKI_OID           = "1.3.101.112"
L2_BLOCK_MAGIC             = "UCX Sig Block 1\0"   // 16 字节
L2_PAIR_ID                 = 0x55435801
L2_CHUNK_PREFIX            = 0xA5
L2_TOP_PREFIX              = 0x5A
```

---

## 7. 能力分级（Capability Levels）

每个 SDK 声明其达到的等级（`capabilities()` 返回布尔集）。**至少 Level 1**。

| 等级 | 能力 | 依赖 |
|------|------|------|
| **L1 解析** | open/codex/structure/chapters/readChapter/listFiles/verifyIntegrity | ZIP + JSON + BLAKE3 + Base64 |
| **L2 验签** | + verifySignatures (Ed25519 双层) | + Ed25519 + X.509/DER + PEM |
| **L3 解密** | + decryptWithKey / decryptWithPassphrase | + AES-GCM/CBC + ChaCha20-Poly1305 + Argon2id + PBKDF2 + Unicode NFC |

```
Capabilities {
  parse: bool          // 恒 true
  integrity: bool      // BLAKE3 可用
  verifySignatures: bool
  decryptDirectKey: bool
  decryptPassphrase: bool
  algorithms: string[] // 实际支持的算法子集，如 ["AES-256-GCM","ChaCha20-Poly1305"]
  kdfs: string[]       // 如 ["argon2id","pbkdf2"]
}
```
> 目标全部 12 SDK 达 **L3**。若某语言缺库（如 PHP 的 BLAKE3、某些 ChaCha20 实现），降级并在 README「Limitations」与 `capabilities()` 中标注；未实现项调用应抛 `Unsupported`，不得静默返回错误数据。

---

## 8. 一致性测试（Conformance）

每个 SDK 必须含针对 `testdata/`（各 SDK 自带副本）与 `testdata/expected.json` 的测试，断言：

| 编号 | 操作 | 期望 |
|------|------|------|
| T1 | `open("sample.ucx").codex` | ucxId=`urn:ucx:fe944340-2d18-4346-ba19-f291e7b00604`；title.main=`Sample Novel`；creators[0].name=`UCX Team`、role=`author`；language=`en` |
| T2 | `chapters()` | `[{title:"第一章", file:"chapter-001.md", path:"content/chapter-001.md"}]` |
| T3 | `readChapterText("chapter-001.md")` | `# Chapter One\n\nThe quick brown fox jumps over the lazy dog.\n` |
| T4 | `verifyIntegrity()`(sample.ucx) | valid=true；3 条目全 valid；digest 用 Base64 比对 |
| T5 | `verifySignatures()`(sample-signed.ucx) | status=VERIFIED；layer1Valid&&layer2Valid；signers[0].signerId=`AUTHOR`、subjectCn=`UCX Sample Signer`、fingerprint=`c7eda2f7b775e395c583c220ff171a7b22c1c0ce3c887b9a3db3d74c944219d0` |
| T6 | `verifySignatures()`(sample.ucx) | status=UNSIGNED |
| T7 | `decryptWithKey(plain-aesgcm.ucxe, KEY)` | == plaintext（KEY=base64 `MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=`） |
| T8 | `decryptWithKey(plain-chacha.ucxe, KEY)` | == plaintext |
| T9 | `decryptWithPassphrase(plain-pass.ucxe, "sdktest-passphrase")` | == plaintext |
| T10 | `decryptWithKey(tampered ucxe, KEY)` | 抛 `DecryptionError`（翻转任意密文/头部字节） |

plaintext = `The quick brown fox jumps over the lazy dog.\n`（45 字节）。
未达对应等级的 SDK 跳过该层测试并在 README 注明。

---

## 9. 每个 SDK 的目录与交付

```
sdk/<lang>/
├── <源码>            // 按语言惯例（src/、lib/ 等）
├── <包清单>          // Cargo.toml / go.mod / pyproject.toml / package.json / pom.xml / *.csproj / *.gemspec / composer.json / Package.swift / build.gradle.kts / CMakeLists.txt / pubspec.yaml
├── README.md         // 安装、用法示例、能力矩阵(capabilities)、Limitations
├── LICENSE           // MIT OR Apache-2.0（与父项目一致，双许可放 LICENSE-MIT / LICENSE-APACHE）
├── testdata/         // 复制自 sdk/testdata/（自包含，便于独立开源）
└── <测试>            // 实现 §8 T1–T10（力所能及）
```
- **Rust SDK** 为 facade：path 依赖复用 `ucx-parse`/`ucx-verify`/`ucx-crypto`，re-export 本契约的整洁 API。
- 各 SDK 为独立 git 子仓库（父仓库 `.gitignore` 已忽略 `/sdk/*/`，仅保留索引/规范/夹具）。

---

*本契约与 [UCX-FORMAT.md](UCX-FORMAT.md) 共同构成 SDK 实现的唯一权威依据。版本：对齐 ucx 0.4.0-alpha.2。*
