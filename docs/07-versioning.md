# UCX 版本控制规范

> 版本：0.1.0-draft
> 前置依赖：[00-overview.md](./00-overview.md)、[01-file-structure.md](./01-file-structure.md)

## 1. 概述

UCX 版本控制涵盖三个层面：

```
┌─────────────────────────────────────────────────────────────────┐
│                     UCX 版本控制层面                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. UCX 规范版本（Specification Version）                       │
│     └── 规范文档本身的版本，决定文件格式的兼容性                │
│                                                                 │
│  2. UCX 文件版本（File Version）                                │
│     └── 单个 UCX 文件的版本，记录作品的修订历史                 │
│                                                                 │
│  3. UCX 工具版本（Tool Version）                                │
│     └── 实现工具（unicodex-core 等）的版本                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 2. UCX 规范版本

### 2.1 版本号格式

```
MAJOR.MINOR
```

| 部分 | 含义 | 变更影响 |
|------|------|---------|
| `MAJOR` | 主版本号 | 不向后兼容的结构性变化 |
| `MINOR` | 次版本号 | 向后兼容的功能新增 |

**示例**：`1.0`、`1.1`、`2.0`

### 2.2 版本记录位置

规范版本号出现在 UCX 文件的以下位置：

| 位置 | 字段 | 示例 |
|------|------|------|
| MANIFEST.MF 主块 | `UCX-Version` | `UCX-Version: 1.0` |
| 签名文件 (.SF) | `UCX-Version` | `UCX-Version: 1.0` |
| codex.json | `version` | `"version": "1.0"` |
| struct.json | `version` | `"version": "1.0"` |
| genesis.proof | `version` | `"version": "1.0"` |
| extensions/manifest.json | `version` | `"version": "1.0"` |

### 2.3 MAJOR 版本变更（不兼容）

以下变更导致 MAJOR 版本递增：

| 变更类型 | 示例 |
|---------|------|
| 必需目录/文件结构变化 | 移除 `META-INF/`，改用其他安全机制 |
| 必需字段删除或语义变更 | `codex.json` 中 `ucx_id` 格式变更 |
| 签名机制根本性变化 | 替换双层签名为新体系 |
| 哈希算法默认值变更 | 默认从 BLAKE3 改为其他算法 |
| MANIFEST.MF 格式变更 | 从 RFC 822 改为 JSON |

### 2.4 MINOR 版本变更（兼容）

以下变更导致 MINOR 版本递增：

| 变更类型 | 示例 |
|---------|------|
| 新增可选目录/文件 | 新增 `analytics/` 目录 |
| 新增可选字段 | `codex.json` 新增 `ai_summary` 字段 |
| 新增加密算法支持 | 新增 XChaCha20-Poly1305 |
| 新增签名算法支持 | 新增 Ed448 |
| 新增章节格式 | 新增 `.rst` (reStructuredText) 支持 |
| 新增扩展能力 | 扩展新增 `analytics` capability |

### 2.5 兼容性矩阵

```
┌─────────────────────────────────────────────────────────────────┐
│                     版本兼容性处理                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  工具支持 1.1，遇到不同版本的文件：                             │
│                                                                 │
│  ├── 文件版本 1.0 → 完全兼容，正常处理                          │
│  ├── 文件版本 1.1 → 完全兼容，所有功能可用                      │
│  ├── 文件版本 1.2 → 向后兼容，忽略未知字段                      │
│  │   └── 警告：部分新功能可能不可用                             │
│  ├── 文件版本 2.0 → 不兼容，尝试读取                            │
│  │   └── 警告：MAJOR 版本不匹配，可能出现解析错误               │
│  └── 文件版本 2.1 → 不兼容，尝试读取                            │
│      └── 警告：MAJOR 版本不匹配                                 │
│                                                                 │
│  原则：尽最大努力读取，不轻易拒绝                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 3. UCX 文件版本

### 3.1 版本号格式

```
X.Y.Z
```

| 部分 | 含义 | 递增条件 |
|------|------|---------|
| `X` | 第一位 | 作者自定义（推荐语义见 §3.3） |
| `Y` | 第二位 | 作者自定义（推荐语义见 §3.3） |
| `Z` | 第三位 | 修正性更新（修改已有内容但未发布新章节） |

### 3.2 作品版本号

