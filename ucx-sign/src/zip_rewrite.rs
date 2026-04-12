//! ZIP Rewrite for Layer 1 Signature File Injection.
//!
//! This module provides a function to add Layer 1 signature files
//! (`.SF`, `.EC`, and certificate `.cert.pem`) into an existing UCX
//! ZIP archive. The original archive entries are copied into a new
//! ZIP, and the signature files are appended.
//!
//! ZIP 重写 — 注入 Layer 1 签名文件。
//!
//! 本模块提供将 Layer 1 签名文件（`.SF`、`.EC` 和证书 `.cert.pem`）
//! 注入现有 UCX ZIP 归档的功能。原始归档条目被复制到新 ZIP 中，
//! 签名文件追加在末尾。

use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::SignError;

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Add Layer 1 signature files to an existing UCX ZIP archive.
///
/// Reads all entries from the original ZIP, adds the new signature files
/// (`META-INF/signatures/{signer_id}.SF`, `META-INF/signatures/{signer_id}.EC`,
/// and `META-INF/certs/{signer_id}.cert.pem`), and writes a new ZIP with
/// the correct entry ordering.
///
/// 向现有 UCX ZIP 归档添加 Layer 1 签名文件。
/// 从原始 ZIP 中读取所有条目，添加新的签名文件
/// （`META-INF/signatures/{signer_id}.SF`、`META-INF/signatures/{signer_id}.EC`
/// 和 `META-INF/certs/{signer_id}.cert.pem`），并写入保持正确条目顺序的新 ZIP。
///
/// # Arguments / 参数
///
/// * `zip_data`   - Raw bytes of the original UCX ZIP archive.
///   原始 UCX ZIP 归档的字节数据。
/// * `signer_id`  - Signer identifier (e.g., "AUTHOR"). Used as the filename prefix.
///   签名者标识（如 "AUTHOR"），用作文件名前缀。
/// * `sf_content`  - UTF-8 bytes of the Signature File (.SF).
///   签名文件（.SF）的 UTF-8 字节。
/// * `ec_content`  - Binary bytes of the Ed25519 signature blob (.EC).
///   Ed25519 签名块（.EC）的二进制字节。
/// * `cert_der`    - DER-encoded X.509 certificate bytes.
///   DER 编码的 X.509 证书字节。
///
/// # Returns / 返回
///
/// New ZIP archive bytes containing all original entries plus the signature files.
/// 包含所有原始条目加签名文件的新 ZIP 归档字节。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if ZIP reading or writing fails.
/// ZIP 读取或写入失败时返回 `SignError::SigningFailed`。
pub fn add_signature_files(
    zip_data: &[u8],
    signer_id: &str,
    sf_content: &[u8],
    ec_content: &[u8],
    cert_der: &[u8],
) -> Result<Vec<u8>, SignError> {
    // --- Open the original archive for reading ---
    // 打开原始归档用于读取。
    let cursor = Cursor::new(zip_data);
    let mut archive = ZipArchive::new(cursor).map_err(|e| {
        SignError::SigningFailed(format!("failed to open ZIP archive for reading: {e}"))
    })?;

    // --- Create a new ZIP writer ---
    // 创建新的 ZIP writer。
    let output_buf: Vec<u8> = Vec::with_capacity(zip_data.len() + sf_content.len() + ec_content.len() + cert_der.len() + 4096);
    let output_cursor = Cursor::new(output_buf);
    let mut writer = ZipWriter::new(output_cursor);

    // --- Copy all existing entries from the original archive ---
    // 从原始归档中复制所有现有条目。
    for i in 0..archive.len() {
        // Read the entry metadata and content.
        // 读取条目元数据和内容。
        let mut entry = archive.by_index(i).map_err(|e| {
            SignError::SigningFailed(format!("failed to read ZIP entry at index {i}: {e}"))
        })?;

        // Get entry name and compression method before reading content.
        // 在读取内容之前获取条目名和压缩方法。
        let entry_name = entry.name().to_string();
        let compression = entry.compression();

        // Read the decompressed content.
        // 读取解压后的内容。
        let mut content = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut content).map_err(|e| {
            SignError::SigningFailed(format!(
                "failed to read content of ZIP entry '{}': {e}",
                entry_name
            ))
        })?;

        // Write the entry to the new ZIP with the same compression method.
        // 使用相同的压缩方法将条目写入新 ZIP。
        let options = SimpleFileOptions::default().compression_method(compression);
        writer.start_file(&entry_name, options).map_err(|e| {
            SignError::SigningFailed(format!(
                "failed to start ZIP entry '{}' in new archive: {e}",
                entry_name
            ))
        })?;
        writer.write_all(&content).map_err(|e| {
            SignError::SigningFailed(format!(
                "failed to write content for ZIP entry '{}': {e}",
                entry_name
            ))
        })?;
    }

    // --- Add META-INF/signatures/{SIGNER_ID}.SF (DEFLATE) ---
    // 添加 META-INF/signatures/{SIGNER_ID}.SF（DEFLATE 压缩）。
    let sf_path = format!("META-INF/signatures/{signer_id}.SF");
    let deflate_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated);
    writer.start_file(&sf_path, deflate_options).map_err(|e| {
        SignError::SigningFailed(format!("failed to start SF entry '{sf_path}': {e}"))
    })?;
    writer.write_all(sf_content).map_err(|e| {
        SignError::SigningFailed(format!("failed to write SF content: {e}"))
    })?;

    // --- Add META-INF/signatures/{SIGNER_ID}.EC (STORED — binary data) ---
    // 添加 META-INF/signatures/{SIGNER_ID}.EC（STORED — 二进制数据）。
    let ec_path = format!("META-INF/signatures/{signer_id}.EC");
    let stored_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored);
    writer.start_file(&ec_path, stored_options).map_err(|e| {
        SignError::SigningFailed(format!("failed to start EC entry '{ec_path}': {e}"))
    })?;
    writer.write_all(ec_content).map_err(|e| {
        SignError::SigningFailed(format!("failed to write EC content: {e}"))
    })?;

    // --- Add META-INF/certs/{SIGNER_ID}.cert.pem (DEFLATE) ---
    // 添加 META-INF/certs/{SIGNER_ID}.cert.pem（DEFLATE 压缩）。
    //
    // Encode the DER certificate to PEM format before writing.
    // 将 DER 证书编码为 PEM 格式后再写入。
    let cert_pem = pem_rfc7468::encode_string("CERTIFICATE", pem_rfc7468::LineEnding::LF, cert_der)
        .map_err(|e| {
            SignError::SigningFailed(format!("failed to encode certificate to PEM: {e}"))
        })?;

    let cert_path = format!("META-INF/certs/{signer_id}.cert.pem");
    writer.start_file(&cert_path, deflate_options).map_err(|e| {
        SignError::SigningFailed(format!("failed to start cert entry '{cert_path}': {e}"))
    })?;
    writer.write_all(cert_pem.as_bytes()).map_err(|e| {
        SignError::SigningFailed(format!("failed to write cert PEM content: {e}"))
    })?;

    // --- Finalize the ZIP and return bytes ---
    // 完成 ZIP 并返回字节。
    let result_cursor = writer.finish().map_err(|e| {
        SignError::SigningFailed(format!("failed to finalize ZIP archive: {e}"))
    })?;

    Ok(result_cursor.into_inner())
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal ZIP file in memory for testing.
    /// 辅助函数：在内存中创建最小 ZIP 文件用于测试。
    fn create_test_zip() -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip_writer = ZipWriter::new(&mut buf);

            // Add mimetype (STORED, as per UCX spec).
            // 添加 mimetype（STORED，按 UCX 规范）。
            let stored = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored);
            zip_writer.start_file("mimetype", stored).unwrap();
            zip_writer
                .write_all(b"application/vnd.unicodex+zip")
                .unwrap();

            // Add a MANIFEST.MF file.
            // 添加 MANIFEST.MF 文件。
            let deflate = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated);
            zip_writer
                .start_file("META-INF/MANIFEST.MF", deflate)
                .unwrap();
            zip_writer
                .write_all(b"Manifest-Version: 1.0\nHash-Algorithm: BLAKE3\n")
                .unwrap();

            // Add a content file.
            // 添加一个内容文件。
            zip_writer
                .start_file("content/chapter-001.md", deflate)
                .unwrap();
            zip_writer
                .write_all(b"# Chapter 1\n\nHello world.\n")
                .unwrap();

            zip_writer.finish().unwrap();
        }
        buf.into_inner()
    }

    /// Test: add_signature_files produces a valid ZIP with the new entries.
    /// 测试：add_signature_files 生成包含新条目的有效 ZIP。
    #[test]
    fn test_add_signature_files_produces_valid_zip() {
        let original = create_test_zip();
        let sf = b"Signature-Version: 1.0\nTest: value\n";
        let ec = b"\x01\x00\x00\x00test-ec-data";
        let cert_der = b"fake-cert-der-for-testing";

        let result = add_signature_files(&original, "AUTHOR", sf, ec, cert_der)
            .expect("add_signature_files should succeed");

        // The result should be parseable as a ZIP.
        // 结果应可被解析为 ZIP。
        let cursor = Cursor::new(&result);
        let mut archive =
            ZipArchive::new(cursor).expect("result should be a valid ZIP archive");

        // Verify the original entries are preserved.
        // 验证原始条目被保留。
        assert!(
            archive.by_name("mimetype").is_ok(),
            "mimetype should still exist"
        );
        assert!(
            archive.by_name("META-INF/MANIFEST.MF").is_ok(),
            "MANIFEST.MF should still exist"
        );
        assert!(
            archive.by_name("content/chapter-001.md").is_ok(),
            "content file should still exist"
        );

        // Verify the new signature files were added.
        // 验证新的签名文件已添加。
        assert!(
            archive.by_name("META-INF/signatures/AUTHOR.SF").is_ok(),
            "SF file should exist"
        );
        assert!(
            archive.by_name("META-INF/signatures/AUTHOR.EC").is_ok(),
            "EC file should exist"
        );
        assert!(
            archive.by_name("META-INF/certs/AUTHOR.cert.pem").is_ok(),
            "cert PEM file should exist"
        );

        // Verify SF content is correct.
        // 验证 SF 内容正确。
        {
            let mut sf_entry = archive
                .by_name("META-INF/signatures/AUTHOR.SF")
                .unwrap();
            let mut sf_read = String::new();
            sf_entry.read_to_string(&mut sf_read).unwrap();
            assert_eq!(sf_read, "Signature-Version: 1.0\nTest: value\n");
        }

        // Verify EC content is correct (binary).
        // 验证 EC 内容正确（二进制）。
        {
            let mut ec_entry = archive
                .by_name("META-INF/signatures/AUTHOR.EC")
                .unwrap();
            let mut ec_read = Vec::new();
            ec_entry.read_to_end(&mut ec_read).unwrap();
            assert_eq!(ec_read, ec);
        }

        // Verify cert PEM exists and starts with the correct header.
        // 验证证书 PEM 存在并以正确的头部开始。
        {
            let mut cert_entry = archive
                .by_name("META-INF/certs/AUTHOR.cert.pem")
                .unwrap();
            let mut cert_read = String::new();
            cert_entry.read_to_string(&mut cert_read).unwrap();
            assert!(
                cert_read.contains("-----BEGIN CERTIFICATE-----"),
                "cert PEM should contain the CERTIFICATE header"
            );
        }
    }

    /// Test: add_signature_files preserves original file content.
    /// 测试：add_signature_files 保留原始文件内容。
    #[test]
    fn test_add_signature_files_preserves_content() {
        let original = create_test_zip();
        let result = add_signature_files(
            &original,
            "SIGNER1",
            b"sf-data",
            b"ec-data",
            b"cert-data",
        )
        .expect("add_signature_files should succeed");

        let cursor = Cursor::new(&result);
        let mut archive = ZipArchive::new(cursor).unwrap();

        // Read the mimetype and verify content.
        // 读取 mimetype 并验证内容。
        {
            let mut mt = archive.by_name("mimetype").unwrap();
            let mut content = String::new();
            mt.read_to_string(&mut content).unwrap();
            assert_eq!(content, "application/vnd.unicodex+zip");
        }

        // Read the chapter and verify content.
        // 读取章节并验证内容。
        {
            let mut ch = archive.by_name("content/chapter-001.md").unwrap();
            let mut chapter = String::new();
            ch.read_to_string(&mut chapter).unwrap();
            assert_eq!(chapter, "# Chapter 1\n\nHello world.\n");
        }
    }
}
