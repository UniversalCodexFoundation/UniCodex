# UCX 官方服务与服务商规范

> 版本：0.1.0-draft
> 前置依赖：[01-file-structure.md](./01-file-structure.md)、[05-signature-spec.md](./05-signature-spec.md)、[08-keys-identity.md](./08-keys-identity.md)

## 1. 概述

Unicodex 官方提供一系列基础服务，用于支撑 UCX 生态的密钥分发、身份验证、签名验证等核心功能。
所有官方服务均可被替换为第三方服务商，保持生态的开放性和灵活性。

### 1.1 设计原则

```
┌─────────────────────────────────────────────────────────────────┐
│                   UCX 服务体系设计原则                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ● 开放性：所有服务接口公开，任何人可实现兼容服务               │
│  ● 可替换：官方服务可被第三方服务商完全替换                     │
│  ● 去中心化：UCX 文件本身不依赖任何在线服务即可使用             │
│  ● 渐进增强：服务提供额外能力（如在线验证），而非必需功能       │
│  ● 安全传输：所有服务通信必须使用 HTTPS / TLS 1.3+              │
│                                                                 │
│  核心原则：UCX 文件的基本功能（阅读、离线签名验证）             │
│  永远不依赖在线服务。服务仅为增强功能提供支撑。                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 服务架构总览

```
┌────────────────────────────────────────────────────────┐
│                     UCX 服务架构                       │
├────────────────────────────────────────────────────────┤
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │               UCX 阅读器 / CLI 工具              │  │
│  └──────────┬───────────┬───────────┬───────────────┘  │
│             │           │           │                  │
│     ┌───────▼───┐ ┌─────▼─────┐ ┌───▼─────────┐        │
│     │ 密钥分发  │ │ 身份验证  │ │ 包验证服务  │        │
│     │ Service   │ │ Service   │ │ Service     │        │
│     └───────┬───┘ └─────┬─────┘ └──┬──────────┘        │
│             │           │          │                   │
│     ┌───────▼───────────▼──────────▼──────────┐        │
│     │          服务提供商抽象层               │        │
│     │  ┌─────────┐ ┌─────────┐ ┌───────────┐  │        │
│     │  │ Unicodex│ │ 第三方A │ │ 第三方B   │  │        │
│     │  │ 官方    │ │ Provider│ │ Provider  │  │        │
│     │  └─────────┘ └─────────┘ └───────────┘  │        │
│     └─────────────────────────────────────────┘        │
│                                                        │
└────────────────────────────────────────────────────────┘
```

## 2. 服务清单

### 2.1 官方服务总览

| 服务 ID | 服务名称 | 官方端点 | 说明 |
|---------|---------|---------|------|
| `key-distribution` | 密钥分发服务 | `https://api.unicodex.org/keys` | 加密内容的密钥管理与分发 |
| `identity` | 身份验证服务 | `https://api.unicodex.org/identity` | 作者/出版方身份认证 |
| `certificate` | 证书签发服务 | `https://ca.unicodex.org` | X.509 证书签发（UCX CA） |
| `package-verify` | 包验证服务 | `https://api.unicodex.org/verify` | UCX 文件在线完整性验证 |
| `public-key` | 公钥注册服务 | `https://api.unicodex.org/pubkeys` | 公钥发现与注册 |
| `timestamp` | 可信时间戳服务 | `https://api.unicodex.org/timestamp` | RFC 3161 可信时间戳 |

### 2.2 服务提供商配置

所有服务均支持配置为第三方服务商。配置方式：

**unicodex.toml 中配置**：

```toml
# 服务商配置（可选，默认使用 Unicodex 官方服务）
[services]

# 密钥分发服务
[services.key-distribution]
provider = "https://api.unicodex.org/keys"       # 默认官方
# provider = "https://keys.example.com/api"      # 可替换为第三方

# 身份验证服务
[services.identity]
provider = "https://api.unicodex.org/identity"    # 默认官方
# provider = "https://id.my-platform.com/api"     # 可替换

# 证书签发服务
[services.certificate]
provider = "https://ca.unicodex.org"              # 默认官方
# provider = "https://ca.example.com"             # 可替换为任何 CA

# 包验证服务
[services.package-verify]
provider = "https://api.unicodex.org/verify"      # 默认官方

# 公钥注册服务
[services.public-key]
provider = "https://api.unicodex.org/pubkeys"     # 默认官方

# 可信时间戳服务
[services.timestamp]
provider = "https://api.unicodex.org/timestamp"   # 默认官方
# provider = "https://freetsa.org/tsr"            # 可替换为任何 RFC 3161 TSA
```

