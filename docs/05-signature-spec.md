# UCX 签名机制规范

> 版本：0.1.0-draft
> 前置依赖：[01-file-structure.md](./01-file-structure.md)

## 1. 概述

UCX 采用**强制双层签名体系**，借鉴 APK Signature Scheme v1 + v2 的设计，
同时修复 JAR v1 签名已知的安全缺陷。

### 1.1 核心原则：签名验证与高自由度

```
┌─────────────────────────────────────────────────────────────────┐
│                   UCX 签名验证原则（高自由度）                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ● UCX 标准鼓励完整的双层签名验证                               │
│  ● 签名缺失或无效时：警告用户，但允许强制打开                   │
│  ● 用户有权选择信任未签名或签名无效的文件                       │
│  ● 实现应清晰标识文件的签名状态                                 │
│                                                                 │
│  验证状态分级：                                                 │
│  ├── ✓ VERIFIED   - 双层签名完整且有效                          │
│  ├── ⚠ PARTIAL   - 部分签名有效（如仅 Layer 1 通过）            │
│  ├── ⚠ UNSIGNED  - 无签名，用户确认后可打开                     │
│  └── ✗ INVALID   - 签名存在但验证失败，警告后可强制打开         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 为什么需要双层签名

**JAR v1 签名（仅 META-INF）的已知缺陷**：

| 漏洞 | 描述 | 严重程度 |
|------|------|---------|
| ZIP 元数据未保护 | 攻击者可修改 ZIP Central Directory 而不破坏签名 | 高 |
| 可注入额外文件 | 在不影响签名的情况下向 ZIP 中添加文件 | 高 |
| META-INF 目录未保护 | META-INF 中未列入清单的文件可被替换 | 中 |
| Janus 漏洞 (CVE-2017-13156) | 在 ZIP 头部之前注入数据，文件仍通过签名验证 | 严重 |

**UCX 的解决方案**：

```
┌─────────────────────────────────────────────────────────────────┐
│                     UCX 双层签名体系                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Layer 1: META-INF 签名（文件级）【必需】                       │
│  ├── MANIFEST.MF     列出每个文件的哈希                         │
│  ├── {SIGNER}.SF     对 MANIFEST.MF 做摘要                      │
│  └── {SIGNER}.RSA/EC 用私钥对 SF 签名                           │
│  作用：定位哪个具体文件被篡改                                   │
│                                                                 │
│  Layer 2: UCX Signing Block（全局级）【必需】                   │
│  └── 嵌入 ZIP 结构中，对整个 ZIP 做全局签名                     │
│  作用：防止 ZIP 结构篡改、注入攻击、元数据修改                  │
│                                                                 │
│  验证顺序：Layer 2 优先 → 通过后验证 Layer 1 定位细节           │
│  两层都必须通过，否则文件无效                                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 设计参考

| 来源 | 借鉴点 | UCX 中对应 |
|-----|--------|-----------|
| JAR Signing (v1) | MANIFEST.MF / .SF / .RSA 三层 | Layer 1 |
| APK Signature Scheme v2 | ZIP 全局签名块、分块摘要 | Layer 2 |
| APK Signature Scheme v3 | 密钥轮换 proof-of-rotation | 未来扩展预留 |

## 2. Layer 1：META-INF 文件级签名

### 2.1 签名体系结构

```
META-INF/
├── MANIFEST.MF             # 第一层：文件 → 哈希
├── signatures/             # 【必需】至少一个签名者
│   ├── AUTHOR.SF           # 第二层：MANIFEST.MF → 摘要
│   ├── AUTHOR.EC           # 第三层：SF → 数字签名
│   ├── PUBLISHER.SF
│   ├── PUBLISHER.RSA
│   └── ...
└── certs/
    ├── AUTHOR.cert.pem     # 签名者证书
    ├── AUTHOR.chain.pem    # 证书链
    └── ...
```

### 2.2 MANIFEST.MF 规范

**格式**：RFC 822 风格键值对，与 JAR MANIFEST 格式兼容

**编码**：UTF-8，行结束符为 `\r\n`，每行不超过 72 字节（超长行需折行）

**折行规则**：
- 超过 72 字节的行，在第 70 字节处断开
- 续行以一个空格（0x20）开头

**结构**：

