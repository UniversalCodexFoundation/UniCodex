//! UCX Parse & Read Module — parse `.ucx` archive files and access content.
//!
//! This module parses `.ucx` archive files (ZIP-based), extracts metadata,
//! and provides access to chapter content. It is the primary module for
//! reader/viewer integration.
//!
//! UCX 解析与读取模块 — 解析 `.ucx` 归档文件并访问内容。
//! 解析基于 ZIP 的 `.ucx` 归档文件，提取元数据，并提供章节内容访问。
//! 这是阅读器/查看器集成的核心模块。

use std::io::Read as IoRead;
use std::path::Path;

use thiserror::Error;
use tracing::{debug, info, warn};
use ucx_types::{Codex, Manifest, Structure};

// =============================================================================
// Constants / 常量
// =============================================================================

/// The expected MIME type for UCX archives.
/// UCX 归档的预期 MIME 类型。
const UCX_MIMETYPE: &str = "application/vnd.unicodex+zip";

// =============================================================================
// Error types / 错误类型
// =============================================================================

/// Errors that can occur during UCX file parsing.
///
/// UCX 文件解析过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum ParseError {
    /// The file is not a valid UCX archive.
    /// 文件不是有效的 UCX 归档。
    #[error("not a valid UCX file: {0}")]
    InvalidFormat(String),

    /// Required file is missing from the archive.
    /// 归档中缺少必需文件。
    #[error("required file missing: {0}")]
    MissingFile(String),

    /// Failed to parse metadata JSON.
    /// 解析元数据 JSON 失败。
    #[error("metadata parse error: {0}")]
    MetadataParse(String),

    /// MANIFEST.MF parsing error.
    /// MANIFEST.MF 解析错误。
    #[error("manifest parse error: {0}")]
    ManifestParse(String),

    /// I/O error during parsing.
    /// 解析过程中的 I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// ZIP archive reading error.
    /// ZIP 归档读取错误。
    #[error("ZIP archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// Content encoding error (e.g., invalid UTF-8).
    /// 内容编码错误（如无效的 UTF-8）。
    #[error("content encoding error: {0}")]
    Encoding(String),
}

// =============================================================================
// HashVerifyResult / 哈希验证结果
// =============================================================================

/// Result of verifying a single file's hash against the manifest.
///
/// 单个文件的哈希验证结果。
#[derive(Debug, Clone)]
pub struct HashVerifyResult {
    /// File name (path within the archive).
    /// 文件名（归档内路径）。
    pub name: String,

    /// Expected hash digest from the manifest.
    /// 清单中的预期哈希摘要。
    pub expected: String,

    /// Actual hash digest computed from the file content.
    /// 从文件内容计算出的实际哈希摘要。
    pub actual: String,

    /// Whether the expected and actual hashes match.
    /// 预期和实际哈希是否匹配。
    pub valid: bool,
}

// =============================================================================
// UcxArchive / 已解析的 UCX 归档
// =============================================================================

/// A parsed UCX archive, providing access to metadata and content.
///
/// Holds a ZIP reader internally and caches parsed metadata (Codex,
/// Structure, Manifest) for efficient repeated access.
///
/// 一个已解析的 UCX 归档，提供元数据和内容的访问接口。
/// 内部持有 ZIP reader，并缓存已解析的元数据（Codex、Structure、Manifest）。
pub struct UcxArchive {
    /// The underlying ZIP archive reader (does not implement Debug).
    /// 底层 ZIP 归档读取器（未实现 Debug）。
    archive: zip::ZipArchive<std::io::BufReader<std::fs::File>>,

    /// Original file path on disk.
    /// 磁盘上的原始文件路径。
    file_path: std::path::PathBuf,

    /// Cached codex metadata (from `metadata/codex.json`).
    /// 缓存的作品元数据（来自 `metadata/codex.json`）。
    codex: Codex,

    /// Cached content structure (from `content/struct.json`).
    /// 缓存的内容结构（来自 `content/struct.json`）。
    structure: Structure,

    /// Cached resource manifest (from `META-INF/MANIFEST.MF`).
    /// 缓存的资源清单（来自 `META-INF/MANIFEST.MF`）。
    manifest: Manifest,
}

