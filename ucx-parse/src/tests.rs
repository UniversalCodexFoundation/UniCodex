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

/// Test: extract_to should extract all files to the output directory with correct
/// directory structure and file contents.
///
/// 测试：extract_to 应将所有文件解压到输出目录，保持正确的目录结构和文件内容。
#[test]
fn test_extract_to() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("extract.ucx");
    create_test_ucx(&ucx_path);

    let mut archive = open(&ucx_path).expect("open() should succeed");

    // Create a subdirectory for extraction output.
    // 为解压输出创建子目录。
    let output_dir = tmp.path().join("output");

    // Extract all files.
    // 解压所有文件。
    let extracted = archive
        .extract_to(&output_dir)
        .expect("extract_to should succeed");

    // Should have extracted 5 files (no directory entries).
    // 应已解压 5 个文件（无目录条目）。
    assert_eq!(extracted.len(), 5, "should extract exactly 5 files");

    // Verify all expected files exist on disk.
    // 验证所有预期文件在磁盘上存在。
    let expected_files = vec![
        "mimetype",
        "META-INF/MANIFEST.MF",
        "metadata/codex.json",
        "content/struct.json",
        "content/chapter-001.md",
    ];
    for expected in &expected_files {
        let file_path = output_dir.join(expected);
        assert!(
            file_path.exists(),
            "expected file should exist: {expected}"
        );
    }

    // Verify the returned list contains all expected entries.
    // 验证返回的列表包含所有预期条目。
    for expected in &expected_files {
        assert!(
            extracted.contains(&expected.to_string()),
            "extracted list should contain: {expected}"
        );
    }

    // Verify content of mimetype matches the expected value.
    // 验证 mimetype 的内容与预期值匹配。
    let mimetype_content = std::fs::read_to_string(output_dir.join("mimetype"))
        .expect("should read mimetype");
    assert_eq!(mimetype_content, "application/vnd.unicodex+zip");

    // Verify content of chapter file matches the original.
    // 验证章节文件的内容与原始内容匹配。
    let chapter_content = std::fs::read_to_string(output_dir.join("content/chapter-001.md"))
        .expect("should read chapter");
    assert_eq!(chapter_content, test_chapter_content());

    // Verify codex.json content can be parsed correctly.
    // 验证 codex.json 内容可以正确解析。
    let codex_content = std::fs::read_to_string(output_dir.join("metadata/codex.json"))
        .expect("should read codex.json");
    let codex: ucx_types::Codex = serde_json::from_str(&codex_content)
        .expect("extracted codex.json should be valid JSON");
    assert_eq!(codex.title.main, "测试小说");
}

// =============================================================================
// Encryption detection tests / 加密检测测试
// =============================================================================

/// UCXE magic number: [0x55, 0x43, 0x58, 0x45] = "UCXE".
/// UCXE 魔数：[0x55, 0x43, 0x58, 0x45] = "UCXE"。
const UCXE_MAGIC: [u8; 4] = [0x55, 0x43, 0x58, 0x45];

/// Create a UCX ZIP where `content/chapter-001.md` contains UCXE-encrypted data.
///
/// The chapter content starts with the UCXE magic number followed by dummy
/// ciphertext, simulating an encrypted chapter file.
///
/// 创建一个 UCX ZIP，其中 `content/chapter-001.md` 包含 UCXE 加密数据。
/// 章节内容以 UCXE 魔数开头，后接虚拟密文，模拟加密的章节文件。
fn create_test_ucx_with_encrypted_chapter(path: &Path) {
    let codex_json = test_codex_json();
    let struct_json = test_struct_json();

    // 构造以 UCXE 魔数开头的假加密数据。
    let mut encrypted_content: Vec<u8> = Vec::new();
    encrypted_content.extend_from_slice(&UCXE_MAGIC);
    encrypted_content.extend_from_slice(b"\x00\x01\x02\x03fake-ciphertext-data");

    // 计算清单哈希（包含加密章节的原始字节）。
    let manifest_files: Vec<(&str, &[u8])> = vec![
        ("metadata/codex.json", codex_json.as_bytes()),
        ("content/struct.json", struct_json.as_bytes()),
        ("content/chapter-001.md", &encrypted_content),
    ];
    let manifest_mf = build_manifest_mf(&manifest_files);

    let file = std::fs::File::create(path).expect("failed to create test ZIP file");
    let mut zip = zip::ZipWriter::new(file);

    let stored_opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    let deflated_opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("mimetype", stored_opts).unwrap();
    zip.write_all(UCX_MIMETYPE.as_bytes()).unwrap();

    zip.start_file("META-INF/MANIFEST.MF", deflated_opts).unwrap();
    zip.write_all(manifest_mf.as_bytes()).unwrap();

    zip.start_file("metadata/codex.json", deflated_opts).unwrap();
    zip.write_all(codex_json.as_bytes()).unwrap();

    zip.start_file("content/struct.json", deflated_opts).unwrap();
    zip.write_all(struct_json.as_bytes()).unwrap();

    zip.start_file("content/chapter-001.md", deflated_opts).unwrap();
    zip.write_all(&encrypted_content).unwrap();

    zip.finish().unwrap();
}

