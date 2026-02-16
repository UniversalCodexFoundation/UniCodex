# UCX 文件结构规范

> 版本：0.1.0-draft
> 依赖：ZIP 格式（PKWARE APPNOTE 6.3.10）

## 1. 概述

UCX 文件本质是一个 **ZIP 压缩包**，内部按照本规范组织目录与文件。

**核心原则**：UCX 文件**必须经过签名验证**才能被视为有效文件。

### 1.1 设计参考

| 格式 | 借鉴点 |
|-----|-------|
| JAR (Java) | META-INF 签名机制、MANIFEST.MF / .SF / .RSA 三层验证 |
| APK (Android) | 双层签名体系（v1 + v2），全局签名块 |
| EPUB | 内容组织方式、mimetype 文件 |

### 1.2 文件识别

```
文件后缀    : .ucx
MIME Type  : application/vnd.unicodex+zip
Magic Number: ZIP 头 (50 4B 03 04) + 首文件必须为 mimetype
```

## 2. 目录结构

```
example.ucx (ZIP Archive)
│
├── mimetype                    # [必需] MIME 类型声明，ZIP 第一个文件，不压缩
│
├── META-INF/                   # [必需]【核心安全区】签名与校验
│   │
│   ├── MANIFEST.MF             # [必需] 资源清单：所有文件的哈希列表
│   │
│   ├── signatures/             # [必需] 签名目录（至少一个签名者）
│   │   │
│   │   ├── AUTHOR.SF           # 作者签名文件：对 MANIFEST.MF 的摘要
│   │   ├── AUTHOR.EC           # 作者证书块：公钥 + 证书 + SF 签名
│   │   │
│   │   ├── PUBLISHER.SF        # 出版方签名文件（可选）
│   │   ├── PUBLISHER.RSA       # 出版方证书块（可选）
│   │   │
│   │   └── ... (更多签名者)
│   │
│   └── certs/                  # [可选] 证书链存储
│       ├── AUTHOR.chain.pem    # 作者证书链
│       └── PUBLISHER.chain.pem # 出版方证书链
│
├── metadata/                   # [必需]【元数据区】作品描述信息
│   ├── codex.json              # [必需] 作品核心元数据
│   └── author.json             # [可选] 作者/贡献者详细信息
│
├── content/                    # [必需]【正文区】小说内容
│   ├── struct.json             # [必需] 内容结构定义（支持无限嵌套）
│   │
│   ├── chapter-001.ucxc        # 章节文件（UCX Content，最推荐）
│   ├── chapter-002.md          # 章节文件（Markdown）
│   ├── chapter-003.mdx         # 章节文件（UCX 扩展 Markdown）
│   ├── chapter-004.txt         # 章节文件（纯文本）
│   ├── chapter-005.typ         # 章节文件（Typst，只需入口文件）
│   └── ...
│
├── assets/                     # [可选]【资源区】媒体文件
│   ├── resource.json           # [可选] 资源映射表（兼容非规范项目）
│   ├── cover.jpg               # 封面图
│   ├── images/                 # 插图目录
│   │   ├── img-001.png
│   │   └── ...
│   └── fonts/                  # 嵌入字体
│       └── custom.ttf
│
├── extensions/                 # [可选]【扩展区】第三方扩展功能
│   ├── manifest.json           # 扩展清单（声明已安装的扩展）
│   └── {extension-id}/         # 各扩展独立目录
│       └── ...
│
└── chain/                      # [可选]【溯源区】区块链存证
    └── genesis.proof           # 区块链锚定文件（TxID、链上哈希根）
```

### 2.1 目录必需性总结

| 目录/文件 | 必需性 | 说明 |
|----------|-------|------|
| `mimetype` | **必需** | 固定内容，便于文件识别 |
| `META-INF/` | **必需** | 安全区，签名与校验 |
| `META-INF/MANIFEST.MF` | **必需** | 文件哈希清单 |
| `META-INF/signatures/` | **必需** | 至少需要一个有效签名 |
| `metadata/` | **必需** | 作品元数据 |
| `metadata/codex.json` | **必需** | 核心元数据 |
| `content/` | **必需** | 正文内容 |
| `content/struct.json` | **必需** | 内容结构定义 |
| `assets/` | 可选 | 媒体资源（封面、插图、字体） |
| `extensions/` | 可选 | 第三方扩展功能 |
| `chain/` | 可选 | 区块链溯源存证 |

## 3. META-INF 核心安全区

### 3.1 设计理念

UCX 采用**强制签名验证**机制：

