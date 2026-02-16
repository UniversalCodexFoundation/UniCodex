# UCX 正文内容格式规范

> 版本：0.1.0-draft
> 前置依赖：[01-file-structure.md](./01-file-structure.md)

## 1. 概述

UCX 支持多种章节文件格式，适应不同的创作场景。本文档定义各格式的详细规范。

### 1.1 格式总览

| 后缀 | 格式名称 | 特性 | 推荐场景 |
|-----|---------|------|---------|
| `.ucxc` | UCX Content | 标签式标记语言，段落级评论区绑定，可扩展标签 | **最推荐**，功能最完整 |
| `.mdx` | UCX Markdown | Markdown 增强，frontmatter 参数，评论区列表 | 推荐，兼顾可读性与功能 |
| `.md` | Markdown | 标准 CommonMark | 通用，轻量 |
| `.txt` | Plain Text | 纯文本 | 简单小说，无格式需求 |
| `.typ` | Typst | 现代排版系统 | 复杂版式，数学公式 |
| `.tex` | LaTeX | 传统学术排版 | 学术向，公式密集 |

### 1.2 评论区支持对比

```
┌─────────────────────────────────────────────────────────────────┐
│                   各格式评论区支持对比                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  .ucxc   ★★★★★  段落级精确绑定，每段独立 UUID               │
│          优势：自由设置、手动调整、永不混乱                     │
│                                                                 │
│  .mdx    ★★★★☆  frontmatter 评论区列表                      │
│          优势：可视化编辑器自动调整，差异对比修正               │
│                                                                 │
│  .md     ★★☆☆☆  基于作品ID+文件序号+段落序号自动生成        │
│  .txt    ★★☆☆☆  增删段落会导致评论区错位                    │
│  .typ    ★★☆☆☆                                              │
│  .tex    ★★☆☆☆                                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 2. UCX Content 格式（.ucxc）

### 2.1 设计理念

.ucxc 是 UCX 规范的核心内容格式，采用类似 XML/HTML 的标签式标记语言。

**设计原则**：
- 标签语法接近 HTML，降低学习成本
- 每个段落可独立绑定评论区（UUID 唯一标识）
- 内置丰富的格式标签（粗体、斜体、加密、注释等）
- 通过扩展机制支持自定义标签，自由度极高
- 由编辑器自动生成和维护，不要求手写

### 2.2 基本结构

```xml
<?ucxc version="1.0" encoding="UTF-8"?>
<chapter>
  <!-- 章节元数据（可选） -->
  <meta>
    <title>第一章 开端</title>
    <subtitle>一切的起点</subtitle>
    <word-count>3200</word-count>
  </meta>

  <!-- 正文内容 -->
  <body>
    <p cid="550e8400-e29b-41d4-a716-446655440001">
      这是第一段正文内容。阳光透过窗帘的缝隙洒进屋内。
    </p>

    <p cid="550e8400-e29b-41d4-a716-446655440002">
      他缓缓睁开眼睛，感受到一阵<bold>刺眼</bold>的光芒。
    </p>

    <p cid="550e8400-e29b-41d4-a716-446655440003">
      <italic>这是哪里？</italic>他心中涌起一股疑惑。
    </p>
  </body>
</chapter>
```

### 2.3 段落与评论区绑定

每个 `<p>` 标签通过 `cid`（Comment ID）属性绑定到唯一的评论区。

**cid 规则**：

| 规则 | 说明 |
|------|------|
| 格式 | UUID v4（推荐）或任意唯一字符串 |
| 唯一性 | 在整个 UCX 文件范围内必须唯一 |
| 自动生成 | 编辑器在创建段落时自动计算 UUID |
| 手动调整 | 可手动修改，适用于段落合并/拆分后保留原评论区 |
| 可选 | `cid` 属性本身是可选的，不设置则该段落无评论区 |

**评论区 URL 生成规则**：

```
{comment_base_url}/{ucx_id}/{cid}

示例：
https://comments.unicodex.org/urn:ucx:550e8400-e29b/550e8400-e29b-41d4-a716-446655440001
```

**段落操作与评论区的关系**：

```
┌─────────────────────────────────────────────────────────────────┐
│              .ucxc 段落操作 vs 评论区                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  新增段落：编辑器自动生成新 UUID，不影响其他段落                │
│  删除段落：对应 cid 的评论区可保留（归档）或删除                │
│  拆分段落：原段落保留 cid，新段落生成新 UUID                    │
│  合并段落：保留其中一个 cid，另一个归档                         │
│  移动段落：cid 跟随段落移动，评论区不受影响                     │
│  插入段落：中间插入新段落，前后段落的 cid 不变                  │
│                                                                 │
│  核心优势：无论如何增删改，已有段落的评论区永不错位              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 2.4 内置标签