/// Test: is_chapter_encrypted should return true for UCXE magic data.
///
/// 测试：对于 UCXE 魔数数据，is_chapter_encrypted 应返回 true。
#[test]
fn test_is_chapter_encrypted_with_ucxe_magic() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("encrypted.ucx");
    create_test_ucx_with_encrypted_chapter(&ucx_path);

    let mut archive = open(&ucx_path).expect("open() should succeed");

    let encrypted = archive
        .is_chapter_encrypted("chapter-001.md")
        .expect("is_chapter_encrypted should not error");

    assert!(encrypted, "chapter with UCXE magic should be detected as encrypted");
}

/// Test: is_chapter_encrypted should return false for normal plaintext data.
///
/// 测试：对于普通明文数据，is_chapter_encrypted 应返回 false。
#[test]
fn test_is_chapter_encrypted_with_normal_data() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("normal.ucx");
    create_test_ucx(&ucx_path);

    let mut archive = open(&ucx_path).expect("open() should succeed");

    let encrypted = archive
        .is_chapter_encrypted("chapter-001.md")
        .expect("is_chapter_encrypted should not error");

    assert!(!encrypted, "normal plaintext chapter should not be detected as encrypted");
}

/// Test: read_chapter on an encrypted chapter should return ParseError::Encrypted.
///
/// 测试：对加密章节调用 read_chapter 应返回 ParseError::Encrypted 错误。
#[test]
fn test_read_chapter_encrypted_returns_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ucx_path = tmp.path().join("read_encrypted.ucx");
    create_test_ucx_with_encrypted_chapter(&ucx_path);

    let mut archive = open(&ucx_path).expect("open() should succeed");

    let result = archive.read_chapter("chapter-001.md");
    assert!(result.is_err(), "read_chapter should fail for encrypted chapter");

    let err = result.unwrap_err();
    assert!(
        matches!(err, ParseError::Encrypted(_)),
        "expected ParseError::Encrypted, got: {err:?}"
    );
}

// =============================================================================
// ROB-2 / ROB-1: ZIP signature & UCX-Version compatibility tests
// ROB-2 / ROB-1：ZIP 签名与 UCX-Version 兼容性测试
// =============================================================================

/// Test: concatenating arbitrary bytes before a valid UCX should cause `open()` to
/// reject the file with `InvalidFormat` due to a missing ZIP signature at offset 0.
///
/// 测试：在有效 UCX 之前拼接任意字节，应使 `open()` 因 offset 0 处
/// 缺少 ZIP 签名而返回 `InvalidFormat`。
#[test]
fn test_open_rejects_prefixed_zip() {
    let tmp = tempfile::TempDir::new().unwrap();

    // Build a valid UCX first.
    // 先构建一个有效的 UCX。
    let valid_path = tmp.path().join("valid.ucx");
    create_test_ucx(&valid_path);
    let valid_bytes = std::fs::read(&valid_path).unwrap();

    // Prepend arbitrary junk bytes (not PK\x03\x04).
    // 在开头拼接任意非 PK\x03\x04 字节。
    let mut prefixed = Vec::new();
    prefixed.extend_from_slice(b"JUNK-PREFIX-NOT-A-ZIP-SIGNATURE");
    prefixed.extend_from_slice(&valid_bytes);

    let prefixed_path = tmp.path().join("prefixed.ucx");
    std::fs::write(&prefixed_path, &prefixed).unwrap();

    let result = open(&prefixed_path);
    assert!(
        result.is_err(),
        "open() must reject files missing ZIP signature at offset 0"
    );
    let err = result.unwrap_err();
    match err {
        ParseError::InvalidFormat(ref msg) => {
            assert!(
                msg.contains("ZIP signature"),
                "error message should mention ZIP signature: {msg}"
            );
        }
        other => panic!("expected InvalidFormat, got: {other:?}"),
    }
}

