//! MANIFEST.MF types — resource manifest for UCX archives.
//!
//! This module defines the data structures for `META-INF/MANIFEST.MF`,
//! as specified in `01-file-structure.md §3.2`. The manifest lists every
//! protected file in the archive along with its hash digest.
//!
//! The format follows the JAR MANIFEST specification (RFC 822 style):
//! - Key-value pairs separated by `: `
//! - Sections separated by blank lines
//! - Main section contains global attributes
//! - Per-entry sections contain file-specific attributes
//!
//! MANIFEST.MF 类型 — UCX 归档的资源清单。
//! 本模块定义 `META-INF/MANIFEST.MF` 的数据结构（规范见 `01-file-structure.md §3.2`）。
//! 清单列出归档中所有受保护文件及其哈希摘要。
//!
//! 格式遵循 JAR MANIFEST 规范（RFC 822 风格）：
//! - 键值对以 `: ` 分隔
//! - 段落以空行分隔
//! - 主段包含全局属性
//! - 条目段包含文件特定属性

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// =============================================================================
// HashAlgorithm / 哈希算法
// =============================================================================

/// Supported hash algorithms for MANIFEST.MF digests.
///
/// MANIFEST.MF 摘要支持的哈希算法。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashAlgorithm {
    /// BLAKE3 (recommended) — 256-bit output, high performance.
    /// BLAKE3（推荐）— 256 位输出，高性能。
    #[serde(rename = "BLAKE3")]
    Blake3,

    /// SHA-256 — 256-bit output, widely compatible.
    /// SHA-256 — 256 位输出，广泛兼容。
    #[serde(rename = "SHA256")]
    Sha256,

    /// SHA-512 — 512-bit output.
    /// SHA-512 — 512 位输出。
    #[serde(rename = "SHA512")]
    Sha512,
}

impl HashAlgorithm {
    /// Return the string identifier used in MANIFEST.MF (e.g., "BLAKE3").
    ///
    /// 返回 MANIFEST.MF 中使用的字符串标识（如 "BLAKE3"）。
    pub fn as_str(&self) -> &'static str {
        match self {
            HashAlgorithm::Blake3 => "BLAKE3",
            HashAlgorithm::Sha256 => "SHA256",
            HashAlgorithm::Sha512 => "SHA512",
        }
    }

    /// Return the digest header name used in manifest entries (e.g., "BLAKE3-Digest").
    ///
    /// 返回清单条目中的摘要头名称（如 "BLAKE3-Digest"）。
    pub fn digest_header(&self) -> String {
        format!("{}-Digest", self.as_str())
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for HashAlgorithm {
    type Err = ManifestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BLAKE3" => Ok(HashAlgorithm::Blake3),
            "SHA256" => Ok(HashAlgorithm::Sha256),
            "SHA512" => Ok(HashAlgorithm::Sha512),
            _ => Err(ManifestError::UnknownAlgorithm(s.to_string())),
        }
    }
}

// =============================================================================
// Manifest / 资源清单
// =============================================================================

/// The MANIFEST.MF resource manifest, listing all protected files and their hashes.
///
/// MANIFEST.MF 资源清单，列出所有受保护文件及其哈希值。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Manifest format version (always "1.0").
    /// 清单格式版本（固定 "1.0"）。
    pub manifest_version: String,

    /// UCX specification version (e.g., "1.0").
    /// UCX 规范版本（如 "1.0"）。
    pub ucx_version: String,

    /// Tool that created this manifest (optional).
    /// 创建此清单的工具标识（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// Hash algorithm used for all digests.
    /// 所有摘要使用的哈希算法。
    pub hash_algorithm: HashAlgorithm,

    /// Per-file manifest entries.
    /// 每个文件的清单条目。
    pub entries: Vec<ManifestEntry>,
}

impl Manifest {
    /// Create a new empty manifest with the specified hash algorithm.
    ///
    /// 使用指定的哈希算法创建新的空清单。
    pub fn new(ucx_version: &str, hash_algorithm: HashAlgorithm) -> Self {
        Self {
            manifest_version: "1.0".to_string(),
            ucx_version: ucx_version.to_string(),
            created_by: Some(format!("unicodex {}", env!("CARGO_PKG_VERSION"))),
            hash_algorithm,
            entries: Vec::new(),
        }
    }