#### 2.4.1 文本格式标签

| 标签 | 说明 | 示例 |
|------|------|------|
| `<bold>` | 粗体 | `<bold>重要内容</bold>` |
| `<italic>` | 斜体 | `<italic>心理活动</italic>` |
| `<strike>` | 删除线 | `<strike>被划掉的文字</strike>` |
| `<underline>` | 下划线 | `<underline>关键信息</underline>` |
| `<sup>` | 上标 | `公式 E=mc<sup>2</sup>` |
| `<sub>` | 下标 | `H<sub>2</sub>O` |
| `<code>` | 行内代码 | `使用 <code>ucx verify</code> 命令` |

#### 2.4.2 段落级标签

| 标签 | 说明 | 示例 |
|------|------|------|
| `<p>` | 段落 | 基本内容单元，可绑定 `cid` |
| `<heading>` | 段内标题 | `<heading level="2">小节标题</heading>` |
| `<quote>` | 引用块 | `<quote source="某某">引用内容</quote>` |
| `<divider>` | 分隔线 | `<divider style="scene-break"/>` |
| `<image>` | 插图 | `<image src="res.images.img-001[jpg]" alt="描述"/>` |
| `<codeblock>` | 代码块 | `<codeblock lang="python">...</codeblock>` |

#### 2.4.3 语义标签

| 标签 | 说明 | 示例 |
|------|------|------|
| `<dialog>` | 对话 | `<dialog speaker="主角">"我明白了。"</dialog>` |
| `<thought>` | 心理活动 | `<thought>这一切太不真实了。</thought>` |
| `<narration>` | 旁白 | `<narration>时间回到三年前。</narration>` |
| `<footnote>` | 脚注 | `<footnote id="fn-1">注释内容</footnote>` |
| `<annotation>` | 作者批注 | `<annotation>此处伏笔，第三卷回收</annotation>` |

#### 2.4.4 功能标签

##### `<encrypted>` — 段落级加密

对段落内容进行加密，阅读器需解密后才能显示。加密参数直接内联在标签属性中。

```xml
<p cid="550e8400-...">
  这段是免费内容，所有人可见。
</p>

<!-- 使用服务验证解密 -->
<encrypted algorithm="AES-256-GCM">
  <key-access method="service" provider="https://api.unicodex.org/keys" service-type="purchase_verify">
    <param name="product_id">novel-001-ch02</param>
  </key-access>
  <content>
    <p cid="550e8401-...">
      这段是付费内容，需要解密后才能阅读。
    </p>
    <p cid="550e8402-...">
      加密区域可以包含多个段落。
    </p>
  </content>
</encrypted>

<!-- 使用消息提示 -->
<encrypted algorithm="AES-256-GCM">
  <key-access method="message">
    <text>本章为付费内容，请联系 author@example.com 获取密钥</text>
  </key-access>
  <content>
    <p cid="550e8403-...">加密内容...</p>
  </content>
</encrypted>

<!-- 使用 URL 验证 -->
<encrypted algorithm="AES-256-GCM">
  <key-access method="url" url="https://example.com/api/keys/chapter-002" auth-type="login"/>
  <content>
    <p cid="550e8404-...">加密内容...</p>
  </content>
</encrypted>

<!-- 使用扩展自定义解密 -->
<encrypted algorithm="AES-256-GCM">
  <key-access method="extension" ext="ext.custom-drm">
    <param name="license_server">https://drm.example.com</param>
    <param name="content_id">ch-002</param>
  </key-access>
  <content>
    <p cid="550e8405-...">加密内容...</p>
  </content>
</encrypted>

<!-- 多种解密方式并存（阅读器按顺序尝试） -->
<encrypted algorithm="AES-256-GCM">
  <key-access method="service" provider="https://api.unicodex.org/keys" service-type="purchase_verify">
    <param name="product_id">novel-001-ch02</param>
  </key-access>
  <key-access method="url" url="https://my-novel.com/api/unlock/ch02" auth-type="oauth"/>
  <key-access method="message">
    <text>若以上方式不可用，请联系 author@example.com</text>
  </key-access>
  <content>
    <p cid="550e8406-...">VIP 专属内容...</p>
  </content>
</encrypted>
```