```manifest
Manifest-Version: 1.0
UCX-Version: 1.0
Created-By: unicodex 0.1.0
Hash-Algorithm: BLAKE3

Name: metadata/codex.json
Size: 2048
BLAKE3-Digest: rxNJufX5oapgQE3qNtzMlJm8slya3BEr
 fMmjyuQfMmI=

Name: content/struct.json
Size: 512
BLAKE3-Digest: fYZelZskZpGMmGOvyplC0PiJ18msGMm/
 w3SVBe3Zl3A=

Name: content/chapter-001.ucxc
Size: 15360
BLAKE3-Digest: 5M+jmj03vjHFlgnoh5cHmcqmihm/qhUT
 8RZQWOHUHRY=

Name: assets/cover.jpg
Size: 102400
BLAKE3-Digest: n4bQgYhMfWWaL+qgxVrQFaO/TxsrC4Is
 0V1sFbDwCgg=
```

**主块（Main Section）字段**：

| 字段 | 必需 | 说明 |
|-----|------|------|
| `Manifest-Version` | 是 | 固定 `1.0` |
| `UCX-Version` | 是 | UCX 规范版本号 |
| `Created-By` | 否 | 生成工具名称与版本 |
| `Hash-Algorithm` | 是 | 使用的哈希算法标识 |

**条目（Per-Entry Section）字段**：

| 字段 | 必需 | 说明 |
|-----|------|------|
| `Name` | 是 | 文件路径，相对于 ZIP 根目录 |
| `Size` | 是 | 原始文件大小（字节） |
| `{Algorithm}-Digest` | 是 | 文件内容的哈希值，**Base64 编码** |

**不纳入清单的文件**：

| 文件 | 原因 |
|------|------|
| `mimetype` | 固定内容，由规范保证 |
| `META-INF/MANIFEST.MF` | 自身不能包含自身的哈希 |
| `META-INF/signatures/*` | 签名文件不参与自身验证 |
| `META-INF/certs/*` | 证书文件由签名块内部引用验证 |

### 2.3 签名文件（.SF）规范

**位置**：`META-INF/signatures/{SIGNER_ID}.SF`

**命名规则**：
- `SIGNER_ID` 仅允许：大写字母 `A-Z`、数字 `0-9`、下划线 `_`
- 长度限制：1-32 字符
- 预定义标识：`AUTHOR`、`PUBLISHER`、`PLATFORM`、`EDITOR`
- 自定义标识：任意合法名称，如 `COAUTHOR_01`

**格式**：

```manifest
Signature-Version: 1.0
UCX-Version: 1.0
Hash-Algorithm: BLAKE3
Created-By: unicodex 0.1.0

BLAKE3-Digest-Manifest: jH3ZIq1HSU/ALMiOEsAAqs
 EsfdiSiR/qhUT8hUFBe3R=
BLAKE3-Digest-Manifest-Main-Attr: kR7LjKf7zBcU+oP1bHTMvZ
 4dB5f8G8msGMm/w3SVBe0=
```

**字段说明**：

| 字段 | 必需 | 说明 |
|-----|------|------|
| `Signature-Version` | 是 | 固定 `1.0` |
| `UCX-Version` | 是 | UCX 规范版本 |
| `Hash-Algorithm` | 是 | 摘要算法标识 |
| `Created-By` | 否 | 签名工具标识 |
| `{Alg}-Digest-Manifest` | 是 | MANIFEST.MF 整体哈希（Base64） |
| `{Alg}-Digest-Manifest-Main-Attr` | 是 | MANIFEST.MF 主块属性哈希（Base64） |

### 2.4 证书块（.RSA / .EC）规范

**位置**：`META-INF/signatures/{SIGNER_ID}.RSA` 或 `{SIGNER_ID}.EC`

**后缀选择**：

| 后缀 | 适用算法 |
|------|---------|
| `.EC` | Ed25519、ECDSA-P256 |
| `.RSA` | RSA-2048、RSA-4096 |

**格式**：PKCS#7 SignedData 结构（DER 编码，ASN.1）

**内容**：
1. **签名值** - 对同名 `.SF` 文件的数字签名
2. **签名者证书** - X.509 v3 证书（含公钥）
3. **签名算法标识** - OID
4. **可选：证书链** - 签名者证书 → 中间 CA → 根 CA

### 2.5 多签名机制

UCX 支持多个签名者**独立签名、互不干扰**：