/// Manual `Debug` implementation because `ZipArchive` does not derive `Debug`.
///
/// 手动实现 `Debug`，因为 `ZipArchive` 未派生 `Debug`。
impl std::fmt::Debug for UcxArchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UcxArchive")
            .field("file_path", &self.file_path)
            .field("codex", &self.codex)
            .field("structure", &self.structure)
            .field("manifest", &self.manifest)
            .field("archive", &format_args!("<ZipArchive>"))
            .finish()
    }
}

// =============================================================================
// Public API — top-level function / 公开接口 — 顶层函数
// =============================================================================

/// Open and parse a UCX file.
///
/// Reads the ZIP archive, validates the mimetype, and parses all required
/// metadata files (MANIFEST.MF, codex.json, struct.json).
///
/// 打开并解析 UCX 文件。
/// 读取 ZIP 归档，验证 mimetype，并解析所有必需的元数据文件。
///
/// # Arguments / 参数
///
/// * `path` — Path to the `.ucx` file. / `.ucx` 文件路径。
///
/// # Errors / 错误
///
/// Returns `ParseError` if the file cannot be opened, is not a valid UCX
/// archive, or is missing required metadata files.
pub fn open(path: &Path) -> Result<UcxArchive, ParseError> {
    info!("Opening UCX file: {}", path.display());

    // Step 1: Open the file and create a BufReader.
    // 步骤 1：打开文件并创建 BufReader。
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);

    // Step 2: Create ZipArchive from the buffered reader.
    // 步骤 2：从 BufReader 创建 ZipArchive。
    let mut archive = zip::ZipArchive::new(reader)?;
    debug!("ZIP archive opened, {} entries found", archive.len());

    // Step 3: Validate mimetype — must be the first entry.
    // 步骤 3：验证 mimetype — 必须是第一个条目。
    validate_mimetype(&mut archive)?;

    // Step 4: Parse MANIFEST.MF.
    // 步骤 4：解析 MANIFEST.MF。
    let manifest = parse_manifest(&mut archive)?;
    debug!("MANIFEST.MF parsed, {} entries", manifest.entries.len());

    // Step 5: Parse codex.json.
    // 步骤 5：解析 codex.json。
    let codex = parse_codex(&mut archive)?;
    debug!("codex.json parsed: \"{}\"", codex.title.main);

    // Step 6: Parse struct.json.
    // 步骤 6：解析 struct.json。
    let structure = parse_structure(&mut archive)?;
    debug!(
        "struct.json parsed, {} top-level nodes",
        structure.structure.len()
    );

    info!("UCX file parsed successfully: \"{}\"", codex.title.main);

    Ok(UcxArchive {
        archive,
        file_path: path.to_path_buf(),
        codex,
        structure,
        manifest,
    })
}

// =============================================================================
// UcxArchive methods / UcxArchive 方法
// =============================================================================

impl UcxArchive {
    /// Get a reference to the work's codex metadata.
    ///
    /// 获取作品的 codex 元数据引用。
    pub fn codex(&self) -> &Codex {
        &self.codex
    }

    /// Get a reference to the content structure tree.
    ///
    /// 获取内容结构树的引用。
    pub fn structure(&self) -> &Structure {
        &self.structure
    }

    /// Get a reference to the resource manifest.
    ///
    /// 获取资源清单的引用。
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Get the file size in bytes of the UCX archive on disk.
    ///
    /// 获取磁盘上 UCX 归档文件的大小（字节）。
    pub fn file_size(&self) -> u64 {
        std::fs::metadata(&self.file_path)
            .map(|m| m.len())
            .unwrap_or(0)
    }

    /// Count the total number of leaf chapters (files) in the structure tree.
    ///
    /// Recursively traverses the structure, counting only nodes with a `file` field.
    ///
    /// 统计结构树中叶子章节（文件）的总数。
    /// 递归遍历结构，仅计算有 `file` 字段的节点。
    pub fn chapter_count(&self) -> usize {
        fn count_leaves(nodes: &[ucx_types::StructureNode]) -> usize {
            let mut count = 0;
            for node in nodes {
                if node.file.is_some() {
                    count += 1;
                }
                if let Some(ref children) = node.children {
                    count += count_leaves(children);
                }
            }
            count
        }
        count_leaves(&self.structure.structure)
    }

