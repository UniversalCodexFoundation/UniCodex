# UCX 元数据规范

> 版本：0.1.0-draft
> 前置依赖：[01-file-structure.md](./01-file-structure.md)

## 1. 概述

元数据区（`metadata/`）存储作品的描述性信息，是 UCX 文件的必需组成部分。

### 1.1 文件结构

```
metadata/
├── codex.json              # [必需] 作品核心元数据
└── author.json             # [可选] 作者/贡献者详细信息
```

## 2. codex.json（作品核心元数据）

**位置**：`metadata/codex.json`

**编码**：UTF-8，JSON 格式

### 2.1 完整结构

```json
{
  "$schema": "https://unicodex.org/schemas/codex-v1.json",
  "version": "1.0",

  "identifier": {
    "ucx_id": "urn:ucx:550e8400-e29b-41d4-a716-446655440000",
    "isbn": "978-3-16-148410-0",
    "custom": {
      "platform_id": "qidian-123456"
    }
  },

  "title": {
    "main": "作品主标题",
    "subtitle": "副标题",
    "original": "原文标题（翻译作品适用）",
    "short": "简称"
  },

  "series": {
    "name": "系列名称",
    "index": 1,
    "total": 5
  },

  "creators": [
    {
      "name": "张三",
      "role": "author",
      "signature_ref": "AUTHOR"
    }
  ],

  "publisher": {
    "name": "某某出版社",
    "signature_ref": "PUBLISHER"
  },

  "language": "zh-CN",
  "genre": ["fantasy", "romance"],
  "tags": ["穿越", "重生", "系统"],
  "status": "completed",
  "word_count": 1500000,

  "description": {
    "short": "一句话简介",
    "long": "详细简介，支持多行文本..."
  },

  "rights": {
    "statement": "版权所有 © 2025 张三",
    "license": "CC-BY-NC-4.0"
  },

  "dates": {
    "created": "2025-01-01",
    "published": "2025-06-01",
    "modified": "2025-12-01"
  },

  "cover": "assets/cover.jpg",

  "rating": {
    "system": "age",
    "value": "16+"
  }
}
```

### 2.2 字段定义

#### identifier（标识符）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `ucx_id` | **是** | string | UCX 唯一标识，格式 `urn:ucx:{UUID v4}` |
| `isbn` | 否 | string | ISBN（正式出版物） |
| `issn` | 否 | string | ISSN（连载刊物） |
| `doi` | 否 | string | DOI（学术/数字出版） |
| `custom` | 否 | object | 自定义标识符键值对（如平台内部 ID） |

**ucx_id 生成规则**：
- 格式：`urn:ucx:{UUID}`
- UUID 版本：v4（随机生成）
- 一旦生成不可变更，作为作品的全局唯一标识
- 同一作品不同版本共享相同 `ucx_id`

#### title（标题）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `main` | **是** | string | 作品主标题 |
| `subtitle` | 否 | string | 副标题 |
| `original` | 否 | string | 原文标题（翻译作品） |
| `short` | 否 | string | 简称/缩写 |

#### series（系列）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `name` | **是**（若存在 series） | string | 系列名称 |
| `index` | 否 | integer | 本作在系列中的序号（从 1 开始） |
| `total` | 否 | integer | 系列总数（已知时填写） |

#### creators（创作者列表）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `name` | **是** | string | 创作者名称/笔名 |
| `role` | **是** | string | 角色标识（见下表） |
| `signature_ref` | 否 | string | 关联的签名者 ID（对应 META-INF/signatures/） |

**角色标识**：

| role 值 | 含义 |
|---------|------|
| `author` | 作者 |
| `coauthor` | 合著者 |
| `translator` | 译者 |
| `editor` | 编辑 |
| `illustrator` | 插画师 |
| `cover_artist` | 封面画师 |
| `proofreader` | 校对 |
| `narrator` | 朗读者（有声书） |
| `adapter` | 改编者 |