**UCX 文件内引用服务**（在 encryption.key_access 中）：

```json
{
  "method": "service",
  "provider": "https://api.unicodex.org/keys",
  "service_type": "purchase_verify"
}
```

> **服务商替换规则**：UCX 文件内的 `provider` 字段指定了具体服务商。阅读器应直接使用文件中指定的服务商，`unicodex.toml` 中的配置仅用于打包阶段的默认值。

## 3. 密钥分发服务（Key Distribution Service）

### 3.1 服务职责

管理和分发加密章节/资源的解密密钥，支持多种验证方式。

```
┌─────────────────────────────────────────────────────────────────┐
│                   密钥分发服务流程                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  请求方（阅读器）                     服务端                    │
│  │                                    │                         │
│  ├── 1. 发送密钥请求 ──────────────►  │                         │
│  │   (content_id + 认证信息)          │                         │
│  │                                    ├── 2. 验证身份/权限      │
│  │                                    │                         │
│  │                                    ├── 3. 检查授权状态       │
│  │                                    │   (已购买/已订阅/...)   │
│  │                                    │                         │
│  │ ◄── 4. 返回加密密钥 ────────────   │                         │
│  │   (用请求方公钥加密)               │                         │
│  │                                    │                         │
│  ├── 5. 用私钥解密，获得内容密钥      │                         │
│  │                                    │                         │
│  └── 6. 解密章节内容                  │                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 API 接口

#### 3.2.1 获取密钥

**请求**：

```
POST {provider}/v1/keys/request
Content-Type: application/json
Authorization: Bearer {token}

{
  "ucx_id": "urn:ucx:550e8400-e29b-41d4-a716-446655440000",
  "content_id": "chapter-002",
  "service_type": "purchase_verify",
  "client_public_key": "MCowBQYDK2VwAyEA...",
  "params": {
    "product_id": "novel-001-ch02"
  }
}
```

**响应（成功）**：

```json
{
  "status": "granted",
  "encrypted_key": "Base64 编码的加密密钥（用 client_public_key 加密）",
  "algorithm": "AES-256-GCM",
  "iv": "Base64 编码的初始化向量",
  "expires_at": "2025-12-31T23:59:59Z",
  "grant_type": "purchase"
}
```

**响应（拒绝）**：

```json
{
  "status": "denied",
  "reason": "not_purchased",
  "message": "请先购买该章节",
  "purchase_url": "https://example.com/buy/novel-001-ch02"
}
```

### 3.3 service_type 详细说明

#### purchase_verify（购买验证）

验证用户已购买该加密内容。

```json
{
  "method": "service",
  "provider": "https://api.unicodex.org/keys",
  "service_type": "purchase_verify",
  "params": {
    "product_id": "novel-001-ch02",
    "store": "unicodex-store"
  }
}
```

#### author_verify（作者身份验证）

验证请求者为该作品的关联作者，作者始终拥有自己作品的解密权限。

```json
{
  "method": "service",
  "provider": "https://api.unicodex.org/keys",
  "service_type": "author_verify",
  "params": {
    "signature_ref": "AUTHOR"
  }
}
```

#### subscription（订阅验证）

验证用户的订阅状态是否覆盖该内容。

```json
{
  "method": "service",
  "provider": "https://api.unicodex.org/keys",
  "service_type": "subscription",
  "params": {
    "plan_id": "premium",
    "content_tier": "vip"
  }
}
```

#### public_key（公钥交换）

使用读者的公钥加密内容密钥，实现端到端加密分发。

```json
{
  "method": "service",
  "provider": "https://api.unicodex.org/keys",
  "service_type": "public_key",
  "params": {
    "key_exchange_algorithm": "X25519"
  }
}
```

## 4. 身份验证服务（Identity Service）

### 4.1 服务职责

提供作者、出版方的身份认证功能。支持多层级身份验证。

### 4.2 身份验证层级

```
┌─────────────────────────────────────────────────────────────────┐
│                     身份验证层级                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Level 0: 匿名（Anonymous）                                     │
│  ├── 无身份信息                                                 │
│  └── 仅凭签名密钥区分身份                                       │
│                                                                 │
│  Level 1: 基础认证（Basic）                                     │
│  ├── 注册邮箱已验证                                             │
│  ├── 公钥已登记                                                 │
│  └── 可证明：邮箱持有者 = 签名者                                │
│                                                                 │
│  Level 2: 实名认证（Verified）                                  │
│  ├── 真实身份已验证（身份证/护照等）                            │
│  ├── 笔名与真实身份绑定                                         │
│  └── 可证明：签名者为特定自然人/法人                            │
│                                                                 │
│  Level 3: 机构认证（Certified）                                 │
│  ├── 通过官方 CA 或授权 CA 签发证书                             │
│  ├── 机构审核流程完成                                           │
│  └── 可证明：签名者为经认证的机构成员                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.3 API 接口