作品版本号记录在 `metadata/codex.json` 中：

```json
{
  "identifier": {
    "ucx_id": "urn:ucx:550e8400-e29b-41d4-a716-446655440000"
  },
  "file_version": {
    "version": "1.12.0",
    "revision": 15,
    "released_at": "2025-12-01T00:00:00Z",
    "changelog": "修复第三章错别字，新增第十二章"
  }
}
```

**字段说明**：

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `version` | 否 | string | `X.Y.Z` 格式版本号 |
| `revision` | 否 | integer | 修订计数器（每次构建 +1，自动管理） |
| `released_at` | 否 | string | 发布时间（ISO 8601） |
| `changelog` | 否 | string | 更新说明 |

### 3.3 推荐版本语义：卷.章.修订

UCX **推荐**（非强制）将 `X.Y.Z` 赋予以下语义，让版本号直接反映作品进度：

```
卷号.章节数.修订号
 X  .  Y  .  Z
```

| 部分 | 语义 | 变更规则 |
|------|------|---------|
| `X`（卷号） | 当前卷数 | 开始新一卷时 +1 |
| `Y`（章节数） | 当前卷内的最新章节编号 | 发布新章节时更新为该章节编号 |
| `Z`（修订号） | 修正计数 | 修改已有内容但未发布新章节时 +1，发布新章节时归零 |

**Z 的初始值为 `0`**，表示该章节首次发布、尚未经过修正。

**示例版本历史**：

```
1.1.0   — 第一卷，发布第 1 章（首发）
1.2.0   — 第一卷，发布第 2 章
1.2.1   — 修复第 1 章错别字（Z: 0→1，Y 不变）
1.2.2   — 调整第 2 章排版（Z: 1→2）
1.5.0   — 第一卷，发布第 5 章（Z 归零）
1.10.0  — 第一卷，发布第 10 章
1.10.1  — 修复第 8 章配图（Z: 0→1）
2.1.0   — 第二卷，发布第 1 章（新卷 X: 1→2，Z 归零）
2.3.0   — 第二卷，发布第 3 章
2.3.1   — 全卷校对修正
```

**其他版本语义**：

作者也可选择不遵循 `卷.章.修订` 语义，使用自定义含义（如 `MAJOR.MINOR.PATCH`），UCX 不做强制约束。

### 3.4 自动版本生成（`ucx version`）

UCX 提供**可选但推荐**的自动版本生成命令，让作者专注于写作而非手动管理版本号。

#### 3.4.1 命令

```bash
# 自动检测变更并生成版本号（推荐，日常使用）
ucx version auto

# 发布新章节（更新 Y，Z 归零）
ucx version chapter

# 开始新卷（更新 X，Y 和 Z 归零）
ucx version volume

# 记录修正（Z +1）
ucx version patch

# 手动指定版本号（覆盖自动生成）
ucx version set 2.5.0

# 查看当前版本
ucx version
```

#### 3.4.2 `ucx version auto` 工作原理

```
┌─────────────────────────────────────────────────────────────────┐
│                   ucx version auto 自动检测逻辑                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 读取当前版本号（如 1.5.0）                                  │
│                                                                 │
│  2. 对比 struct.json 与上次构建的快照：                          │
│     ├── 检测是否有新的卷（children 顶层新增）                   │
│     ├── 检测是否有新的章节文件（叶子节点新增 file）             │
│     └── 检测已有文件是否被修改（哈希变化）                      │
│                                                                 │
│  3. 判定版本变更类型：                                          │
│     ├── 新增卷 → X +1, Y = 首章编号, Z = 0                     │
│     ├── 新增章节（无新卷） → Y = 新章节编号, Z = 0             │
│     └── 仅修改已有内容 → X.Y 不变, Z +1                        │
│                                                                 │
│  4. 更新 codex.json 中的 file_version                           │
│     ├── version: 新版本号                                       │
│     ├── revision: +1                                            │
│     └── released_at: 当前时间                                   │
│                                                                 │
│  示例：                                                         │
│  当前版本 1.5.0                                                 │
│  ├── 新增 chapter-006.ucxc → 自动生成 1.6.0                    │
│  ├── 修改 chapter-003.ucxc → 自动生成 1.5.1                    │
│  └── 新增第二卷 + chapter-001 → 自动生成 2.1.0                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### 3.4.3 章节编号提取规则

`ucx version auto` 根据 `struct.json` 的结构自动推断章节编号：

```
┌─────────────────────────────────────────────────────────────────┐
│                   章节编号提取规则                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  规则 1：按叶子节点在当前卷内的顺序计数                         │
│  └── 第 N 个有 file 的叶子节点 → 章节编号 = N                  │
│                                                                 │
│  示例 struct.json：                                             │
│  structure:                                                     │
│  └── 第一卷                                                     │
│      ├── 第一章 (file: ch-001.ucxc)    → 章节 1                │
│      ├── 第二章 (file: ch-002.ucxc)    → 章节 2                │
│      ├── 番外篇                                                 │
│      │   └── 番外一 (file: ex-001.ucxc) → 章节 3               │
│      └── 第三章 (file: ch-003.ucxc)    → 章节 4                │
│                                                                 │
│  最新章节 = 4 → 版本 Y = 4                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### 3.4.4 配置