**`<encrypted>` 属性**：

| 属性 | 必需 | 说明 |
|------|------|------|
| `algorithm` | **是** | 加密算法标识（AES-256-GCM / AES-256-CBC / ChaCha20-Poly1305） |

**`<key-access>` 属性**（可包含多个，按顺序尝试）：

| 属性 | 必需 | 说明 |
|------|------|------|
| `method` | **是** | 密钥获取方式：`direct` / `message` / `url` / `service` / `extension` |

**method 取值与对应属性**：

| method | 属性 | 说明 |
|--------|------|------|
| `direct` | `key`、`iv` | Base64 编码的密钥和初始化向量 |
| `message` | — | 子元素 `<text>` 包含提示文本 |
| `url` | `url`、`auth-type` | 验证接口地址和认证方式（none/login/token/oauth/certificate） |
| `service` | `provider`、`service-type` | 服务商地址和服务类型（purchase_verify/author_verify/subscription/public_key） |
| `extension` | `ext` | 扩展引用路径（ext.xxx） |

**`<param>` 子元素**：用于传递额外参数（键值对形式）。

##### `<ext>` — 扩展标签

通过安装的扩展提供自定义标签，实现任意功能扩展。

```xml
<!-- 使用扩展：互动选项（由 interactive 扩展提供） -->
<ext ref="ext.interactive" tag="choice">
  <option target="chapter-003a.ucxc">选择 A：追上去</option>
  <option target="chapter-003b.ucxc">选择 B：留下来</option>
</ext>

<!-- 使用扩展：AI 摘要占位符 -->
<ext ref="ext.ai-summary" tag="summary-placeholder">
  <config max-words="100" style="brief"/>
</ext>

<!-- 使用扩展：有声书音频绑定 -->
<ext ref="ext.audio-book" tag="audio">
  <source src="ext.audio-book/chapter-001.mp3" start="00:00" end="03:45"/>
</ext>
```

| 属性 | 必需 | 说明 |
|------|------|------|
| `ref` | **是** | 扩展引用路径（`ext.{alias}` 或完整名称） |
| `tag` | **是** | 扩展定义的标签名称 |

> **扩展标签的解析**：阅读器遇到不识别的 `<ext>` 标签时应忽略该标签内容（或显示为折叠区域），不影响正文阅读。

##### `<comment-config>` — 评论区配置（可选）

在 `<meta>` 中配置本章评论区的全局参数。

```xml
<meta>
  <title>第一章 开端</title>
  <comment-config>
    <provider>https://comments.unicodex.org</provider>
    <enabled>true</enabled>
    <moderation>author</moderation>
  </comment-config>
</meta>
```

### 2.5 完整示例

```xml
<?ucxc version="1.0" encoding="UTF-8"?>
<chapter>
  <meta>
    <title>第二章 相遇</title>
    <word-count>2800</word-count>
    <comment-config>
      <provider>https://comments.unicodex.org</provider>
      <enabled>true</enabled>
    </comment-config>
  </meta>

  <body>
    <heading level="1">第二章 相遇</heading>

    <p cid="a1b2c3d4-0001-4000-8000-000000000001">
      清晨的街道上弥漫着薄雾。<bold>李明</bold>背着书包，匆匆走在上学的路上。
    </p>

    <p cid="a1b2c3d4-0001-4000-8000-000000000002">
      <dialog speaker="李明">"要迟到了！"</dialog>他加快了脚步。
    </p>

    <p cid="a1b2c3d4-0001-4000-8000-000000000003">
      转角处，一个身影突然出现。
    </p>

    <divider style="scene-break"/>

    <p cid="a1b2c3d4-0001-4000-8000-000000000004">
      <thought>那个人是谁？</thought>李明停下脚步，看着面前的女孩。
    </p>

    <p cid="a1b2c3d4-0001-4000-8000-000000000005">
      她有着银色的长发，在晨光中闪闪发光。
    </p>

    <image src="res.images.img-001[jpg]" alt="女孩的立绘" width="100%"/>

    <encrypted algorithm="AES-256-GCM">
      <key-access method="service" provider="https://api.unicodex.org/keys" service-type="purchase_verify">
        <param name="product_id">novel-001-ch02-vip</param>
      </key-access>
      <key-access method="message">
        <text>付费内容，请联系 author@example.com 获取密钥</text>
      </key-access>
      <content>
        <p cid="a1b2c3d4-0001-4000-8000-000000000006">
          她微微一笑，说出了一句令李明震惊的话。
        </p>
        <p cid="a1b2c3d4-0001-4000-8000-000000000007">
          <dialog speaker="???">
            "我等你很久了，<bold>命运之子</bold>。"
          </dialog>
        </p>
      </content>
    </encrypted>

    <!-- 互动扩展：读者选择 -->
    <ext ref="ext.interactive" tag="choice">
      <option target="chapter-003a.ucxc">追问她的身份</option>
      <option target="chapter-003b.ucxc">转身跑开</option>
    </ext>

    <footnote id="fn-1">
      "命运之子"一词出自作品世界观设定，详见附录。
    </footnote>
  </body>
</chapter>
```