#### 4.3.1 注册身份

```
POST {provider}/v1/identity/register
Content-Type: application/json

{
  "display_name": "张三",
  "email": "author@example.com",
  "public_key": "MCowBQYDK2VwAyEA...",
  "key_algorithm": "Ed25519"
}
```

**响应**：

```json
{
  "identity_id": "uid-550e8400-e29b-41d4-a716-446655440000",
  "level": 1,
  "status": "pending_email_verification",
  "verification_url": "https://api.unicodex.org/identity/verify/..."
}
```

#### 4.3.2 查询身份

```
GET {provider}/v1/identity/lookup?fingerprint={公钥指纹}
```

**响应**：

```json
{
  "identity_id": "uid-550e8400-...",
  "display_name": "张三",
  "level": 2,
  "level_label": "verified",
  "public_key_fingerprint": "af13:49b9:f5f9:a1a6:...",
  "verified_at": "2025-06-01T12:00:00Z",
  "ucx_works": [
    {
      "ucx_id": "urn:ucx:550e8400-...",
      "title": "我的小说",
      "signature_ref": "AUTHOR"
    }
  ]
}
```

#### 4.3.3 验证身份声明

验证某个 UCX 文件中声明的作者身份是否与注册身份匹配。

```
POST {provider}/v1/identity/verify-claim
Content-Type: application/json

{
  "ucx_id": "urn:ucx:550e8400-...",
  "signature_ref": "AUTHOR",
  "certificate_fingerprint": "af13:49b9:f5f9:..."
}
```

**响应**：

```json
{
  "verified": true,
  "identity": {
    "display_name": "张三",
    "level": 2,
    "level_label": "verified"
  },
  "match_type": "fingerprint_match",
  "verified_at": "2025-06-01T12:00:00Z"
}
```

## 5. 证书签发服务（Certificate Authority）

### 5.1 服务职责

UCX 官方运营 CA（Certificate Authority），为作者和出版方签发 X.509 签名证书。

### 5.2 证书类型

| 类型 | 说明 | 身份要求 | 有效期 |
|------|------|---------|--------|
| `self-signed` | 自签名证书（本地生成，不涉及 CA） | 无 | 自定义 |
| `basic` | 基础证书（邮箱验证） | Level 1 | 1 年 |
| `verified` | 实名证书 | Level 2 | 2 年 |
| `organization` | 机构证书 | Level 3 | 3 年 |

### 5.3 证书链结构

```
UCX Root CA（根证书，预装在阅读器信任库中）
├── UCX Author CA（作者证书中间 CA）
│   ├── 张三的证书（basic / verified）
│   └── 李四的证书（basic / verified）
├── UCX Publisher CA（出版方证书中间 CA）
│   ├── 某出版社的证书（organization）
│   └── 某平台的证书（organization）
└── UCX Platform CA（平台证书中间 CA）
    └── 分发平台的证书
```

### 5.4 API 接口

#### 5.4.1 申请证书