```
META-INF/signatures/
├── AUTHOR.SF + AUTHOR.EC           # 作者（Ed25519）
├── PUBLISHER.SF + PUBLISHER.RSA    # 出版方（RSA-4096）
├── PLATFORM.SF + PLATFORM.EC       # 分发平台（Ed25519）
└── COAUTHOR_01.SF + COAUTHOR_01.EC # 联合作者
```

**每个签名者独立完成**：
1. 计算 MANIFEST.MF 的摘要 → 写入自己的 .SF
2. 用自己的私钥对 .SF 签名 → 写入自己的 .RSA/.EC

**验证规则**：
- **至少一个签名者必须验证通过**
- 每个签名者独立验证，互不影响
- 验证结果分别报告
- 应用层可定义更严格策略（如：必须作者+出版方同时通过）

**签名顺序**：
- 签名者之间无顺序依赖
- 所有签名者签的是同一份 MANIFEST.MF

## 3. Layer 2：UCX Signing Block（全局签名）

### 3.1 设计理念

借鉴 APK Signature Scheme v2，对整个 ZIP 文件做全局签名，
保护 ZIP 结构本身不被篡改。

**Layer 2 是强制的**：没有 UCX Signing Block 的文件不是有效的 UCX 文件。

### 3.2 ZIP 文件结构与签名块位置

标准 ZIP 文件结构：

```
┌──────────────────────────────────┐
│  Section 1: ZIP Entries          │ ← 文件数据（Local File Headers + Data）
│  (mimetype, META-INF/*, ...)     │
├──────────────────────────────────┤
│  Section 2: Central Directory    │ ← 文件索引
├──────────────────────────────────┤
│  Section 3: EOCD                 │ ← End of Central Directory
└──────────────────────────────────┘
```

UCX 签名后的结构：

```
┌──────────────────────────────────┐
│  Section 1: ZIP Entries          │ ← 受签名保护
│  (mimetype, META-INF/*, ...)     │
├──────────────────────────────────┤
│  Section 2: UCX Signing Block    │ ← 签名块【必需】
├──────────────────────────────────┤
│  Section 3: Central Directory    │ ← 受签名保护
├──────────────────────────────────┤
│  Section 4: EOCD                 │ ← 受签名保护（修正偏移量后）
└──────────────────────────────────┘
```

**签名块插入在 ZIP Entries 和 Central Directory 之间**，
与 APK v2 相同。这确保标准 ZIP 工具仍可正确解压文件。

### 3.3 UCX Signing Block 格式

```
┌─────────────────────────────────────────────────────────┐
│  size_of_block          (uint64, little-endian)         │ ← 块大小（不含此字段和尾部）
├─────────────────────────────────────────────────────────┤
│  ID-Value Pairs:                                        │
│  ┌───────────────────────────────────────────────────┐  │
│  │  pair_size     (uint64)                           │  │
│  │  pair_id       (uint32)                           │  │
│  │  pair_value    (variable length)                  │  │
│  └───────────────────────────────────────────────────┘  │
│  ... (more pairs)                                       │
├─────────────────────────────────────────────────────────┤
│  size_of_block          (uint64, little-endian)         │ ← 块大小（重复，用于反向定位）
├─────────────────────────────────────────────────────────┤
│  magic                  (16 bytes)                      │ ← "UCX Sig Block 1"
└─────────────────────────────────────────────────────────┘
```

**定位方式**：
1. 从 EOCD 找到 Central Directory 偏移
2. 在 Central Directory 之前查找 magic `UCX Sig Block 1`
3. 通过 `size_of_block` 反向定位块起始位置
4. **若找不到签名块 → 文件无效**

**Magic 值**：
```
ASCII: "UCX Sig Block 1"
Hex:   55 43 58 20 53 69 67 20 42 6C 6F 63 6B 20 31 00
```

### 3.4 Pair ID 分配

| ID | 名称 | 说明 |
|----|------|------|
| `0x55435801` | UCX Signature Scheme v1 | UCX 全局签名数据【必需】 |
| `0x55435802` | 保留 | UCX Signature Scheme v2（未来扩展） |
| `0x55435810` | Timestamp | 可信时间戳数据 |
| `0x55435820` | Key Rotation | 密钥轮换证明（未来扩展） |
| `0x00000000` - `0x554257FF` | 保留 | 规范保留 |
| `0x80000000` - `0xFFFFFFFF` | 自定义 | 第三方扩展可用 |

