# UCX 密钥与身份验证规范

> 版本：0.1.0-draft
> 前置依赖：[05-signature-spec.md](./05-signature-spec.md)

## 1. 概述

本文档定义 UCX 签名体系中的密钥管理规范和作者身份验证机制。

### 1.1 密钥体系架构

```
┌─────────────────────────────────────────────────────────────────┐
│                     UCX 密钥与身份体系                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  私钥 (Private Key)                                             │
│  └── 签名者持有，绝不公开，用于生成签名                         │
│                                                                 │
│  公钥 (Public Key)                                              │
│  └── 嵌入证书中，用于验证签名                                   │
│                                                                 │
│  证书 (Certificate)                                             │
│  └── X.509 格式，绑定公钥与身份信息                             │
│                                                                 │
│  证书链 (Certificate Chain)                                     │
│  └── 签名者证书 → 中间 CA → 根 CA，用于身份信任传递             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 2. 私钥规范

### 2.1 支持的密钥算法

| 算法 | 私钥长度 | 推荐程度 | 说明 |
|------|---------|---------|------|
| **Ed25519** | 32 bytes (256 bit) | **最推荐** | 现代椭圆曲线，高安全高性能 |
| ECDSA P-256 | 32 bytes (256 bit) | 推荐 | NIST 标准，广泛支持 |
| RSA-4096 | 512 bytes (4096 bit) | 兼容 | 传统算法，体积大 |
| RSA-2048 | 256 bytes (2048 bit) | 最低兼容 | 安全性边界，不推荐新项目 |

### 2.2 私钥存储格式

#### 2.2.1 PEM 格式（推荐）

```
-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIHKVarg1z+xvuGZ8gH3m7VBHZ3qN3Ln9cHfXKRDwQxxx
-----END PRIVATE KEY-----
```

**规范**：
- 编码：Base64
- 标准：PKCS#8（`PRIVATE KEY`）或算法特定（`ED25519 PRIVATE KEY`）
- 行长度：64 字符

#### 2.2.2 加密的私钥（生产环境推荐）

```
-----BEGIN ENCRYPTED PRIVATE KEY-----
MIIBvTBXBgkqhkiG9w0BBQ0wSjApBgkqhkiG9w0BBQwwHAQIxxxxxxxxxxxxxxxxx
...
-----END ENCRYPTED PRIVATE KEY-----
```

**加密参数**：
- 算法：PBES2 (PKCS#5 v2.0)
- KDF：PBKDF2 with HMAC-SHA256
- 迭代次数：≥ 100,000
- 加密：AES-256-GCM

### 2.3 私钥生成规范

#### Ed25519（推荐）

```rust
// 伪代码示例
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

// 从操作系统安全随机源生成
let signing_key = SigningKey::generate(&mut OsRng);

// 导出私钥字节
let private_key_bytes: [u8; 32] = signing_key.to_bytes();
```

**要求**：
1. 必须使用密码学安全的随机数生成器（CSPRNG）
2. 禁止使用可预测的种子
3. 生成后立即安全存储

#### ECDSA P-256

```rust
// 伪代码示例
use p256::ecdsa::SigningKey;
use rand::rngs::OsRng;

let signing_key = SigningKey::random(&mut OsRng);
```

#### RSA

```rust
// 伪代码示例
use rsa::{RsaPrivateKey, pkcs8::EncodePrivateKey};
use rand::rngs::OsRng;

// 4096 位 RSA
let private_key = RsaPrivateKey::new(&mut OsRng, 4096)?;
```

### 2.4 私钥安全要求

```
┌─────────────────────────────────────────────────────────────────┐
│                     私钥安全要求                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ✗ 禁止：                                                       │
│  ├── 将私钥写入 UCX 文件                                        │
│  ├── 将私钥提交到版本控制系统                                   │
│  ├── 通过不安全渠道传输私钥                                     │
│  ├── 在日志中输出私钥内容                                       │
│  └── 使用弱密码保护私钥                                         │
│                                                                 │
│  ✓ 要求：                                                       │
│  ├── 使用加密存储（加密私钥文件或硬件设备）                     │
│  ├── 设置严格的文件权限（仅所有者可读：0600）                   │
│  ├── 定期轮换密钥（建议每 1-2 年）                              │
│  ├── 备份时使用加密介质                                         │
│  └── 生产环境使用 HSM（硬件安全模块）                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 3. 公钥规范