```
┌─────────────────────────────────────────────────────────────┐
│                  UCX 强制签名要求                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ● 有效的 UCX 文件必须包含至少一个有效签名                  │
│  ● 无签名或签名无效的文件被视为【无效 UCX 文件】            │
│  ● 双层签名体系确保防篡改：                                 │
│                                                             │
│    Layer 1: META-INF 文件级签名                             │
│    ├── MANIFEST.MF     → 每个文件的哈希                     │
│    ├── {SIGNER}.SF     → MANIFEST.MF 的摘要                 │
│    └── {SIGNER}.RSA/EC → SF 的数字签名 + 证书               │
│                                                             │
│    Layer 2: UCX Signing Block 全局签名                      │
│    └── 对整个 ZIP 结构签名，防止 ZIP 元数据篡改             │
│                                                             │
│  详细签名规范见：05-signature-spec.md                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 MANIFEST.MF（资源清单）

**位置**：`META-INF/MANIFEST.MF`

**职责**：
1. 声明 UCX 规范版本
2. 列出所有受保护文件的哈希值
3. 作为签名的基础对象

**格式**：采用 JAR MANIFEST 格式（RFC 822 风格）

```manifest
Manifest-Version: 1.0
UCX-Version: 1.0
Created-By: unicodex 0.1.0
Hash-Algorithm: BLAKE3

Name: metadata/codex.json
Size: 2048
BLAKE3-Digest: af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262

Name: content/struct.json
Size: 512
BLAKE3-Digest: 7d865e959b2466918c9863afca942d0fb89d7c9ac0c99bafc3749504ded97730