自定义角色：任意字符串均合法，推荐使用小写英文 + 下划线。

#### publisher（出版方）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `name` | **是**（若存在 publisher） | string | 出版方名称 |
| `imprint` | 否 | string | 出版品牌/副牌 |
| `signature_ref` | 否 | string | 关联的签名者 ID |

#### 顶层字段

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `language` | **是** | string | BCP 47 语言标签（如 `zh-CN`、`en-US`、`ja`） |
| `genre` | 否 | string[] | 体裁/分类标签列表 |
| `tags` | 否 | string[] | 自由标签（关键词） |
| `status` | 否 | string | 作品状态 |
| `word_count` | 否 | integer | 总字数 |
| `cover` | 否 | string | 封面图路径（相对于 ZIP 根目录） |

**status 取值**：

| 值 | 含义 |
|----|------|
| `ongoing` | 连载中 |
| `completed` | 已完结 |
| `hiatus` | 暂停更新 |
| `abandoned` | 已弃坑 |
| `draft` | 草稿 |

**genre 推荐值**（非限定，可自由扩展）：

| 分类 | 值 |
|------|-----|
| 奇幻 | `fantasy` |
| 科幻 | `sci-fi` |
| 武侠 | `wuxia` |
| 仙侠 | `xianxia` |
| 都市 | `urban` |
| 历史 | `historical` |
| 悬疑 | `mystery` |
| 恐怖 | `horror` |
| 言情 | `romance` |
| 军事 | `military` |
| 游戏 | `game` |
| 体育 | `sports` |
| 轻小说 | `light-novel` |
| 同人 | `fanfiction` |
| 非虚构 | `non-fiction` |

#### description（作品简介）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `short` | 否 | string | 一句话简介（≤ 100 字符） |
| `long` | 否 | string | 详细简介（支持换行符 `\n`） |

#### rights（版权信息）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `statement` | 否 | string | 版权声明文本 |
| `license` | 否 | string | 许可证标识（SPDX 格式或自定义） |

**常见 license 值**：

| 值 | 含义 |
|----|------|
| `CC-BY-4.0` | 知识共享 署名 4.0 |
| `CC-BY-NC-4.0` | 知识共享 署名-非商业性 4.0 |
| `CC-BY-NC-ND-4.0` | 知识共享 署名-非商业性-禁止演绎 4.0 |
| `All-Rights-Reserved` | 保留所有权利 |
| `Public-Domain` | 公有领域 |

#### dates（日期信息）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `created` | 否 | string | 创作日期（ISO 8601：`YYYY-MM-DD`） |
| `published` | 否 | string | 出版/发布日期 |
| `modified` | 否 | string | 最后修改日期 |

#### rating（分级信息）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `system` | **是**（若存在 rating） | string | 分级系统标识 |
| `value` | **是**（若存在 rating） | string | 分级值 |

**分级系统**：

| system | 说明 | 示例值 |
|--------|------|--------|
| `age` | 年龄分级 | `all`、`12+`、`16+`、`18+` |
| `custom` | 自定义 | 任意字符串 |

### 2.3 最小有效示例

```json
{
  "$schema": "https://unicodex.org/schemas/codex-v1.json",
  "version": "1.0",
  "identifier": {
    "ucx_id": "urn:ucx:550e8400-e29b-41d4-a716-446655440000"
  },
  "title": {
    "main": "我的小说"
  },
  "creators": [
    {
      "name": "作者",
      "role": "author"
    }
  ],
  "language": "zh-CN"
}
```

**必需字段总结**：
- `identifier.ucx_id`
- `title.main`
- `creators`（至少一个，含 `name` 和 `role`）
- `language`

## 3. author.json（作者/贡献者详细信息）【可选】

**位置**：`metadata/author.json`

**用途**：当创作者信息较复杂时（多人协作、详细个人信息），从 codex.json 分离出来单独管理。

