//! Content structure types — corresponds to `content/struct.json`.
//!
//! This module defines the data structures for the UCX content structure tree,
//! as specified in `01-file-structure.md §4.1`. The `Structure` struct is the root
//! type that maps directly to the `struct.json` file inside a UCX archive.
//!
//! 内容结构类型 — 对应 `content/struct.json`。
//! 本模块定义 UCX 内容结构树的数据结构（规范见 `01-file-structure.md §4.1`）。
//! `Structure` 结构体是根类型，直接映射到 UCX 归档中的 `struct.json` 文件。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Structure — root structure type / 根结构类型
// =============================================================================

/// The root structure definition for a UCX work, corresponding to `content/struct.json`.
///
/// Defines the logical hierarchy of the work (volumes, chapters, extras, etc.)
/// using a tree of `StructureNode` items.
///
/// UCX 作品的根结构定义，对应 `content/struct.json`。
/// 使用 `StructureNode` 的树形结构定义作品的逻辑层次（卷、章节、番外等）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Structure {
    /// JSON Schema URL (optional).
    /// JSON Schema URL（可选）。
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Structure schema version (e.g., "1.0").
    /// 结构 schema 版本（如 "1.0"）。
    pub version: String,

    /// The top-level nodes of the content structure.
    /// 内容结构的顶层节点。
    pub structure: Vec<StructureNode>,
}

// =============================================================================
// StructureNode — recursive tree node / 递归树节点
// =============================================================================

/// A node in the content structure tree.
///
/// Nodes can be either:
/// - **Leaf nodes**: have a `file` field pointing to a content file (no `children`).
/// - **Container nodes**: have a `children` array (no `file`).
///
/// The `file` and `children` fields are **mutually exclusive** — a node must have
/// exactly one of them.
///
/// 内容结构树中的一个节点。
///
/// 节点可以是：
/// - **叶子节点**：有 `file` 字段指向内容文件（无 `children`）。
/// - **容器节点**：有 `children` 数组（无 `file`）。
///
/// `file` 和 `children` 字段**互斥** — 节点必须恰好有其中一个。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureNode {
    /// Display title of this node (required).
    /// 节点的显示标题（必需）。
    pub title: String,

    /// Content file path relative to `content/` (leaf node).
    /// Mutually exclusive with `children`.
    ///
    /// 内容文件路径，相对于 `content/`（叶子节点）。
    /// 与 `children` 互斥。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,

    /// Child nodes (container node).
    /// Mutually exclusive with `file`.
    ///
    /// 子节点（容器节点）。
    /// 与 `file` 互斥。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<StructureNode>>,

    /// Type marker (e.g., "volume", "chapter", "extra") — annotation only.
    /// 类型标记（如 "volume"、"chapter"、"extra"）— 仅作注释。
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,

    /// Unique identifier — annotation or program reference.
    /// 唯一标识符 — 用于注释或程序引用。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Descriptive note — annotation only.
    /// 备注说明 — 仅作注释。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Style identifier for reader rendering (optional).
    /// 样式标识，供阅读器渲染使用（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    /// Encryption configuration — only applies to leaf nodes (optional).
    /// 加密配置 — 仅对叶子节点生效（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<Encryption>,
}

impl StructureNode {
    /// Check if this node is a leaf node (has `file` and no `children`).
    ///
    /// 检查此节点是否为叶子节点（有 `file` 且无 `children`）。
    pub fn is_leaf(&self) -> bool {
        self.file.is_some() && self.children.is_none()
    }

    /// Check if this node is a container node (has `children` and no `file`).
    ///
    /// 检查此节点是否为容器节点（有 `children` 且无 `file`）。
    pub fn is_container(&self) -> bool {
        self.children.is_some() && self.file.is_none()
    }

    /// Recursively collect all leaf file paths under this node.
    ///
    /// 递归收集此节点下的所有叶子文件路径。
    pub fn collect_files(&self) -> Vec<&str> {
        let mut files = Vec::new();
        if let Some(ref f) = self.file {
            files.push(f.as_str());
        }
        if let Some(ref children) = self.children {
            for child in children {
                files.extend(child.collect_files());
            }
        }
        files
    }
}

