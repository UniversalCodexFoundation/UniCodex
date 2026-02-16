# UCX 区块链溯源规范

> 版本：0.1.0-draft
> 前置依赖：[01-file-structure.md](./01-file-structure.md)、[05-signature-spec.md](./05-signature-spec.md)

## 1. 概述

区块链溯源是 UCX 的**可选扩展功能**，通过将作品的哈希指纹锚定到区块链上，提供不可篡改的存在性证明和时间证明。

### 1.1 设计原则

```
┌─────────────────────────────────────────────────────────────────┐
│                   区块链溯源设计原则                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ● 可选性：区块链存证为可选功能，不影响 UCX 核心功能            │
│  ● 不上链内容：仅将哈希指纹上链，作品内容永远不上链            │
│  ● 多链支持：支持多条区块链，不绑定特定链                      │
│  ● 可验证：任何人可独立验证链上记录                             │
│  ● 离线可用：UCX 文件本身不依赖区块链即可使用                  │
│                                                                 │
│  核心用途：                                                     │
│  ├── 存在性证明（Proof of Existence）                           │
│  ├── 时间戳证明（Proof of Timestamp）                           │
│  ├── 版权预防（Copyright Preemption）                           │
│  └── 来源追溯（Provenance Tracking）                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 不上链的内容

| 内容 | 上链 | 说明 |
|------|------|------|
| 作品原文 | **否** | 隐私保护，避免不可逆泄露 |
| 作者个人信息 | **否** | GDPR 等隐私法规要求 |
| 加密密钥 | **否** | 安全要求 |
| MANIFEST.MF 哈希 | **是** | 证明文件集的完整性 |
| Merkle Root | **是** | 证明每个文件的存在性 |
| 签名者公钥指纹 | **是** | 关联签名身份 |
| UCX ID | **是** | 作品唯一标识 |
| 时间戳 | **是** | 由区块链共识提供 |

## 2. chain/ 目录结构

```
chain/
├── genesis.proof          # [必需] 首次存证记录
├── updates/               # [可选] 版本更新存证
│   ├── update-001.proof   # 第一次更新存证
│   ├── update-002.proof   # 第二次更新存证
│   └── ...
└── merkle.json            # [可选] Merkle Tree 完整数据
```

## 3. genesis.proof（首次存证）

### 3.1 完整结构

```json
{
  "$schema": "https://unicodex.org/schemas/genesis-proof-v1.json",
  "version": "1.0",
  "ucx_id": "urn:ucx:550e8400-e29b-41d4-a716-446655440000",
  "created_at": "2025-06-01T12:00:00Z",

  "content_hash": {
    "algorithm": "BLAKE3",
    "manifest_hash": "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
    "merkle_root": "7d865e959b2466918c9863afca942d0fb89d7c9ac0c99bafc3749504ded97730",
    "file_count": 12
  },

  "signers": [
    {
      "signer_id": "AUTHOR",
      "public_key_fingerprint": "af13:49b9:f5f9:a1a6:a040:4dea:36dc:c949",
      "algorithm": "Ed25519"
    }
  ],

  "anchors": [
    {
      "chain": "ethereum",
      "network": "mainnet",
      "tx_id": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
      "block_number": 19000000,
      "block_hash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
      "timestamp": "2025-06-01T12:00:30Z",
      "contract_address": "0x0000000000000000000000000000000000000000",
      "explorer_url": "https://etherscan.io/tx/0x1234...",
      "gas_used": 21000,
      "method": "op_return"
    },
    {
      "chain": "bitcoin",
      "network": "mainnet",
      "tx_id": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab",
      "block_number": 850000,
      "block_hash": "000000000000000000012345abcdef...",
      "timestamp": "2025-06-01T12:15:00Z",
      "explorer_url": "https://mempool.space/tx/abcdef...",
      "method": "op_return"
    }
  ]
}
```

### 3.2 字段说明

#### content_hash（内容哈希）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `algorithm` | **是** | string | 哈希算法标识 |
| `manifest_hash` | **是** | string | MANIFEST.MF 文件本身的哈希值（Hex 编码） |
| `merkle_root` | **是** | string | 所有文件哈希的 Merkle Tree 根（Hex 编码） |
| `file_count` | **是** | integer | 受保护的文件总数 |

#### signers（签名者列表）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `signer_id` | **是** | string | 签名者 ID（对应 META-INF/signatures/） |
| `public_key_fingerprint` | **是** | string | 公钥指纹 |
| `algorithm` | **是** | string | 签名算法 |

#### anchors（区块链锚定记录）

| 字段 | 必需 | 类型 | 说明 |
|------|------|------|------|
| `chain` | **是** | string | 区块链标识（见 §3.3） |
| `network` | **是** | string | 网络标识（`mainnet` / `testnet` / `devnet`） |
| `tx_id` | **是** | string | 交易哈希 |
| `block_number` | 否 | integer | 区块高度 |
| `block_hash` | 否 | string | 区块哈希 |
| `timestamp` | **是** | string | 区块时间戳（ISO 8601） |
| `contract_address` | 否 | string | 合约地址（若使用智能合约） |
| `explorer_url` | 否 | string | 区块浏览器链接 |
| `gas_used` | 否 | integer | Gas 消耗（EVM 链） |
| `method` | **是** | string | 存证方式（见 §3.4） |

### 3.3 支持的区块链

| chain 标识 | 区块链 | 状态 |
|-----------|--------|------|
| `ethereum` | Ethereum | 支持 |
| `bitcoin` | Bitcoin | 支持 |
| `polygon` | Polygon (PoS) | 支持 |
| `arbitrum` | Arbitrum One | 支持 |
| `optimism` | Optimism | 支持 |
| `base` | Base | 支持 |
| `solana` | Solana | 支持 |
| `avalanche` | Avalanche C-Chain | 支持 |

> **扩展**：任何区块链均可使用，`chain` 字段为自由字符串。以上为预定义标识。

### 3.4 存证方式

| method | 说明 | 适用链 |
|--------|------|--------|
| `op_return` | 将哈希写入交易的 OP_RETURN 字段 | Bitcoin |
| `calldata` | 将哈希写入交易的 calldata | EVM 链 |
| `contract` | 调用智能合约存储哈希 | EVM 链、Solana |
| `memo` | 将哈希写入交易的 memo 字段 | 通用 |
| `nft` | 铸造 NFT 关联哈希 | EVM 链、Solana |

## 4. Merkle Tree

### 4.1 构建方式

UCX 使用 Merkle Tree 将所有受保护文件的哈希聚合为一个根哈希。

```
┌─────────────────────────────────────────────────────────────────┐
│                     Merkle Tree 结构                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│                    ┌──────────────────┐                         │
│                    │   Merkle Root    │ ← 上链的根哈希          │
│                    └────────┬─────────┘                         │
│                   ┌─────────┴─────────┐                        │
│              ┌────┴────┐         ┌────┴────┐                   │
│              │  H(AB)  │         │  H(CD)  │                   │
│              └────┬────┘         └────┬────┘                   │
│            ┌──────┴──────┐     ┌──────┴──────┐                 │
│         ┌──┴──┐       ┌──┴──┐ ┌──┴──┐      ┌──┴──┐            │
│         │H(A) │       │H(B) │ │H(C) │      │H(D) │            │
│         └──┬──┘       └──┬──┘ └──┬──┘      └──┬──┘            │
│            │             │       │             │               │
│         file A        file B   file C       file D            │
│                                                                 │
│  叶子节点 = HASH(0x00 || file_path || 0x00 || file_hash)      │
│  内部节点 = HASH(0x01 || left_child || right_child)            │
│  奇数叶子 → 复制最后一个叶子补齐                               │
│                                                                 │
│  前缀字节防止二级原像攻击：                                    │
│  ├── 0x00 → 叶子节点                                           │
│  └── 0x01 → 内部节点                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 叶子节点计算

