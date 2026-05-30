//! ZIP archive creation logic for UCX files.
//!
//! Handles the creation of `.ucx` ZIP archives with correct entry ordering,
//! compression strategies, and MANIFEST.MF generation.
//!
//! UCX 文件的 ZIP 归档创建逻辑。
//! 处理 `.ucx` ZIP 归档的创建，包括正确的条目顺序、压缩策略和 MANIFEST.MF 生成。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tracing::debug;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::FileOptions;

use ucx_types::{Codex, HashAlgorithm, Manifest, ManifestEntry, Structure};

use crate::BuildError;

// =============================================================================
// Constants / 常量
// =============================================================================

/// The MIME type for UCX archives, written as the first entry in the ZIP.
/// UCX 归档的 MIME 类型，作为 ZIP 中的第一个条目写入。
const UCX_MIMETYPE: &[u8] = b"application/vnd.unicodex+zip";

// =============================================================================
// CollectedFile — a file to be packed into the archive / 待打包文件
// =============================================================================

/// A file collected from the project directory, ready to be packed.
///
/// 从项目目录收集的文件，准备打包。
#[derive(Debug)]
pub struct CollectedFile {
    /// Path inside the ZIP archive (e.g., "content/chapter-001.md").
    /// ZIP 归档内的路径（如 "content/chapter-001.md"）。
    pub archive_path: String,

    /// Absolute path on disk.
    /// 磁盘上的绝对路径。
    pub disk_path: PathBuf,
}

// =============================================================================
// collect_project_files — gather files from content/ and assets/ directories
// =============================================================================

/// Collect all files from `content/` and `assets/` directories.
///
/// Walks the `content/` directory (required) and `assets/` directory (optional)
/// recursively, collecting all regular files.
///
/// 从 `content/` 和 `assets/` 目录收集所有文件。
/// 递归遍历 `content/`（必需）和 `assets/`（可选）目录，收集所有常规文件。
///
/// # Arguments / 参数
///
/// * `project_path` - Root of the project directory. / 项目根目录。
///
/// # Returns / 返回
///
/// A list of `CollectedFile` entries with archive-relative paths using forward slashes.
/// `CollectedFile` 条目列表，归档路径使用正斜杠。
pub fn collect_project_files(project_path: &Path) -> Result<Vec<CollectedFile>, BuildError> {
    let mut files = Vec::new();

    // --- Collect from content/ directory (required). ---
    // --- 从 content/ 目录收集（必需）。 ---
    let content_dir = project_path.join("content");
    if content_dir.is_dir() {
        collect_dir_recursive(&content_dir, "content", &mut files)?;
    }

    // --- Collect from assets/ directory (optional). ---
    // --- 从 assets/ 目录收集（可选）。 ---
    let assets_dir = project_path.join("assets");
    if assets_dir.is_dir() {
        collect_dir_recursive(&assets_dir, "assets", &mut files)?;
    }

    Ok(files)
}

/// Recursively collect files from a directory, building archive-relative paths.
///
/// 递归收集目录中的文件，构建归档相对路径。
///
/// # Arguments / 参数
///
/// * `dir` - The directory to walk. / 要遍历的目录。
/// * `prefix` - The archive path prefix (e.g., "content"). / 归档路径前缀（如 "content"）。
/// * `out` - Output vector to append collected files. / 用于追加收集到的文件的输出向量。
fn collect_dir_recursive(
    dir: &Path,
    prefix: &str,
    out: &mut Vec<CollectedFile>,
) -> Result<(), BuildError> {
    // Use walkdir for robust recursive traversal.
    // 使用 walkdir 进行可靠的递归遍历。
    for entry in walkdir::WalkDir::new(dir).sort_by_file_name() {
        let entry = entry.map_err(|e| BuildError::Io(std::io::Error::other(e.to_string())))?;

        // Skip directories — only collect regular files.
        // 跳过目录 — 仅收集常规文件。
        if !entry.file_type().is_file() {
            continue;
        }

        // Build the archive-relative path using forward slashes.
        // 使用正斜杠构建归档相对路径。
        let rel = entry
            .path()
            .strip_prefix(dir)
            .map_err(|e| BuildError::InvalidStructure(e.to_string()))?;

        // Convert path separators to forward slashes (important on Windows).
        // 将路径分隔符转换为正斜杠（在 Windows 上很重要）。
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let archive_path = format!("{prefix}/{rel_str}");

        out.push(CollectedFile {
            archive_path,
            disk_path: entry.path().to_path_buf(),
        });
    }
    Ok(())
}

// =============================================================================
// compute_blake3_hash — hash a file's contents with BLAKE3
// =============================================================================

/// Compute the BLAKE3 hash of a file and return the raw hash bytes.
///
/// 计算文件的 BLAKE3 哈希并返回原始哈希字节。
pub fn compute_blake3_hash(path: &Path) -> Result<blake3::Hash, BuildError> {
    let data = fs::read(path)?;
    Ok(blake3::hash(&data))
}

