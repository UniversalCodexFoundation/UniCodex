//! Unit tests for the ucx-parse module.
//!
//! These tests create minimal valid UCX archives (ZIP format) in memory,
//! write them to temporary files, then parse them using the public API.
//!
//! ucx-parse 模块的单元测试。
//! 这些测试在内存中创建最小有效的 UCX 归档（ZIP 格式），
//! 写入临时文件，然后使用公开 API 解析。

use std::io::Write as IoWrite;
use std::path::Path;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use zip::write::SimpleFileOptions;

use super::*;

// =============================================================================
// Test helpers / 测试辅助函数
// =============================================================================

/// Minimal codex.json content for testing.
///
/// 测试用的最小 codex.json 内容。
fn test_codex_json() -> String {
    r#"{
    "version": "1.0",
    "identifier": {
        "ucx_id": "urn:ucx:550e8400-e29b-41d4-a716-446655440000"
    },
    "title": {
        "main": "测试小说"
    },
    "creators": [
        {
            "name": "测试作者",
            "role": "author"
        }
    ],
    "language": "zh-CN"
}"#
    .to_string()
}

/// Minimal struct.json content for testing.
///
/// 测试用的最小 struct.json 内容。
fn test_struct_json() -> String {
    r#"{
    "version": "1.0",
    "structure": [
        {
            "title": "第一章 开端",
            "file": "chapter-001.md"
        }
    ]
}"#
    .to_string()
}

/// Sample chapter content for testing.
///
/// 测试用的示例章节内容。
fn test_chapter_content() -> String {
    "# 第一章 开端\n\n这是测试内容。\n".to_string()
}

/// Build a MANIFEST.MF string with BLAKE3 hashes for the given files.
///
/// Takes a list of `(name, content)` pairs and computes BLAKE3 hashes.
///
/// 为给定文件构建 MANIFEST.MF 字符串（BLAKE3 哈希）。
/// 接受 `(name, content)` 对列表并计算 BLAKE3 哈希。
fn build_manifest_mf(files: &[(&str, &[u8])]) -> String {
    let mut mf = String::new();

    // Main section / 主段
    mf.push_str("Manifest-Version: 1.0\n");
    mf.push_str("UCX-Version: 1.0\n");
    mf.push_str("Created-By: ucx-parse-test\n");
    mf.push_str("Hash-Algorithm: BLAKE3\n");

    // Per-entry sections / 条目段
    // Use Base64 encoding for BLAKE3 digests (matching spec §3.2).
    // 使用 Base64 编码 BLAKE3 摘要（符合规范 §3.2）。
    for (name, content) in files {
        let hash = blake3::hash(content);
        let b64 = BASE64_STANDARD.encode(hash.as_bytes());
        mf.push('\n');
        mf.push_str(&format!("Name: {name}\n"));
        mf.push_str(&format!("Size: {}\n", content.len()));
        mf.push_str(&format!("BLAKE3-Digest: {b64}\n"));
    }

    mf
}