```
POST {provider}/v1/certificates/request
Content-Type: application/json
Authorization: Bearer {token}

{
  "csr": "-----BEGIN CERTIFICATE REQUEST-----\n...\n-----END CERTIFICATE REQUEST-----",
  "cert_type": "verified",
  "identity_id": "uid-550e8400-..."
}
```

> **CSR（Certificate Signing Request）**：由申请者本地生成，包含公钥和身份信息，私钥不会上传。

**响应**：

```json
{
  "request_id": "req-xxxx",
  "status": "pending_review",
  "estimated_time": "24h",
  "certificate": null
}
```

#### 5.4.2 获取已签发证书

```
GET {provider}/v1/certificates/{request_id}
```

**响应（已签发）**：

```json
{
  "request_id": "req-xxxx",
  "status": "issued",
  "certificate": "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----",
  "chain": "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----\n-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----",
  "issued_at": "2025-06-02T10:00:00Z",
  "expires_at": "2027-06-02T10:00:00Z"
}
```

### 5.5 第三方 CA 支持

UCX 不限制使用官方 CA。以下 CA 签发的证书同样有效：

- 任何公共 CA（如 Let's Encrypt、DigiCert 等）签发的代码签名证书
- 企业内部 CA
- 其他 UCX 兼容 CA

阅读器应维护可配置的信任库，允许用户添加自定义根证书。

## 6. 包验证服务（Package Verification Service）

### 6.1 服务职责

提供 UCX 文件的在线辅助验证，作为离线验证的补充。**所有验证均不上传 UCX 文件**，仅提交摘要信息，文件内容始终留在本地。

### 6.2 验证类型

| 验证类型 | 说明 | 上传内容 | 在线/本地 |
|---------|------|---------|----------|
| `quick` | 快速验证（签名 + 全局哈希比对） | 签名块摘要 + MANIFEST.MF 哈希 | 在线 |
| `full` | 完整验证（快速验证 + 哈希清单比对 + 本地文件校验） | 签名块摘要 + MANIFEST.MF 哈希 + 哈希清单 | 在线 + 本地 |
| `certificate-check` | 证书状态检查（有效期 + 吊销） | 证书指纹 | 在线 |

### 6.3 验证流程

```
┌─────────────────────────────────────────────────────────────────┐
│                   包验证流程（全程不上传文件）                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌── 快速验证（quick）──────────────────────────────────────┐   │
│  │                                                          │   │
│  │  本地提取：                                              │   │
│  │  ├── UCX Signing Block 摘要                              │   │
│  │  ├── MANIFEST.MF 哈希值                                  │   │
│  │  └── 签名者证书指纹                                      │   │
│  │                          ↓                               │   │
│  │  在线比对：将以上摘要提交至服务端                        │   │
│  │  ├── 签名块摘要 是否与注册记录一致                       │   │
│  │  ├── MANIFEST.MF 哈希 是否与注册记录一致                 │   │
│  │  └── 证书状态 是否有效（未吊销、未过期）                 │   │
│  │                          ↓                               │   │
│  │  结果：签名有效 / 签名不匹配 / 未注册                    │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                             ↓                                   │
│  ┌── 完整验证（full）= 快速验证 + 以下步骤 ─────────────────┐   │
│  │                                                          │   │
│  │  在线比对哈希清单：                                      │   │
│  │  ├── 本地提取 MANIFEST.MF 中的全部文件哈希条目           │   │
│  │  ├── 提交哈希清单至服务端                                │   │
│  │  └── 服务端逐条比对，返回差异列表                        │   │
│  │                          ↓                               │   │
│  │  本地文件校验（由本地工具链执行，不上传任何文件）：      │   │
│  │  ├── 遍历 UCX 内所有文件                                 │   │
│  │  ├── 逐个计算实际哈希值                                  │   │
│  │  ├── 与 MANIFEST.MF 中记录的哈希值比对                   │   │
│  │  └── 报告：匹配 / 不匹配 / 缺失 / 多余                   │   │
│  │                          ↓                               │   │
│  │  综合结果：在线验证结果 + 本地文件校验结果               │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 6.4 API 接口

#### 6.4.1 快速验证（签名 + 哈希比对）

无需上传文件，仅提交签名块摘要和 MANIFEST.MF 哈希值。

```
POST {provider}/v1/verify/quick
Content-Type: application/json

