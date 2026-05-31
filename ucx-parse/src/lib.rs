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

/// Expected ZIP local file header signature (PK\x03\x04) at offset 0.
///
/// A valid `.ucx` archive must start with the ZIP Local File Header magic,
/// otherwise the file may be a concatenation of multiple ZIPs (e.g., via
/// `cat a.ucx a.ucx > doubled.ucx`) where the embedded ZIP could be parsed
/// only by relying on the trailing EOCD record. Enforcing this signature
/// at offset 0 rejects such forged/trailing payloads.
///
/// 合法 `.ucx` 归档开头必须是 ZIP 本地文件头魔数（PK\x03\x04）。
/// 否则文件可能是多个 ZIP 拼接而成（例如 `cat a.ucx a.ucx > doubled.ucx`），
/// 仅通过尾部 EOCD 记录仍可被解析。在 offset 0 强制校验此签名可拒绝此类伪造/尾缀载荷。
const ZIP_LOCAL_FILE_HEADER_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x03, 0x04];

/// The highest UCX-Version MAJOR number this library understands.
///
/// When parsing MANIFEST.MF, an archive whose `UCX-Version` MAJOR exceeds
/// this constant is rejected to avoid silently reading forward-incompatible
/// formats. Bump this constant whenever the on-disk spec introduces a
/// breaking MAJOR bump.
///
/// 本库支持的最高 UCX-Version MAJOR 号。
/// 解析 MANIFEST.MF 时，若归档 `UCX-Version` 的 MAJOR 高于此常量，
/// 将被拒绝，以避免静默读取向前不兼容的格式。
/// 每当磁盘格式规范出现破坏性 MAJOR 升级时，请同步调整此常量。
pub const SUPPORTED_UCX_MAJOR: u32 = 1;

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

    /// Path traversal attack detected (e.g., entry contains `..` or starts with `/`).
    /// 检测到路径遍历攻击（如条目包含 `..` 或以 `/` 开头）。
    #[error("path traversal detected: {0}")]
    PathTraversal(String),

    /// The file is encrypted (UCXE format) and cannot be read as plaintext.
    /// 文件已加密（UCXE 格式），无法作为明文读取。
    #[error("file is encrypted: {0}")]
    Encrypted(String),

    /// The archive's UCX-Version MAJOR is higher than this library supports.
    /// 归档的 UCX-Version MAJOR 超出本库支持范围。
    #[error("unsupported UCX-Version: {0}")]
    UnsupportedVersion(String),
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

    // Step 1: Validate ZIP local file header signature at offset 0.
    // This guards against concatenated/prefixed ZIP files that the
    // underlying `zip` crate would otherwise accept by parsing the
    // trailing EOCD record.
    // 步骤 1：校验 offset 0 处的 ZIP 本地文件头签名。
    // 这可防止底层 `zip` crate 通过解析尾部 EOCD 而误接受
    // 被拼接或带前缀的 ZIP 文件。
    validate_zip_signature(path)?;

    // Step 2: Open the file and create a BufReader.
    // 步骤 2：打开文件并创建 BufReader。
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);

    // Step 3: Create ZipArchive from the buffered reader.
    // 步骤 3：从 BufReader 创建 ZipArchive。
    let mut archive = zip::ZipArchive::new(reader)?;
    debug!("ZIP archive opened, {} entries found", archive.len());

    // Step 4: Validate mimetype — must be the first entry.
    // 步骤 4：验证 mimetype — 必须是第一个条目。
    validate_mimetype(&mut archive)?;

    // Step 5: Parse MANIFEST.MF.
    // 步骤 5：解析 MANIFEST.MF。
    let manifest = parse_manifest(&mut archive)?;
    debug!("MANIFEST.MF parsed, {} entries", manifest.entries.len());

    // Step 6: Enforce UCX-Version MAJOR compatibility from the manifest.
    // Archives whose MAJOR exceeds `SUPPORTED_UCX_MAJOR` are forward-incompatible
    // and must be rejected rather than silently read.
    // 步骤 6：根据 manifest 强制校验 UCX-Version MAJOR 兼容性。
    // 若 MAJOR 超过 `SUPPORTED_UCX_MAJOR`，该归档向前不兼容，必须拒绝而非静默读取。
    validate_ucx_version(&manifest.ucx_version)?;

    // Step 7: Parse codex.json.
    // 步骤 7：解析 codex.json。
    let codex = parse_codex(&mut archive)?;
    debug!("codex.json parsed: \"{}\"", codex.title.main);

    // Step 8: Parse struct.json.
    // 步骤 8：解析 struct.json。
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
    /// If the file is encrypted (starts with UCXE magic), returns
    /// `ParseError::Encrypted` instead of attempting UTF-8 decode.
    ///
    /// 读取指定章节文件的文本内容。
    /// `file` 参数是 `struct.json` 中列出的文件路径（如 `"chapter-001.md"`）。
    /// 实际 ZIP 条目为 `content/{file}`。
    ///
    /// 如果文件已加密（以 UCXE 魔数开头），返回 `ParseError::Encrypted`
    /// 而非尝试 UTF-8 解码。
    ///
    /// # Errors / 错误
    ///
    /// - `MissingFile` if the chapter entry does not exist.
    /// - `Encrypted` if the content starts with the UCXE magic number.
    /// - `Encoding` if the content is not valid UTF-8.
    pub fn read_chapter(&mut self, file: &str) -> Result<String, ParseError> {
        // Build the full path within the ZIP archive.
        // 构建 ZIP 归档内的完整路径。
        let entry_path = format!("content/{file}");
        debug!("Reading chapter: {}", entry_path);

        // Read the entry content as bytes.
        // 读取条目内容为字节。
        let bytes = read_entry_bytes(&mut self.archive, &entry_path)?;

        // Check for UCXE encrypted format (magic: "UCXE").
        // 检查 UCXE 加密格式（魔数："UCXE"）。
        if ucx_crypto::is_encrypted(&bytes) {
            return Err(ParseError::Encrypted(format!(
                "chapter '{entry_path}' is encrypted (UCXE format) — decrypt before reading"
            )));
        }

        // Decode as UTF-8 string.
        // 解码为 UTF-8 字符串。
        String::from_utf8(bytes).map_err(|e| {
            ParseError::Encoding(format!("chapter '{entry_path}' is not valid UTF-8: {e}"))
        })
    }

    /// Read the raw bytes of a specific chapter file (encrypted or not).
    ///
    /// Unlike `read_chapter`, this does not check for UCXE magic or attempt
    /// UTF-8 decoding. Useful for encrypted files that need external decryption.
    ///
    /// 读取指定章节文件的原始字节（无论是否加密）。
    /// 与 `read_chapter` 不同，此方法不检查 UCXE 魔数，也不尝试 UTF-8 解码。
    /// 适用于需要外部解密的加密文件。
    pub fn read_chapter_raw(&mut self, file: &str) -> Result<Vec<u8>, ParseError> {
        let entry_path = format!("content/{file}");
        debug!("Reading chapter raw bytes: {}", entry_path);
        read_entry_bytes(&mut self.archive, &entry_path)
    }

    /// Check if a chapter file is encrypted by examining its first bytes.
    ///
    /// Reads the file content and checks for the UCXE magic number (0x55 0x43 0x58 0x45).
    ///
    /// 通过检查文件首字节判断章节是否已加密。
    /// 读取文件内容并检查 UCXE 魔数（0x55 0x43 0x58 0x45）。
    pub fn is_chapter_encrypted(&mut self, file: &str) -> Result<bool, ParseError> {
        let entry_path = format!("content/{file}");
        let bytes = read_entry_bytes(&mut self.archive, &entry_path)?;
        Ok(ucx_crypto::is_encrypted(&bytes))
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
        use subtle::ConstantTimeEq;

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

            // Security: use constant-time comparison to prevent timing side-channel
            // attacks on digest verification.
            // 安全：使用常量时间比较防止摘要验证的时序侧信道攻击。
            let valid: bool = actual_b64.as_bytes().ct_eq(entry.digest.as_bytes()).into();
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

    /// Extract all files from the UCX archive to the given output directory.
    ///
    /// Recreates the archive's internal directory structure under `output_dir`.
    /// Files extracted: mimetype, META-INF/MANIFEST.MF, metadata/codex.json,
    /// content/struct.json, and all content/chapter files.
    ///
    /// Returns the list of extracted file paths (relative to `output_dir`).
    ///
    /// 将 UCX 归档中的所有文件解压到指定的输出目录。
    /// 在 `output_dir` 下重建归档的内部目录结构。
    /// 提取的文件：mimetype、META-INF/MANIFEST.MF、metadata/codex.json、
    /// content/struct.json 以及所有 content/chapter 文件。
    ///
    /// 返回已提取文件路径的列表（相对于 `output_dir`）。
    ///
    /// # Arguments / 参数
    ///
    /// * `output_dir` — Target directory to extract into. / 提取目标目录。
    ///
    /// # Errors / 错误
    ///
    /// - `PathTraversal` if any entry name is not a safe relative path — i.e. it
    ///   is absolute (incl. a Windows drive letter like `C:/` or `C:\`), contains
    ///   a backslash, a `..` segment, a control character, a Windows reserved
    ///   device name, or a trailing dot/space (security: prevents Zip-Slip
    ///   arbitrary file write). Validated via the shared
    ///   [`ucx_types::path_safety::validate_safe_relative_path`].
    /// - `Io` for filesystem errors (creating directories, writing files).
    /// - `Zip` for archive reading errors.
    pub fn extract_to(&mut self, output_dir: &Path) -> Result<Vec<String>, ParseError> {
        info!("Extracting UCX archive to: {}", output_dir.display());

        // Ensure the output directory exists.
        // 确保输出目录存在。
        std::fs::create_dir_all(output_dir)?;

        let mut extracted_files = Vec::new();

        // Iterate over all entries in the archive by index.
        // 按索引遍历归档中的所有条目。
        for i in 0..self.archive.len() {
            let mut entry = self.archive.by_index(i)?;
            let entry_name = entry.name().to_string();

            // Skip directory entries (names ending with '/').
            // 跳过目录条目（名称以 '/' 结尾）。
            if entry_name.ends_with('/') {
                debug!("Skipping directory entry: {}", entry_name);
                continue;
            }

            // Security (Zip-Slip / arbitrary file write): validate the entry
            // name with the SAME shared validator the producer uses, before
            // joining it to `output_dir`. The previous guard only checked for
            // ".." and a leading "/", which let Windows drive-absolute names
            // (`C:/Windows/Temp/x`, `C:\...`) and backslash-absolute names slip
            // through — and `output_dir.join(absolute)` DISCARDS `output_dir`,
            // landing the attacker's file at an arbitrary absolute location
            // while `unpack` reported success. The shared validator rejects
            // absolute paths (incl. drive letters), backslashes, `..` segments,
            // control characters, reserved names, and trailing dot/space, so any
            // accepted entry is a strict relative path that stays under
            // `output_dir`.
            //
            // 安全（Zip-Slip / 任意文件写）：在 join 到 `output_dir` 之前，用与
            // 生产侧**相同**的共享校验器校验条目名。旧守卫只查 ".." 与以 "/" 开头，
            // 放过了 Windows 盘符绝对名（`C:/Windows/Temp/x`、`C:\...`）与反斜杠
            // 绝对名——而 `output_dir.join(绝对路径)` 会**丢弃** `output_dir`，使
            // 攻击者文件落到任意绝对位置，且 `unpack` 仍报成功。共享校验器拒绝
            // 绝对路径（含盘符）、反斜杠、`..` 段、控制字符、保留名与尾随点/空格，
            // 因此任何被接受的条目都是严格相对路径、必落在 `output_dir` 之下。
            if let Err(e) = ucx_types::path_safety::validate_safe_relative_path(&entry_name) {
                return Err(ParseError::PathTraversal(format!(
                    "unsafe entry name rejected: {e}"
                )));
            }

            // Build the output file path.
            // 构建输出文件路径。
            let out_path = output_dir.join(&entry_name);

            // Create parent directories if needed.
            // 如有需要，创建父目录。
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Read entry content (bounded; decompression-bomb guard) and write to disk.
            // 读取条目内容（有界；解压炸弹防护）并写入磁盘。
            let declared = entry.size();
            let buf = read_entry_capped(&mut entry, declared, &entry_name)?;
            std::fs::write(&out_path, &buf)?;

            debug!("Extracted: {}", entry_name);
            extracted_files.push(entry_name);
        }

        info!(
            "Extraction complete: {} files extracted to {}",
            extracted_files.len(),
            output_dir.display()
        );

        Ok(extracted_files)
    }
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Validate that the file begins with the ZIP Local File Header magic bytes.
///
/// Reads only the first 4 bytes of the file and compares them with
/// `ZIP_LOCAL_FILE_HEADER_SIGNATURE` (`PK\x03\x04`). Files that start with
/// any other prefix — including ZIPs concatenated after arbitrary payloads —
/// are rejected with a clear error message.
///
/// 校验文件开头必须为 ZIP 本地文件头魔数。
/// 仅读取文件前 4 字节并与 `ZIP_LOCAL_FILE_HEADER_SIGNATURE`（`PK\x03\x04`）对比。
/// 以其他字节开头的文件（包括拼接在任意载荷之后的 ZIP）会被明确拒绝。
fn validate_zip_signature(path: &Path) -> Result<(), ParseError> {
    use std::io::Read as _;

    // Open the file and attempt to read exactly 4 bytes at offset 0.
    // 打开文件，尝试从 offset 0 精确读取 4 字节。
    let mut file = std::fs::File::open(path)?;
    let mut magic = [0u8; 4];

    // If the file is shorter than 4 bytes, read_exact returns UnexpectedEof.
    // We translate that into an InvalidFormat error so callers get a clean
    // message rather than a generic IO error.
    // 若文件不足 4 字节，read_exact 会返回 UnexpectedEof。
    // 在此转换为 InvalidFormat，调用方得到干净的提示而非通用 IO 错误。
    if let Err(e) = file.read_exact(&mut magic) {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            return Err(ParseError::InvalidFormat(
                "file is too short to be a ZIP archive (<4 bytes)".to_string(),
            ));
        }
        return Err(ParseError::Io(e));
    }

    // Compare the first 4 bytes against the expected PK local file header.
    // 将前 4 字节与预期 PK 本地文件头对比。
    if magic != ZIP_LOCAL_FILE_HEADER_SIGNATURE {
        return Err(ParseError::InvalidFormat(format!(
            "invalid UCX file: expected ZIP signature at offset 0, got {:02X} {:02X} {:02X} {:02X}",
            magic[0], magic[1], magic[2], magic[3]
        )));
    }

    debug!("ZIP signature validated at offset 0");
    Ok(())
}