/// Create a minimal valid UCX ZIP file at the given path.
///
/// The archive contains:
/// 1. `mimetype` (STORED, no compression)
/// 2. `META-INF/MANIFEST.MF`
/// 3. `metadata/codex.json`
/// 4. `content/struct.json`
/// 5. `content/chapter-001.md`
///
/// 在给定路径创建一个最小有效的 UCX ZIP 文件。
fn create_test_ucx(path: &Path) {
    // Prepare content bytes for manifest hash computation.
    // 准备内容字节用于清单哈希计算。
    let codex_json = test_codex_json();
    let struct_json = test_struct_json();
    let chapter_md = test_chapter_content();

    // Compute the manifest content with correct hashes.
    // 计算包含正确哈希的清单内容。
    let manifest_files: Vec<(&str, &[u8])> = vec![
        ("metadata/codex.json", codex_json.as_bytes()),
        ("content/struct.json", struct_json.as_bytes()),
        ("content/chapter-001.md", chapter_md.as_bytes()),
    ];
    let manifest_mf = build_manifest_mf(&manifest_files);

    // Create the ZIP file.
    // 创建 ZIP 文件。
    let file = std::fs::File::create(path).expect("failed to create test ZIP file");
    let mut zip = zip::ZipWriter::new(file);

    // Options for STORED (no compression) — required for mimetype.
    // STORED（无压缩）选项 — mimetype 条目要求。
    let stored_opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    // Options for DEFLATED (compressed) — used for all other entries.
    // DEFLATED（压缩）选项 — 用于其他所有条目。
    let deflated_opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // 1. mimetype — must be the first entry, STORED.
    // 1. mimetype — 必须是第一个条目，STORED。
    zip.start_file("mimetype", stored_opts).unwrap();
    zip.write_all(UCX_MIMETYPE.as_bytes()).unwrap();

    // 2. META-INF/MANIFEST.MF
    zip.start_file("META-INF/MANIFEST.MF", deflated_opts).unwrap();
    zip.write_all(manifest_mf.as_bytes()).unwrap();

    // 3. metadata/codex.json
    zip.start_file("metadata/codex.json", deflated_opts).unwrap();
    zip.write_all(codex_json.as_bytes()).unwrap();

    // 4. content/struct.json
    zip.start_file("content/struct.json", deflated_opts).unwrap();
    zip.write_all(struct_json.as_bytes()).unwrap();

    // 5. content/chapter-001.md
    zip.start_file("content/chapter-001.md", deflated_opts).unwrap();
    zip.write_all(chapter_md.as_bytes()).unwrap();

    // Finalize the archive.
    // 完成归档。
    zip.finish().unwrap();
}

/// Create a UCX ZIP with an invalid mimetype for error testing.
///
/// 创建一个 mimetype 无效的 UCX ZIP 用于错误测试。
fn create_test_ucx_bad_mimetype(path: &Path) {
    let file = std::fs::File::create(path).expect("failed to create test ZIP file");
    let mut zip = zip::ZipWriter::new(file);

    let stored_opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    // Write an invalid mimetype.
    // 写入无效的 mimetype。
    zip.start_file("mimetype", stored_opts).unwrap();
    zip.write_all(b"application/zip").unwrap();

    zip.finish().unwrap();
}

/// Create a UCX ZIP missing the MANIFEST.MF for error testing.
///
/// 创建一个缺少 MANIFEST.MF 的 UCX ZIP 用于错误测试。
fn create_test_ucx_no_manifest(path: &Path) {
    let file = std::fs::File::create(path).expect("failed to create test ZIP file");
    let mut zip = zip::ZipWriter::new(file);

    let stored_opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    let deflated_opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // mimetype is valid, but no MANIFEST.MF.
    // mimetype 有效，但没有 MANIFEST.MF。
    zip.start_file("mimetype", stored_opts).unwrap();
    zip.write_all(UCX_MIMETYPE.as_bytes()).unwrap();

    // Include codex and struct but skip manifest.
    // 包含 codex 和 struct 但跳过 manifest。
    zip.start_file("metadata/codex.json", deflated_opts).unwrap();
    zip.write_all(test_codex_json().as_bytes()).unwrap();

    zip.start_file("content/struct.json", deflated_opts).unwrap();
    zip.write_all(test_struct_json().as_bytes()).unwrap();

    zip.finish().unwrap();
}

// =============================================================================
// Test cases / 测试用例
// =============================================================================

/// Test: Opening a valid UCX file should succeed and return correct metadata.
///
/// 测试：打开有效的 UCX 文件应成功并返回正确的元数据。
#[test]
fn test_open_valid_ucx() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("test.ucx");
    create_test_ucx(&ucx_path);

    // Open the UCX file — should succeed.
    // 打开 UCX 文件 — 应成功。
    let archive = open(&ucx_path).expect("open() should succeed for valid UCX");

    // Verify codex metadata.
    // 验证 codex 元数据。
    assert_eq!(archive.codex().title.main, "测试小说");
    assert_eq!(archive.codex().language, "zh-CN");
    assert_eq!(archive.codex().creators.len(), 1);
    assert_eq!(archive.codex().creators[0].name, "测试作者");

    // Verify structure.
    // 验证结构。
    assert_eq!(archive.structure().structure.len(), 1);
    assert_eq!(archive.structure().structure[0].title, "第一章 开端");
    assert_eq!(
        archive.structure().structure[0].file.as_deref(),
        Some("chapter-001.md")
    );

    // Verify manifest.
    // 验证清单。
    assert_eq!(archive.manifest().manifest_version, "1.0");
    assert!(!archive.manifest().entries.is_empty());
}