在 `unicodex.toml` 中可配置自动版本行为：

```toml
[version]
# 版本策略：auto（推荐）/ manual
strategy = "auto"

# 是否在 ucx build 时自动执行 ucx version auto
auto_on_build = true

# 自定义版本语义（默认 "volume.chapter.patch"）
# 设置为 "semver" 则使用标准语义化版本
semantic = "volume.chapter.patch"
```

#### 3.4.5 技术实现：基于 git2（libgit2）

`ucx version auto` 使用 `git2` crate（libgit2 的 Rust 绑定）实现变更检测，直接读取 Git 仓库历史，无需调用外部 `git` 命令。

**为什么选择 git2**：

| 方案 | 优势 | 劣势 |
|------|------|------|
| **git2（选用）** | 无需系统安装 Git；纯 Rust 调用；跨平台一致性；可嵌入库 | 需要链接 libgit2 |
| 调用 git CLI | 无额外依赖 | 需系统安装 Git；解析命令行输出脆弱；跨平台行为差异 |
| 自实现 diff | 零依赖 | 工作量大；不如 Git 成熟 |

**核心工作流**：

```
┌─────────────────────────────────────────────────────────────────┐
│              ucx version auto — git2 检测流程                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 打开 Git 仓库                                               │
│     └── repo = Repository::open(project_root)                  │
│                                                                 │
│  2. 获取上次版本标记的 commit                                    │
│     ├── 查找最近的 ucx-v* tag（如 ucx-v1.5.0）                 │
│     └── base_commit = tag.target().peel_to_commit()            │
│                                                                 │
│  3. 获取当前工作树状态                                          │
│     └── current_tree = repo.head().peel_to_tree()              │
│                                                                 │
│  4. 计算 diff                                                   │
│     └── diff = repo.diff_tree_to_tree(base_tree, current_tree) │
│                                                                 │
│  5. 分析变更文件                                                │
│     ├── 筛选 content/ 目录下的变更                              │
│     ├── 筛选 struct.json 的变更                                 │
│     └── 分类：new_files / modified_files / deleted_files       │
│                                                                 │
│  6. 解析 struct.json 判定变更类型                                │
│     ├── 对比新旧 struct.json 的 structure 树                    │
│     ├── 检测新增卷（顶层 children 新增）                        │
│     ├── 检测新增章节（叶子节点 file 新增）                      │
│     └── 检测仅修改（已有 file 内容变化）                        │
│                                                                 │
│  7. 生成版本号                                                  │
│     ├── 新增卷 → X +1, Y = 首章编号, Z = 0                     │
│     ├── 新增章节 → Y = 新章节编号, Z = 0                       │
│     └── 仅修改 → Z +1                                          │
│                                                                 │
│  8. 更新 codex.json 并创建 Git tag                              │
│     ├── 写入新版本号到 file_version                             │
│     └── git tag ucx-v{X.Y.Z}（可选）                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Git Tag 约定**：

| Tag 格式 | 说明 |
|---------|------|
| `ucx-v1.5.0` | UCX 文件版本标记 |
| `ucx-v1.5.1` | 修订版标记 |

**代码示例**（ucx-version 模块核心逻辑）：

```rust
use git2::{Repository, DiffOptions, DiffDelta};