### 3.1 结构

```json
{
  "$schema": "https://unicodex.org/schemas/author-v1.json",
  "version": "1.0",
  "authors": [
    {
      "id": "author-001",
      "name": "张三",
      "pen_name": "笔名xxx",
      "role": "author",
      "signature_ref": "AUTHOR",
      "bio": "简短作者介绍",
      "contacts": {
        "email": "author@example.com",
        "website": "https://example.com",
        "social": {
          "weibo": "@zhangsan",
          "twitter": "@zhangsan_en"
        }
      },
      "chapters": ["*"]
    },
    {
      "id": "illustrator-001",
      "name": "李四",
      "role": "illustrator",
      "bio": "自由插画师",
      "chapters": ["chapter-003", "chapter-007"]
    },
    {
      "id": "translator-001",
      "name": "王五",
      "role": "translator",
      "signature_ref": "TRANSLATOR",
      "language_pair": {
        "from": "ja",
        "to": "zh-CN"
      }
    }
  ]
}
```

### 3.2 字段定义

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `id` | 否 | string | 贡献者内部标识符 |
| `name` | **是** | string | 真实姓名或显示名称 |
| `pen_name` | 否 | string | 笔名 |
| `role` | **是** | string | 角色标识（同 codex.json creators） |
| `signature_ref` | 否 | string | 关联签名者 ID |
| `bio` | 否 | string | 简短介绍 |
| `contacts` | 否 | object | 联系方式 |
| `chapters` | 否 | string[] | 参与的章节（`["*"]` 表示全部） |
| `language_pair` | 否 | object | 翻译语言对（译者适用） |

### 3.3 与 codex.json 的关系

- `codex.json` 中的 `creators` 为精简版，列出核心创作者
- `author.json` 为详细版，包含所有贡献者的完整信息
- 两者可独立存在
- 若同时存在，`author.json` 应覆盖 `codex.json` 中的对应信息

## 4. resource.json（资源映射表）【可选】

**位置**：`assets/resource.json`

**用途**：记录资源文件的映射关系，支持非规范项目结构的兼容打包。

### 4.1 设计理念

```
┌─────────────────────────────────────────────────────────────────┐
│                  resource.json 设计目的                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  场景：非规范项目（图片和小说文件混杂在一起）                       │
│                                                                 │
│  原始项目结构：                                                   │
│  project/                                                       │
│  ├── 第一章.txt                                                 │
│  ├── 第一章插图.jpg           ← 图片与文本混杂                   │
│  ├── 第二章.txt                                                 │
│  ├── images/封面.png                                            │
│  └── 角色立绘/主角.png                                          │
│                                                                 │
│  打包后 UCX 结构：                                               │
│  novel.ucx/                                                     │
│  ├── assets/                                                    │
│  │   ├── resource.json        ← 记录映射关系                    │
│  │   ├── cover.png                                              │
│  │   └── images/                                                │
│  │       ├── img-001.jpg                                        │
│  │       └── img-002.png                                        │
│  └── content/                                                   │
│      ├── chapter-001.txt                                        │
│      └── chapter-002.txt                                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 结构

```json
{
  "$schema": "https://unicodex.org/schemas/resource-v1.json",
  "version": "1.0",
  "mappings": [
    {
      "res": "res.cover[png]",
      "path": "images/封面.png"
    },
    {
      "res": "res.images.img-001[jpg]",
      "path": "第一章插图.jpg",
      "encryption": {
        "algorithm": "AES-256-GCM",
        "key_access": [
          {
            "method": "service",
            "provider": "https://api.unicodex.org/keys",
            "service_type": "purchase_verify",
            "params": { "product_id": "novel-001-img-001" }
          }
        ]
      }
    },
    {
      "res": "res.images.img-002[png]",
      "path": "角色立绘/主角.png"
    },
    {
      "res": "res.fonts.custom[ttf]",
      "path": "my-fonts/特殊字体.ttf"
    }
  ]
}
```

### 4.3 字段定义

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `res` | **是** | string | 打包后资源路径，格式：`res.path.to.file[ext]` |
| `path` | **是** | string | 原始文件相对路径（相对于项目根目录） |
| `encryption` | 否 | object | 加密配置对象，结构与 struct.json 中的 encryption 一致 |

> **资源加密说明**：每个资源可独立设置加密。无 `encryption` 字段时默认不加密。加密结构（`algorithm` + `key_access`）与 struct.json 章节加密完全一致，详见 [01-file-structure.md §4.1.1](./01-file-structure.md)。
> **官方服务规范**：Unicodex 官方提供的密钥分发、身份验证等服务详见 [09-official-services.md](./09-official-services.md)。

### 4.4 res 路径格式

```
res.{category}.{name}[{extension}]