    /// Read the text content of a specific chapter file.
    ///
    /// The `file` parameter is the file path as listed in `struct.json`
    /// (e.g., `"chapter-001.md"`). The actual ZIP entry is `content/{file}`.
    ///
    /// 读取指定章节文件的文本内容。
    /// `file` 参数是 `struct.json` 中列出的文件路径（如 `"chapter-001.md"`）。
    /// 实际 ZIP 条目为 `content/{file}`。
    ///
    /// # Errors / 错误
    ///
    /// - `MissingFile` if the chapter entry does not exist.
    /// - `Encoding` if the content is not valid UTF-8.
    pub fn read_chapter(&mut self, file: &str) -> Result<String, ParseError> {
        // Build the full path within the ZIP archive.
        // 构建 ZIP 归档内的完整路径。
        let entry_path = format!("content/{file}");
        debug!("Reading chapter: {}", entry_path);

        // Read the entry content as bytes.
        // 读取条目内容为字节。
        let bytes = read_entry_bytes(&mut self.archive, &entry_path)?;

        // Decode as UTF-8 string.
        // 解码为 UTF-8 字符串。
        String::from_utf8(bytes).map_err(|e| {
            ParseError::Encoding(format!(
                "chapter '{entry_path}' is not valid UTF-8: {e}"
            ))
        })
    }

    /// Verify the hashes of all files listed in the manifest.
    ///
    /// For each entry in the manifest, reads the file from the archive,
    /// computes its BLAKE3 hash, encodes it as Base64, and compares it
    /// to the expected digest stored in MANIFEST.MF.
    ///
    /// 验证清单中列出的所有文件的哈希值。
    /// 对清单中的每个条目，从归档中读取文件，计算 BLAKE3 哈希，
    /// 编码为 Base64，并与 MANIFEST.MF 中存储的预期摘要对比。
    pub fn verify_hashes(&mut self) -> Result<Vec<HashVerifyResult>, ParseError> {
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

        let mut results = Vec::with_capacity(self.manifest.entries.len());

        // Clone entries to avoid borrowing issues (manifest is borrowed immutably
        // while archive needs mutable borrow for reading).
        // 克隆条目以避免借用冲突（manifest 不可变借用的同时 archive 需要可变借用读取）。
        let entries: Vec<_> = self.manifest.entries.clone();

        for entry in &entries {
            // Read file content from archive.
            // 从归档中读取文件内容。
            let bytes = read_entry_bytes(&mut self.archive, &entry.name)?;

            // Compute BLAKE3 hash and encode as Base64 (matching MANIFEST.MF format).
            // 计算 BLAKE3 哈希并编码为 Base64（匹配 MANIFEST.MF 格式）。
            let actual_hash = blake3::hash(&bytes);
            let actual_b64 = BASE64_STANDARD.encode(actual_hash.as_bytes());

            let valid = actual_b64 == entry.digest;
            if !valid {
                warn!(
                    "Hash mismatch for '{}': expected={}, actual={}",
                    entry.name, entry.digest, actual_b64
                );
            }

            results.push(HashVerifyResult {
                name: entry.name.clone(),
                expected: entry.digest.clone(),
                actual: actual_b64,
                valid,
            });
        }

        Ok(results)
    }