/// Detect changes since the last UCX version tag.
///
/// 检测自上次 UCX 版本标签以来的变更。
pub fn detect_changes(repo_path: &Path) -> Result<ChangeSet, VersionError> {
    // Open the Git repository via git2 (libgit2 bindings).
    let repo = Repository::open(repo_path)?;

    // Find the latest ucx-v* tag as our baseline.
    let base_tag = find_latest_ucx_tag(&repo)?;

    // Get the tree from the base tag and the current HEAD.
    let base_tree = base_tag
        .peel_to_commit()?
        .tree()?;
    let head_tree = repo
        .head()?
        .peel_to_tree()?;

    // Compute diff between the two trees.
    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec("content/*");
    let diff = repo.diff_tree_to_tree(
        Some(&base_tree),
        Some(&head_tree),
        Some(&mut diff_opts),
    )?;

    // Classify changed files.
    let mut changes = ChangeSet::new();
    diff.foreach(
        &mut |delta: DiffDelta, _progress| {
            match delta.status() {
                git2::Delta::Added => changes.added.push(delta_path(&delta)),
                git2::Delta::Modified => changes.modified.push(delta_path(&delta)),
                git2::Delta::Deleted => changes.deleted.push(delta_path(&delta)),
                _ => {}
            }
            true
        },
        None, None, None,
    )?;

    Ok(changes)
}
```

**无 Git 仓库时的回退**：

若项目未使用 Git（`Repository::open` 失败），`ucx version auto` 回退为基于文件快照的 diff：

1. 读取 `.ucx-snapshot.json`（上次 `ucx build` 时自动生成的文件哈希快照）
2. 对比当前文件哈希与快照
3. 分类变更并生成版本号
4. 更新快照文件

```json
// .ucx-snapshot.json（自动生成，不需手动维护）
{
  "version": "1.5.0",
  "timestamp": "2025-08-01T12:00:00Z",
  "files": {
    "content/struct.json": "af1349b9...",
    "content/chapter-001.ucxc": "7d865e95...",
    "content/chapter-005.ucxc": "e4cfa39a..."
  }
}
```

### 3.5 UCX ID 不变性

- 同一作品的不同版本共享**相同的 `ucx_id`**
- `ucx_id` 在首次生成后**不可变更**
- 通过 `file_version` 区分不同版本

```
UCX ID: urn:ucx:550e8400-e29b-41d4-a716-446655440000

  ├── file_version: 1.1.0   (2025-06-01, 第一卷第 1 章)
  ├── file_version: 1.5.0   (2025-08-01, 第一卷第 5 章)
  ├── file_version: 1.10.0  (2025-10-01, 第一卷第 10 章)
  ├── file_version: 1.10.2  (2025-10-15, 两次修正)
  └── file_version: 2.1.0   (2026-01-01, 第二卷第 1 章)
```

## 4. 版本更新策略

### 4.1 更新流程

```
┌─────────────────────────────────────────────────────────────────┐
│                     UCX 文件更新流程                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  步骤 1：修改内容                                               │
│  ├── 编辑章节文件                                               │
│  ├── 更新 struct.json（若结构变化）                             │
│  └── 更新 codex.json 的 file_version                           │
│                                                                 │
│  步骤 2：重新构建                                               │
│  └── ucx build                                                 │
│      ├── 重新计算所有文件哈希                                   │
│      ├── 重新生成 MANIFEST.MF                                   │
│      └── 处理加密文件（若有变更）                               │
│                                                                 │
│  步骤 3：重新签名                                               │
│  └── ucx sign --key author.key                                 │
│      ├── 重新生成 Layer 1 签名                                  │
│      └── 重新生成 Layer 2 签名                                  │
│                                                                 │
│  步骤 4：更新存证（可选）                                       │
│  └── ucx chain update --chain ethereum                         │
│      └── 将新的 Merkle Root 上链                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 增量更新（未来扩展）

当前版本规范要求每次发布完整的 UCX 文件。未来可扩展支持增量更新（Patch）：