    /// Add an entry to the manifest.
    ///
    /// 向清单添加一个条目。
    pub fn add_entry(&mut self, entry: ManifestEntry) {
        self.entries.push(entry);
    }

    /// Serialize the manifest to RFC 822 format (MANIFEST.MF text).
    ///
    /// 将清单序列化为 RFC 822 格式（MANIFEST.MF 文本）。
    ///
    /// # Format example / 格式示例
    ///
    /// ```text
    /// Manifest-Version: 1.0
    /// UCX-Version: 1.0
    /// Created-By: unicodex 0.1.0
    /// Hash-Algorithm: BLAKE3
    ///
    /// Name: metadata/codex.json
    /// Size: 2048
    /// BLAKE3-Digest: af1349b9...
    /// ```
    pub fn to_manifest_string(&self) -> String {
        let mut output = String::new();

        // Main section / 主段
        output.push_str(&format!("Manifest-Version: {}\n", self.manifest_version));
        output.push_str(&format!("UCX-Version: {}\n", self.ucx_version));
        if let Some(ref created_by) = self.created_by {
            output.push_str(&format!("Created-By: {created_by}\n"));
        }
        output.push_str(&format!("Hash-Algorithm: {}\n", self.hash_algorithm));

        // Per-entry sections / 条目段
        let digest_header = self.hash_algorithm.digest_header();
        for entry in &self.entries {
            output.push('\n');
            output.push_str(&format!("Name: {}\n", entry.name));
            output.push_str(&format!("Size: {}\n", entry.size));
            output.push_str(&format!("{digest_header}: {}\n", entry.digest));
        }

        output
    }

    /// Parse a manifest from RFC 822 format text.
    ///
    /// 从 RFC 822 格式文本解析清单。
    pub fn from_manifest_str(text: &str) -> Result<Self, ManifestError> {
        let mut manifest_version = None;
        let mut ucx_version = None;
        let mut created_by = None;
        let mut hash_algorithm = None;
        let mut entries = Vec::new();

        // Split into sections by blank lines.
        // 按空行分割为段落。
        let sections: Vec<&str> = text.split("\n\n").collect();

        if sections.is_empty() {
            return Err(ManifestError::EmptyManifest);
        }

        // Parse the main section (first section).
        // 解析主段（第一段）。
        for line in sections[0].lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once(": ") {
                match key {
                    "Manifest-Version" => manifest_version = Some(value.to_string()),
                    "UCX-Version" => ucx_version = Some(value.to_string()),
                    "Created-By" => created_by = Some(value.to_string()),
                    "Hash-Algorithm" => {
                        hash_algorithm = Some(HashAlgorithm::from_str(value)?);
                    }
                    _ => {} // Ignore unknown headers / 忽略未知头
                }
            }
        }

        let manifest_version = manifest_version
            .ok_or_else(|| ManifestError::MissingField("Manifest-Version".to_string()))?;
        let ucx_version = ucx_version
            .ok_or_else(|| ManifestError::MissingField("UCX-Version".to_string()))?;
        let hash_algorithm = hash_algorithm
            .ok_or_else(|| ManifestError::MissingField("Hash-Algorithm".to_string()))?;

        // Parse entry sections (remaining sections).
        // 解析条目段（剩余段落）。
        let digest_header = hash_algorithm.digest_header();
        for section in &sections[1..] {
            let section = section.trim();
            if section.is_empty() {
                continue;
            }
            let mut name = None;
            let mut size = None;
            let mut digest = None;

            for line in section.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Some((key, value)) = line.split_once(": ") {
                    match key {
                        "Name" => name = Some(value.to_string()),
                        "Size" => {
                            size = Some(value.parse::<u64>().map_err(|e| {
                                ManifestError::InvalidField(format!("Size: {e}"))
                            })?);
                        }
                        k if k == digest_header => digest = Some(value.to_string()),
                        _ => {} // Ignore unknown headers / 忽略未知头
                    }
                }
            }