{
  "ucx_id": "urn:ucx:550e8400-...",
  "manifest_hash": "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9...",
  "signing_block_digest": "7d865e959b2466918c9863afca942d0fb89d7c9a...",
  "signer_fingerprints": ["af13:49b9:f5f9:a1a6:..."],
  "hash_algorithm": "BLAKE3"
}
```

**响应**：

```json
{
  "status": "verified",
  "signature": {
    "match": true,
    "registered_at": "2025-06-01T12:00:00Z"
  },
  "manifest": {
    "match": true,
    "registered_hash": "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9..."
  },
  "signers": [
    {
      "signer_id": "AUTHOR",
      "display_name": "张三",
      "identity_level": 2,
      "certificate_status": "valid"
    }
  ]
}
```

#### 6.4.2 完整验证 — 在线部分（哈希清单比对）

在快速验证的基础上，额外提交 MANIFEST.MF 中的完整哈希清单，服务端逐条比对。

```
POST {provider}/v1/verify/full
Content-Type: application/json

{
  "ucx_id": "urn:ucx:550e8400-...",
  "manifest_hash": "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9...",
  "signing_block_digest": "7d865e959b2466918c9863afca942d0fb89d7c9a...",
  "signer_fingerprints": ["af13:49b9:f5f9:a1a6:..."],
  "hash_algorithm": "BLAKE3",
  "file_hashes": [
    { "path": "content/chapter-001.ucxc", "hash": "a1b2c3d4..." },
    { "path": "content/chapter-002.md", "hash": "e5f6a7b8..." },
    { "path": "metadata/codex.json", "hash": "c9d0e1f2..." },
    { "path": "assets/cover.jpg", "hash": "11223344..." }
  ]
}
```

**响应**：

```json
{
  "status": "verified",
  "signature": {
    "match": true,
    "registered_at": "2025-06-01T12:00:00Z"
  },
  "manifest": {
    "match": true
  },
  "file_hashes": {
    "total": 12,
    "matched": 12,
    "mismatched": 0,
    "missing": 0,
    "extra": 0,
    "details": []
  },
  "signers": [
    {
      "signer_id": "AUTHOR",
      "status": "valid",
      "algorithm": "Ed25519",
      "certificate": {
        "subject": "CN=张三",
        "issuer": "CN=UCX Author CA",
        "type": "verified",
        "valid": true,
        "expires_at": "2027-06-02T10:00:00Z",
        "revoked": false
      }
    }
  ],
  "overall": "VERIFIED"
}
```

**哈希不匹配时的 details 示例**：

```json
"details": [
  {
    "path": "content/chapter-002.md",
    "status": "mismatched",
    "submitted_hash": "e5f6a7b8...",
    "registered_hash": "ffffffff..."
  },
  {
    "path": "content/chapter-999.md",
    "status": "extra",
    "submitted_hash": "aabbccdd..."
  }
]
```

#### 6.4.3 本地验证（默认行为）

默认的 `ucx verify` 命令执行完整的本地验证，**不涉及任何网络请求**，流程类似 APK 签名验证。

```bash
# 本地验证（默认行为，包含签名验证 + 文件完整性校验）
ucx verify novel.ucx