示例：
res.cover[jpg]                    → assets/cover.jpg
res.images.chapter01[png]         → assets/images/chapter01.png
res.images.characters.hero[png]   → assets/images/characters/hero.png
res.fonts.main[ttf]               → assets/fonts/main.ttf
res.audio.bgm[mp3]                → assets/audio/bgm.mp3
```

**分类（category）推荐值**：

| 分类 | 说明 | 打包位置 |
|------|------|---------|
| `cover` | 封面（单个） | `assets/cover.{ext}` |
| `images` | 插图 | `assets/images/` |
| `fonts` | 字体 | `assets/fonts/` |
| `audio` | 音频 | `assets/audio/` |
| `video` | 视频 | `assets/video/` |

### 4.5 使用场景

**内容文件引用资源**：

```markdown
<!-- chapter-001.md -->

# 第一章

![主角立绘](res.images.img-002[png])

这是正文内容...
```

**阅读器解析流程**：
1. 解析到 `res.images.img-002[png]`
2. 查找 `assets/resource.json`
3. 找到映射：`res.images.img-002[png]` → 原路径 `角色立绘/主角.png`
4. 实际文件位置：`assets/images/img-002.png`

## 5. 项目结构规范

### 5.1 推荐项目结构（ucx init 创建）

```
my-novel/                           # 项目根目录
├── unicodex.toml                   # [必需] 项目配置文件
├── content/                        # 正文目录
│   ├── volume-01/                  # 第一卷
│   │   ├── chapter-001.md
│   │   ├── chapter-002.md
│   │   └── ...
│   └── volume-02/
│       └── ...
├── assets/                         # 资源目录
│   ├── cover.jpg                   # 封面
│   └── images/                     # 插图
│       └── ...
├── extras/                         # 额外内容（草稿、素材等，不打包）
│   └── ...
└── dist/                           # 输出目录（打包后的 .ucx 文件）
    └── my-novel.ucx
```

### 5.2 unicodex.toml（项目配置文件）

**用途**：开发时的配置文件，打包时转换为 JSON 存入 UCX。

```toml
# Unicodex 项目配置
# 此文件用于开发阶段，打包时自动转换为 metadata/codex.json

[project]
version = "1.0"                     # UCX 规范版本

[identifier]
ucx_id = "urn:ucx:550e8400-e29b-41d4-a716-446655440000"
# isbn = "978-3-16-148410-0"        # 可选

[title]
main = "我的小说"
subtitle = "一个精彩的故事"

[series]
name = "XX 系列"
index = 1

[[creators]]
name = "张三"
role = "author"
signature_ref = "AUTHOR"

[[creators]]
name = "李四"
role = "illustrator"

[publisher]
name = "某某出版社"
signature_ref = "PUBLISHER"

[book]
language = "zh-CN"
genre = ["fantasy", "romance"]
tags = ["穿越", "重生"]
status = "ongoing"

[description]
short = "一句话简介"
long = """
详细简介，
支持多行文本。
"""