            if let (Some(name), Some(size), Some(digest)) = (name, size, digest) {
                entries.push(ManifestEntry {
                    name,
                    size,
                    digest,
                });
            }
        }

        Ok(Manifest {
            manifest_version,
            ucx_version,
            created_by,
            hash_algorithm,
            entries,
        })
    }
}

// =============================================================================
// ManifestEntry / 清单条目
// =============================================================================

/// A single entry in the MANIFEST.MF, representing one protected file.
///
/// MANIFEST.MF 中的单个条目，代表一个受保护文件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// File path relative to the ZIP root (e.g., "metadata/codex.json").
    /// 文件路径，相对于 ZIP 根目录（如 "metadata/codex.json"）。
    pub name: String,

    /// File size in bytes.
    /// 文件大小（字节）。
    pub size: u64,

    /// Hash digest (hex-encoded or Base64-encoded depending on context).
    /// 哈希摘要（十六进制或 Base64 编码，取决于上下文）。
    pub digest: String,
}

impl ManifestEntry {
    /// Create a new manifest entry from raw hash bytes (encodes to Base64).
    ///
    /// Uses standard Base64 encoding (with padding) as specified in
    /// `01-file-structure.md §3.2`, consistent with JAR MANIFEST format.
    ///
    /// 从原始哈希字节创建新的清单条目（编码为 Base64）。
    /// 使用标准 Base64 编码（含 padding），符合 `01-file-structure.md §3.2` 规范，
    /// 与 JAR MANIFEST 格式保持一致。
    pub fn new(name: String, size: u64, hash_bytes: &[u8]) -> Self {
        Self {
            name,
            size,
            digest: BASE64_STANDARD.encode(hash_bytes),
        }
    }
}

/// Decode a Base64-encoded digest string back to raw bytes.
///
/// 将 Base64 编码的摘要字符串解码回原始字节。
///
/// # Errors / 错误
///
/// Returns `ManifestError::InvalidField` if the string is not valid Base64.
/// 如果字符串不是有效的 Base64，返回 `ManifestError::InvalidField`。
pub fn decode_digest(digest: &str) -> Result<Vec<u8>, ManifestError> {
    BASE64_STANDARD
        .decode(digest)
        .map_err(|e| ManifestError::InvalidField(format!("invalid Base64 digest: {e}")))
}

// =============================================================================
// Error type / 错误类型
// =============================================================================