# 在线完整验证（在线比对 + 本地验证）
ucx verify --online novel.ucx
```

**本地验证流程**（类似 APK v1 + v2 签名验证）：

```
┌────────────────────────────────────────────────────────────┐
│                       本地验证流程                         │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  第一步：UCX Signing Block 验证（Layer 2，类似 APK v2）    │
│  ├── 1. 检查 UCX Signing Block 是否存在                    │
│  ├── 2. 提取签名块中的签名数据                             │
│  ├── 3. 提取签名者证书和公钥                               │
│  ├── 4. 用公钥验证签名块的数字签名                         │
│  ├── 5. 验证签名覆盖的 ZIP 数据完整性                      │
│  └── 结果：签名有效 / 签名无效 / 无签名块（警告）          │
│                          ↓                                 │
│  第二步：META-INF 签名验证（Layer 1，类似 JAR / APK v1）   │
│  ├── 1. 检查 META-INF/AUTHOR.SF 是否存在                   │
│  ├── 2. 用 AUTHOR.RSA/EC 中的证书验证 .SF 文件的签名       │
│  ├── 3. 验证 .SF 中记录的 MANIFEST.MF 摘要是否匹配         │
│  ├── 4. 验证证书链（自签名 → 本地信任库中的根证书）        │
│  └── 结果：签名有效 / 签名无效 / 无签名文件（警告）        │
│                          ↓                                 │
│  第三步：文件完整性校验                                    │
│  ├── 1. 解析 META-INF/MANIFEST.MF 中每个文件的哈希记录     │
│  ├── 2. 遍历 UCX 中所有文件，逐个计算实际哈希值            │
│  ├── 3. 比对：记录哈希 vs 实际哈希                         │
│  ├── 4. 检查是否有未记录的多余文件                         │
│  ├── 5. 检查是否有记录但缺失的文件                         │
│  └── 结果：全部通过 / 存在不匹配 / 存在缺失或多余          │
│                          ↓                                 │
│  综合结果：                                                │
│  ├── VERIFIED       — 签名有效 + 文件完整                  │
│  ├── UNSIGNED       — 无签名（警告，可强制打开）           │
│  ├── SIG_INVALID    — 签名验证失败（警告，可强制打开）     │
│  └── FILE_TAMPERED  — 文件哈希不匹配（警告，可强制打开）   │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

> **高自由度原则**：所有验证结果均为**警告**而非**拒绝**，用户始终可以强制打开文件。

**本地验证输出示例**：

```
UCX Verification: novel.ucx
Algorithm: BLAKE3

[1/3] UCX Signing Block (Layer 2):
  [PASS] Signing block found
  [PASS] Signature algorithm: Ed25519
  [PASS] Signature valid
  [PASS] ZIP data integrity verified

[2/3] META-INF Signature (Layer 1):
  [PASS] AUTHOR.SF found
  [PASS] AUTHOR.RSA certificate valid
  [PASS] .SF digest matches MANIFEST.MF
  [PASS] Certificate chain: CN=张三 → CN=UCX Author CA → CN=UCX Root CA
  [INFO] Signer: 张三 (Identity Level 2: Verified)

[3/3] File Integrity:
  [PASS] metadata/codex.json
  [PASS] metadata/author.json
  [PASS] content/struct.json
  [PASS] content/chapter-001.ucxc
  [FAIL] content/chapter-002.md        ← 哈希不匹配
  [PASS] assets/cover.jpg
  [PASS] assets/images/img-001.jpg
  ...
  Files: 11/12 passed, 1 FAILED

Overall: FILE_TAMPERED
Warning: content/chapter-002.md hash mismatch. This file may have been tampered with.
```

**无签名文件的输出示例**：

```
UCX Verification: unsigned-novel.ucx
Algorithm: BLAKE3

[1/3] UCX Signing Block (Layer 2):
  [WARN] No signing block found

[2/3] META-INF Signature (Layer 1):
  [WARN] No signature files found (.SF/.RSA/.EC)

[3/3] File Integrity:
  [PASS] metadata/codex.json
  [PASS] content/struct.json
  [PASS] content/chapter-001.ucxc
  ...
  Files: 8/8 passed

Overall: UNSIGNED
Warning: This UCX file has no digital signature. Author identity cannot be verified.
```

#### 6.4.4 证书状态检查

```
GET {provider}/v1/verify/certificate?fingerprint={证书指纹}&algorithm=BLAKE3
```

**响应**：

```json
{
  "fingerprint": "af13:49b9:f5f9:...",
  "status": "valid",
  "subject": "CN=张三",
  "issuer": "CN=UCX Author CA",
  "not_before": "2025-06-02T10:00:00Z",
  "not_after": "2027-06-02T10:00:00Z",
  "revoked": false,
  "identity_level": 2
}
```

## 7. 公钥注册服务（Public Key Registry）

### 7.1 服务职责

提供公钥的发现和注册功能，使读者和验证者可以查找作者的公钥。

### 7.2 功能