### 3.1 公钥格式

公钥通常不单独存储，而是嵌入在 X.509 证书中。

#### 3.1.1 独立公钥（调试/测试用）

**PEM 格式**：
```
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAGb9F2P2xKxPs0k3bH0lNbMxQo3xmEsNpmB3Lh9cP3xY=
-----END PUBLIC KEY-----
```

**规范**：
- 编码：SubjectPublicKeyInfo (SPKI) 格式
- 标准：RFC 5280

#### 3.1.2 公钥长度

| 算法 | 公钥长度 |
|------|---------|
| Ed25519 | 32 bytes |
| ECDSA P-256 | 65 bytes (未压缩) / 33 bytes (压缩) |
| RSA-2048 | ~256 bytes |
| RSA-4096 | ~512 bytes |

### 3.2 从私钥派生公钥

```rust
// Ed25519 示例
let signing_key: SigningKey = /* ... */;
let verifying_key = signing_key.verifying_key();
let public_key_bytes: [u8; 32] = verifying_key.to_bytes();
```

## 4. X.509 证书规范

### 4.1 证书结构

```
┌─────────────────────────────────────────────────────────────────┐
│                    X.509 v3 证书结构                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  tbsCertificate (待签名证书体)                                  │
│  ├── version              : v3 (2)                              │
│  ├── serialNumber         : 唯一序列号                          │
│  ├── signature            : 签名算法 OID                        │
│  ├── issuer               : 颁发者 DN                           │
│  ├── validity                                                   │
│  │   ├── notBefore        : 生效时间                            │
│  │   └── notAfter         : 过期时间                            │
│  ├── subject              : 主体 DN（签名者身份）               │
│  ├── subjectPublicKeyInfo : 公钥信息                            │
│  └── extensions           : v3 扩展                             │
│      ├── keyUsage         : digitalSignature                    │
│      ├── extKeyUsage      : codeSigning (推荐)                  │
│      ├── subjectAltName   : 可选的替代名称                      │
│      └── ...                                                    │
│                                                                 │
│  signatureAlgorithm       : 签名算法 OID                        │
│  signatureValue           : CA 对 tbsCertificate 的签名         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Subject DN（主体可分辨名称）

Subject DN 是证书中标识签名者身份的核心字段。

**推荐格式**：

```
CN=作者笔名
O=组织名称（可选）
OU=部门（可选）
L=城市（可选）
ST=省份（可选）
C=国家代码（如 CN）
```

**示例**：

```
# 个人作者
CN=张三, C=CN

# 机构作者
CN=某出版社编辑部, O=某某出版社, C=CN

# 网文作者（使用笔名）
CN=笔名xxx, pseudonym=true
```

### 4.3 证书扩展字段

| 扩展 | OID | 必需 | 说明 |
|------|-----|------|------|
| Key Usage | 2.5.29.15 | 是 | 必须包含 `digitalSignature` |
| Extended Key Usage | 2.5.29.37 | 推荐 | 建议包含 `codeSigning` (1.3.6.1.5.5.7.3.3) |
| Subject Alt Name | 2.5.29.17 | 可选 | 邮箱、URI 等替代身份标识 |
| Basic Constraints | 2.5.29.19 | 是 | `CA:FALSE`（终端证书） |
| Authority Key ID | 2.5.29.35 | 推荐 | 颁发者公钥标识 |
| Subject Key ID | 2.5.29.14 | 推荐 | 本证书公钥标识 |

### 4.4 UCX 自定义扩展（可选）

UCX 定义了可选的自定义证书扩展：

| 扩展名 | OID（示例） | 说明 |
|-------|------------|------|
| UCX Author Role | 1.3.6.1.4.1.xxxxx.1.1 | 作者角色：author/coauthor/editor |
| UCX Platform ID | 1.3.6.1.4.1.xxxxx.1.2 | 关联的平台标识符 |
| UCX Pen Name | 1.3.6.1.4.1.xxxxx.1.3 | 笔名标记 |

## 5. 身份验证机制

### 5.1 身份验证层级

```
┌─────────────────────────────────────────────────────────────┐
│                     身份验证信任层级                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Level 1: 自签名证书（Self-Signed）                         │
│  ├── 验证内容：签名数学正确性                               │
│  ├── 可证明：文件由同一私钥签署                             │
│  ├── 不可证明：私钥持有者的真实身份                         │
│  └── 适用场景：个人创作、测试、内部使用                     │
│                                                             │
│  Level 2: CA 签发证书（CA-Issued）                          │
│  ├── 验证内容：证书链 + 签名正确性                          │
│  ├── 可证明：签名者身份经过 CA 审核                         │
│  ├── 信任来源：CA 的审核流程                                │
│  └── 适用场景：正式发布、商业出版                           │
│                                                             │
│  Level 3: CA 证书 + 区块链存证                              │
│  ├── 验证内容：证书链 + 签名 + 链上记录                     │
│  ├── 可证明：身份 + 时间 + 存在性                           │
│  ├── 不可篡改：区块链提供时间戳和存在证明                   │
│  └── 适用场景：版权纠纷预防、法律证据                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 自签名证书验证