/// Validate that a manifest `UCX-Version` string is compatible with this library.
///
/// The version string is expected to be of the form `MAJOR.MINOR[.PATCH...]`
/// where MAJOR is a non-negative integer. Only the MAJOR component is checked;
/// archives whose MAJOR exceeds `SUPPORTED_UCX_MAJOR` are rejected.
/// A malformed or non-numeric MAJOR segment is also rejected.
///
/// 校验 manifest 中 `UCX-Version` 字符串与本库的兼容性。
/// 预期版本字符串形如 `MAJOR.MINOR[.PATCH...]`，其中 MAJOR 为非负整数。
/// 仅检查 MAJOR 分量；若 MAJOR 超过 `SUPPORTED_UCX_MAJOR`，拒绝此归档。
/// 若 MAJOR 缺失或非数字也一并拒绝。
fn validate_ucx_version(ucx_version: &str) -> Result<(), ParseError> {
    // Take the substring before the first '.' as MAJOR (or the whole string if no '.').
    // 取第一个 '.' 之前的子串作为 MAJOR（若无 '.' 则取整个字符串）。
    let major_str = ucx_version.split('.').next().unwrap_or("");

    // Parse MAJOR as u32. Reject empty / non-numeric strings with a clear error.
    // 将 MAJOR 解析为 u32。空串或非数字将被拒绝并给出明确错误。
    let major: u32 = major_str.parse().map_err(|_| {
        ParseError::UnsupportedVersion(format!(
            "UCX-Version MAJOR is not a non-negative integer: '{ucx_version}'"
        ))
    })?;

    // Forward-incompatible archives: MAJOR greater than what this library supports.
    // 向前不兼容归档：MAJOR 高于本库所支持。
    if major > SUPPORTED_UCX_MAJOR {
        return Err(ParseError::UnsupportedVersion(format!(
            "UCX-Version MAJOR {major} exceeds supported MAJOR {SUPPORTED_UCX_MAJOR} (archive version '{ucx_version}')"
        )));
    }

    debug!("UCX-Version '{ucx_version}' is compatible (MAJOR={major})");
    Ok(())
}

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

    let text = String::from_utf8(bytes)
        .map_err(|e| ParseError::ManifestParse(format!("MANIFEST.MF is not valid UTF-8: {e}")))?;

    Manifest::from_manifest_str(&text).map_err(|e| ParseError::ManifestParse(format!("{e}")))
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

    let text = String::from_utf8(bytes)
        .map_err(|e| ParseError::MetadataParse(format!("codex.json is not valid UTF-8: {e}")))?;

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

    let text = String::from_utf8(bytes)
        .map_err(|e| ParseError::MetadataParse(format!("struct.json is not valid UTF-8: {e}")))?;

    serde_json::from_str::<Structure>(&text)
        .map_err(|e| ParseError::MetadataParse(format!("struct.json: {e}")))
}