```
┌─────────────────────────────────────────────────────────────────┐
│                   公钥注册服务功能                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ● 公钥注册：作者将公钥登记到注册表                             │
│  ● 公钥发现：通过作者名、邮箱、UCX ID 查找公钥                  │
│  ● 公钥指纹比对：验证 UCX 文件中的公钥是否已注册                │
│  ● 密钥轮换记录：追踪公钥更新历史                               │
│  ● 吊销公告：公钥泄露时发布吊销通知                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 7.3 API 接口

#### 7.3.1 注册公钥

```
POST {provider}/v1/pubkeys/register
Content-Type: application/json
Authorization: Bearer {token}

{
  "public_key": "MCowBQYDK2VwAyEA...",
  "algorithm": "Ed25519",
  "identity_id": "uid-550e8400-...",
  "label": "author-primary-key"
}
```

#### 7.3.2 查找公钥

```
GET {provider}/v1/pubkeys/lookup?identity_id={identity_id}
GET {provider}/v1/pubkeys/lookup?email={email}
GET {provider}/v1/pubkeys/lookup?fingerprint={fingerprint}
```

**响应**：

```json
{
  "keys": [
    {
      "public_key": "MCowBQYDK2VwAyEA...",
      "algorithm": "Ed25519",
      "fingerprint": "af13:49b9:f5f9:...",
      "label": "author-primary-key",
      "registered_at": "2025-01-01T00:00:00Z",
      "status": "active",
      "identity": {
        "display_name": "张三",
        "level": 2
      }
    }
  ]
}
```

#### 7.3.3 吊销公钥

```
POST {provider}/v1/pubkeys/revoke
Content-Type: application/json
Authorization: Bearer {token}

{
  "fingerprint": "af13:49b9:f5f9:...",
  "reason": "key_compromise",
  "revocation_proof": "用旧密钥签署的吊销声明（Base64）"
}
```

## 8. 可信时间戳服务（Timestamp Service）

### 8.1 服务职责

提供 RFC 3161 兼容的可信时间戳，证明签名在特定时间之前已存在。

### 8.2 用途

- 证明作品在特定时间点已完成创作
- 签名证书过期后，时间戳可证明签名在证书有效期内生成
- 版权纠纷中作为时间证据

### 8.3 API 接口

遵循 RFC 3161 时间戳协议。

#### 8.3.1 请求时间戳

```
POST {provider}/v1/timestamp
Content-Type: application/timestamp-query

[RFC 3161 TimeStampReq 二进制数据]
```

**响应**：

```
Content-Type: application/timestamp-reply

[RFC 3161 TimeStampResp 二进制数据]
```

#### 8.3.2 简化 JSON 接口（UCX 扩展）

```
POST {provider}/v1/timestamp/json
Content-Type: application/json

{
  "digest": "af1349b9f5f9a1a6...",
  "algorithm": "BLAKE3",
  "ucx_id": "urn:ucx:550e8400-..."
}
```

**响应**：

```json
{
  "timestamp": "2025-06-01T12:00:00.000Z",
  "digest": "af1349b9f5f9a1a6...",
  "algorithm": "BLAKE3",
  "tsa_signature": "Base64 编码的时间戳签名",
  "tsa_certificate": "Base64 编码的 TSA 证书",
  "rfc3161_token": "Base64 编码的标准 RFC 3161 令牌（兼容用途）"
}
```

## 9. 服务商规范

### 9.1 第三方服务商接入要求

任何第三方均可实现兼容的服务，但需满足以下要求：

| 要求 | 说明 |
|------|------|
| **协议兼容** | 实现本规范定义的 API 接口 |
| **HTTPS** | 必须使用 TLS 1.2+ 加密传输（推荐 TLS 1.3） |
| **API 版本** | 支持 URL 路径中的版本号（`/v1/`） |
| **错误格式** | 统一的错误响应格式 |
| **SLA** | 建议提供服务可用性保证 |

### 9.2 统一错误响应格式

所有服务的错误响应必须遵循以下格式：

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "资源未找到",
    "details": "指定的 UCX ID 在注册表中不存在",
    "request_id": "req-xxxxx"
  }
}
```

**错误码**：