/// Compute the BLAKE3 hash of a byte slice.
///
/// 计算字节切片的 BLAKE3 哈希。
pub fn compute_blake3_bytes(data: &[u8]) -> blake3::Hash {
    blake3::hash(data)
}

// =============================================================================
// build_manifest — generate MANIFEST.MF content
// =============================================================================

/// Files that should NOT be included in the manifest.
/// 不应纳入清单的文件。
const MANIFEST_EXCLUDED_PREFIXES: &[&str] = &[
    "mimetype",
    "META-INF/MANIFEST.MF",
    "META-INF/signatures/",
    "META-INF/certs/",
];

/// Check if a file path should be excluded from the manifest.
///
/// 检查文件路径是否应从清单中排除。
fn is_manifest_excluded(path: &str) -> bool {
    MANIFEST_EXCLUDED_PREFIXES
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(prefix))
}

/// Build a manifest from a list of (archive_path, data_bytes) tuples.
///
/// 从 (归档路径, 数据字节) 元组列表构建清单。
///
/// # Arguments / 参数
///
/// * `ucx_version` - The UCX spec version (e.g., "1.0"). / UCX 规范版本。
/// * `entries` - Tuples of (archive_path, file_content_bytes). / (归档路径, 文件内容字节) 元组。
pub fn build_manifest(ucx_version: &str, entries: &[(&str, &[u8])]) -> Manifest {
    let mut manifest = Manifest::new(ucx_version, HashAlgorithm::Blake3);

    for (path, data) in entries {
        // Skip excluded files.
        // 跳过排除的文件。
        if is_manifest_excluded(path) {
            continue;
        }

        // Compute BLAKE3 hash for this entry.
        // 为此条目计算 BLAKE3 哈希。
        let hash = compute_blake3_bytes(data);
        let entry = ManifestEntry::new(path.to_string(), data.len() as u64, hash.as_bytes());

        manifest.add_entry(entry);
        debug!(path = path, "Added manifest entry / 已添加清单条目");
    }

    manifest
}

// =============================================================================
// compression_method — choose compression strategy by file extension
// =============================================================================

/// Determine the ZIP compression method based on file extension.
///
/// Already-compressed formats (images, audio, video, fonts) use STORED (no compression)
/// since re-compressing them wastes CPU for negligible size reduction.
/// Text-based formats use DEFLATE for good compression ratios.
///
/// 根据文件扩展名确定 ZIP 压缩方式。
/// 已压缩的格式（图片、音频、视频、字体）使用 STORED（不压缩），
/// 因为重复压缩浪费 CPU 且压缩率微乎其微。
/// 文本格式使用 DEFLATE 以获得良好的压缩率。
///
/// # Returns / 返回
///
/// - `CompressionMethod::Stored` for already-compressed formats.
///   对已压缩格式返回 STORED。
/// - `CompressionMethod::Deflated` for text and other formats.
///   对文本和其他格式返回 DEFLATE。
pub fn compression_method(file_name: &str) -> CompressionMethod {
    // Extract the file extension (lowercase for case-insensitive matching).
    // 提取文件扩展名（转小写以进行不区分大小写的匹配）。
    let ext = file_name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        // --- Already-compressed image formats / 已压缩的图片格式 ---
        "jpg" | "jpeg" | "png" | "webp" | "gif" => CompressionMethod::Stored,

        // --- Already-compressed audio formats / 已压缩的音频格式 ---
        "mp3" | "ogg" => CompressionMethod::Stored,

        // --- Already-compressed video formats / 已压缩的视频格式 ---
        "mp4" => CompressionMethod::Stored,

        // --- Already-compressed document formats / 已压缩的文档格式 ---
        "pdf" => CompressionMethod::Stored,

        // --- Already-compressed font formats / 已压缩的字体格式 ---
        "woff2" | "ttf" | "otf" => CompressionMethod::Stored,

        // --- Text-based formats → DEFLATE / 文本格式 → DEFLATE ---
        "json" | "md" | "mdx" | "ucxc" | "txt" | "typ" | "tex" | "toml" | "xml" | "html"
        | "css" | "js" => CompressionMethod::Deflated,

        // --- Default: DEFLATE for unknown formats / 默认：未知格式使用 DEFLATE ---
        _ => CompressionMethod::Deflated,
    }
}

// =============================================================================
// create_ucx_archive — write the .ucx ZIP file
// =============================================================================

