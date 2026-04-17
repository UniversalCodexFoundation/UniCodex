# UCX 加密规范

> 版本：0.1.0-draft
> 前置依赖：[01-file-structure.md](./01-file-structure.md)、[05-signature-spec.md](./05-signature-spec.md)

## 1. 概述

UCX 加密是**可选功能**，允许对章节内容和资源文件进行独立加密。加密不影响签名验证——签名覆盖的是加密后的密文。

### 1.1 设计原则

```
┌─────────────────────────────────────────────────────────────────┐
│                     UCX 加密设计原则                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ● 可选性：加密为可选功能，未加密文件同样是有效 UCX             │
│  ● 独立性：每个章节/资源独立加密，互不影响                      │
│  ● 签名先于加密：加密后的密文参与签名，确保密文完整性           │
│  ● 多方式密钥获取：支持多种密钥获取方式并存                     │
│  ● 算法可选：支持多种加密算法，推荐 AES-256-GCM                │
│  ● 离线优先：加密/解密操作本身不依赖网络                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 加密层级

```
┌─────────────────────────────────────────────────────────────────┐
│                       UCX 加密层级                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  章节级加密（Chapter-Level Encryption）                         │
│  ├── 在 struct.json 中配置 encryption 字段                     │
│  ├── 对整个章节文件加密                                         │
│  └── 每个章节可使用不同的密钥和配置                             │
│                                                                 │
│  资源级加密（Resource-Level Encryption）                        │
│  ├── 在 resource.json 中配置 encryption 字段                   │
│  ├── 对单个资源文件加密                                         │
│  └── 每个资源可使用不同的密钥和配置                             │
│                                                                 │
│  段落级加密（Paragraph-Level Encryption）                       │
│  ├── 在 .ucxc / .mdx 中使用 <encrypted> 标签                  │
│  ├── 对章节内的部分段落加密                                     │
│  └── 详见 03-content-format.md                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 2. 加密算法

### 2.1 支持的算法

| 标识 | 算法 | 模式 | 密钥长度 | IV/Nonce 长度 | Tag 长度 | 状态 |
|------|------|------|---------|--------------|---------|------|
| `AES-256-GCM` | AES-256 | GCM (AEAD) | 256 bit (32 bytes) | 96 bit (12 bytes) | 128 bit (16 bytes) | **推荐** |
| `AES-256-CBC` | AES-256 | CBC + HMAC-SHA256 | 256 bit (32 bytes) | 128 bit (16 bytes) | 256 bit (32 bytes) | 支持 |
| `ChaCha20-Poly1305` | ChaCha20 | Poly1305 (AEAD) | 256 bit (32 bytes) | 96 bit (12 bytes) | 128 bit (16 bytes) | 支持 |

### 2.2 AES-256-GCM（推荐）

**推荐理由**：

1. **AEAD**：同时提供加密和认证，无需单独的 HMAC
2. **硬件加速**：现代 CPU 内置 AES-NI 指令集
3. **标准化**：NIST SP 800-38D，广泛使用
4. **流式处理**：支持分块加密，适合大文件

**参数**：

```
密钥长度   : 256 bit (32 bytes)
IV 长度    : 96 bit (12 bytes)，必须唯一
Tag 长度   : 128 bit (16 bytes)
最大明文   : 2^39 - 256 bit（约 64 GiB）
```

**安全要求**：
- 每次加密**必须**使用不同的 IV（随机生成或计数器模式）
- 同一密钥下 IV 不得重复，否则安全性完全丧失
- 推荐每个密钥最多加密 2^32 条消息

### 2.3 AES-256-CBC

**参数**：

```
密钥长度         : 256 bit (32 bytes)
IV 长度          : 128 bit (16 bytes)，必须随机
填充方式         : PKCS#7
认证方式         : HMAC-SHA256（Encrypt-then-MAC）
HMAC 密钥长度    : 256 bit (32 bytes)
```

**Encrypt-then-MAC 流程**：
1. 使用 PKCS#7 填充明文
2. AES-256-CBC 加密（得到密文）
3. HMAC-SHA256(MAC 密钥, IV || 密文) → 认证标签
4. 输出：IV || 密文 || HMAC 标签