### 2.6 标签扩展机制

.ucxc 的标签系统是开放的。除内置标签外，任何已安装的扩展均可注册自定义标签。

**扩展注册标签的方式**：

在扩展的 `extension.json` 中声明：

```json
{
  "id": "interactive",
  "provider": "org.unicodex",
  "version": "2.1.0",
  "capabilities": ["ucxc_tags"],
  "custom_tags": [
    {
      "tag": "choice",
      "description": "互动选项，读者选择不同分支",
      "attributes": {
        "style": { "type": "string", "required": false }
      },
      "children": ["option"]
    },
    {
      "tag": "option",
      "description": "单个选项",
      "attributes": {
        "target": { "type": "string", "required": true }
      }
    }
  ]
}
```

**阅读器处理规则**：

```
┌─────────────────────────────────────────────────────────────────┐
│                   标签解析优先级                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 内置标签（<p>、<bold>、<encrypted> 等）     → 直接渲染      │
│  2. 扩展标签（<ext ref="..."> 包裹）           → 查找扩展处理  │
│  3. 未知标签                                   → 忽略，显示    │
│     内部纯文本（graceful degradation）                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 3. UCX Markdown 格式（.mdx）

### 3.1 设计理念

.mdx 是标准 Markdown 的增强版，在保持可读性的同时，通过 frontmatter 提供评论区绑定等 UCX 特性。

**设计原则**：
- 完全兼容 CommonMark 语法
- 通过 `---` frontmatter 头部扩展功能
- 评论区映射表实现段落级评论绑定
- 可视化编辑器可自动维护映射关系

### 3.2 基本结构

```markdown
---
ucx_version: "1.0"
title: "第一章 开端"
word_count: 3200
comments:
  enabled: true
  provider: "https://comments.unicodex.org"
  map:
    - cid: "550e8400-e29b-41d4-a716-446655440001"
      paragraph: 1
      hash: "a1b2c3d4"
    - cid: "550e8400-e29b-41d4-a716-446655440002"
      paragraph: 2
      hash: "e5f6a7b8"
    - cid: "550e8400-e29b-41d4-a716-446655440003"
      paragraph: 3
      hash: "c9d0e1f2"
---

这是第一段正文内容。阳光透过窗帘的缝隙洒进屋内。

他缓缓睁开眼睛，感受到一阵**刺眼**的光芒。

*这是哪里？*他心中涌起一股疑惑。
```

### 3.3 Frontmatter 规范

#### 3.3.1 基础字段

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `ucx_version` | 否 | string | UCX 规范版本 |
| `title` | 否 | string | 章节标题 |
| `subtitle` | 否 | string | 章节副标题 |
| `word_count` | 否 | number | 字数统计 |
| `style` | 否 | string | 样式标识 |

#### 3.3.2 评论区配置（comments）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `enabled` | 否 | boolean | 是否启用评论区（默认跟随全局配置） |
| `provider` | 否 | string | 评论区服务地址 |
| `map` | 否 | array | 段落到评论区的映射列表 |

#### 3.3.3 评论区映射表（comments.map）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `cid` | **是** | string | 评论区唯一标识（UUID） |
| `paragraph` | **是** | number | 段落序号（从 1 开始） |
| `hash` | 否 | string | 段落内容哈希摘要（前 8 位），用于变更检测 |

**hash 字段的作用**：

当段落内容发生变化时（编辑/插入/删除），可视化编辑器通过 hash 检测变更并自动调整映射。

```
┌─────────────────────────────────────────────────────────────────┐
│              .mdx 评论区自动调整机制                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  编辑器保存时自动执行：                                         │
│                                                                 │
│  1. 计算每个段落的内容哈希                                      │
│  2. 与 map 中记录的 hash 对比                                   │
│  3. 匹配策略：                                                  │
│     ├── hash 匹配 → 段落未变，保持 cid 绑定                    │
│     ├── hash 不匹配但位置不变 → 内容修改，保持 cid             │
│     ├── hash 匹配但位置变化 → 段落移动，更新 paragraph 序号    │
│     ├── 无匹配 hash → 新段落，生成新 UUID                      │
│     └── 残留 cid 无对应段落 → 标记为归档                       │
│                                                                 │
│  结果：即使频繁编辑，评论区映射始终保持正确                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.4 MDX 扩展语法