    /// List all file paths in the archive.
    ///
    /// Returns the name of every entry in the ZIP archive.
    ///
    /// 列出归档中的所有文件路径。
    /// 返回 ZIP 归档中每个条目的名称。
    pub fn list_files(&self) -> Vec<String> {
        self.archive
            .file_names()
            .map(|name| name.to_string())
            .collect()
    }
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Validate that the first entry in the ZIP is `mimetype` with the correct value.
///
/// 验证 ZIP 中第一个条目是 `mimetype` 且值正确。
fn validate_mimetype(
    archive: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>,
) -> Result<(), ParseError> {
    // The first entry (index 0) must be named "mimetype".
    // 第一个条目（索引 0）必须命名为 "mimetype"。
    let first_name = archive.name_for_index(0).ok_or_else(|| {
        ParseError::InvalidFormat("archive is empty, no mimetype entry".to_string())
    })?;

    if first_name != "mimetype" {
        return Err(ParseError::InvalidFormat(format!(
            "first entry must be 'mimetype', found '{first_name}'"
        )));
    }

    // Read the mimetype content and validate.
    // 读取 mimetype 内容并验证。
    let content_bytes = read_entry_bytes(archive, "mimetype")?;
    let content = String::from_utf8(content_bytes)
        .map_err(|e| ParseError::InvalidFormat(format!("mimetype is not UTF-8: {e}")))?;

    // Trim trailing whitespace/newlines for comparison.
    // 修剪尾部空白/换行用于比较。
    let trimmed = content.trim();
    if trimmed != UCX_MIMETYPE {
        return Err(ParseError::InvalidFormat(format!(
            "mimetype mismatch: expected '{UCX_MIMETYPE}', found '{trimmed}'"
        )));
    }

    debug!("mimetype validated: {UCX_MIMETYPE}");
    Ok(())
}

/// Parse the `META-INF/MANIFEST.MF` entry from the archive.
///
/// 从归档中解析 `META-INF/MANIFEST.MF` 条目。
fn parse_manifest(
    archive: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>,
) -> Result<Manifest, ParseError> {
    let bytes = read_entry_bytes(archive, "META-INF/MANIFEST.MF").map_err(|e| match e {
        ParseError::MissingFile(_) => ParseError::MissingFile("META-INF/MANIFEST.MF".to_string()),
        other => other,
    })?;

    let text = String::from_utf8(bytes).map_err(|e| {
        ParseError::ManifestParse(format!("MANIFEST.MF is not valid UTF-8: {e}"))
    })?;

    Manifest::from_manifest_str(&text)
        .map_err(|e| ParseError::ManifestParse(format!("{e}")))
}

/// Parse the `metadata/codex.json` entry from the archive.
///
/// 从归档中解析 `metadata/codex.json` 条目。
fn parse_codex(
    archive: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>,
) -> Result<Codex, ParseError> {
    let bytes = read_entry_bytes(archive, "metadata/codex.json").map_err(|e| match e {
        ParseError::MissingFile(_) => ParseError::MissingFile("metadata/codex.json".to_string()),
        other => other,
    })?;

    let text = String::from_utf8(bytes).map_err(|e| {
        ParseError::MetadataParse(format!("codex.json is not valid UTF-8: {e}"))
    })?;

    serde_json::from_str::<Codex>(&text)
        .map_err(|e| ParseError::MetadataParse(format!("codex.json: {e}")))
}

/// Parse the `content/struct.json` entry from the archive.
///
/// 从归档中解析 `content/struct.json` 条目。
fn parse_structure(
    archive: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>,
) -> Result<Structure, ParseError> {
    let bytes = read_entry_bytes(archive, "content/struct.json").map_err(|e| match e {
        ParseError::MissingFile(_) => ParseError::MissingFile("content/struct.json".to_string()),
        other => other,
    })?;

    let text = String::from_utf8(bytes).map_err(|e| {
        ParseError::MetadataParse(format!("struct.json is not valid UTF-8: {e}"))
    })?;

    serde_json::from_str::<Structure>(&text)
        .map_err(|e| ParseError::MetadataParse(format!("struct.json: {e}")))
}

/// Read the raw bytes of a ZIP entry by name.
///
/// Returns `ParseError::MissingFile` if the entry does not exist.
///
/// 按名称读取 ZIP 条目的原始字节。
/// 如果条目不存在，返回 `ParseError::MissingFile`。
fn read_entry_bytes(
    archive: &mut zip::ZipArchive<std::io::BufReader<std::fs::File>>,
    name: &str,
) -> Result<Vec<u8>, ParseError> {
    let mut entry = archive.by_name(name).map_err(|e| match e {
        zip::result::ZipError::FileNotFound => {
            ParseError::MissingFile(name.to_string())
        }
        other => ParseError::Zip(other),
    })?;

    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests;