```
┌─────────────────────────────────────────────────────────────────┐
│                     增量更新（未来扩展）                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  完整文件：novel-v1.2.0.ucx (15 MB)                            │
│                                                                 │
│  增量补丁：novel-v1.2.0-to-v1.2.1.ucxpatch (50 KB)            │
│  ├── 仅包含变更的文件                                           │
│  ├── 包含新版本的 MANIFEST.MF                                   │
│  ├── 包含新签名                                                 │
│  └── 指定基准版本（v1.2.0）                                     │
│                                                                 │
│  应用方式：                                                     │
│  ucx patch --base novel-v1.2.0.ucx \                           │
│            --patch novel-v1.2.0-to-v1.2.1.ucxpatch             │
│  → 生成 novel-v1.2.1.ucx                                       │
│                                                                 │
│  状态：预留设计，待未来规范版本详细定义                         │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 5. 工具版本

### 5.1 版本号格式

工具版本遵循 [Semantic Versioning 2.0.0](https://semver.org/)：

```
MAJOR.MINOR.PATCH[-prerelease][+build]
```

**示例**：`0.1.0`、`1.0.0`、`1.2.3-beta.1`、`2.0.0-rc.1+build.123`

### 5.2 版本记录

工具版本记录在 UCX 文件的以下位置：

| 位置 | 字段 | 示例 |
|------|------|------|
| MANIFEST.MF | `Created-By` | `Created-By: unicodex-core 0.1.0` |
| 签名文件 (.SF) | `Created-By` | `Created-By: unicodex-core 0.1.0` |

### 5.3 工具兼容性

工具应声明支持的 UCX 规范版本范围：

```toml
# unicodex-core/Cargo.toml
[package.metadata.ucx]
spec_versions = ["1.0", "1.1"]    # 支持的规范版本
min_spec_version = "1.0"           # 最低支持版本
```

## 6. 向后兼容策略

### 6.1 解析器行为

| 场景 | 行为 |
|------|------|
| 遇到未知 JSON 字段 | **忽略但保留**（编辑时不删除） |
| 遇到未知目录/文件 | **忽略**，正常读取其他内容 |
| 遇到未知章节格式 | **跳过**，警告用户 |
| 遇到未知加密算法 | **跳过**，标记为不可解密 |
| 遇到未知签名算法 | **跳过**，标记为不可验证 |
| 遇到未知扩展 | **忽略**，不影响正文阅读 |
| MINOR 版本高于支持 | **警告**，尽力解析 |
| MAJOR 版本高于支持 | **警告**，尝试解析 |

### 6.2 编辑器行为

当编辑器打开高版本文件并保存时：

```
┌─────────────────────────────────────────────────────────────────┐
│                     高版本文件编辑策略                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ● 保留所有未知字段（JSON 中未识别的键值对）                    │
│  ● 保留所有未知文件（不属于已知目录结构的文件）                 │
│  ● 保留所有未知扩展（extensions/ 中不认识的扩展）               │
│  ● 不修改未变更的文件（避免不必要的哈希变化）                   │
│                                                                 │
│  禁止：                                                         │
│  ● 删除未知字段                                                 │
│  ● 删除未知文件                                                 │
│  ● 降级 UCX-Version 字段                                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 6.3 版本迁移

当需要将旧版本文件升级到新规范时：

```bash
# 检查文件使用的规范版本
ucx info --version novel.ucx
# UCX Specification: 1.0
# File Version: 1.2.0
# Tool: unicodex-core 0.1.0

# 迁移到新规范版本
ucx migrate --target-spec 1.1 novel.ucx
# Migrating novel.ucx from spec 1.0 to 1.1...
# [INFO] Added new optional fields to codex.json
# [INFO] Updated MANIFEST.MF UCX-Version to 1.1
# [WARN] File needs re-signing after migration
# Done. Please run 'ucx sign' to re-sign the file.
```

## 7. 版本号分配计划

### 7.1 规范版本路线图

| 版本 | 状态 | 主要内容 |
|------|------|---------|
| 0.1.0-draft | **当前** | 初始设计稿，所有规范文档 |
| 1.0 | 计划中 | 首个正式版本，核心功能稳定 |
| 1.1 | 预留 | 向后兼容的功能扩展 |
| 2.0 | 远期 | 根据生态反馈的重大改进 |

### 7.2 版本冻结规则

- **1.0 发布后**：MAJOR 变更需社区 RFC 讨论
- **核心安全区**（META-INF、签名机制）变更需 MAJOR 版本递增
- **可选功能**（加密、扩展、区块链）的新增不影响 MAJOR 版本

---

*请审阅并指出需要修改的部分*