在标准 CommonMark 基础上，.mdx 额外支持以下语法：

#### 3.4.1 对话标记

```markdown
::dialog[speaker="主角"]
"我明白了。"
::
```

#### 3.4.2 加密区域

```markdown
:::encrypted{algorithm="AES-256-GCM"}
@key-access method="service" provider="https://api.unicodex.org/keys" service-type="purchase_verify" product_id="novel-001-ch01"
@key-access method="message" text="付费内容，请联系 author@example.com 获取密钥"

这段是加密内容，需要解密后才能阅读。

加密区域可以跨越多个段落。
:::
```

> **@key-access 行**：紧跟在 `:::encrypted` 开头之后，以 `@key-access` 开头的行为密钥获取配置（可多行，按顺序尝试）。第一个空行之后为加密的正文内容。

#### 3.4.3 扩展块

```markdown
:::ext{ref="ext.interactive" tag="choice"}
- [追问她的身份](chapter-003a.ucxc)
- [转身跑开](chapter-003b.ucxc)
:::
```

### 3.5 完整示例

```markdown
---
ucx_version: "1.0"
title: "第二章 相遇"
word_count: 2800
comments:
  enabled: true
  provider: "https://comments.unicodex.org"
  map:
    - cid: "a1b2c3d4-0001-4000-8000-000000000001"
      paragraph: 1
      hash: "f3a1b2c3"
    - cid: "a1b2c3d4-0001-4000-8000-000000000002"
      paragraph: 2
      hash: "d4e5f6a7"
    - cid: "a1b2c3d4-0001-4000-8000-000000000003"
      paragraph: 3
      hash: "b8c9d0e1"
    - cid: "a1b2c3d4-0001-4000-8000-000000000004"
      paragraph: 4
      hash: "f2a3b4c5"
---

# 第二章 相遇

清晨的街道上弥漫着薄雾。**李明**背着书包，匆匆走在上学的路上。

::dialog[speaker="李明"]
"要迟到了！"
::
他加快了脚步。

转角处，一个身影突然出现。

---

她有着银色的长发，在晨光中闪闪发光。

![女孩的立绘](res.images.img-001[jpg])

:::encrypted{algorithm="AES-256-GCM"}
@key-access method="service" provider="https://api.unicodex.org/keys" service-type="purchase_verify" product_id="novel-001-ch02-vip"
@key-access method="message" text="付费内容，请联系 author@example.com 获取密钥"

她微微一笑，说出了一句令李明震惊的话。

::dialog[speaker="???"]
"我等你很久了，**命运之子**。"
::
:::

:::ext{ref="ext.interactive" tag="choice"}
- [追问她的身份](chapter-003a.ucxc)
- [转身跑开](chapter-003b.ucxc)
:::

[^1]: "命运之子"一词出自作品世界观设定，详见附录。
```

## 4. Markdown 格式（.md）

### 4.1 规范

标准 CommonMark 格式，不做任何扩展。

**评论区支持**：

若在 `unicodex.toml` 或 `codex.json` 中全局启用了评论区，.md 文件采用**自动生成**的评论区 URL。

**自动生成规则**：

```
{comment_base_url}/{ucx_id}/{file_index}/{paragraph_index}

示例：
https://comments.unicodex.org/urn:ucx:550e8400-e29b/003/007
                                                      │    │
                                              文件序号 ┘    └ 段落序号
```

**文件序号**：该文件在 struct.json 中的顺序位置（从 001 开始）。
**段落序号**：以空行分隔的段落序号（从 001 开始）。

> **局限性**：一旦增加或删除段落，后续所有段落的序号都会改变，导致评论区错位。这是 .md 格式的固有缺陷，建议需要评论区的作品使用 .ucxc 或 .mdx。

### 4.2 示例

```markdown
# 第一章 开端

这是第一段正文内容。阳光透过窗帘的缝隙洒进屋内。

他缓缓睁开眼睛，感受到一阵**刺眼**的光芒。

*这是哪里？*他心中涌起一股疑惑。
```