/// Create the `.ucx` ZIP archive at the given output path.
///
/// Writes entries in the correct order:
/// 1. `mimetype` — STORED, no compression (must be first for magic-number detection)
/// 2. `META-INF/MANIFEST.MF` — DEFLATE
/// 3. `metadata/codex.json` — DEFLATE
/// 4. `content/struct.json` — DEFLATE
/// 5. All other content and asset files — compression chosen by extension
///
/// 在给定的输出路径创建 `.ucx` ZIP 归档。
/// 按正确顺序写入条目：
/// 1. `mimetype` — STORED，不压缩（必须为第一个条目以便 magic-number 检测）
/// 2. `META-INF/MANIFEST.MF` — DEFLATE
/// 3. `metadata/codex.json` — DEFLATE
/// 4. `content/struct.json` — DEFLATE
/// 5. 所有其他内容和资源文件 — 根据扩展名选择压缩方式
///
/// # Arguments / 参数
///
/// * `output_path` - The path for the output `.ucx` file. / 输出 `.ucx` 文件的路径。
/// * `codex` - The Codex metadata to serialize as `metadata/codex.json`. / 序列化为 codex.json 的元数据。
/// * `structure` - The Structure to serialize as `content/struct.json`. / 序列化为 struct.json 的结构。
/// * `collected_files` - Additional files from content/ and assets/. / content/ 和 assets/ 中的其他文件。
/// * `ucx_version` - The UCX spec version string. / UCX 规范版本字符串。
#[allow(clippy::too_many_arguments)]
pub fn create_ucx_archive(
    output_path: &Path,
    codex: &Codex,
    structure: &Structure,
    collected_files: &[CollectedFile],
    ucx_version: &str,
) -> Result<(), BuildError> {
    // --- Serialize metadata to bytes. ---
    // --- 将元数据序列化为字节。 ---
    // NOTE: `serde_json::to_string_pretty` does NOT append a trailing newline.
    // Well-formed text files should end with a newline character, so we append "\n" explicitly.
    // 注意：`serde_json::to_string_pretty` 不会追加尾部换行符。
    // 规范的文本文件应以换行符结尾，因此我们显式追加 "\n"。
    let codex_json = serde_json::to_string_pretty(codex)
        .map_err(|e| BuildError::Serialization(e.to_string()))?
        + "\n";
    let struct_json = serde_json::to_string_pretty(structure)
        .map_err(|e| BuildError::Serialization(e.to_string()))?
        + "\n";

    // --- Read all collected file contents into memory for manifest computation. ---
    // --- 将所有收集到的文件内容读入内存以计算清单。 ---
    let mut file_contents: Vec<(String, Vec<u8>)> = Vec::new();
    for cf in collected_files {
        // Skip struct.json since we already have the serialized version.
        // 跳过 struct.json，因为我们已有序列化版本。
        if cf.archive_path == "content/struct.json" {
            continue;
        }
        let data = fs::read(&cf.disk_path)?;
        file_contents.push((cf.archive_path.clone(), data));
    }

    // --- Build manifest entries list for hashing. ---
    // --- 构建用于哈希的清单条目列表。 ---
    let mut manifest_input: Vec<(&str, &[u8])> = Vec::new();
    manifest_input.push(("metadata/codex.json", codex_json.as_bytes()));
    manifest_input.push(("content/struct.json", struct_json.as_bytes()));
    for (path, data) in &file_contents {
        manifest_input.push((path.as_str(), data.as_slice()));
    }

    // --- Generate MANIFEST.MF. ---
    // --- 生成 MANIFEST.MF。 ---
    let manifest = build_manifest(ucx_version, &manifest_input);
    let manifest_str = manifest.to_manifest_string();
    debug!(
        "Generated MANIFEST.MF with {} entries / 已生成包含 {} 个条目的 MANIFEST.MF",
        manifest.entries.len(),
        manifest.entries.len()
    );

    // --- Create the ZIP file. ---
    // --- 创建 ZIP 文件。 ---
    let file = fs::File::create(output_path)?;
    let mut zip = ZipWriter::new(file);

    // Entry 1: mimetype — STORED, no compression, no extra data.
    // 条目 1：mimetype — STORED，不压缩，无额外数据。
    let options_stored: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::Stored);
    zip.start_file("mimetype", options_stored)
        .map_err(|e| BuildError::Zip(e.to_string()))?;
    zip.write_all(UCX_MIMETYPE)?;

    // Entry 2: META-INF/MANIFEST.MF — DEFLATE.
    // 条目 2：META-INF/MANIFEST.MF — DEFLATE。
    let options_deflate: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("META-INF/MANIFEST.MF", options_deflate)
        .map_err(|e| BuildError::Zip(e.to_string()))?;
    zip.write_all(manifest_str.as_bytes())?;

    // Entry 3: metadata/codex.json — DEFLATE.
    // 条目 3：metadata/codex.json — DEFLATE。
    zip.start_file("metadata/codex.json", options_deflate)
        .map_err(|e| BuildError::Zip(e.to_string()))?;
    zip.write_all(codex_json.as_bytes())?;

    // Entry 4: content/struct.json — DEFLATE.
    // 条目 4：content/struct.json — DEFLATE。
    zip.start_file("content/struct.json", options_deflate)
        .map_err(|e| BuildError::Zip(e.to_string()))?;
    zip.write_all(struct_json.as_bytes())?;

    // Remaining entries: content and asset files.
    // 剩余条目：内容和资源文件。
    for (archive_path, data) in &file_contents {
        let method = compression_method(archive_path);
        let opts: FileOptions<'_, ()> = FileOptions::default().compression_method(method);
        zip.start_file(archive_path.as_str(), opts)
            .map_err(|e| BuildError::Zip(e.to_string()))?;
        zip.write_all(data)?;
    }

    // Finalize the ZIP archive.
    // 完成 ZIP 归档。
    zip.finish().map_err(|e| BuildError::Zip(e.to_string()))?;

    debug!(
        path = %output_path.display(),
        "UCX archive created / UCX 归档已创建"
    );

    Ok(())
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: compression_method should return STORED for already-compressed formats.
    /// 测试：compression_method 对已压缩格式应返回 STORED。
    #[test]
    fn test_compression_method_stored() {
        // Image formats.
        // 图片格式。
        assert_eq!(compression_method("cover.jpg"), CompressionMethod::Stored);
        assert_eq!(compression_method("image.jpeg"), CompressionMethod::Stored);
        assert_eq!(compression_method("icon.png"), CompressionMethod::Stored);
        assert_eq!(compression_method("photo.webp"), CompressionMethod::Stored);
        assert_eq!(compression_method("anim.gif"), CompressionMethod::Stored);

        // Audio formats.
        // 音频格式。
        assert_eq!(compression_method("track.mp3"), CompressionMethod::Stored);
        assert_eq!(compression_method("sound.ogg"), CompressionMethod::Stored);

        // Video format.
        // 视频格式。
        assert_eq!(compression_method("video.mp4"), CompressionMethod::Stored);

        // Document format.
        // 文档格式。
        assert_eq!(compression_method("doc.pdf"), CompressionMethod::Stored);

        // Font formats.
        // 字体格式。
        assert_eq!(compression_method("font.woff2"), CompressionMethod::Stored);
        assert_eq!(compression_method("font.ttf"), CompressionMethod::Stored);
        assert_eq!(compression_method("font.otf"), CompressionMethod::Stored);
    }

    /// Test: compression_method should return DEFLATE for text-based formats.
    /// 测试：compression_method 对文本格式应返回 DEFLATE。
    #[test]
    fn test_compression_method_deflate() {
        assert_eq!(
            compression_method("codex.json"),
            CompressionMethod::Deflated
        );
        assert_eq!(
            compression_method("chapter.md"),
            CompressionMethod::Deflated
        );
        assert_eq!(
            compression_method("content.mdx"),
            CompressionMethod::Deflated
        );
        assert_eq!(
            compression_method("encrypted.ucxc"),
            CompressionMethod::Deflated
        );
        assert_eq!(
            compression_method("readme.txt"),
            CompressionMethod::Deflated
        );
        assert_eq!(
            compression_method("config.toml"),
            CompressionMethod::Deflated
        );
        assert_eq!(compression_method("data.xml"), CompressionMethod::Deflated);
        assert_eq!(compression_method("page.html"), CompressionMethod::Deflated);
    }

    /// Test: compression_method should return DEFLATE for unknown extensions.
    /// 测试：compression_method 对未知扩展名应返回 DEFLATE。
    #[test]
    fn test_compression_method_default() {
        assert_eq!(compression_method("data.xyz"), CompressionMethod::Deflated);
        assert_eq!(
            compression_method("no_extension"),
            CompressionMethod::Deflated
        );
    }

    /// Test: compression_method should be case-insensitive.
    /// 测试：compression_method 应不区分大小写。
    #[test]
    fn test_compression_method_case_insensitive() {
        assert_eq!(compression_method("image.JPG"), CompressionMethod::Stored);
        assert_eq!(compression_method("image.Png"), CompressionMethod::Stored);
    }

    /// Test: is_manifest_excluded should correctly exclude special paths.
    /// 测试：is_manifest_excluded 应正确排除特殊路径。
    #[test]
    fn test_manifest_excluded() {
        assert!(is_manifest_excluded("mimetype"));
        assert!(is_manifest_excluded("META-INF/MANIFEST.MF"));
        assert!(is_manifest_excluded("META-INF/signatures/author.sig"));
        assert!(is_manifest_excluded("META-INF/certs/author.pem"));

        // Regular files should not be excluded.
        // 常规文件不应被排除。
        assert!(!is_manifest_excluded("metadata/codex.json"));
        assert!(!is_manifest_excluded("content/chapter-001.md"));
    }
}