**验证流程**：

```
┌────────────────────────────────────────┐
│         自签名证书验证流程             │
├────────────────────────────────────────┤
│                                        │
│  1. 提取证书中的公钥                   │
│  2. 用公钥验证证书自身的签名           │
│  3. 检查证书有效期                     │
│  4. 用公钥验证 UCX 文件签名            │
│                                        │
│  输出：                                │
│  ├── 签名有效性：✓/✗                   │
│  ├── 证书类型：self-signed             │
│  ├── 主体信息：CN=xxx                  │
│  └── 公钥指纹：xxxx...                 │
│                                        │
└────────────────────────────────────────┘
```

**局限性**：
- 无法验证 Subject DN 中声明的身份是否真实
- 任何人都可以创建声称是"著名作者"的自签名证书
- 适合建立"同一作者"的一致性，而非验证"是谁"

### 5.3 CA 签发证书验证

**验证流程**：

```
┌────────────────────────────────────────┐
│         CA 证书链验证流程              │
├────────────────────────────────────────┤
│                                        │
│  1. 获取完整证书链                     │
│     签名者证书 → 中间 CA → 根 CA       │
│                                        │
│  2. 验证链的完整性                     │
│     每级证书用上级公钥验证签名         │
│                                        │
│  3. 验证根 CA 是否可信                 │
│     检查是否在信任库中                 │
│                                        │
│  4. 验证每级证书的有效期               │
│                                        │
│  5. 检查证书吊销状态                   │
│     CRL 或 OCSP 查询                   │
│                                        │
│  6. 验证 UCX 文件签名                  │
│                                        │
│  输出：                                │
│  ├── 签名有效性：✓/✗                   │
│  ├── 证书类型：CA-issued               │
│  ├── 主体信息：CN=xxx, O=xxx           │
│  ├── 颁发者：CA 名称                   │
│  └── 信任路径：完整证书链              │
│                                        │
└────────────────────────────────────────┘
```

### 5.4 证书链存储

**UCX 文件内存储**：

```
META-INF/certs/
├── AUTHOR.cert.pem          # 签名者证书
├── AUTHOR.chain.pem         # 完整证书链（含中间 CA）
├── PUBLISHER.cert.pem
└── PUBLISHER.chain.pem
```

**chain.pem 格式**：

```
-----BEGIN CERTIFICATE-----
[签名者证书]
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
[中间 CA 证书]
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
[根 CA 证书（可选，通常在系统信任库）]
-----END CERTIFICATE-----
```

### 5.5 证书指纹

用于快速识别和比对证书。

**计算方法**：

```rust
// BLAKE3 指纹（推荐）
let cert_der: &[u8] = /* 证书的 DER 编码 */;
let fingerprint = blake3::hash(cert_der);

// SHA-256 指纹（兼容性）
let fingerprint = sha256::hash(cert_der);
```

**显示格式**：

```
BLAKE3: af13:49b9:f5f9:a1a6:a040:4dea:36dc:c949
SHA256: 9B:51:C4:E3:8B:57:F4:77:...
```

## 6. 作者身份关联

### 6.1 metadata 中的身份关联

在 `metadata/codex.json` 中关联签名者与作品角色：