**安全要求**：
- 加密密钥和 MAC 密钥**必须不同**
- IV 必须随机生成，不可预测
- 验证时必须先校验 HMAC，再解密（防止 Padding Oracle 攻击）

### 2.4 ChaCha20-Poly1305

**参数**：

```
密钥长度   : 256 bit (32 bytes)
Nonce 长度 : 96 bit (12 bytes)
Tag 长度   : 128 bit (16 bytes)
```

**适用场景**：
- 无 AES 硬件加速的设备（如某些移动设备/嵌入式平台）
- 软件实现性能优于 AES-GCM

## 3. 密钥派生

### 3.1 从密码派生密钥

当用户提供密码（passphrase）作为加密依据时，使用密钥派生函数（KDF）生成加密密钥。

**推荐 KDF**：Argon2id

```
算法         : Argon2id
内存开销     : ≥ 64 MiB
迭代次数     : ≥ 3
并行度       : 4
Salt 长度    : 128 bit (16 bytes)，随机生成
输出密钥长度 : 256 bit (32 bytes)
```

**Argon2id 参数上下限**（实现者 MUST 校验）：

| 参数          | 最小值     | 最大值       | 说明                                                 |
|---------------|-----------|-------------|------------------------------------------------------|
| memory (KiB)  | 8 192     | 4 194 304   | 8 MiB 下限防弱参数；4 GiB 上限防 DoS 风险。         |
| iterations    | 1         | 10          | 过高迭代会拖慢合法用户，实测 10 已覆盖极端场景。    |
| parallelism   | 1         | 255         | 与 Argon2 规范上限一致。                             |
| output length | 32        | 64          | UCX 仅使用 32 或 64 字节（CBC 模式下需双密钥）。    |

**备选 KDF**：PBKDF2-HMAC-SHA256

```
算法         : PBKDF2
PRF          : HMAC-SHA256
迭代次数     : ≥ 600,000
Salt 长度    : 128 bit (16 bytes)，随机生成
输出密钥长度 : 256 bit (32 bytes)
```

**PBKDF2 参数上下限**（实现者 MUST 校验）：

| 参数        | 最小值     | 最大值        | 说明                                         |
|-------------|-----------|--------------|----------------------------------------------|
| iterations  | 100 000   | 10 000 000   | 下限符合 OWASP 2023 基线；上限防 DoS。      |
| salt (bytes)| 16        | 64           | 16 字节是碰撞安全最小值；过长无益。         |

> 解析 UCXE 头部时若发现 KDF 参数越界，实现 MUST 拒绝解密并返回明确错误，不得尝试解密后失败。

### 3.2 密钥派生流程

```
┌─────────────────────────────────────────────────────────────────┐
│                     密钥派生流程                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  输入：                                                         │
│  ├── passphrase (用户密码)                                      │
│  └── salt (随机生成的盐值)                                      │
│                                                                 │
│  AES-256-GCM / ChaCha20-Poly1305：                             │
│  └── content_key = KDF(passphrase, salt, 32 bytes)             │
│                                                                 │
│  AES-256-CBC（需要两个密钥）：                                  │
│  ├── derived = KDF(passphrase, salt, 64 bytes)                 │
│  ├── enc_key = derived[0..32]    (加密密钥)                    │
│  └── mac_key = derived[32..64]   (MAC 密钥)                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 4. 加密文件格式

### 4.1 密文文件结构

加密后的文件替换原始文件，文件扩展名不变。文件内容遵循以下二进制格式：

```
┌─────────────────────────────────────────────────────────────────┐
│                     UCX 加密文件格式                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────────────────────────────────┐              │
│  │  Magic Number      (4 bytes)                  │  "UCXE"     │
│  ├───────────────────────────────────────────────┤              │
│  │  Format Version    (1 byte)                   │  0x01       │
│  ├───────────────────────────────────────────────┤              │
│  │  Algorithm ID      (1 byte)                   │  算法标识   │
│  ├───────────────────────────────────────────────┤              │
│  │  KDF ID            (1 byte)                   │  KDF 标识   │
│  ├───────────────────────────────────────────────┤              │
│  │  Flags             (1 byte)                   │  见下方     │
│  ├───────────────────────────────────────────────┤              │
│  │  Salt Length        (2 bytes, LE)             │              │
│  ├───────────────────────────────────────────────┤              │
│  │  Salt              (variable)                 │              │
│  ├───────────────────────────────────────────────┤              │
│  │  IV/Nonce Length    (2 bytes, LE)             │              │
│  ├───────────────────────────────────────────────┤              │
│  │  IV/Nonce          (variable)                 │              │
│  ├───────────────────────────────────────────────┤              │
│  │  Ciphertext Length  (8 bytes, LE)             │              │
│  ├───────────────────────────────────────────────┤              │
│  │  Ciphertext         (variable)                │              │
│  ├───────────────────────────────────────────────┤              │
│  │  Auth Tag           (variable, 算法决定)      │              │
│  └───────────────────────────────────────────────┘              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Magic Number**：
```
ASCII: "UCXE"
Hex:   55 43 58 45
```