```
leaf_hash = HASH(0x00 || UTF8(file_path) || 0x00 || file_content_hash)
```

- `file_path`：相对于 ZIP 根目录的路径（如 `content/chapter-001.ucxc`）
- `file_content_hash`：MANIFEST.MF 中记录的文件哈希值（原始字节）
- 路径排序：叶子节点按 `file_path` 的字典序（UTF-8）排列

### 4.3 内部节点计算

```
internal_hash = HASH(0x01 || left_child_hash || right_child_hash)
```

### 4.4 merkle.json（完整 Merkle 数据）

```json
{
  "$schema": "https://unicodex.org/schemas/merkle-v1.json",
  "version": "1.0",
  "algorithm": "BLAKE3",
  "root": "7d865e959b2466918c9863afca942d0fb89d7c9ac0c99bafc3749504ded97730",
  "leaves": [
    {
      "index": 0,
      "path": "assets/cover.jpg",
      "hash": "n4bQgYhMfWWaL+qgxVrQFaO/TxsrC4Is0V1sFbDwCgg="
    },
    {
      "index": 1,
      "path": "content/chapter-001.ucxc",
      "hash": "5M+jmj03vjHFlgnoh5cHmcqmihm/qhUT8hUFBe3R"
    },
    {
      "index": 2,
      "path": "content/struct.json",
      "hash": "fYZelZskZpGMmGOvyplC0PiJ18msGMm/w3SVBe3Zl3A="
    },
    {
      "index": 3,
      "path": "metadata/codex.json",
      "hash": "rxNJufX5oapgQE3qNtzMlJm8slya3BErfMmjyuQfMmI="
    }
  ]
}
```