| HTTP 状态码 | 错误码 | 说明 |
|------------|--------|------|
| 400 | `BAD_REQUEST` | 请求参数错误 |
| 401 | `UNAUTHORIZED` | 未认证或认证失败 |
| 403 | `FORBIDDEN` | 无权访问 |
| 404 | `NOT_FOUND` | 资源不存在 |
| 409 | `CONFLICT` | 资源冲突（如重复注册） |
| 429 | `RATE_LIMITED` | 请求频率超限 |
| 500 | `INTERNAL_ERROR` | 服务内部错误 |

### 9.3 认证方式

服务 API 支持以下认证方式：

| 方式 | Header | 说明 |
|------|--------|------|
| Bearer Token | `Authorization: Bearer {token}` | 标准 OAuth2 Token |
| API Key | `X-API-Key: {key}` | 简单 API 密钥 |
| 签名认证 | `X-UCX-Signature: {sig}` | 用 UCX 私钥签署请求 |
| 无认证 | — | 部分查询接口无需认证 |

### 9.4 服务发现

阅读器可通过标准端点发现服务商支持的功能：

```
GET {provider}/.well-known/ucx-services
```

**响应**：

```json
{
  "provider": "Unicodex Official",
  "version": "1.0",
  "base_url": "https://api.unicodex.org",
  "services": {
    "key-distribution": {
      "enabled": true,
      "endpoint": "/keys",
      "version": "v1"
    },
    "identity": {
      "enabled": true,
      "endpoint": "/identity",
      "version": "v1"
    },
    "certificate": {
      "enabled": true,
      "endpoint": "https://ca.unicodex.org",
      "version": "v1"
    },
    "package-verify": {
      "enabled": true,
      "endpoint": "/verify",
      "version": "v1"
    },
    "public-key": {
      "enabled": true,
      "endpoint": "/pubkeys",
      "version": "v1"
    },
    "timestamp": {
      "enabled": true,
      "endpoint": "/timestamp",
      "version": "v1",
      "rfc3161_compatible": true
    }
  },
  "auth_methods": ["bearer", "api_key", "ucx_signature"],
  "rate_limits": {
    "anonymous": 100,
    "authenticated": 1000,
    "unit": "requests_per_hour"
  }
}
```

## 10. 工具命令参考

### 10.1 服务相关命令

```bash
# 注册身份
ucx identity register --name "张三" --email "author@example.com"

# 查看身份信息
ucx identity info

# 申请证书
ucx cert request --type verified --key author.key

# 注册公钥
ucx pubkey register --key author.key

# 查找公钥
ucx pubkey lookup --email "author@example.com"

# 在线验证 UCX 文件（完整验证：在线哈希清单比对 + 本地文件校验）
ucx verify --online novel.ucx

# 快速在线验证（仅签名 + MANIFEST 哈希比对，不提交文件清单）
ucx verify --online --quick novel.ucx

# 本地文件校验（默认行为，不联网，逐文件比对 MANIFEST.MF 中的哈希记录）
ucx verify novel.ucx

# 检查证书状态
ucx cert check author.cert.pem

# 获取可信时间戳
ucx timestamp novel.ucx

# 查看服务商信息
ucx service info
ucx service info --provider https://keys.example.com

# 配置第三方服务商
ucx config set services.key-distribution.provider "https://keys.example.com/api"
ucx config set services.identity.provider "https://id.my-platform.com/api"
```

### 10.2 加密相关命令

```bash
# 加密指定章节
ucx encrypt --chapter chapter-002.md \
            --algorithm AES-256-GCM \
            --key-access service \
            --provider "https://api.unicodex.org/keys" \
            --service-type purchase_verify

# 加密指定资源
ucx encrypt --resource assets/images/img-001.jpg \
            --algorithm AES-256-GCM \
            --key-access direct \
            --key "base64-key-here"

# 批量加密（根据配置文件）
ucx encrypt --config encryption.toml

# 解密测试
ucx decrypt --chapter chapter-002.md --key "base64-key"

# 查看加密状态
ucx info --encryption novel.ucx
# 输出示例：
# Encryption Status:
#   chapter-002.md      : AES-256-GCM (service: purchase_verify)
#   extra-002.ucxc      : AES-256-GCM (message + url)
#   assets/img-001.jpg  : AES-256-GCM (service: purchase_verify)
#   [9 files unencrypted]
```

---

*请审阅并指出需要修改的部分*