### 3.5 全局签名数据格式（Pair ID: 0x55435801）

```
UCX Signature Scheme v1 Data:
┌─────────────────────────────────────────────────────────┐
│  signers          (length-prefixed sequence)            │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Signer:                                          │  │
│  │  ├── signed_data     (length-prefixed)            │  │
│  │  │   ├── digests     (length-prefixed sequence)   │  │
│  │  │   │   ├── algorithm_id    (uint32)             │  │
│  │  │   │   └── digest          (length-prefixed)    │  │
│  │  │   ├── certificates (length-prefixed sequence)  │  │
│  │  │   │   └── X.509 cert     (DER, length-prefix)  │  │
│  │  │   └── additional_attrs (length-prefixed seq)   │  │
│  │  │       ├── attr_id         (uint32)             │  │
│  │  │       └── attr_value      (length-prefixed)    │  │
│  │  ├── signatures     (length-prefixed sequence)    │  │
│  │  │   ├── algorithm_id    (uint32)                 │  │
│  │  │   └── signature       (length-prefixed)        │  │
│  │  └── public_key     (length-prefixed)             │  │
│  └───────────────────────────────────────────────────┘  │
│  ... (more signers for multi-sign)                      │
└─────────────────────────────────────────────────────────┘
```

### 3.6 内容摘要计算

签名覆盖范围：Section 1 + Section 3 + Section 4（签名块本身除外）

**分块摘要算法**：

```
输入：受保护的数据（Section 1 || Section 3 || Section 4）
分块大小：1 MiB (1,048,576 bytes)

步骤 1：将数据拆分为 1 MiB 的连续块
  chunk[0] = data[0 .. 1MiB]
  chunk[1] = data[1MiB .. 2MiB]
  ...
  chunk[n] = data[n*1MiB .. end]  （最后一块可小于 1 MiB）

步骤 2：计算每个块的摘要
  chunk_digest[i] = HASH(0xa5 || uint32_le(chunk_length) || chunk[i])
  前缀 0xa5 用于区分块摘要与其他哈希用途

步骤 3：计算顶层摘要
  top_digest = HASH(0x5a || uint32_le(chunk_count) || chunk_digest[0] || chunk_digest[1] || ... )
  前缀 0x5a 用于区分顶层摘要
```

**前缀字节说明**：

| 前缀 | 含义 | 用途 |
|------|------|------|
| `0xa5` | Chunk | 标识这是块级摘要 |
| `0x5a` | Top | 标识这是顶层摘要 |

**分块的好处**：
1. 大文件可并行计算摘要
2. 损坏时可定位到具体的 1MiB 区域
3. 流式处理，内存占用恒定

### 3.7 签名算法 ID

| ID | 算法 | 摘要 | 状态 |
|----|------|------|------|
| `0x0001` | Ed25519 | BLAKE3 | **推荐** |
| `0x0002` | Ed25519 | SHA-256 | 支持 |
| `0x0101` | ECDSA-P256 | SHA-256 | 支持 |
| `0x0201` | RSA-PKCS1-v1.5 | SHA-256 | 兼容 |
| `0x0202` | RSA-PKCS1-v1.5 | SHA-512 | 兼容 |
| `0x0301` | RSA-PSS | SHA-256 | 支持 |
| `0x0302` | RSA-PSS | SHA-512 | 支持 |

## 4. 完整签名流程

### 4.1 打包与签名