### 4.5 Merkle Proof（单文件存在证明）

验证某个特定文件是否包含在存证中，无需下载整个 UCX 文件：

```json
{
  "file_path": "content/chapter-001.ucxc",
  "file_hash": "5M+jmj03vjHFlgnoh5cHmcqmihm/qhUT8hUFBe3R",
  "proof": [
    {
      "position": "right",
      "hash": "rxNJufX5oapgQE3qNtzMlJm8slya3BErfMmjyuQfMmI="
    },
    {
      "position": "left",
      "hash": "a1b2c3d4e5f6..."
    }
  ],
  "root": "7d865e959b2466918c9863afca942d0fb89d7c9ac0c99bafc3749504ded97730"
}
```

**验证流程**：
1. 计算叶子哈希：`HASH(0x00 || file_path || 0x00 || file_hash)`
2. 沿 proof 路径逐层计算内部节点
3. 最终结果 == `root` → 文件确实在存证中

## 5. 存证流程

### 5.1 首次存证

```
┌─────────────────────────────────────────────────────────────────┐
│                     首次存证流程                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  步骤 1：构建 Merkle Tree                                       │
│  ├── 从 MANIFEST.MF 提取所有文件哈希                            │
│  ├── 按文件路径排序                                             │
│  ├── 构建 Merkle Tree                                           │
│  └── 得到 merkle_root                                           │
│                                                                 │
│  步骤 2：计算 manifest_hash                                     │
│  └── manifest_hash = HASH(MANIFEST.MF 文件内容)                │
│                                                                 │
│  步骤 3：构造上链数据                                           │
│  └── on_chain_data = HASH(                                     │
│        ucx_id ||                                                │
│        manifest_hash ||                                         │
│        merkle_root ||                                           │
│        signer_fingerprint                                       │
│      )                                                          │
│                                                                 │
│  步骤 4：提交交易                                               │
│  ├── 选择目标区块链                                             │
│  ├── 将 on_chain_data 写入交易                                  │
│  └── 等待交易确认                                               │
│                                                                 │
│  步骤 5：记录存证                                               │
│  ├── 创建 chain/genesis.proof                                  │
│  ├── 创建 chain/merkle.json（可选）                             │
│  └── 将存证文件加入 MANIFEST.MF                                │
│                                                                 │
│  步骤 6：重新签名                                               │
│  └── 加入存证文件后需重新生成签名                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 版本更新存证

作品更新后（修改章节、新增内容等），可追加更新存证：

```json
{
  "$schema": "https://unicodex.org/schemas/update-proof-v1.json",
  "version": "1.0",
  "ucx_id": "urn:ucx:550e8400-e29b-41d4-a716-446655440000",
  "update_index": 1,
  "previous_merkle_root": "7d865e959b2466918c9863afca942d0fb89d7c9ac0c99bafc3749504ded97730",

  "content_hash": {
    "algorithm": "BLAKE3",
    "manifest_hash": "new_manifest_hash...",
    "merkle_root": "new_merkle_root...",
    "file_count": 15
  },

  "changes": {
    "added": ["content/chapter-013.ucxc", "content/chapter-014.ucxc"],
    "modified": ["content/chapter-012.ucxc", "metadata/codex.json"],
    "removed": []
  },

  "anchors": [
    {
      "chain": "ethereum",
      "network": "mainnet",
      "tx_id": "0xaabbccdd...",
      "block_number": 19100000,
      "timestamp": "2025-07-01T12:00:00Z",
      "method": "contract"
    }
  ]
}
```

## 6. 验证流程

### 6.1 链上验证

```
┌─────────────────────────────────────────────────────────────────┐
│                   区块链存证验证流程                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  步骤 1：读取 genesis.proof                                     │
│  ├── 提取 manifest_hash、merkle_root                           │
│  └── 提取 anchors 列表                                         │
│                                                                 │
│  步骤 2：本地验证                                               │
│  ├── 计算当前 MANIFEST.MF 的哈希                                │
│  ├── 比对 manifest_hash → 确认文件集未变                        │
│  ├── 从 MANIFEST.MF 重建 Merkle Tree                           │
│  └── 比对 merkle_root → 确认每个文件的完整性                    │
│                                                                 │
│  步骤 3：链上验证（可选，需网络）                               │
│  ├── 对每个 anchor：                                            │
│  │   ├── 通过 RPC 或区块浏览器 API 查询交易                    │
│  │   ├── 提取交易中的 on_chain_data                             │
│  │   ├── 重新计算 on_chain_data 并比对                          │
│  │   └── 确认区块时间戳                                         │
│  └── 记录验证结果                                               │
│                                                                 │
│  步骤 4：汇总结果                                               │
│  ├── 本地验证：Merkle Root 匹配 / 不匹配                       │
│  ├── 链上验证：交易存在且数据匹配 / 不匹配 / 无法连接          │
│  └── 时间证明：区块时间 = 作品最早存在时间                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 验证输出示例