[rights]
statement = "版权所有 © 2025 张三"
license = "CC-BY-NC-4.0"

[cover]
path = "assets/cover.jpg"

[rating]
system = "age"
value = "16+"

# ===== 可选功能配置 =====

[extensions]
# 扩展列表
[[extensions.list]]
url = "https://unicodex.org/extensions/audio-book@1.0.0"

[[extensions.list]]
url = "https://unicodex.org/extensions/ai-summary@1.0.0"
[extensions.list.params]
api_url = "https://api.example.com/v1"
# api_key 通过环境变量或密钥管理器提供，不写入配置

[chain]
# 区块链存证配置
enabled = true
networks = ["ethereum", "polygon"]

[signing]
# 签名配置
key_path = "~/.ucx/keys/author.key"
cert_path = "~/.ucx/certs/author.cert.pem"
signer_id = "AUTHOR"

[build]
# 构建配置
output_dir = "dist"
output_name = "my-novel"            # 输出文件名（不含 .ucx）
include_source = false              # 是否包含源文件（如 .typ 源码）
compression = "deflate"             # 压缩方式
```

### 5.3 unicodex.toml 到 JSON 的转换

**打包命令**：

```bash
ucx build
# 或
ucx pack
```

**转换规则**：

| TOML 配置 | 打包后位置 |
|-----------|-----------|
| `[identifier]`、`[title]`、`[book]` 等 | `metadata/codex.json` |
| `[[creators]]` | `metadata/codex.json` → `creators[]` |
| `[extensions]` | `extensions/manifest.json` |
| `[chain]` | `chain/genesis.proof`（仅在存证后） |
| `[signing]` | 用于签名过程，不写入 UCX |
| `[build]` | 用于构建过程，不写入 UCX |

### 5.4 快速初始化

```bash
# 创建新项目
ucx init my-novel

# 交互式创建（询问书名、作者等）
ucx init my-novel --interactive

# 从现有目录初始化（兼容非规范结构）
ucx init --from-existing ./messy-project

# 查看项目信息
ucx info

# 构建/打包
ucx build

# 带签名的构建
ucx build --sign
```

### 5.5 目录职责说明

| 目录 | 说明 | 是否打包 |
|------|------|---------|
| `content/` | 小说正文 | ✓ |
| `assets/` | 封面、插图、字体 | ✓ |
| `extras/` | 草稿、素材、备份 | ✗ |
| `dist/` | 构建输出 | - |

## 6. 元数据与签名的关联

### 6.1 signature_ref 关联机制

```
┌────────────────────────────────────────────────────────────────┐
│                  元数据 ↔ 签名 关联                             │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  metadata/codex.json                                           │
│  ├── creators[0].signature_ref: "AUTHOR"                       │
│  └── publisher.signature_ref: "PUBLISHER"                      │
│           ↓                  ↓                                 │
│  META-INF/signatures/                                          │
│  ├── AUTHOR.SF + AUTHOR.EC                                     │
│  └── PUBLISHER.SF + PUBLISHER.RSA                              │
│                                                                │
│  验证时：                                                       │
│  ├── 解析 signature_ref → 定位签名文件                          │
│  ├── 验证签名有效性                                             │
│  └── 将结果关联到对应创作者/出版方                               │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## 7. 国际化支持

### 7.1 语言标签

遵循 BCP 47 规范：

| 标签 | 含义 |
|------|------|
| `zh-CN` | 简体中文 |
| `zh-TW` | 繁体中文 |
| `en-US` | 美式英语 |
| `ja` | 日语 |
| `ko` | 韩语 |

### 7.2 多语言元数据

若需同时提供多种语言的元数据（如中英双语标题），使用 `_i18n` 后缀：

```json
{
  "title": {
    "main": "三体",
    "main_i18n": {
      "en": "The Three-Body Problem",
      "ja": "三体"
    }
  }
}
```

---

*请审阅并指出需要修改的部分*