```
┌──────────────────────────────────────────────────────────────┐
│                      UCX 签名流程                            │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  阶段 1：构建 ZIP                                            │
│  ├── 写入 mimetype（STORED，第一个条目）                     │
│  ├── 写入 content/、metadata/、assets/、extensions/、chain/  │
│  └── 生成基础 ZIP（Section 1 + Section 3 + Section 4）       │
│                                                              │
│  阶段 2：生成 Layer 1 签名                                   │
│  ├── 遍历所有受保护文件，计算哈希                            │
│  ├── 生成 META-INF/MANIFEST.MF                               │
│  ├── 对每个签名者：                                          │
│  │   ├── 计算 MANIFEST.MF 的摘要 → {SIGNER}.SF               │
│  │   └── 用私钥对 SF 签名 → {SIGNER}.RSA/.EC                 │
│  └── 将签名文件写入 ZIP                                      │
│                                                              │
│  阶段 3：生成 Layer 2 签名【必需】                           │
│  ├── 确定 Section 1, Section 3, Section 4 的内容             │
│  ├── 分块计算摘要（1 MiB 块）                                │
│  ├── 对每个签名者：                                          │
│  │   ├── 构建 signed_data（摘要 + 证书 + 属性）              │
│  │   └── 用私钥对 signed_data 签名                           │
│  ├── 构建 UCX Signing Block                                  │
│  └── 插入到 Section 1 和 Section 3 之间                      │
│                                                              │
│  阶段 4：修正 ZIP 偏移                                       │
│  ├── Central Directory 偏移量增加 Signing Block 大小         │
│  └── EOCD 中的 Central Directory 偏移量对应更新              │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### 4.2 验证流程

```
┌──────────────────────────────────────────────────────────────┐
│                    UCX 验证流程（高自由度）                  │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  步骤 1：基础检查                                            │
│  ├── 检查文件是否为有效 ZIP                                  │
│  ├── 读取 mimetype，确认为 application/vnd.unicodex+zip      │
│  └── 检查 UCX Signing Block 是否存在                         │
│                                                              │
│  步骤 2：Layer 2 验证（全局完整性）                          │
│  ├── 定位 UCX Signing Block（通过 magic 反向查找）           │
│  ├── 若不存在 → 标记为 UNSIGNED                              │
│  ├── 解析签名数据                                            │
│  ├── 对每个 signer：                                         │
│  │   ├── 验证 public_key 与 certificates 中的公钥一致        │
│  │   ├── 验证 signatures 对 signed_data 的签名有效           │
│  │   ├── 重新计算 Section 1 + 3 + 4 的分块摘要               │
│  │   └── 比对摘要值                                          │
│  └── 记录 Layer 2 验证状态                                   │
│                                                              │
│  步骤 3：Layer 1 验证（文件级完整性）                        │
│  ├── 解析 META-INF/MANIFEST.MF                               │
│  ├── 逐文件计算哈希，与 MANIFEST.MF 记录比对                 │
│  ├── 对每个签名者：                                          │
│  │   ├── 读取 {SIGNER}.SF                                    │
│  │   ├── 验证 MANIFEST.MF 摘要                               │
│  │   ├── 读取 {SIGNER}.RSA/.EC                               │
│  │   └── 用证书公钥验证 SF 的签名                            │
│  └── 记录 Layer 1 验证状态                                   │
│                                                              │
│  步骤 4：汇总结果与决策                                      │
│  ├── archive_integrity: Layer 2 状态                         │
│  ├── file_integrity: 各文件哈希状态                          │
│  ├── signers[]: 各签名者验证状态与身份信息                   │
│  │                                                           │
│  ├── 【VERIFIED】全部通过 → 正常打开，显示签名者信息         │
│  ├── 【PARTIAL】部分通过 → 警告，允许打开                    │
│  ├── 【UNSIGNED】无签名 → 警告，用户确认后可打开             │
│  └── 【INVALID】签名无效 → 强警告，用户确认后可强制打开      │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## 5. 证书规范

### 5.1 X.509 证书要求

UCX 签名使用标准 X.509 v3 证书，要求：

| 字段 | 要求 |
|------|------|
| Version | v3 |
| Serial Number | 唯一 |
| Validity | 合法的 notBefore / notAfter |
| Subject | 包含签名者身份信息 |
| Public Key | 与签名算法匹配 |
| Key Usage | 必须包含 `digitalSignature` |
| Extended Key Usage | 建议包含 `codeSigning` |

### 5.2 证书存储

**签名块内**：`.RSA/.EC` 文件中的 PKCS#7 结构包含签名者证书

**证书链**：`META-INF/certs/{SIGNER_ID}.chain.pem` 存储完整链

```
META-INF/certs/
├── AUTHOR.cert.pem          # 作者证书（单个）
├── AUTHOR.chain.pem         # 作者证书链（完整：签名者 → 中间 CA → 根 CA）
├── PUBLISHER.cert.pem       # 出版方证书
└── PUBLISHER.chain.pem      # 出版方证书链
```

### 5.3 自签名证书

UCX 允许使用自签名证书（适用于个人作者、测试场景）：
- 验证工具应标注 `self-signed`
- 信任判断由应用层决定
- 建议：正式发布作品使用 CA 签发的证书