```
Blockchain Verification: novel.ucx

[1/2] Local Verification:
  [PASS] Manifest hash matches genesis.proof
  [PASS] Merkle root matches (12 files)
  [PASS] All file hashes verified

[2/2] On-Chain Verification:
  Anchor #1 (Ethereum mainnet):
    [PASS] Transaction 0x1234... confirmed (block #19000000)
    [PASS] On-chain data matches local hash
    [INFO] Timestamp: 2025-06-01T12:00:30Z
    [INFO] Explorer: https://etherscan.io/tx/0x1234...

  Anchor #2 (Bitcoin mainnet):
    [PASS] Transaction abcdef... confirmed (block #850000)
    [PASS] OP_RETURN data matches local hash
    [INFO] Timestamp: 2025-06-01T12:15:00Z

Overall: ANCHORED
  Earliest proof of existence: 2025-06-01T12:00:30Z (Ethereum)
  Author fingerprint: af13:49b9:f5f9:a1a6:...
```

## 7. 智能合约（可选）

### 7.1 UCX Registry 合约

Unicodex 官方可部署 Registry 智能合约，提供更丰富的链上查询功能。

**合约接口**（Solidity 示例）：

```solidity
// SPDX-License-Identifier: MIT
interface IUCXRegistry {
    /// @notice Register a new UCX work on-chain.
    /// @param ucxId The UCX unique identifier (keccak256 hash of urn:ucx:UUID).
    /// @param manifestHash BLAKE3 hash of MANIFEST.MF.
    /// @param merkleRoot Merkle root of all file hashes.
    /// @param signerFingerprint Author's public key fingerprint.
    function register(
        bytes32 ucxId,
        bytes32 manifestHash,
        bytes32 merkleRoot,
        bytes32 signerFingerprint
    ) external;

    /// @notice Verify a UCX work against on-chain record.
    /// @param ucxId The UCX unique identifier.
    /// @return exists Whether the work is registered.
    /// @return manifestHash Stored manifest hash.
    /// @return merkleRoot Stored merkle root.
    /// @return timestamp Registration timestamp.
    function verify(bytes32 ucxId)
        external
        view
        returns (
            bool exists,
            bytes32 manifestHash,
            bytes32 merkleRoot,
            uint256 timestamp
        );

    /// @notice Register an update to an existing work.
    /// @param ucxId The UCX unique identifier.
    /// @param newManifestHash Updated manifest hash.
    /// @param newMerkleRoot Updated merkle root.
    /// @param updateIndex Sequential update number.
    function registerUpdate(
        bytes32 ucxId,
        bytes32 newManifestHash,
        bytes32 newMerkleRoot,
        uint256 updateIndex
    ) external;
}
```

### 7.2 合约部署（预留）

| 网络 | 合约地址 | 状态 |
|------|---------|------|
| Ethereum Mainnet | 待部署 | 预留 |
| Polygon PoS | 待部署 | 预留 |
| Base | 待部署 | 预留 |

## 8. 工具命令参考

```bash
# 首次存证（Ethereum）
ucx chain anchor --chain ethereum --network mainnet

# 多链存证
ucx chain anchor --chain ethereum --chain bitcoin

# 存证并指定合约
ucx chain anchor --chain ethereum \
                 --contract 0x1234...abcd

# 验证存证（本地 + 链上）
ucx chain verify novel.ucx

# 仅本地验证（不查询链上）
ucx chain verify --offline novel.ucx

# 查看存证信息
ucx chain info novel.ucx

# 更新存证（作品更新后）
ucx chain update --chain ethereum

# 生成 Merkle Proof（单文件）
ucx chain proof --file content/chapter-001.ucxc novel.ucx
```

## 9. 安全考量

### 9.1 注意事项

| 问题 | 说明 |
|------|------|
| 区块链不可变 | 一旦上链无法删除，谨慎选择上链内容 |
| Gas 费用 | 链上操作需要费用，选择合适的链和时机 |
| 隐私 | 哈希不可逆，但 UCX ID 可关联到作品 |
| 区块重组 | 等待足够确认数（Bitcoin ≥ 6，Ethereum ≥ 12） |
| 时间精度 | 区块时间非精确时间，Bitcoin 精度约 10 分钟 |
| 链可用性 | 多链存证提高可靠性 |

### 9.2 推荐策略

- **个人作者**：选择一条低费用链（如 Polygon / Base）
- **商业出版**：多链存证（Ethereum + Bitcoin）
- **更新频率**：仅在正式版本发布时存证，避免频繁上链
- **合约 vs 裸交易**：合约提供查询功能，裸交易成本更低

---

*请审阅并指出需要修改的部分*