**Algorithm ID**：

| ID | 算法 |
|----|------|
| `0x01` | AES-256-GCM |
| `0x02` | AES-256-CBC (Encrypt-then-MAC) |
| `0x03` | ChaCha20-Poly1305 |

**KDF ID**：

| ID | KDF | 说明 |
|----|-----|------|
| `0x00` | 无 KDF | 密钥直接提供（非密码派生） |
| `0x01` | Argon2id | 推荐 |
| `0x02` | PBKDF2-HMAC-SHA256 | 兼容 |

**Flags**：

| Bit | 名称 | 说明 |
|-----|------|------|
| bit 0 | Chunked | `0` = 普通加密（整体加密），`1` = 分块加密（Chunked Encryption，见 §8.3） |
| bit 1–7 | Reserved | 保留位，**必须为 `0`**。阅读器应忽略未知 bit，写入时必须置零 |

**AAD 构造规则**（AES-256-GCM / ChaCha20-Poly1305 的 Associated Authenticated Data）：

AEAD 算法 MUST 将 UCXE 文件头绑定到密文，防止攻击者剥离或替换头部字段（典型攻击：把 Algorithm ID 从 `0x01` 改为 `0x02` 以降级、把 Flags 的 Chunked bit 由 `1` 改为 `0` 以触发非法长度解析等）。

AAD 按下列字节序拼接，总长度固定为头部前 8 字节（至 Salt Length 为止）：

```
AAD = MagicNumber(4) || FormatVersion(1) || AlgorithmID(1) || KdfID(1) || Flags(1)
```

即整个 UCXE 固定头部，不含可变长度的 Salt / IV / KDF 参数区（这些字段在其内部已由密文完整性覆盖）。

- 长度固定为 **8 字节**；无需长度前缀。
- 解密时实现 MUST 从实际读取的头部重新组装 AAD，并传入 AEAD `decrypt(..., aad)`；任何头部篡改将导致 AEAD tag 校验失败。
- 对 AES-256-CBC（Encrypt-then-HMAC）同样的 8 字节前缀 MUST 被 HMAC 覆盖，详见 §4.3。

### 4.2 KDF 参数存储

当 KDF ID ≠ 0x00 时，在 Salt 字段之前插入 KDF 参数块：

**Argon2id 参数**：

```
┌─────────────────────────────────────────────┐
│  Memory Cost      (4 bytes, LE)  以 KiB 为单位 │
│  Time Cost        (4 bytes, LE)  迭代次数      │
│  Parallelism      (4 bytes, LE)  并行度        │
└─────────────────────────────────────────────┘
```

**PBKDF2 参数**：

```
┌─────────────────────────────────────────────┐
│  Iterations       (4 bytes, LE)  迭代次数      │
└─────────────────────────────────────────────┘
```

### 4.3 各算法的 Auth Tag