// =============================================================================
// Encryption / 加密配置
// =============================================================================

/// Encryption configuration for a chapter or resource.
///
/// Each encrypted item must specify an algorithm and at least one key access method.
///
/// 章节或资源的加密配置。
/// 每个加密项必须指定算法和至少一种密钥获取方式。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encryption {
    /// Encryption algorithm identifier.
    /// Supported: "AES-256-GCM" (recommended), "AES-256-CBC", "ChaCha20-Poly1305".
    ///
    /// 加密算法标识。
    /// 支持："AES-256-GCM"（推荐）、"AES-256-CBC"、"ChaCha20-Poly1305"。
    pub algorithm: String,

    /// Key access methods — at least one required.
    /// The reader should try them in order.
    ///
    /// 密钥获取方式列表 — 至少需要一项。
    /// 阅读器应按数组顺序尝试。
    pub key_access: Vec<KeyAccess>,
}

// =============================================================================
// KeyAccess / 密钥获取方式
// =============================================================================

/// A method for obtaining the decryption key.
///
/// UCX supports multiple key access methods that can coexist as fallbacks.
///
/// 获取解密密钥的方式。
/// UCX 支持多种密钥获取方式，可作为备选并存。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyAccess {
    /// Key access method identifier.
    /// Values: "direct", "message", "url", "service", "extension".
    ///
    /// 密钥获取方式标识。
    /// 取值："direct"、"message"、"url"、"service"、"extension"。
    pub method: String,

    /// Base64-encoded encryption key (method: "direct").
    /// Base64 编码的加密密钥（方式："direct"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// Base64-encoded initialization vector (method: "direct").
    /// Base64 编码的初始化向量（方式："direct"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iv: Option<String>,

    /// Human-readable hint text (method: "message").
    /// 人类可读的提示文本（方式："message"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Key retrieval URL (method: "url").
    /// 密钥获取 URL（方式："url"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Authentication type (method: "url").
    /// Values: "none", "login", "token", "oauth", "certificate".
    ///
    /// 认证方式（方式："url"）。
    /// 取值："none"、"login"、"token"、"oauth"、"certificate"。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<String>,

    /// Service provider URL (method: "service").
    /// 服务提供商 URL（方式："service"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,

    /// Service type identifier (method: "service").
    /// Values: "purchase_verify", "author_verify", "subscription", "public_key".
    ///
    /// 服务类型标识（方式："service"）。
    /// 取值："purchase_verify"、"author_verify"、"subscription"、"public_key"。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_type: Option<String>,

    /// Extension reference path (method: "extension").
    /// Format: "ext.{alias}" or "ext.{provider}.{id}@{version}".
    ///
    /// 扩展引用路径（方式："extension"）。
    /// 格式："ext.{alias}" 或 "ext.{provider}.{id}@{version}"。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<String>,

    /// Additional parameters (methods: "service", "url", "extension").
    /// 额外参数（方式："service"、"url"、"extension"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, String>>,
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a simple structure for testing.
    /// 辅助函数：创建一个简单的结构用于测试。
    fn sample_structure() -> Structure {
        Structure {
            schema: Some("https://unicodex.org/schemas/struct-v1.json".to_string()),
            version: "1.0".to_string(),
            structure: vec![
                StructureNode {
                    title: "第一卷 起始".to_string(),
                    file: None,
                    children: Some(vec![
                        StructureNode {
                            title: "第一章 开端".to_string(),
                            file: Some("chapter-001.md".to_string()),
                            children: None,
                            node_type: None,
                            id: None,
                            name: None,
                            style: None,
                            encryption: None,
                        },
                        StructureNode {
                            title: "第二章 相遇".to_string(),
                            file: Some("chapter-002.md".to_string()),
                            children: None,
                            node_type: None,
                            id: Some("ch-002".to_string()),
                            name: None,
                            style: None,
                            encryption: None,
                        },
                    ]),
                    node_type: Some("volume".to_string()),
                    id: None,
                    name: None,
                    style: Some("volume-title".to_string()),
                    encryption: None,
                },
            ],
        }
    }

    #[test]
    fn test_structure_serialize() {
        // Structure should serialize to valid JSON.
        // Structure 应序列化为有效 JSON。
        let s = sample_structure();
        let json = serde_json::to_string_pretty(&s).unwrap();
        assert!(json.contains("\"第一卷 起始\""));
        assert!(json.contains("\"chapter-001.md\""));
    }

    #[test]
    fn test_structure_deserialize() {
        // The sample struct.json should deserialize correctly.
        // 示例 struct.json 应正确反序列化。
        let json = r#"{
            "version": "1.0",
            "structure": [
                {
                    "title": "第一章 开端",
                    "file": "chapter-001.md"
                }
            ]
        }"#;
        let s: Structure = serde_json::from_str(json).unwrap();
        assert_eq!(s.structure.len(), 1);
        assert_eq!(s.structure[0].title, "第一章 开端");
        assert_eq!(s.structure[0].file.as_deref(), Some("chapter-001.md"));
    }

    #[test]
    fn test_structure_serde_round_trip() {
        // Serialize then deserialize should be equivalent.
        // 序列化后反序列化应等价。
        let original = sample_structure();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Structure = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.structure.len(), original.structure.len());
        assert_eq!(
            deserialized.structure[0].title,
            original.structure[0].title
        );
    }

    #[test]
    fn test_node_is_leaf() {
        // A node with `file` and no `children` is a leaf.
        // 有 `file` 无 `children` 的节点是叶子。
        let leaf = StructureNode {
            title: "第一章".to_string(),
            file: Some("chapter-001.md".to_string()),
            children: None,
            node_type: None,
            id: None,
            name: None,
            style: None,
            encryption: None,
        };
        assert!(leaf.is_leaf());
        assert!(!leaf.is_container());
    }

    #[test]
    fn test_node_is_container() {
        // A node with `children` and no `file` is a container.
        // 有 `children` 无 `file` 的节点是容器。
        let container = StructureNode {
            title: "第一卷".to_string(),
            file: None,
            children: Some(vec![]),
            node_type: None,
            id: None,
            name: None,
            style: None,
            encryption: None,
        };
        assert!(container.is_container());
        assert!(!container.is_leaf());
    }

    #[test]
    fn test_collect_files() {
        // collect_files should recursively gather all leaf file paths.
        // collect_files 应递归收集所有叶子文件路径。
        let s = sample_structure();
        let files: Vec<&str> = s
            .structure
            .iter()
            .flat_map(|n| n.collect_files())
            .collect();
        assert_eq!(files, vec!["chapter-001.md", "chapter-002.md"]);
    }

    #[test]
    fn test_encryption_config() {
        // A node with encryption config should serialize correctly.
        // 带加密配置的节点应正确序列化。
        let node = StructureNode {
            title: "VIP 章节".to_string(),
            file: Some("chapter-vip.ucxc".to_string()),
            children: None,
            node_type: None,
            id: None,
            name: None,
            style: None,
            encryption: Some(Encryption {
                algorithm: "AES-256-GCM".to_string(),
                key_access: vec![KeyAccess {
                    method: "message".to_string(),
                    key: None,
                    iv: None,
                    text: Some("付费内容".to_string()),
                    url: None,
                    auth_type: None,
                    provider: None,
                    service_type: None,
                    ext: None,
                    params: None,
                }],
            }),
        };
        let json = serde_json::to_string_pretty(&node).unwrap();
        assert!(json.contains("AES-256-GCM"));
        assert!(json.contains("\"message\""));
        // Round-trip / 往返
        let deserialized: StructureNode = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.encryption.unwrap().algorithm,
            "AES-256-GCM"
        );
    }
}