/// Maximum number of bytes any single ZIP entry may decompress to.
///
/// Decompression-bomb guard: a few-KB DEFLATE entry can declare (and expand to)
/// many GiB. `ZipFile::size()` returns the **attacker-controlled** declared
/// uncompressed size from the central directory, and the underlying reader puts
/// no bound on its decompressed output — so reading an entry unbounded (or
/// pre-allocating `Vec::with_capacity(entry.size())`) is an OOM/DoS vector on the
/// untrusted-parse path. 512 MiB is far above any legitimate chapter/asset for a
/// reader yet caps the worst case to a single bounded allocation.
///
/// 任何单个 ZIP 条目可解压到的最大字节数。
/// 解压炸弹防护：数 KB 的 DEFLATE 条目可声明（并膨胀到）数 GiB。`ZipFile::size()`
/// 返回中央目录里**攻击者可控**的声明解压大小，且底层 reader 对解压输出无上界——
/// 因此无界读取条目（或 `Vec::with_capacity(entry.size())` 预分配）是不可信解析
/// 路径上的 OOM/DoS 向量。512 MiB 远高于阅读器任何合法章节/资源，又将最坏情况
/// 限制为单次有界分配。
const MAX_ENTRY_BYTES: u64 = 512 * 1024 * 1024;