| 算法 | Auth Tag 内容 | 长度 |
|------|-------------|------|
| AES-256-GCM | GCM Authentication Tag | 16 bytes |
| AES-256-CBC | HMAC-SHA256(mac_key, IV \|\| ciphertext) | 32 bytes |
| ChaCha20-Poly1305 | Poly1305 Authentication Tag | 16 bytes |

## 5. 加密流程

### 5.1 章节级加密

```
┌─────────────────────────────────────────────────────────────────┐
│                   章节加密流程                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  输入：                                                         │
│  ├── 明文章节文件（如 chapter-002.md）                          │
│  ├── struct.json 中的 encryption 配置                           │
│  └── 加密密钥（直接提供或从密码派生）                           │
│                                                                 │
│  步骤 1：读取明文                                               │
│  └── plaintext = read("content/chapter-002.md")                │
│                                                                 │
│  步骤 2：生成随机 IV/Nonce                                      │
│  └── iv = random_bytes(12)  // AES-GCM                         │
│                                                                 │
│  步骤 3：加密                                                   │
│  └── (ciphertext, tag) = AES-256-GCM.encrypt(key, iv, plaintext)│
│                                                                 │
│  步骤 4：组装密文文件                                           │
│  └── encrypted_file = UCXE_HEADER + salt + iv + ciphertext + tag│
│                                                                 │
│  步骤 5：替换原文件                                             │
│  └── write("content/chapter-002.md", encrypted_file)           │
│      文件名不变，内容变为密文                                   │
│                                                                 │
│  步骤 6：更新 MANIFEST.MF                                       │
│  └── 哈希值为加密后文件的哈希（密文参与签名）                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 解密流程

```
┌─────────────────────────────────────────────────────────────────┐
│                   章节解密流程                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  步骤 1：读取密文文件                                           │
│  └── data = read("content/chapter-002.md")                     │
│                                                                 │
│  步骤 2：检查 Magic Number                                      │
│  ├── data[0..4] == "UCXE" → 加密文件                           │
│  └── 否则 → 明文文件，直接返回                                  │
│                                                                 │
│  步骤 3：解析文件头                                             │
│  ├── algorithm_id = data[5]                                    │
│  ├── kdf_id = data[6]                                          │
│  ├── salt, iv, ciphertext, tag = parse_fields(data)            │
│  └── 若有 KDF 参数，一并解析                                    │
│                                                                 │
│  步骤 4：获取密钥                                               │
│  ├── 读取 struct.json 中的 key_access 配置                     │
│  ├── 按 key_access 数组顺序尝试获取密钥                        │
│  │   ├── direct → 直接使用内嵌密钥                              │
│  │   ├── message → 向用户展示提示信息                           │
│  │   ├── url → 请求 URL 获取密钥                                │
│  │   ├── service → 调用服务获取密钥                             │
│  │   └── extension → 调用扩展获取密钥                           │
│  └── 若需 KDF → 用获取的密码 + salt + KDF 参数派生密钥         │
│                                                                 │
│  步骤 5：验证并解密                                             │
│  ├── 先验证 Auth Tag（防止解密伪造数据）                       │
│  └── plaintext = decrypt(key, iv, ciphertext, tag)             │
│                                                                 │
│  步骤 6：返回明文内容                                           │
│  └── 将解密后的明文交给章节解析器处理                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 5.3 段落级加密

段落级加密用于 `.ucxc` 和 `.mdx` 格式中对章节内部分内容加密。

**加密流程**：

1. 解析章节文件，定位 `<encrypted>` / `:::encrypted` 标记的区域
2. 提取区域内的明文内容
3. 使用与章节级相同的算法加密
4. 将密文以 Base64 编码嵌入标签的 `<content>` 内

**密文存储格式**（在 `<content>` 标签内）：

```xml
<encrypted algorithm="AES-256-GCM">
  <key-access method="service" provider="https://api.unicodex.org/keys" service-type="purchase_verify">
    <param name="product_id">novel-001-ch02</param>
  </key-access>
  <content encoding="base64">
    UCXE... (Base64 编码的加密数据，格式同 §4.1，但不含 UCXE 文件头)
    <!-- 格式：iv(12B) || ciphertext || tag(16B)，Base64 编码 -->
  </content>
</encrypted>
```