```json
{
  "creators": [
    {
      "role": "author",
      "name": "张三",
      "signature_ref": "AUTHOR",
      "verified": true
    },
    {
      "role": "illustrator",
      "name": "李四",
      "signature_ref": "ILLUSTRATOR",
      "verified": true
    }
  ],
  "publisher": {
    "name": "某某出版社",
    "signature_ref": "PUBLISHER",
    "verified": true
  }
}
```

**字段说明**：

| 字段 | 说明 |
|------|------|
| `signature_ref` | 对应 META-INF/signatures/ 下的签名者 ID |
| `verified` | 该签名者是否已验证（阅读器填充） |

### 6.2 多作者场景

```
┌─────────────────────────────────────────────────────────────────┐
│                     多作者签名场景                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  场景 1: 独立创作                                               │
│  └── AUTHOR.SF + AUTHOR.EC                                      │
│                                                                 │
│  场景 2: 合著                                                   │
│  ├── AUTHOR_01.SF + AUTHOR_01.EC    # 第一作者                  │
│  └── AUTHOR_02.SF + AUTHOR_02.EC    # 第二作者                  │
│                                                                 │
│  场景 3: 作者 + 出版方                                          │
│  ├── AUTHOR.SF + AUTHOR.EC          # 作者签名                  │
│  └── PUBLISHER.SF + PUBLISHER.RSA   # 出版方签名                │
│                                                                 │
│  场景 4: 完整出版流程                                           │
│  ├── AUTHOR.SF + AUTHOR.EC          # 原作者                    │
│  ├── EDITOR.SF + EDITOR.EC          # 编辑                      │
│  ├── PUBLISHER.SF + PUBLISHER.RSA   # 出版方                    │
│  └── PLATFORM.SF + PLATFORM.EC      # 分发平台                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 7. 密钥轮换

### 7.1 轮换原因

- 私钥可能泄露
- 定期轮换提高安全性
- 算法升级（如从 RSA 迁移到 Ed25519）

### 7.2 轮换流程

```
┌────────────────────────────────────────┐
│           密钥轮换流程                 │
├────────────────────────────────────────┤
│                                        │
│  1. 生成新密钥对                       │
│  2. 用旧密钥签署"轮换声明"             │
│     声明新公钥的合法性                 │
│  3. 获取新证书                         │
│  4. 新作品使用新密钥签名               │
│  5. 保留旧密钥用于验证历史作品         │
│  6. 安全销毁旧私钥（在安全期后）       │
│                                        │
└────────────────────────────────────────┘
```

### 7.3 轮换证明（未来扩展）

参考 APK Signature Scheme v3 的 proof-of-rotation 机制，
UCX 预留 `0x55435820` Pair ID 用于密钥轮换证明。

## 8. 工具命令参考

### 8.1 生成密钥对

```bash
# Ed25519（推荐）
ucx keygen --algorithm ed25519 \
           --output author.key \
           --encrypt  # 可选：加密私钥

# ECDSA P-256
ucx keygen --algorithm ecdsa-p256 \
           --output author.key

# RSA-4096
ucx keygen --algorithm rsa-4096 \
           --output author.key
```

### 8.2 生成自签名证书

```bash
ucx cert create --key author.key \
                --cn "作者笔名" \
                --validity 365 \
                --output author.cert.pem
```

### 8.3 查看证书信息

```bash
ucx cert info author.cert.pem

# 输出示例：
# Subject: CN=作者笔名
# Issuer: CN=作者笔名 (self-signed)
# Valid: 2025-01-29 to 2026-01-29
# Public Key: Ed25519
# Fingerprint (BLAKE3): af13:49b9:f5f9:...
# Fingerprint (SHA256): 9B:51:C4:E3:...
```

### 8.4 验证 UCX 文件签名者身份

```bash
ucx verify --show-signers novel.ucx

# 输出示例：
# Signers:
#   [1] AUTHOR
#       Subject: CN=张三
#       Type: self-signed
#       Fingerprint: af13:49b9:...
#       Status: VALID
#
#   [2] PUBLISHER
#       Subject: CN=某出版社, O=某某出版社
#       Issuer: CN=Example CA
#       Type: CA-issued
#       Fingerprint: 7d86:5e95:...
#       Status: VALID
```

---

*请审阅并指出需要修改的部分*