/// Errors that can occur when parsing or generating a MANIFEST.MF.
///
/// 解析或生成 MANIFEST.MF 时可能发生的错误。
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The manifest text is empty.
    /// 清单文本为空。
    #[error("empty manifest")]
    EmptyManifest,

    /// A required field is missing from the manifest.
    /// 清单中缺少必需字段。
    #[error("missing required field: {0}")]
    MissingField(String),

    /// A field has an invalid value.
    /// 字段值无效。
    #[error("invalid field value: {0}")]
    InvalidField(String),

    /// Unknown hash algorithm.
    /// 未知的哈希算法。
    #[error("unknown hash algorithm: {0}")]
    UnknownAlgorithm(String),
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_algorithm_display() {
        // Each algorithm should display its string identifier.
        // 每种算法应显示其字符串标识。
        assert_eq!(HashAlgorithm::Blake3.to_string(), "BLAKE3");
        assert_eq!(HashAlgorithm::Sha256.to_string(), "SHA256");
        assert_eq!(HashAlgorithm::Sha512.to_string(), "SHA512");
    }

    #[test]
    fn test_hash_algorithm_from_str() {
        // Valid strings should parse correctly.
        // 有效字符串应正确解析。
        assert_eq!(HashAlgorithm::from_str("BLAKE3").unwrap(), HashAlgorithm::Blake3);
        assert_eq!(HashAlgorithm::from_str("SHA256").unwrap(), HashAlgorithm::Sha256);
        assert!(HashAlgorithm::from_str("MD5").is_err());
    }

    #[test]
    fn test_digest_header() {
        // digest_header should return the correct header name.
        // digest_header 应返回正确的头名称。
        assert_eq!(HashAlgorithm::Blake3.digest_header(), "BLAKE3-Digest");
        assert_eq!(HashAlgorithm::Sha256.digest_header(), "SHA256-Digest");
    }

    #[test]
    fn test_manifest_to_string() {
        // Serialization should produce valid RFC 822 format.
        // 序列化应产生有效的 RFC 822 格式。
        let mut manifest = Manifest::new("1.0", HashAlgorithm::Blake3);
        manifest.created_by = Some("test-tool 1.0".to_string());
        manifest.add_entry(ManifestEntry {
            name: "metadata/codex.json".to_string(),
            size: 2048,
            digest: "af1349b9f5f9a1a6".to_string(),
        });
        manifest.add_entry(ManifestEntry {
            name: "content/struct.json".to_string(),
            size: 512,
            digest: "7d865e959b246691".to_string(),
        });

        let text = manifest.to_manifest_string();
        assert!(text.contains("Manifest-Version: 1.0"));
        assert!(text.contains("UCX-Version: 1.0"));
        assert!(text.contains("Hash-Algorithm: BLAKE3"));
        assert!(text.contains("Name: metadata/codex.json"));
        assert!(text.contains("Size: 2048"));
        assert!(text.contains("BLAKE3-Digest: af1349b9f5f9a1a6"));
    }

    #[test]
    fn test_manifest_from_str() {
        // Parsing should correctly extract all fields.
        // 解析应正确提取所有字段。
        let text = "Manifest-Version: 1.0\n\
                     UCX-Version: 1.0\n\
                     Created-By: test-tool 1.0\n\
                     Hash-Algorithm: BLAKE3\n\
                     \n\
                     Name: metadata/codex.json\n\
                     Size: 2048\n\
                     BLAKE3-Digest: af1349b9f5f9a1a6\n\
                     \n\
                     Name: content/struct.json\n\
                     Size: 512\n\
                     BLAKE3-Digest: 7d865e959b246691\n";

        let manifest = Manifest::from_manifest_str(text).unwrap();
        assert_eq!(manifest.manifest_version, "1.0");
        assert_eq!(manifest.ucx_version, "1.0");
        assert_eq!(manifest.hash_algorithm, HashAlgorithm::Blake3);
        assert_eq!(manifest.entries.len(), 2);
        assert_eq!(manifest.entries[0].name, "metadata/codex.json");
        assert_eq!(manifest.entries[0].size, 2048);
        assert_eq!(manifest.entries[1].name, "content/struct.json");
    }

    #[test]
    fn test_manifest_round_trip() {
        // to_manifest_string then from_manifest_str should produce equivalent data.
        // to_manifest_string 后 from_manifest_str 应产生等价数据。
        let mut original = Manifest::new("1.0", HashAlgorithm::Blake3);
        original.created_by = Some("round-trip-test".to_string());
        original.add_entry(ManifestEntry {
            name: "metadata/codex.json".to_string(),
            size: 1024,
            digest: "abcdef1234567890".to_string(),
        });

        let text = original.to_manifest_string();
        let parsed = Manifest::from_manifest_str(&text).unwrap();

        assert_eq!(parsed.manifest_version, original.manifest_version);
        assert_eq!(parsed.ucx_version, original.ucx_version);
        assert_eq!(parsed.hash_algorithm, original.hash_algorithm);
        assert_eq!(parsed.entries.len(), original.entries.len());
        assert_eq!(parsed.entries[0].name, original.entries[0].name);
        assert_eq!(parsed.entries[0].size, original.entries[0].size);
        assert_eq!(parsed.entries[0].digest, original.entries[0].digest);
    }

    #[test]
    fn test_manifest_entry_from_bytes() {
        // ManifestEntry::new should Base64-encode the hash bytes.
        // ManifestEntry::new 应将哈希字节编码为 Base64。
        let entry = ManifestEntry::new(
            "test.txt".to_string(),
            100,
            &[0xab, 0xcd, 0xef, 0x01],
        );
        // Base64 of [0xab, 0xcd, 0xef, 0x01] = "q83vAQ=="
        assert_eq!(entry.digest, "q83vAQ==");
    }

    #[test]
    fn test_manifest_missing_version() {
        // Missing Manifest-Version should produce an error.
        // 缺少 Manifest-Version 应产生错误。
        let text = "UCX-Version: 1.0\nHash-Algorithm: BLAKE3\n";
        let result = Manifest::from_manifest_str(text);
        assert!(result.is_err());
    }
}
