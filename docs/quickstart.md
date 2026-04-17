# UCX 5 分钟上手（Quickstart）

> 目标：从零构建一个可签名、可校验的最小 UCX 文件。全流程 ≤ 5 分钟。

## 0. 前置条件

- 已安装 `ucx` CLI（构建自本仓库的 `unicodex-core` crate）。
  ```bash
  cargo build --release -p unicodex-core
  # 二进制位于 ./target/release/ucx（或 ucx.exe）
  ```
- 确认版本：
  ```bash
  ucx --version
  ```

---

## 1. 初始化项目（30 秒）

```bash
ucx init ./hello-ucx --name "Hello UCX" --author "Me" --language en
cd hello-ucx
```

**产物**：
```
hello-ucx/
├── unicodex.toml
└── content/
    ├── struct.json
    └── chapter-001.md
```

---

## 2. 写一章（1 分钟）

编辑 `content/chapter-001.md`：

```markdown
# Chapter 1

Hello, UCX world.
```

（可选）追加一章 `content/chapter-002.md`，并在 `content/struct.json` 中登记：

```json
{
  "version": "1.0",
  "structure": [
    { "title": "Chapter 1", "file": "chapter-001.md" },
    { "title": "Chapter 2", "file": "chapter-002.md" }
  ]
}
```

---

## 3. 预检查（10 秒）

```bash
ucx build . --dry-run
```

看到 "Validation passed. Ready to build." 即可继续。

---

## 4. 打包（10 秒）

```bash
ucx build .
# → ./dist/hello-ucx.ucx
```

---

## 5. 校验完整性（5 秒）

```bash
ucx verify ./dist/hello-ucx.ucx
```

所有行应为 `[OK]`，末尾 `Result: N/N files valid`。

---

## 6. （可选）加密一章（30 秒）

```bash
ucx encrypt content/chapter-002.md --passphrase --kdf argon2id
# 交互输入口令（≥ 8 字符）
ucx build .             # 重新打包，密文参与哈希/签名
```

---

## 7. （可选）签名（30 秒）

首次需要生成一对私钥/证书：

```bash
ucx cert init --id AUTHOR --common-name "Me"
```

在 `unicodex.toml` 增加：

```toml
[signing]
key_path  = "keys/AUTHOR.key"
cert_path = "keys/AUTHOR.cert.pem"
signer_id = "AUTHOR"
```

然后：

```bash
ucx sign ./dist/hello-ucx.ucx
ucx verify ./dist/hello-ucx.ucx --show-signers
```

应该看到 `Signature status: VERIFIED`。

---

## 下一步

- 阅读 [`workflow.md`](./workflow.md) 了解完整开发流程。
- 规范细节：[`01-file-structure.md`](./01-file-structure.md)、[`04-crypto-spec.md`](./04-crypto-spec.md)、[`05-signature-spec.md`](./05-signature-spec.md)、[`07-versioning.md`](./07-versioning.md)、[`08-keys-identity.md`](./08-keys-identity.md)。

---

## 常见错误速查

| 报错 | 根因 | 对策 |
|------|------|------|
| `invalid UCX file: expected ZIP signature at offset 0` | 文件被拼接/前缀污染 | 重新 `ucx build`。 |
| `unsupported UCX-Version: MAJOR > 1` | 文件来自更新版工具 | 升级本地 `ucx`。 |
| `this file appears to be UCXE encrypted, please run ucx decrypt first` | 把 UCXE 裸文件当成 `.ucx` 校验 | 先 `ucx decrypt`，或对正确的 `.ucx` 打包产物 verify。 |
| `passphrase must be at least 8 characters` | 交互输入的口令过短 | 使用 ≥ 8 字符口令，或临时加 `--allow-weak`。 |
| `directory already contains a UCX project (unicodex.toml ...)` | 在已有项目目录重复 `init` | 加 `--force` 强制覆盖，或换一个目录。 |