### 5.4 证书指纹

每个证书应计算指纹，用于快速比对：

```
BLAKE3 指纹：BLAKE3(DER 编码的证书)
SHA-256 指纹：SHA-256(DER 编码的证书)（兼容性用途）
```

## 6. 签名算法详细规范

### 6.1 Ed25519（推荐）

```
密钥生成：Ed25519 标准流程
私钥长度：32 bytes (256 bit)
公钥长度：32 bytes (256 bit)
签名长度：64 bytes (512 bit)
哈希函数：内置 SHA-512（Ed25519 标准）
特点：
  - 确定性签名（相同输入 → 相同签名）
  - 无需额外随机数生成
  - 抗侧信道攻击
```

### 6.2 ECDSA-P256

```
曲线：secp256r1 (NIST P-256)
私钥长度：32 bytes
公钥长度：64 bytes (uncompressed) / 33 bytes (compressed)
签名长度：64 bytes (r || s)
哈希函数：SHA-256
特点：
  - 广泛的硬件/软件支持
  - NIST 标准，合规场景友好
```

### 6.3 RSA（兼容）

```
密钥长度：2048 bit（最低） / 4096 bit（推荐）
填充方案：PKCS#1 v1.5 或 PSS
哈希函数：SHA-256 / SHA-512
特点：
  - 最广泛的生态兼容性
  - 密钥和签名体积大
  - 计算较慢
```

## 7. 安全考量

### 7.1 已应对的攻击向量

| 攻击方式 | 防护措施 |
|---------|---------|
| 修改文件内容 | Layer 1 (MANIFEST.MF 哈希) + Layer 2 (全局摘要) |
| 添加/删除文件 | MANIFEST.MF 文件列表校验 |
| 修改 ZIP 元数据 | Layer 2 全局签名覆盖 Central Directory 和 EOCD |
| 注入攻击 (Janus) | Layer 2 从 ZIP 第一字节开始签名 |
| 剥离签名 | **强制双层签名**，缺少任一层即为无效文件 |
| 伪造签名者身份 | X.509 证书链验证 |
| 重放旧版本 | Manifest 中的时间戳 + 可选区块链存证 |
| 降级攻击 | 版本号检查 + 强制签名机制 |

### 7.2 信任模型

由于 UCX 强制要求签名，不存在"无签名"的有效文件状态。

```
┌─────────────────────────────────────────────────────┐
│                   信任层级                          │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Level 1: 自签名证书                                │
│  └── 可验证"同一把密钥签署"，但无法确认真实身份     │
│                                                     │
│  Level 2: CA 签发证书                               │
│  └── 可验证签名者真实身份                           │
│                                                     │
│  Level 3: CA 证书 + 区块链存证                      │
│  └── 身份可验 + 存在性可追溯 + 时间可证             │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### 7.3 实现者安全建议

1. **私钥保护**：私钥绝不能写入 UCX 文件，建议使用硬件安全模块 (HSM)
2. **算法协商**：验证时应拒绝不安全的算法（如未来被攻破的算法）
3. **时间戳**：签名时建议包含可信时间戳（RFC 3161）
4. **证书吊销**：实现应支持 CRL 或 OCSP 检查
5. **最小权限**：签名工具应仅读取必要文件，不写入私钥到磁盘

---

## 附录 A：签名工具命令行示例（参考）

```bash
# 生成密钥对（Ed25519）
ucx keygen --algorithm ed25519 --output author.key

# 签名文件（Layer 1 + Layer 2，强制两层都生成）
ucx sign --key author.key \
         --cert author.cert.pem \
         --signer-id AUTHOR \
         novel.ucx

# 追加出版方签名（多签名）
ucx sign --key publisher.key \
         --cert publisher.cert.pem \
         --signer-id PUBLISHER \
         novel.ucx

# 验证签名（强制双层验证）
ucx verify novel.ucx

# 验证输出示例
# ✓ UCX Signing Block: PRESENT
# ✓ Archive integrity: PASSED (Layer 2, Ed25519+BLAKE3)
# ✓ File integrity: 12/12 files verified (Layer 1)
# ✓ Signer AUTHOR: VALID (Ed25519, self-signed)
# ✓ Signer PUBLISHER: VALID (RSA-4096, CA: Example CA)
# → File is VALID
```

---

*请审阅并指出需要修改的部分*