/// Test: a file that is shorter than 4 bytes should be rejected with
/// `InvalidFormat` (not a generic I/O error).
///
/// 测试：少于 4 字节的文件应以 `InvalidFormat`（而非通用 I/O 错误）拒绝。
#[test]
fn test_open_rejects_too_short_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let path = tmp.path().join("tiny.ucx");
    std::fs::write(&path, b"PK").unwrap();

    let result = open(&path);
    assert!(result.is_err(), "tiny file must be rejected");
    assert!(
        matches!(result.unwrap_err(), ParseError::InvalidFormat(_)),
        "expected InvalidFormat for too-short file"
    );
}

/// Build a UCX archive with a custom `UCX-Version` value in its MANIFEST.MF.
///
/// 构建一个 MANIFEST.MF 中 `UCX-Version` 为自定义值的 UCX 归档。
fn create_test_ucx_with_ucx_version(path: &Path, ucx_version: &str) {
    let codex_json = test_codex_json();
    let struct_json = test_struct_json();
    let chapter_md = test_chapter_content();

    // Build manifest text with a user-specified UCX-Version.
    // 使用指定的 UCX-Version 构建 manifest 文本。
    let mut mf = String::new();
    mf.push_str("Manifest-Version: 1.0\n");
    mf.push_str(&format!("UCX-Version: {ucx_version}\n"));
    mf.push_str("Created-By: ucx-parse-test\n");
    mf.push_str("Hash-Algorithm: BLAKE3\n");
    for (name, content) in [
        ("metadata/codex.json", codex_json.as_bytes()),
        ("content/struct.json", struct_json.as_bytes()),
        ("content/chapter-001.md", chapter_md.as_bytes()),
    ] {
        let hash = blake3::hash(content);
        let b64 = BASE64_STANDARD.encode(hash.as_bytes());
        mf.push('\n');
        mf.push_str(&format!("Name: {name}\n"));
        mf.push_str(&format!("Size: {}\n", content.len()));
        mf.push_str(&format!("BLAKE3-Digest: {b64}\n"));
    }

    let file = std::fs::File::create(path).expect("failed to create test ZIP file");
    let mut zip = zip::ZipWriter::new(file);

    let stored_opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    let deflated_opts = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("mimetype", stored_opts).unwrap();
    zip.write_all(UCX_MIMETYPE.as_bytes()).unwrap();

    zip.start_file("META-INF/MANIFEST.MF", deflated_opts).unwrap();
    zip.write_all(mf.as_bytes()).unwrap();

    zip.start_file("metadata/codex.json", deflated_opts).unwrap();
    zip.write_all(codex_json.as_bytes()).unwrap();

    zip.start_file("content/struct.json", deflated_opts).unwrap();
    zip.write_all(struct_json.as_bytes()).unwrap();

    zip.start_file("content/chapter-001.md", deflated_opts).unwrap();
    zip.write_all(chapter_md.as_bytes()).unwrap();

    zip.finish().unwrap();
}

/// Test: a UCX whose MANIFEST declares MAJOR higher than supported must be rejected.
///
/// 测试：MANIFEST 中 MAJOR 高于支持值的 UCX 必须被拒绝。
#[test]
fn test_open_rejects_unsupported_ucx_major() {
    let tmp = tempfile::TempDir::new().unwrap();
    let path = tmp.path().join("future.ucx");
    create_test_ucx_with_ucx_version(&path, "99.0");

    let result = open(&path);
    assert!(
        result.is_err(),
        "open() must reject archives with MAJOR > SUPPORTED_UCX_MAJOR"
    );
    match result.unwrap_err() {
        ParseError::UnsupportedVersion(msg) => {
            assert!(
                msg.contains("99"),
                "error should reference the offending MAJOR: {msg}"
            );
        }
        other => panic!("expected UnsupportedVersion, got: {other:?}"),
    }
}

/// Test: MAJOR equal to the supported value should be accepted.
///
/// 测试：MAJOR 等于支持值时应被接受。
#[test]
fn test_open_accepts_supported_ucx_major() {
    let tmp = tempfile::TempDir::new().unwrap();
    let path = tmp.path().join("v1.ucx");
    create_test_ucx_with_ucx_version(&path, "1.5");

    open(&path).expect("UCX with supported MAJOR (1.x) should be accepted");
}

/// Test: a non-numeric UCX-Version MAJOR must be rejected.
///
/// 测试：非数字的 UCX-Version MAJOR 必须被拒绝。
#[test]
fn test_open_rejects_non_numeric_ucx_version() {
    let tmp = tempfile::TempDir::new().unwrap();
    let path = tmp.path().join("bad_ver.ucx");
    create_test_ucx_with_ucx_version(&path, "abc");

    let result = open(&path);
    assert!(result.is_err(), "non-numeric MAJOR must be rejected");
    assert!(matches!(
        result.unwrap_err(),
        ParseError::UnsupportedVersion(_)
    ));
}