**段落级加密的密文格式**（Base64 解码后）：

```
IV/Nonce (12 bytes) || Ciphertext (variable) || Auth Tag (16 bytes)
```

与文件级加密不同，段落级加密不使用 UCXE 文件头（因为算法信息已在标签属性中声明）。

## 6. 加密与签名的关系

### 6.1 处理顺序

```
┌─────────────────────────────────────────────────────────────────┐
│                   加密与签名顺序                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  打包流程（加密在签名之前）：                                   │
│                                                                 │
│  1. 准备所有明文文件                                            │
│  2. 按 struct.json / resource.json 配置加密文件                │
│     └── 明文 → 密文（UCXE 格式）                               │
│  3. 生成 MANIFEST.MF（对密文计算哈希）                          │
│  4. 生成签名（Layer 1 + Layer 2）                               │
│                                                                 │
│  验证流程（先验签名，再解密）：                                 │
│                                                                 │
│  1. Layer 2 全局签名验证                                        │
│  2. Layer 1 文件级签名验证                                      │
│  3. MANIFEST.MF 哈希验证（对密文校验）                          │
│  4. 获取密钥 → 解密                                             │
│                                                                 │
│  关键：签名保护的是密文，因此：                                 │
│  ● 篡改密文 → 签名验证失败                                     │
│  ● 替换密文 → 哈希不匹配                                       │
│  ● 无需解密即可验证文件完整性                                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 MANIFEST.MF 中的加密文件

加密文件在 MANIFEST.MF 中记录的是**密文**的哈希：

```manifest
Name: content/chapter-002.md
Size: 18432
BLAKE3-Digest: 7d865e959b2466918c9863afca942d0fb89d7c9ac0c99bafc3749504ded97730
Encrypted: true
Original-Size: 15360
```

| 字段 | 说明 |
|------|------|
| `Size` | 密文文件大小（字节） |
| `BLAKE3-Digest` | 密文文件的哈希值 |
| `Encrypted` | 加密标记（`true`），可选字段 |
| `Original-Size` | 原始明文大小（字节），可选字段 |

## 7. 密钥获取方式

密钥获取方式在 `struct.json` 和 `resource.json` 的 `key_access` 数组中配置。详细说明见 [01-file-structure.md §4.1.1](./01-file-structure.md)。

### 7.1 汇总

| method | 说明 | 是否需要网络 |
|--------|------|-------------|
| `direct` | 密钥直接嵌入配置 | 否 |
| `message` | 人工提示信息 | 否 |
| `url` | 通过 URL 接口获取 | 是 |
| `service` | 通过服务商获取 | 是 |
| `extension` | 通过扩展获取 | 视扩展实现 |

### 7.2 密钥获取优先级

阅读器按 `key_access` 数组顺序尝试：

```
┌─────────────────────────────────────────────────────────────────┐
│                   密钥获取流程                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  key_access: [method_1, method_2, method_3, ...]               │
│                                                                 │
│  for method in key_access:                                     │
│    ├── 尝试获取密钥                                             │
│    ├── 成功 → 返回密钥，结束                                    │
│    ├── 失败（网络错误/权限不足/...） → 继续下一个               │
│    └── message 类型 → 展示提示信息，等待用户输入               │
│                                                                 │
│  全部失败 → 提示用户无法解密                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 8. 安全考量

### 8.1 已应对的攻击向量

| 攻击方式 | 防护措施 |
|---------|---------|
| 密文篡改 | AEAD 认证标签（GCM Tag / Poly1305 Tag / HMAC） |
| IV 重用 | 每次加密随机生成 IV，IV 存储在密文头部 |
| Padding Oracle | AES-CBC 使用 Encrypt-then-MAC，先验证 HMAC 再解密 |
| 密钥暴力破解 | KDF 使用高内存/高迭代参数（Argon2id / PBKDF2） |
| 降级攻击 | 算法 ID 嵌入密文头部，不可在外部修改（受签名保护） |
| 密文替换 | MANIFEST.MF 哈希 + 双层签名保护 |
| 密钥泄露 | 密钥获取方式分离，支持多种密钥分发机制 |