Name: content/chapter-001.ucxc
Size: 15360
BLAKE3-Digest: e4cfa39a3d37be31c59609e807970799caa68a19bfaa15135f165085e01d41a65
```

**字段说明**：

| 字段 | 位置 | 必需 | 说明 |
|-----|------|-----|------|
| `Manifest-Version` | 主块 | 是 | 固定为 `1.0` |
| `UCX-Version` | 主块 | 是 | UCX 规范版本 |
| `Created-By` | 主块 | 否 | 生成工具标识 |
| `Hash-Algorithm` | 主块 | 是 | 哈希算法：`BLAKE3` / `SHA256` / `SHA512` |
| `Name` | 条目 | 是 | 文件路径（相对于 ZIP 根目录） |
| `Size` | 条目 | 是 | 文件大小（字节） |
| `{Algorithm}-Digest` | 条目 | 是 | Base64 编码的哈希值 |

**不纳入清单的文件**：
- `mimetype` - 固定内容，由规范保证
- `META-INF/MANIFEST.MF` - 自身
- `META-INF/signatures/*` - 签名文件
- `META-INF/certs/*` - 证书文件

### 3.3 签名文件（.SF）

**位置**：`META-INF/signatures/{SIGNER}.SF`

**命名规则**：
- `AUTHOR.SF` - 作者签名
- `PUBLISHER.SF` - 出版方签名
- `PLATFORM.SF` - 平台签名
- 自定义：`{IDENTIFIER}.SF`（大写字母、数字、下划线）

**格式**：

```manifest
Signature-Version: 1.0
UCX-Version: 1.0
Hash-Algorithm: BLAKE3
Created-By: unicodex 0.1.0

BLAKE3-Digest-Manifest: 8c7dd922ad47494fc02c388e12c00eac12c7dd922ad47494fc02c388e12c00ea
BLAKE3-Digest-Manifest-Main-Attr: kR7LjKf7zBcU+oP1bHTMvZ4dB5f8G8msGMm/w3SVBe0=
```

### 3.4 证书块（.RSA / .EC）

**位置**：`META-INF/signatures/{SIGNER}.RSA` 或 `{SIGNER}.EC`

**后缀规则**：
- `.RSA` - RSA 算法签名
- `.EC` - ECDSA / Ed25519 椭圆曲线签名

**格式**：PKCS#7 SignedData（DER 编码）

**内容**：
1. 签名者的 X.509 证书（含公钥）
2. 对 `.SF` 文件的数字签名
3. 可选：证书链

### 3.5 多签名机制

UCX 支持多个签名者独立签名，互不干扰：

```
META-INF/signatures/
├── AUTHOR.SF + AUTHOR.EC           # 作者（必需，至少有一个签名者）
├── PUBLISHER.SF + PUBLISHER.RSA    # 出版方（可选）
├── PLATFORM.SF + PLATFORM.EC       # 分发平台（可选）
└── COAUTHOR_01.SF + COAUTHOR_01.EC # 联合作者（可选）
```

**验证规则**：
1. **至少一个签名必须有效**，否则文件无效
2. 每个签名独立验证
3. 验证结果分别报告
4. 应用层可定义更严格策略（如：必须有作者+出版方）

## 4. 内容区规范

### 4.1 struct.json（内容结构）

**位置**：`content/struct.json`

**职责**：定义作品的逻辑结构，支持无限嵌套层级

**设计原则**：
- 仅 `title` 为必需字段
- `file` 和 `children` **互斥**，不可同时存在
- `type`、`id`、`name` 为可选，作为注释/标记用途
- 支持任意深度的 `children` 嵌套
- 可选 `style` 参数用于样式控制
- 可选 `encryption` 参数用于章节级加密（每个章节独立控制）

```json
{
  "$schema": "https://unicodex.org/schemas/struct-v1.json",
  "version": "1.0",
  "structure": [
    {
      "title": "第一卷 起始",
      "type": "volume",
      "style": "volume-title",
      "children": [
        {
          "title": "第一章 开端",
          "file": "chapter-001.ucxc"
        },
        {
          "title": "第二章 相遇",
          "file": "chapter-002.md",
          "id": "ch-002",
          "encryption": {
            "algorithm": "AES-256-GCM",
            "key_access": [
              {
                "method": "service",
                "provider": "https://api.unicodex.org/keys",
                "service_type": "purchase_verify"
              }
            ]
          }
        },
        {
          "title": "番外篇",
          "children": [
            {
              "title": "番外一 前传",
              "file": "extra-001.txt"
            },
            {
              "title": "番外二 后日谈",
              "file": "extra-002.ucxc",
              "style": "epilogue",
              "encryption": {
                "algorithm": "AES-256-GCM",
                "key_access": [
                  {
                    "method": "message",
                    "text": "本章为付费内容，请联系 author@example.com 获取密钥"
                  },
                  {
                    "method": "url",
                    "url": "https://example.com/unlock/extra-002",
                    "auth_type": "login"
                  }
                ]
              }
            }
          ]
        }
      ]
    },
    {
      "title": "附录",
      "children": [
        {
          "title": "世界观设定",
          "file": "appendix/worldview.md"
        },
        {
          "title": "人物关系图",
          "file": "appendix/characters.typ",
          "name": "使用 Typst 排版的人物关系图"
        }
      ]
    }
  ]
}
```

**字段说明**：

| 字段 | 必需 | 说明 |
|------|------|------|
| `title` | **是** | 显示标题 |
| `file` | 互斥 | 内容文件路径（与 `children` 互斥） |
| `children` | 互斥 | 子节点数组（与 `file` 互斥） |
| `type` | 否 | 类型标记（如 volume/chapter/extra），仅作注释 |
| `id` | 否 | 唯一标识符，仅作注释或程序引用 |
| `name` | 否 | 备注说明，仅作注释 |
| `style` | 否 | 样式标识，供阅读器渲染时使用 |
| `encryption` | 否 | 加密配置对象，设置后该章节内容将被加密存储（详见下文） |

**互斥规则**：
- 有 `file` → 叶子节点，指向实际内容文件，**不可有 `children`**
- 有 `children` → 容器节点，组织结构层级，**不可有 `file`**
- 两者必须二选一

#### 4.1.1 章节加密（encryption）

**概述**：每个章节可独立设置加密。无 `encryption` 字段或值为 `null` 时，该章节不加密。一旦设置加密，**必须**同时提供解密方式（`key_access`）。

**encryption 对象结构**：

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `algorithm` | **是** | string | 加密算法标识（详见 [04-crypto-spec.md](./04-crypto-spec.md)） |
| `key_access` | **是** | array | 密钥获取方式列表（至少一项），支持多种方式作为备选 |

**支持的加密算法**：

| 标识 | 算法 | 状态 |
|------|------|------|
| `AES-256-GCM` | AES-256 Galois/Counter Mode | **推荐** |
| `AES-256-CBC` | AES-256 Cipher Block Chaining | 支持 |
| `ChaCha20-Poly1305` | ChaCha20 + Poly1305 AEAD | 支持 |

**key_access 数组**：

数组中的每个元素代表一种获取密钥的方式。阅读器应按数组顺序尝试，支持多种方式并存（如：同时提供直接密钥和 URL 验证两种途径）。

**通用字段**：

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `method` | **是** | string | 获取方式标识 |

**method 取值与对应字段**：

##### method: `direct`（直接嵌入密钥）

适用于简单加密场景，密钥直接嵌入文件中。

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `key` | **是** | string | Base64 编码的加密密钥 |
| `iv` | 否 | string | Base64 编码的初始化向量（部分算法需要） |

```json
{
  "method": "direct",
  "key": "dGhpcyBpcyBhIHNlY3JldCBrZXkgZm9yIGRlbW8=",
  "iv": "c29tZS1pdi12YWx1ZQ=="
}
```

##### method: `message`（人工提示信息）

提供人类可读的文本指引，告知读者如何获取密钥。

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `text` | **是** | string | 提示文本，告知获取密钥的方式 |

```json
{
  "method": "message",
  "text": "本章为付费内容，请联系 author@example.com 获取密钥"
}
```

##### method: `url`（URL 验证获取）

通过 URL 接口获取密钥，支持多种认证方式。

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `url` | **是** | string | 密钥获取接口地址 |
| `auth_type` | **是** | string | 认证方式标识 |
| `params` | 否 | object | 额外参数（由具体接口定义） |

**auth_type 取值**：

| 值 | 含义 | 说明 |
|----|------|------|
| `none` | 无需认证 | 公开接口，直接请求获取密钥 |
| `login` | 账号登录 | 用户名/密码登录后获取 |
| `token` | Token 验证 | Bearer Token / API Key 验证 |
| `oauth` | OAuth 2.0 | 第三方 OAuth 授权流程 |
| `certificate` | 证书认证 | 客户端证书认证 |

```json
{
  "method": "url",
  "url": "https://example.com/api/keys/chapter-002",
  "auth_type": "login"
}
```

##### method: `service`（官方/第三方服务）

通过 Unicodex 官方服务或第三方服务商获取密钥。

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `provider` | **是** | string | 服务提供商 URL |
| `service_type` | **是** | string | 服务类型标识 |
| `params` | 否 | object | 服务参数 |

**service_type 取值**（内置）：

| 值 | 含义 |
|----|------|
| `purchase_verify` | 购买验证（验证用户已购买该内容） |
| `author_verify` | 作者身份验证（验证请求者为关联作者） |
| `subscription` | 订阅验证（验证用户订阅状态） |
| `public_key` | 公钥交换（使用读者公钥加密密钥） |

```json
{
  "method": "service",
  "provider": "https://api.unicodex.org/keys",
  "service_type": "purchase_verify",
  "params": {
    "product_id": "novel-001-ch02"
  }
}
```

##### method: `extension`（扩展自定义）

通过本地安装的扩展实现自定义的解密方式，实现完全灵活的密钥管理。

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `ext` | **是** | string | 扩展引用路径，格式见下文 |
| `params` | 否 | object | 扩展参数 |

**ext 引用格式**：

扩展引用使用 `ext.` 前缀，支持两种格式：

| 格式 | 示例 | 说明 |
|------|------|------|
| 别名引用 | `ext.custom-drm` | 使用 manifest.json 中定义的 `alias`（推荐，简洁） |
| 完整名称 | `ext.org.unicodex.custom-drm@1.0.0` | 使用完整的 `{provider}.{id}@{version}`（无歧义） |

**示例**：

```json
{
  "method": "extension",
  "ext": "ext.custom-drm",
  "params": {
    "license_server": "https://drm.example.com",
    "content_id": "ch-002"
  }
}
```

使用完整名称引用：

```json
{
  "method": "extension",
  "ext": "ext.org.unicodex.custom-drm@1.0.0",
  "params": {
    "content_id": "ch-002"
  }
}
```

> **扩展解析流程**：阅读器解析 `ext.xxx` 时，首先在 `extensions/manifest.json` 中查找匹配的 `alias`，若未找到则尝试解析为完整名称 `{provider}.{id}@{version}`，最终定位到 `extensions/{provider}/{id}/` 目录下的扩展。
>
> **扩展目录结构详见**：[§5. 扩展区](#5-扩展区extensions可选)

##### 完整加密示例（多方式并存）

```json
{
  "title": "第三章 VIP 内容",
  "file": "chapter-003.ucxc",
  "encryption": {
    "algorithm": "AES-256-GCM",
    "key_access": [
      {
        "method": "service",
        "provider": "https://api.unicodex.org/keys",
        "service_type": "purchase_verify",
        "params": { "product_id": "novel-001-ch03" }
      },
      {
        "method": "extension",
        "ext": "ext.custom-drm",
        "params": { "content_id": "ch-003" }
      },
      {
        "method": "url",
        "url": "https://my-novel.com/api/unlock/ch03",
        "auth_type": "oauth"
      },
      {
        "method": "message",
        "text": "若以上方式不可用，请发送邮件至 author@example.com，附上购买凭证"
      }
    ]
  }
}
```

> **容器节点加密传播**：`encryption` 字段仅对叶子节点（有 `file` 的节点）生效。容器节点（有 `children` 的节点）**不支持** `encryption`，每个叶子节点需单独设置。
>
> **详细加密规范**：加密算法细节、密钥派生、加密文件格式等内容详见 [04-crypto-spec.md](./04-crypto-spec.md)。
> **官方服务规范**：Unicodex 官方提供的密钥分发、身份验证等服务详见 [09-official-services.md](./09-official-services.md)。

### 4.2 章节文件格式

UCX 支持多种章节文件格式：

| 后缀 | 格式名称 | 推荐场景 | 说明 |
|-----|---------|---------|------|
| `.ucxc` | UCX Content | **最推荐** | 复杂结构文件，支持富文本标记，由编辑器自动生成与修改 |
| `.mdx` | UCX Markdown | 推荐 | UCX 官方对 Markdown 的扩展，支持自定义组件 |
| `.md` | Markdown | 通用 | 标准 CommonMark 格式 |
| `.txt` | Plain Text | 简单小说 | 纯文本，适合无格式要求的作品 |
| `.typ` | Typst | 推荐 | 现代排版系统，适合复杂版式 |
| `.tex` | LaTeX | 兼容 | 传统学术排版，适合数学公式密集内容 |

**排版系统说明**（Typst / LaTeX）：
- 这类格式通常只需指定**入口文件**
- 入口文件可通过 `#include` / `\input` 引用其他文件
- UCX 打包时应包含所有被引用的文件
- 阅读器需支持对应的渲染引擎或预编译为 PDF

**格式详细规范**：见 [03-content-format.md](./03-content-format.md)

## 5. 扩展区（extensions/）【可选】

### 5.1 设计理念

扩展区允许第三方在不修改核心规范的情况下添加功能。

**原则**：
- 扩展不影响核心文件的解析
- 不支持某扩展的阅读器应忽略该扩展
- 扩展数据同样受 MANIFEST.MF 哈希保护

### 5.2 目录结构

扩展目录采用**命名空间 + 版本**结构，按服务商（provider）分组并支持多版本共存，避免扩展重名冲突和版本依赖问题。

```
extensions/
├── manifest.json                           # 扩展清单
│
├── org.unicodex/                           # Unicodex 官方扩展（服务商命名空间）
│   │
│   ├── audio-book/                         # 有声书扩展
│   │   ├── 1.0.0/                          # 版本 1.0.0
│   │   │   ├── extension.json              # 扩展元数据
│   │   │   ├── chapter-001.mp3
│   │   │   └── chapter-002.mp3
│   │   └── 2.0.0/                          # 版本 2.0.0（新版本共存）
│   │       ├── extension.json
│   │       └── ...
│   │
│   ├── interactive/                        # 互动小说扩展
│   │   └── 2.1.0/
│   │       ├── extension.json
│   │       └── choices.json
│   │
│   └── custom-drm/                         # 自定义 DRM 扩展
│       ├── 1.0.0/
│       │   ├── extension.json
│       │   └── handler.wasm
│       └── 1.2.0/                          # 多版本支持依赖链
│           ├── extension.json
│           └── handler.wasm
│
├── com.example/                            # 第三方服务商 A
│   └── ai-summary/
│       ├── 2.1.0/
│       │   ├── extension.json
│       │   └── summaries.json
│       └── 3.0.0/
│           └── ...
│
└── app.my-platform/                        # 第三方服务商 B
    └── vip-unlock/
        └── 1.0.0/
            ├── extension.json
            └── config.json
```

**目录层级**：`extensions/{provider}/{id}/{version}/`

**命名空间规范**：

| 命名空间格式 | 说明 | 示例 |
|-------------|------|------|
| `org.unicodex` | Unicodex 官方扩展 | `org.unicodex/audio-book/1.0.0/` |
| `com.{domain}` | 企业/组织（域名反转） | `com.example/ai-summary/2.1.0/` |
| `app.{name}` | 独立应用/平台 | `app.my-platform/vip-unlock/1.0.0/` |
| `io.{name}` | 开源项目/社区 | `io.github-user/custom-ext/1.0.0/` |
| `dev.{name}` | 开发者个人扩展 | `dev.zhangsan/my-drm/0.1.0/` |

**扩展完整标识符**：`{provider}.{id}@{version}`

示例：
- `org.unicodex.audio-book@1.0.0` → `extensions/org.unicodex/audio-book/1.0.0/`
- `org.unicodex.audio-book@2.0.0` → `extensions/org.unicodex/audio-book/2.0.0/`
- `com.example.ai-summary@2.1.0` → `extensions/com.example/ai-summary/2.1.0/`

**多版本共存的意义**：

```
┌─────────────────────────────────────────────────────────────────┐
│                   扩展多版本共存机制                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  场景：扩展 A（v2.0）依赖扩展 B（v1.0），扩展 C 依赖 B（v2.0）  │
│                                                                 │
│  extensions/                                                    │
│  └── org.unicodex/                                              │
│      ├── ext-a/                                                 │
│      │   └── 2.0.0/         ← 依赖 ext-b@1.0.0                  │
│      ├── ext-b/                                                 │
│      │   ├── 1.0.0/         ← 被 ext-a 依赖                     │
│      │   └── 2.0.0/         ← 被 ext-c 依赖                     │
│      └── ext-c/                                                 │
│          └── 1.0.0/         ← 依赖 ext-b@2.0.0                  │
│                                                                 │
│  优势：                                                         │
│  ● 避免版本冲突导致的功能异常                                   │
│  ● 支持复杂的依赖链                                             │
│  ● 允许渐进式升级                                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 5.3 extensions/manifest.json

```json
{
  "$schema": "https://unicodex.org/schemas/extensions-manifest-v1.json",
  "version": "1.0",
  "extensions": [
    {
      "url": "https://unicodex.org/extensions/audio-book@1.0.0",
      "alias": "audio-book"
    },
    {
      "url": "https://unicodex.org/extensions/interactive@2.1.0",
      "alias": "interactive",
      "name": "互动小说"
    },
    {
      "url": "https://unicodex.org/extensions/custom-drm@1.0.0",
      "alias": "custom-drm",
      "name": "自定义 DRM"
    },
    {
      "url": "https://example.com/ucx-extensions/ai-summary@2.1.0",
      "alias": "ai-summary",
      "name": "AI 摘要",
      "params": {
        "api_url": "https://api.example.com/v1/summary",
        "model": "gpt-4"
      }
    },
    {
      "url": "https://my-platform.app/extensions/vip-unlock@1.0.0",
      "alias": "vip",
      "name": "VIP 解锁"
    }
  ]
}
```

**字段说明**：

| 字段 | 必需 | 说明 |
|------|------|------|
| `url` | **是** | 扩展规范文档 URL，格式：`https://{host}/extensions/{id}@{version}` |
| `alias` | 否 | 短别名，用于 `ext.{alias}` 引用（推荐设置，避免使用冗长的完整名称） |
| `name` | 否 | 扩展显示名称 |
| `required` | 否 | 是否为必需扩展（默认 false） |
| `params` | 否 | 扩展参数表，键值对形式，供扩展使用 |

**URL 到目录的映射规则**：

| URL | 目录路径 |
|-----|---------|
| `https://unicodex.org/extensions/audio-book@1.0.0` | `org.unicodex/audio-book/1.0.0/` |
| `https://unicodex.org/extensions/audio-book@2.0.0` | `org.unicodex/audio-book/2.0.0/` |
| `https://example.com/ucx-extensions/ai-summary@2.1.0` | `com.example/ai-summary/2.1.0/` |
| `https://my-platform.app/extensions/vip-unlock@1.0.0` | `app.my-platform/vip-unlock/1.0.0/` |

**alias 别名规则**：

- 别名在单个 UCX 文件内必须唯一
- 不同服务商的同名扩展可通过不同别名区分
- 引用时优先使用别名：`ext.custom-drm` 而非 `ext.org.unicodex.custom-drm@1.0.0`
- 若未设置别名，只能使用完整名称引用

**处理重名扩展**：

```json
{
  "extensions": [
    {
      "url": "https://unicodex.org/extensions/drm@1.0.0",
      "alias": "official-drm",
      "name": "官方 DRM"
    },
    {
      "url": "https://example.com/ucx-extensions/drm@2.0.0",
      "alias": "custom-drm",
      "name": "自定义 DRM"
    }
  ]
}
```

引用时：
- `ext.official-drm` → `extensions/org.unicodex/drm/1.0.0/`
- `ext.custom-drm` → `extensions/com.example/drm/2.0.0/`

### 5.4 扩展元数据（extension.json）

每个扩展版本目录下必须包含 `extension.json` 文件，声明扩展的元数据。

**位置**：`extensions/{provider}/{id}/{version}/extension.json`

```json
{
  "$schema": "https://unicodex.org/schemas/extension-v1.json",
  "id": "custom-drm",
  "provider": "org.unicodex",
  "version": "1.0.0",
  "name": "自定义 DRM",
  "description": "提供灵活的数字版权管理解密方式",
  "url": "https://unicodex.org/extensions/custom-drm@1.0.0",
  "capabilities": ["key_access"],
  "entry": "handler.wasm",
  "dependencies": [
    {
      "id": "org.unicodex.crypto-utils@1.2.0",
      "alias": "crypto"
    }
  ],
  "params_schema": {
    "license_server": { "type": "string", "required": true },
    "content_id": { "type": "string", "required": false }
  }
}
```

**字段说明**：

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `id` | **是** | string | 扩展标识符 |
| `provider` | **是** | string | 服务商命名空间 |
| `version` | **是** | string | 扩展版本号（语义化版本） |
| `name` | 否 | string | 扩展显示名称 |
| `description` | 否 | string | 扩展描述 |
| `url` | 否 | string | 扩展规范文档 URL |
| `capabilities` | 否 | array | 扩展能力声明（如 `key_access`、`render`、`export`） |
| `entry` | 否 | string | 扩展入口文件（如 WASM 模块） |
| `dependencies` | 否 | array | 依赖的其他扩展（含版本号） |
| `params_schema` | 否 | object | 参数定义（供验证和 UI 生成） |

**dependencies（依赖声明）**：

扩展可声明对其他扩展的依赖，阅读器在加载时应确保依赖的扩展（及指定版本）已存在。

```json
"dependencies": [
  {
    "id": "org.unicodex.crypto-utils@1.2.0",
    "alias": "crypto",
    "optional": false
  },
  {
    "id": "org.unicodex.ui-components@2.0.0",
    "alias": "ui",
    "optional": true
  }
]
```

### 5.5 官方预留扩展 ID

以下扩展 ID 在 `org.unicodex` 命名空间下预留：

| 扩展 ID | 完整标识符 | 说明 | 状态 |
|--------|-----------|------|------|
| `audio-book` | `org.unicodex.audio-book` | 有声书音频 | 预留 |
| `interactive` | `org.unicodex.interactive` | 互动小说/分支剧情 | 预留 |
| `ai-summary` | `org.unicodex.ai-summary` | AI 生成摘要 | 预留 |
| `translation` | `org.unicodex.translation` | 多语言翻译 | 预留 |
| `annotations` | `org.unicodex.annotations` | 批注/注释 | 预留 |
| `drm` | `org.unicodex.drm` | 数字版权管理 | 预留 |
| `custom-drm` | `org.unicodex.custom-drm` | 自定义 DRM 解密 | 预留 |

### 5.6 扩展引用总结

```
┌─────────────────────────────────────────────────────────────────┐
│                     扩展引用机制                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  引用方式：                                                     │
│  ├── ext.{alias}                  → 使用别名（推荐，简洁）      │
│  └── ext.{provider}.{id}@{ver}    → 使用完整名称（无歧义）      │
│                                                                 │
│  文件目录：extensions/{provider}/{id}/{version}/                │
│  ├── org.unicodex/audio-book/1.0.0/   → 官方有声书 v1.0.0       │
│  ├── org.unicodex/audio-book/2.0.0/   → 官方有声书 v2.0.0       │
│  ├── com.example/ai-summary/2.1.0/    → 第三方 AI 摘要          │
│  └── app.my-platform/vip-unlock/1.0.0/→ 平台 VIP 扩展           │
│                                                                 │
│  解析流程：                                                     │
│  1. ext.xxx → 查找 manifest.json 中 alias="xxx" 的条目          │
│  2. 从条目的 url 解析 {provider}.{id}@{version}                 │
│  3. 若未通过别名找到 → 尝试直接解析完整名称                     │
│  4. 定位目录 → extensions/{provider}/{id}/{version}/            │
│  5. 加载扩展 → 读取 extension.json，检查依赖，执行 entry 模块   │
│                                                                 │
│  多版本共存：                                                   │
│  同一扩展的多个版本可同时存在于 UCX 文件中，                    │
│  由 manifest.json 中的 alias 确定当前使用的版本，               │
│  依赖链中的其他版本由 extension.json 的 dependencies 指定。     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 6. 溯源区（chain/）【可选】

### 6.1 genesis.proof（区块链锚定文件）

**位置**：`chain/genesis.proof`

**职责**：记录作品在区块链上的存证信息

**格式**：

```json
{
  "$schema": "https://unicodex.org/schemas/genesis-proof-v1.json",
  "version": "1.0",
  "anchors": [
    {
      "chain": "ethereum",
      "network": "mainnet",
      "tx_id": "0x1234567890abcdef...",
      "block_number": 19000000,
      "block_hash": "0xabcdef...",
      "timestamp": "2025-01-29T12:00:00Z",
      "manifest_hash": "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
      "merkle_root": "7d865e959b2466918c9863afca942d0fb89d7c9ac0c99bafc3749504ded97730",
      "explorer_url": "https://etherscan.io/tx/0x1234..."
    }
  ]
}
```

**详细规范**：见 [06-blockchain-ext.md](./06-blockchain-ext.md)

## 7. 哈希算法

### 7.1 支持的算法

| 标识 | 算法 | 输出长度 | 状态 |
|-----|------|---------|------|
| `BLAKE3` | BLAKE3 | 256 bit | **推荐** |
| `SHA256` | SHA-256 | 256 bit | 兼容 |
| `SHA512` | SHA-512 | 512 bit | 兼容 |

### 7.2 BLAKE3 推荐理由

1. **性能**：比 SHA-256 快 3-5 倍（SIMD 优化）
2. **安全**：基于 BLAKE2 + ChaCha，充分密码学分析
3. **Rust 原生**：官方参考实现即为 Rust
4. **流式处理**：支持增量哈希，适合大文件

## 8. 签名算法

### 8.1 支持的算法

| 标识 | 算法 | 密钥长度 | 文件后缀 | 状态 |
|-----|------|---------|---------|------|
| `Ed25519` | EdDSA (Curve25519) | 256 bit | `.EC` | **推荐** |
| `ECDSA-P256` | ECDSA (secp256r1) | 256 bit | `.EC` | 支持 |
| `RSA-2048` | RSA-PKCS#1 v1.5 | 2048 bit | `.RSA` | 兼容 |
| `RSA-4096` | RSA-PKCS#1 v1.5 | 4096 bit | `.RSA` | 兼容 |

### 8.2 Ed25519 推荐理由

1. **速度快**：签名/验证比 RSA 快数十倍
2. **密钥短**：64 字节 vs RSA 的 256+ 字节
3. **安全性高**：抗侧信道攻击
4. **确定性签名**：相同输入产生相同签名，便于验证

## 9. 文件命名规范

### 9.1 通用规则

- 使用 UTF-8 编码
- 路径分隔符统一使用 `/`
- META-INF 下文件名使用**大写**
- 其他目录使用**小写字母、数字、连字符（`-`）、下划线（`_`）**

### 9.2 保留路径

```
mimetype                    # 必需，固定内容
META-INF/                   # 安全区
META-INF/MANIFEST.MF        # 资源清单
META-INF/signatures/        # 签名目录
META-INF/certs/             # 证书目录
metadata/                   # 元数据区
metadata/codex.json         # 核心元数据
content/                    # 正文区
content/struct.json         # 结构定义
assets/                     # 资源区（可选）
assets/resource.json        # 资源映射（可选）
extensions/                 # 扩展区（可选）
chain/                      # 溯源区（可选）
```

## 10. 压缩策略

| 内容类型 | 压缩方式 | 原因 |
|---------|---------|------|
| `mimetype` | STORED (不压缩) | 便于文件类型识别 |
| `META-INF/*` | DEFLATE | 文本/二进制均可压缩 |
| `*.json` | DEFLATE | 文本压缩效果好 |
| `*.ucxc` / `*.md` / `*.mdx` / `*.txt` | DEFLATE | 文本压缩效果好 |
| `*.typ` / `*.tex` | DEFLATE | 排版源文件，文本压缩效果好 |
| 图片 (`jpg`/`png`/`webp`) | STORED | 已压缩格式 |
| 字体 (`ttf`/`otf`/`woff2`) | STORED | 已压缩或压缩收益低 |
| 音频 (`mp3`/`ogg`) | STORED | 已压缩格式 |
| PDF (`*.pdf`) | STORED | 已压缩格式 |

## 11. 版本兼容性

### 11.1 版本号规则

语义化版本：`MAJOR.MINOR`

- **MAJOR** 变更：结构性变化，不向后兼容
- **MINOR** 变更：新增功能，向后兼容

### 11.2 处理策略

| 情况 | 处理方式 |
|-----|---------|
| 必需文件缺失 | 拒绝打开，报错 |
| 签名缺失 | **警告**，可强制打开（需用户确认） |
| 签名无效 | **警告**，可强制打开（需用户确认） |
| 未知目录/文件 | 忽略，正常读取 |
| 未知 JSON 字段 | 忽略但保留（编辑时不删除） |
| 版本号高于实现支持 | 警告，尝试读取 |

**高自由度原则**：UCX 标准鼓励签名验证，但不强制阻止用户访问未签名或签名无效的内容。实现应：
1. 默认进行完整签名验证
2. 签名问题时显示明确警告
3. 提供"强制打开"选项（需用户主动确认风险）
4. 在 UI 中清晰标识文件的签名状态

---

## 附录 A：完整验证流程

```
┌─────────────────────────────────────────────────────────────┐
│                    UCX 文件验证流程                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. 基础检查                                                │
│     ├─ 检查文件是否为有效 ZIP                               │
│     └─ 检查 mimetype 内容为 application/vnd.unicodex+zip    │
│                                                             │
│  2. 签名验证（Layer 2 优先）                                │
│     ├─ 定位 UCX Signing Block                               │
│     ├─ 验证全局签名（防止 ZIP 结构篡改）                    │
│     └─ 记录验证结果                                         │
│                                                             │
│  3. 清单验证（Layer 1）                                     │
│     ├─ 解析 META-INF/MANIFEST.MF                            │
│     ├─ 逐文件校验哈希                                       │
│     └─ 记录验证结果                                         │
│                                                             │
│  4. 签名者验证                                              │
│     ├─ 遍历 META-INF/signatures/*.SF                        │
│     ├─ 验证每个 SF 对 MANIFEST.MF 的摘要                    │
│     ├─ 验证每个 RSA/EC 对 SF 的签名                         │
│     ├─ 验证证书有效性（链、有效期、吊销）                   │
│     └─ 记录各签名者验证状态                                 │
│                                                             │
│  5. 返回验证结果与决策                                      │
│     ├─ archive_integrity: 全局签名状态                      │
│     ├─ file_integrity: 各文件哈希状态                       │
│     ├─ signers[]: 各签名者验证状态与身份信息                │
│     │                                                       │
│     ├─ 【全部通过】→ 正常打开                               │
│     ├─ 【签名缺失/无效】→ 警告 + 允许强制打开               │
│     └─ 【结构损坏】→ 拒绝打开                               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

*请审阅并指出需要修改的部分*