## 5. 纯文本格式（.txt）

### 5.1 规范

纯文本格式，适合最简单的小说创作。

**编码**：UTF-8（必须）。

**段落分隔**：以空行分隔。

**评论区支持**：与 .md 相同，采用自动生成的评论区 URL（基于文件序号 + 段落序号），具有相同的局限性。

### 5.2 示例

```
第一章 开端

这是第一段正文内容。阳光透过窗帘的缝隙洒进屋内。

他缓缓睁开眼睛，感受到一阵刺眼的光芒。

这是哪里？他心中涌起一股疑惑。
```

## 6. Typst 格式（.typ）

### 6.1 规范

Typst 是现代排版系统，适合需要复杂版式和数学公式的作品。

**使用方式**：
- struct.json 中指定入口 `.typ` 文件
- 入口文件可通过 `#include` 引用其他 `.typ` 文件
- UCX 打包时包含所有被引用的文件

**评论区支持**：与 .md 相同，采用自动生成的评论区 URL。

### 6.2 示例

```typst
#set text(font: "Source Han Serif SC", size: 11pt)

= 第一章 开端

这是第一段正文内容。阳光透过窗帘的缝隙洒进屋内。

他缓缓睁开眼睛，感受到一阵*刺眼*的光芒。

公式示例：$E = m c^2$
```

## 7. LaTeX 格式（.tex）

### 7.1 规范

LaTeX 传统学术排版系统，适合数学公式密集的作品。

**使用方式**：
- struct.json 中指定入口 `.tex` 文件
- 入口文件可通过 `\input` 或 `\include` 引用其他文件
- UCX 打包时包含所有被引用的文件和样式文件

**评论区支持**：与 .md 相同，采用自动生成的评论区 URL。

### 7.2 示例

```latex
\documentclass{article}
\usepackage{ctex}

\begin{document}

\section{第一章 开端}

这是第一段正文内容。阳光透过窗帘的缝隙洒进屋内。

他缓缓睁开眼睛，感受到一阵\textbf{刺眼}的光芒。

公式示例：$E = mc^2$

\end{document}
```

## 8. 全局评论区配置

### 8.1 unicodex.toml 配置

```toml
# 评论区全局配置
[comments]
enabled = true                                      # 全局开关
provider = "https://comments.unicodex.org"          # 评论区服务地址
# provider = "https://comments.my-platform.com"     # 可替换为第三方
moderation = "author"                               # 审核模式：author / open / closed

# 自定义评论区服务商
[comments.auth]
type = "oauth"                                      # 认证方式
provider = "https://auth.unicodex.org"              # 认证服务
```

### 8.2 各格式评论区机制对比

```
┌──────────────────────────────────────────────────────────────────────┐
│                     评论区机制对比                                    │
├──────┬────────────────────┬──────────────────────────────────────────┤
│ 格式 │ 评论区绑定方式      │ 增删段落影响                             │
├──────┼────────────────────┼──────────────────────────────────────────┤
│.ucxc │ <p cid="UUID">     │ 无影响，UUID 跟随段落                    │
│      │ 段落内联绑定        │                                          │
├──────┼────────────────────┼──────────────────────────────────────────┤
│.mdx  │ frontmatter map    │ 编辑器自动通过 hash 对比调整映射         │
│      │ cid + paragraph +  │ 手动编辑时需注意更新 map                 │
│      │ hash               │                                          │
├──────┼────────────────────┼──────────────────────────────────────────┤
│.md   │ 自动生成           │ 增删段落导致后续所有评论区错位           │
│.txt  │ {ucx_id}/          │                                          │
│.typ  │ {file_idx}/        │                                          │
│.tex  │ {para_idx}         │                                          │
└──────┴────────────────────┴──────────────────────────────────────────┘
```

## 9. 格式选择指南

```
┌─────────────────────────────────────────────────────────────────┐
│                     格式选择决策树                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  需要段落级评论区？                                              │
│  ├── 是 → 需要富文本标记（加密/互动/自定义标签）？              │
│  │        ├── 是 → .ucxc（最推荐）                              │
│  │        └── 否 → .mdx（推荐，兼顾可读性）                    │
│  └── 否 → 需要复杂排版/数学公式？                              │
│           ├── 是 → .typ（推荐）或 .tex                          │
│           └── 否 → 需要基本格式？                               │
│                    ├── 是 → .md                                  │
│                    └── 否 → .txt                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

*请审阅并指出需要修改的部分*
