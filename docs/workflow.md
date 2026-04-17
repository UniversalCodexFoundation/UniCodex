# UCX 推荐工作流（Recommended Workflow）

> 本文档描述一个完整 UCX 项目从零到发布的推荐工作流。阅读时间 ~10 分钟。

## 核心流程图

```
init ──▶ write ──▶ [encrypt] ──▶ build ──▶ [sign] ──▶ verify
  │        │          │            │          │          │
  │        │          │            │          │          └─ 任何环节可独立验证
  │        │          │            │          └─ 可选：私钥签名追加身份证明
  │        │          │            └─ 打包为 .ucx 归档（ZIP 格式）
  │        │          └─ 可选：对章节/资源逐文件加密（UCXE）
  │        └─ 在 content/ 下编写 Markdown 章节并维护 struct.json
  └─ 创建 unicodex.toml 与目录骨架
```

每个阶段都对应一条 `ucx` 子命令，职责单一；所有阶段输出可独立校验。

---

## 阶段 1：init — 初始化项目

- **目的**：生成 `unicodex.toml` 元数据清单和目录骨架。
- **命令**：
  ```bash
  ucx init ./my-novel --name "我的小说" --author "作者名" --language zh-CN
  ```
- **产物**：
  ```
  my-novel/
  ├── unicodex.toml                # 项目元数据（TOML）
  ├── content/
  │   ├── struct.json              # 章节结构索引
  │   └── chapter-001.md           # 示例章节
  ├── assets/                      # （可选，--full）图片等资源
  └── extras/                      # （可选，--full）附加内容
  ```
- **关键参数**：
  - `--full`：创建完整目录结构（`content/ assets/ extras/`）。
  - `--interactive`：交互式提示输入每个字段。
  - `--from-existing`：从目录现有 `.md` 文件反推项目。
  - `--force`：**仅**在目录已含 `unicodex.toml` 时使用，会覆盖现有项目。
- **校验规则**：`project.version` 必须符合 `MAJOR.MINOR[.PATCH][-pre]`（参见 ROB-4）。

---

## 阶段 2：write — 编写章节

直接用任意编辑器修改 `content/*.md`。同时维护 `content/struct.json` 的层级：

```json
{
  "version": "1.0",
  "structure": [
    {
      "title": "第一卷",
      "children": [
        { "title": "第一章 开端", "file": "chapter-001.md" },
        { "title": "第二章 转折", "file": "chapter-002.md" }
      ]
    }
  ]
}
```

**规范要求**：

- 一个节点 MUST 二选一：`file` 或 `children`，不得同时出现。
- `file` 字段 MUST 是**相对于 `content/` 的安全相对路径**：
  - 不得以 `/` 或盘符（`C:`）开头；
  - 不得含反斜杠 `\`；
  - 不得含 `..` 段；
  - 不得使用 Windows 保留名（CON/PRN/AUX/NUL/COM1-9/LPT1-9，含扩展名匹配）。

---

## 阶段 3：encrypt（可选）— 加密章节/资源

仅当部分内容需要付费/订阅解锁或 DRM 保护时执行。

- **命令**（口令模式）：
  ```bash
  ucx encrypt content/chapter-002.md --passphrase --kdf argon2id
  # 交互输入口令（最少 8 字符，--allow-weak 可放行更短口令）
  ```
- **命令**（直接密钥）：
  ```bash
  ucx encrypt content/chapter-002.md \
      --algorithm AES-256-GCM \
      --key "<BASE64_32BYTE_KEY>"
  ```
- **输出**：文件名不变，内容变为 UCXE 二进制（以 `UCXE` 魔数开头）。
- **注意**：
  - UCXE 裸文件不是 `.ucx` 归档；直接 `ucx verify` 会被识别并提示先 `ucx decrypt`。
  - 加密后的文件作为 **密文** 参与后续 MANIFEST.MF 哈希；签名会覆盖"加密状态"本身。

---

## 阶段 4：build — 打包 UCX 归档

- **命令**：
  ```bash
  ucx build ./my-novel                 # 默认输出到 ./my-novel/dist/
  ucx build ./my-novel --dry-run       # 只校验、不生成文件
  ```
- **内部步骤**：
  1. 解析 `unicodex.toml` 并校验 `[project].version` 格式。
  2. 读取 `content/struct.json`，校验 `file`/`children` 互斥与路径安全。
  3. 检查所有 `file` 引用在磁盘上存在。
  4. 发现**重复 `file` 引用**时打印 WARNING（不阻塞）。
  5. 生成 `metadata/codex.json`、`content/struct.json`、`META-INF/MANIFEST.MF`（含 BLAKE3 摘要）。
  6. 写 ZIP：`mimetype`（STORED）必须是首条目，其余 DEFLATED。
- **校验打包结果**：
  ```bash
  ucx info ./my-novel/dist/my-novel.ucx
  ucx verify ./my-novel/dist/my-novel.ucx
  ```

---

## 阶段 5：sign（可选）— 追加身份证明

- **命令**（使用 `unicodex.toml` 的 `[signing]` 段自动完成）：
  ```bash
  ucx sign ./my-novel/dist/my-novel.ucx
  ```
- **产物**：在 UCX 内写入 `META-INF/{SIGNER}.sig`、`META-INF/certs/{SIGNER}.cert.pem` 等。
- **要求**：
  - 证书 `notBefore`/`notAfter` 必须覆盖签名时刻（详见 `08-keys-identity.md` §5.2.1/5.3.1）。
  - 若使用 CA 证书链，提交时需要把中间 CA 一并打包到 `META-INF/certs/*.chain.pem`。

---

## 阶段 6：verify — 端到端完整性+身份校验

- **命令**：
  ```bash
  ucx verify ./my-novel/dist/my-novel.ucx
  ucx verify ./my-novel/dist/my-novel.ucx --show-signers --verbose
  ```
- **校验项**（执行顺序 = 发现问题的顺序）：
  1. ZIP 本地文件头签名（`PK\x03\x04`），防止拼接攻击（ROB-2）。
  2. `META-INF/MANIFEST.MF` 的 `UCX-Version` MAJOR 必须 ≤ 工具支持上限（ROB-1）。
  3. `mimetype` 首条目且内容为 `application/vnd.unicodex+zip`。
  4. 所有 manifest 条目的 BLAKE3 摘要与实际文件一致。
  5. 如有签名：签名链各级证书有效期严格校验，签名本身与公钥匹配。

---

## 典型反模式（Anti-patterns）

- ❌ 直接把多个 `.ucx` 拼接（`cat`）。结果文件会被拒绝（ROB-2）。
- ❌ 在 `struct.json` 里使用 `/etc/passwd` 这类绝对路径或 `../outside.md`。build 会立即失败（ROB-3）。
- ❌ 用 `--allow-weak` 随意放行短口令。仅在迁移老项目时使用，之后 MUST 重新加密。
- ❌ 忽略 build 过程中"Duplicate file references"警告。通常是复制粘贴遗漏，会导致同一文件被重复打包/签名。

---

## 相关规范文档

- `01-file-structure.md` — UCX 归档内部结构。
- `02-metadata-spec.md` — `unicodex.toml` / `codex.json` 字段语义。
- `04-crypto-spec.md` — 加密格式与 AAD 规则。
- `05-signature-spec.md` — 签名算法与 Manifest 扩展。
- `07-versioning.md` — 版本号语义与兼容性矩阵。
- `08-keys-identity.md` — 证书与身份验证。
- `quickstart.md` — 5 分钟上手。