/// Read a ZIP entry's decompressed bytes with a hard upper bound, defusing
/// decompression bombs. The declared size is NOT trusted for allocation; the
/// buffer grows from a small initial capacity and the read is hard-limited to
/// `MAX_ENTRY_BYTES + 1` so an over-large entry is detected without decompressing
/// the whole bomb.
///
/// 以硬上界读取 ZIP 条目的解压字节，化解解压炸弹。分配**不信任**声明大小；缓冲区
/// 从较小初始容量增长，读取被硬限制到 `MAX_ENTRY_BYTES + 1`，使超大条目无需解压
/// 整个炸弹即可被检出。
fn read_entry_capped<R: std::io::Read>(
    reader: &mut R,
    declared_size: u64,
    entry_name: &str,
) -> Result<Vec<u8>, ParseError> {
    // Cap the initial capacity (declared_size is attacker-controlled): start
    // small and let the Vec grow only as bytes actually arrive.
    // 限制初始容量（declared_size 攻击者可控）：从小处起，仅随实际到达的字节增长。
    let initial = declared_size.min(64 * 1024) as usize;
    let mut buf = Vec::with_capacity(initial);
    // Read at most MAX_ENTRY_BYTES + 1 bytes; if we hit the +1 the entry exceeds
    // the cap and is treated as a bomb.
    // 至多读取 MAX_ENTRY_BYTES + 1 字节；若读到 +1 则条目超限，视为炸弹。
    let read = reader.take(MAX_ENTRY_BYTES + 1).read_to_end(&mut buf)? as u64;
    if read > MAX_ENTRY_BYTES {
        return Err(ParseError::InvalidFormat(format!(
            "ZIP entry '{entry_name}' decompresses beyond the {MAX_ENTRY_BYTES}-byte limit \
             (possible decompression bomb) / ZIP 条目 '{entry_name}' 解压超过 \
             {MAX_ENTRY_BYTES} 字节上限（可能为解压炸弹）"
        )));
    }
    Ok(buf)
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
        zip::result::ZipError::FileNotFound => ParseError::MissingFile(name.to_string()),
        other => ParseError::Zip(other),
    })?;

    // Bounded read (decompression-bomb guard); do not trust entry.size().
    // 有界读取（解压炸弹防护）；不信任 entry.size()。
    let declared = entry.size();
    read_entry_capped(&mut entry, declared, name)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests;