/// Test: Opening a file with an invalid mimetype should return InvalidFormat.
///
/// 测试：打开 mimetype 无效的文件应返回 InvalidFormat 错误。
#[test]
fn test_open_invalid_mimetype() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("bad_mime.ucx");
    create_test_ucx_bad_mimetype(&ucx_path);

    let result = open(&ucx_path);
    assert!(result.is_err(), "should fail for invalid mimetype");

    let err = result.unwrap_err();
    assert!(
        matches!(err, ParseError::InvalidFormat(_)),
        "expected InvalidFormat, got: {err:?}"
    );
}

/// Test: Opening a file missing MANIFEST.MF should return MissingFile.
///
/// 测试：打开缺少 MANIFEST.MF 的文件应返回 MissingFile 错误。
#[test]
fn test_open_missing_manifest() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("no_manifest.ucx");
    create_test_ucx_no_manifest(&ucx_path);

    let result = open(&ucx_path);
    assert!(result.is_err(), "should fail for missing manifest");

    let err = result.unwrap_err();
    assert!(
        matches!(err, ParseError::MissingFile(_)),
        "expected MissingFile, got: {err:?}"
    );
}

/// Test: read_chapter should return the correct content for a valid chapter.
///
/// 测试：read_chapter 应返回有效章节的正确内容。
#[test]
fn test_read_chapter() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("read_chapter.ucx");
    create_test_ucx(&ucx_path);

    let mut archive = open(&ucx_path).expect("open() should succeed");

    // Read the chapter content.
    // 读取章节内容。
    let content = archive
        .read_chapter("chapter-001.md")
        .expect("read_chapter should succeed");

    assert_eq!(content, test_chapter_content());
    assert!(content.contains("# 第一章 开端"));
    assert!(content.contains("测试内容"));
}

/// Test: verify_hashes should return all valid results for a correct archive.
///
/// 测试：verify_hashes 对正确的归档应返回全部有效的结果。
#[test]
fn test_verify_hashes() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("verify.ucx");
    create_test_ucx(&ucx_path);

    let mut archive = open(&ucx_path).expect("open() should succeed");

    // Verify all hashes.
    // 验证所有哈希。
    let results = archive.verify_hashes().expect("verify_hashes should succeed");

    // All results should be valid.
    // 所有结果应为有效。
    assert!(!results.is_empty(), "should have verification results");
    for result in &results {
        assert!(
            result.valid,
            "hash mismatch for '{}': expected={}, actual={}",
            result.name, result.expected, result.actual
        );
    }
}

/// Test: list_files should return all file paths in the archive.
///
/// 测试：list_files 应返回归档中的所有文件路径。
#[test]
fn test_list_files() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("list_files.ucx");
    create_test_ucx(&ucx_path);

    let archive = open(&ucx_path).expect("open() should succeed");

    // List all files.
    // 列出所有文件。
    let files = archive.list_files();

    // Should contain all expected entries.
    // 应包含所有预期的条目。
    assert!(files.contains(&"mimetype".to_string()), "should contain mimetype");
    assert!(
        files.contains(&"META-INF/MANIFEST.MF".to_string()),
        "should contain MANIFEST.MF"
    );
    assert!(
        files.contains(&"metadata/codex.json".to_string()),
        "should contain codex.json"
    );
    assert!(
        files.contains(&"content/struct.json".to_string()),
        "should contain struct.json"
    );
    assert!(
        files.contains(&"content/chapter-001.md".to_string()),
        "should contain chapter-001.md"
    );

    // Total count should be 5.
    // 总数应为 5。
    assert_eq!(files.len(), 5, "should have exactly 5 files");
}