### 8.2 实现者安全建议

1. **IV/Nonce 生成**：必须使用 CSPRNG（密码学安全随机数生成器）
2. **内存安全**：解密后的明文应在使用完毕后安全清零
3. **密钥存储**：内容密钥不应持久化存储在客户端
4. **错误信息**：解密失败时不应泄露具体失败原因（防止 Oracle 攻击）
5. **算法选择**：优先使用 AEAD 算法（AES-GCM / ChaCha20-Poly1305）
6. **大文件处理**：大文件应分块加密，每块使用递增 Nonce

### 8.3 大文件分块加密

当文件超过 64 MiB 时，建议使用分块加密：

```
块大小        : 1 MiB (1,048,576 bytes)
Base Nonce    : 随机生成的 64-bit 盐（随 UCXE 头存储）
Nonce 模式    : 拼接式（concatenation），禁止使用 XOR 派生
Nonce 计算    : nonce = base_nonce[0..8] || chunk_index.to_be_bytes()   (共 12 bytes)
              其中 chunk_index 为 32-bit 大端整数（u32 BE）
最后一块      : 可小于 1 MiB
```

> **Rationale（不得使用 XOR 派生）**：早期版本使用 `nonce = base XOR uint96_le(chunk_index)`。在该方案下，攻击者如能影响 96-bit `base_nonce`，即可让两对不同的 `(key, chunk_index)` 产生相同 nonce，违反 AEAD 对 (key, nonce) 唯一性的要求。改为**拼接式**后，不同 `chunk_index` 值位于 nonce 的低 4 字节且互不相交，天然保证不可能碰撞。实现 MUST 使用拼接式，不允许兼容旧的 XOR 方案。

**分块加密文件格式**（在标准 UCXE 头部之后）：

```
┌─────────────────────────────────────────────┐
│  UCXE Header (标准文件头)                   │
├─────────────────────────────────────────────┤
│  Chunk Count    (4 bytes, LE)               │
├─────────────────────────────────────────────┤
│  Chunk 0:                                   │
│  ├── Chunk Size     (4 bytes, LE)           │
│  ├── Ciphertext     (variable)              │
│  └── Auth Tag       (16 bytes)              │
├─────────────────────────────────────────────┤
│  Chunk 1:                                   │
│  ├── Chunk Size     (4 bytes, LE)           │
│  ├── Ciphertext     (variable)              │
│  └── Auth Tag       (16 bytes)              │
├─────────────────────────────────────────────┤
│  ...                                        │
└─────────────────────────────────────────────┘
```

## 9. 工具命令参考

```bash
# 加密章节（使用服务商密钥管理）
ucx encrypt --chapter content/chapter-002.md \
            --algorithm AES-256-GCM \
            --key-access service \
            --provider "https://api.unicodex.org/keys" \
            --service-type purchase_verify

# 加密章节（使用直接密钥）
ucx encrypt --chapter content/chapter-002.md \
            --algorithm AES-256-GCM \
            --key "<BASE64_32BYTE_KEY>"

# 加密章节（使用密码派生）
ucx encrypt --chapter content/chapter-002.md \
            --algorithm AES-256-GCM \
            --passphrase \
            --kdf argon2id

# 加密资源
ucx encrypt --resource assets/images/img-001.jpg \
            --algorithm AES-256-GCM \
            --key "<BASE64_32BYTE_KEY>"

# 批量加密（根据 struct.json / resource.json 配置）
ucx encrypt --all

# 解密测试
ucx decrypt --chapter content/chapter-002.md --key "<BASE64_32BYTE_KEY>"

# 查看加密状态
ucx info --encryption novel.ucx
# Encryption Status:
#   content/chapter-002.md     : AES-256-GCM (service: purchase_verify)
#   content/extra-002.ucxc     : AES-256-GCM (message + url)
#   assets/images/img-001.jpg  : AES-256-GCM (service: purchase_verify)
#   [9 files unencrypted]
```

---

*请审阅并指出需要修改的部分*
