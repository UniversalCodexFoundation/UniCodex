//! UCX Digital Signing & Certificate Management Module.
//!
//! This module generates digital signatures for UCX files using
//! the dual-layer system:
//! - Layer 1: JAR-style (MANIFEST.MF -> SF -> RSA/EC)
//! - Layer 2: APK v2-style global signing block
//!
//! It also manages certificates, multi-signer workflows,
//! and key generation.
//!
//! UCX 数字签名与证书管理模块。
//! 使用双层体系为 UCX 文件生成数字签名：
//! - Layer 1: JAR 式（MANIFEST.MF -> SF -> RSA/EC）
//! - Layer 2: APK v2 式全局签名块
//!
//! 同时管理证书、多签名者工作流和密钥生成。

use thiserror::Error;

// =============================================================================
// Sub-modules / 子模块
// =============================================================================

/// Ed25519 key generation and PEM I/O.
/// Ed25519 密钥生成与 PEM 读写。
pub mod keys;

/// Self-signed X.509 certificate generation.
/// 自签名 X.509 证书生成。
pub mod cert;

/// Layer 1: JAR-style SF generation and Ed25519 signing.
/// Layer 1: JAR 式 SF 生成与 Ed25519 签名。
pub mod layer1;

/// Layer 2: APK v2-style signing block (chunked digest + Ed25519).
/// Layer 2：APK v2 式签名块（分块摘要 + Ed25519）。
pub mod layer2;

/// Low-level ZIP binary operations for signing block insertion/extraction.
/// 签名块插入/提取的底层 ZIP 二进制操作。
pub mod zip_binary;

/// ZIP rewrite for injecting Layer 1 signature files into an archive.
/// ZIP 重写 — 将 Layer 1 签名文件注入归档。
pub mod zip_rewrite;

// =============================================================================
// Error types / 错误类型
// =============================================================================

/// Errors that can occur during signing operations.
///
/// 签名操作过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum SignError {
    /// Private key not found or inaccessible.
    /// 找不到私钥或无法访问。
    #[error("private key not found: {0}")]
    KeyNotFound(String),

    /// Invalid key format.
    /// 密钥格式无效。
    #[error("invalid key format: {0}")]
    InvalidKey(String),

    /// Signing operation failed.
    /// 签名操作失败。
    #[error("signing failed: {0}")]
    SigningFailed(String),

    /// I/O error during signing.
    /// 签名过程中的 I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Certificate-related error.
    /// 证书相关错误。
    #[error("certificate error: {0}")]
    CertificateError(String),

    /// Key generation error.
    /// 密钥生成错误。
    #[error("key generation error: {0}")]
    KeyGenError(String),

    /// PEM encoding/decoding error.
    /// PEM 编解码错误。
    #[error("PEM encoding error: {0}")]
    PemError(String),
}

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Sign a UCX file with the given private key and certificate.
///
/// Generates both Layer 1 (JAR-style) and Layer 2 (APK v2-style) signatures,
/// then writes the signed archive back to disk (in-place).
///
/// 使用给定的私钥和证书对 UCX 文件签名。
/// 同时生成 Layer 1（JAR 式）和 Layer 2（APK v2 式）签名，
/// 然后将已签名的归档就地写回磁盘。
///
/// # Arguments / 参数
///
/// * `ucx_path`  - Path to the `.ucx` file to sign.
///   要签名的 `.ucx` 文件路径。
/// * `key_path`  - Path to the Ed25519 private key file (PKCS#8 PEM).
///   Ed25519 私钥文件路径（PKCS#8 PEM）。
/// * `cert_path` - Path to the X.509 certificate file (PEM).
///   X.509 证书文件路径（PEM）。
/// * `signer_id` - Identifier for the signer (e.g., "AUTHOR").
///   签名者标识（如 "AUTHOR"）。
///   Must be 1-32 characters, uppercase A-Z, digits 0-9, or underscore only.
///   必须为 1-32 个字符，仅允许大写 A-Z、数字 0-9 和下划线。
///
/// # Returns / 返回
///
/// Returns `Ok(())` on success, or a `SignError` on failure.
/// 成功返回 `Ok(())`，失败返回 `SignError`。
pub fn sign(
    ucx_path: &std::path::Path,
    key_path: &std::path::Path,
    cert_path: &std::path::Path,
    signer_id: &str,
) -> Result<(), SignError> {
    // -------------------------------------------------------------------------
    // Step 1: Validate signer_id — must be A-Z, 0-9, _ only, 1-32 chars.
    // 步骤 1：验证 signer_id — 仅允许 A-Z、0-9、_，1-32 个字符。
    // -------------------------------------------------------------------------
    validate_signer_id(signer_id)?;

    // -------------------------------------------------------------------------
    // Step 2: Load the private key from the PEM file.
    // 步骤 2：从 PEM 文件加载私钥。
    // -------------------------------------------------------------------------
    let signing_key = keys::load_private_key(key_path)?;

    // -------------------------------------------------------------------------
    // Step 3: Load the X.509 certificate (DER bytes) from the PEM file.
    // 步骤 3：从 PEM 文件加载 X.509 证书（DER 字节）。
    // -------------------------------------------------------------------------
    let cert_der = cert::load_certificate(cert_path)?;

    // -------------------------------------------------------------------------
    // Step 3.5: Verify that the private key matches the certificate's public key.
    // 步骤 3.5：验证私钥与证书中的公钥一致。
    // -------------------------------------------------------------------------
    // Derive the verifying (public) key from the signing (private) key.
    // 从签名（私）钥导出验证（公）钥。
    let verifying_key = signing_key.verifying_key();

    // Parse the certificate DER to extract the Subject Public Key Info (SPKI).
    // 解析证书 DER 以提取主体公钥信息（SPKI）。
    let parsed_cert = <x509_cert::Certificate as der::Decode>::from_der(&cert_der)
        .map_err(|e| SignError::InvalidKey(format!("failed to parse certificate DER: {e}")))?;
    let cert_public_key_bytes = parsed_cert
        .tbs_certificate
        .subject_public_key_info
        .subject_public_key
        .raw_bytes();

    // Compare the certificate's public key with the one derived from the private key.
    // 将证书中的公钥与从私钥导出的公钥进行比较。
    if cert_public_key_bytes != verifying_key.as_bytes() {
        return Err(SignError::InvalidKey(
            "private key does not match certificate public key".to_string(),
        ));
    }

    // -------------------------------------------------------------------------
    // Step 4: Read the UCX file into memory.
    // 步骤 4：将 UCX 文件读入内存。
    // -------------------------------------------------------------------------
    let ucx_data = std::fs::read(ucx_path)?;

    // -------------------------------------------------------------------------
    // Step 4.5: Check for duplicate signer — reject if this signer_id already exists.
    // 步骤 4.5：检查重复签名者 — 如果该 signer_id 已存在则拒绝。
    // -------------------------------------------------------------------------
    {
        let cursor = std::io::Cursor::new(&ucx_data);
        let archive = zip::ZipArchive::new(cursor)
            .map_err(|e| SignError::SigningFailed(format!("failed to read archive: {e}")))?;
        let sf_path = format!("META-INF/signatures/{signer_id}.SF");
        if archive.file_names().any(|name| name == sf_path) {
            return Err(SignError::SigningFailed(format!(
                "signer '{signer_id}' already exists in this archive. \
                 Use a different signer-id or remove the existing signature first."
            )));
        }
    }

    // -------------------------------------------------------------------------
    // Step 5: Open as ZIP, read MANIFEST.MF content.
    // 步骤 5：作为 ZIP 打开，读取 MANIFEST.MF 内容。
    // -------------------------------------------------------------------------
    let manifest_content = read_manifest_from_zip(&ucx_data)?;

    // -------------------------------------------------------------------------
    // Step 6: Generate SF — compute BLAKE3 digest of MANIFEST.MF.
    // 步骤 6：生成 SF — 计算 MANIFEST.MF 的 BLAKE3 摘要。
    // -------------------------------------------------------------------------
    let sf_content = layer1::generate_sf(&manifest_content, "1.0");

    // -------------------------------------------------------------------------
    // Step 7: Sign SF — produce .EC binary blob.
    // 步骤 7：签署 SF — 生成 .EC 二进制块。
    // -------------------------------------------------------------------------
    let ec_content = layer1::sign_sf(sf_content.as_bytes(), &signing_key, &cert_der)?;

    // -------------------------------------------------------------------------
    // Step 8: Add Layer 1 signature files to the ZIP.
    // 步骤 8：将 Layer 1 签名文件添加到 ZIP 中。
    // -------------------------------------------------------------------------
    let new_zip = zip_rewrite::add_signature_files(
        &ucx_data,
        signer_id,
        sf_content.as_bytes(),
        &ec_content,
        &cert_der,
    )?;

    // -------------------------------------------------------------------------
    // Step 9: Compute Layer 2 digest and sign.
    // 步骤 9：计算 Layer 2 摘要并签名。
    // -------------------------------------------------------------------------
    // Find EOCD and CD offset in the new ZIP data (which has Layer 1 files).
    // 在新 ZIP 数据（包含 Layer 1 文件）中找到 EOCD 和 CD 偏移。
    let eocd_offset = zip_binary::find_eocd(&new_zip)?;
    let cd_offset = zip_binary::get_cd_offset(&new_zip, eocd_offset)? as usize;

    // Compute protected content digest (excludes the signing block area).
    // 计算受保护内容摘要（不含签名块区域）。
    let digest = layer2::compute_protected_digest(&new_zip, cd_offset, eocd_offset)?;

    // Build signer data (signs the digest with Ed25519).
    // 构建签名者数据（用 Ed25519 签署摘要）。
    let signer_data = layer2::build_signer_data(&digest, &signing_key, &cert_der)?;

    // Build the complete signing block.
    // 构建完整的签名块。
    let signing_block = layer2::build_signing_block(&signer_data);

    // -------------------------------------------------------------------------
    // Step 10: Insert signing block between ZIP entries and Central Directory.
    // 步骤 10：在 ZIP 条目和中央目录之间插入签名块。
    // -------------------------------------------------------------------------
    let final_data = zip_binary::insert_signing_block(&new_zip, &signing_block)?;

    // -------------------------------------------------------------------------
    // Step 11: Write the signed data back to disk (in-place).
    // 步骤 11：将签名数据就地写回磁盘。
    // -------------------------------------------------------------------------
    std::fs::write(ucx_path, &final_data)?;

    Ok(())
}

/// Generate an Ed25519 key pair and save to files.
///
/// Creates two files:
/// - `{output_path}` — the private key in PKCS#8 PEM format
/// - `{output_path}.pub` — the public key in SPKI PEM format
///
/// 生成 Ed25519 密钥对并保存到文件。
///
/// 创建两个文件：
/// - `{output_path}` — PKCS#8 PEM 格式的私钥
/// - `{output_path}.pub` — SPKI PEM 格式的公钥
///
/// # Arguments / 参数
///
/// * `output_path` - Path for the private key file. The public key will be
///   saved to `{output_path}.pub`.
///   私钥文件路径。公钥将保存到 `{output_path}.pub`。
///
/// # Errors / 错误
///
/// Returns `SignError` if key generation or file writing fails.
/// 密钥生成或文件写入失败时返回 `SignError`。
pub fn keygen(output_path: &std::path::Path) -> Result<(), SignError> {
    // -------------------------------------------------------------------------
    // Pre-check: refuse to overwrite existing key files.
    // 预检查：拒绝覆盖已存在的密钥文件。
    // -------------------------------------------------------------------------
    if output_path.exists() {
        return Err(SignError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "key file already exists: {}. Use a different path or delete the existing file.",
                output_path.display()
            ),
        )));
    }

    // Build public key path (same logic as below: append ".pub" to OsString).
    // 构建公钥路径（与下方逻辑一致：在 OsString 后追加 ".pub"）。
    let mut pub_path_check = output_path.as_os_str().to_owned();
    pub_path_check.push(".pub");
    let pub_path_check = std::path::PathBuf::from(pub_path_check);

    if pub_path_check.exists() {
        return Err(SignError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "public key file already exists: {}. Use a different path or delete the existing file.",
                pub_path_check.display()
            ),
        )));
    }

    // Generate a new Ed25519 key pair.
    // 生成新的 Ed25519 密钥对。
    let (signing_key, verifying_key) = keys::generate_ed25519_keypair()?;

    // Save private key to the specified path.
    // 将私钥保存到指定路径。
    keys::save_private_key(&signing_key, output_path)?;

    // Build public key path by appending ".pub" to the private key path.
    // 通过在私钥路径后追加 ".pub" 来构建公钥路径。
    let mut pub_path = output_path.as_os_str().to_owned();
    pub_path.push(".pub");
    let pub_path = std::path::PathBuf::from(pub_path);

    // Save public key.
    // 保存公钥。
    keys::save_public_key(&verifying_key, &pub_path)?;

    Ok(())
}

/// Create a self-signed certificate and save to file.
///
/// 创建自签名证书并保存到文件。
///
/// # Arguments / 参数
///
/// * `key_path`    - Path to the Ed25519 private key (PKCS#8 PEM).
///   Ed25519 私钥路径（PKCS#8 PEM）。
/// * `cn`          - Common Name (CN) for the certificate subject.
///   证书主体的通用名称（CN）。
/// * `days`        - Certificate validity period in days.
///   证书有效期（天数）。
/// * `output_path` - Path to save the certificate PEM file.
///   证书 PEM 文件的保存路径。
///
/// # Errors / 错误
///
/// Returns `SignError` if key loading, certificate generation, or file writing fails.
/// 密钥加载、证书生成或文件写入失败时返回 `SignError`。
pub fn create_cert(
    key_path: &std::path::Path,
    cn: &str,
    days: u32,
    output_path: &std::path::Path,
) -> Result<(), SignError> {
    // Load the private key.
    // 加载私钥。
    let signing_key = keys::load_private_key(key_path)?;

    // Build certificate options.
    // 构建证书选项。
    let options = cert::CertOptions {
        common_name: cn.to_string(),
        days_valid: days,
        organization: None,
    };

    // Generate the self-signed certificate (DER bytes).
    // 生成自签名证书（DER 字节）。
    let cert_der = cert::create_self_signed_cert(&signing_key, &options)?;

    // Save the certificate to the specified PEM file.
    // 将证书保存到指定的 PEM 文件。
    cert::save_certificate(&cert_der, output_path)?;

    Ok(())
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Validate that a signer ID matches the required format.
///
/// Rules:
/// - 1 to 32 characters
/// - Only uppercase letters A-Z, digits 0-9, and underscore _
///
/// 验证签名者 ID 是否符合要求的格式。
/// 规则：
/// - 1 到 32 个字符
/// - 仅允许大写字母 A-Z、数字 0-9 和下划线 _
fn validate_signer_id(signer_id: &str) -> Result<(), SignError> {
    // Check length.
    // 检查长度。
    if signer_id.is_empty() || signer_id.len() > 32 {
        return Err(SignError::SigningFailed(format!(
            "signer_id must be 1-32 characters, got {} characters",
            signer_id.len()
        )));
    }

    // Check that all characters are A-Z, 0-9, or _.
    // 检查所有字符是否为 A-Z、0-9 或 _。
    if !signer_id
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(SignError::SigningFailed(format!(
            "signer_id must contain only A-Z, 0-9, or '_', got: \"{signer_id}\""
        )));
    }

    Ok(())
}

/// Read the MANIFEST.MF content from a ZIP archive in memory.
///
/// 从内存中的 ZIP 归档读取 MANIFEST.MF 内容。
fn read_manifest_from_zip(zip_data: &[u8]) -> Result<Vec<u8>, SignError> {
    let cursor = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| SignError::SigningFailed(format!("failed to open UCX archive as ZIP: {e}")))?;

    let mut manifest_entry = archive.by_name("META-INF/MANIFEST.MF").map_err(|e| {
        SignError::SigningFailed(format!("META-INF/MANIFEST.MF not found in archive: {e}"))
    })?;

    let mut content = Vec::with_capacity(manifest_entry.size() as usize);
    std::io::Read::read_to_end(&mut manifest_entry, &mut content)
        .map_err(|e| SignError::SigningFailed(format!("failed to read MANIFEST.MF: {e}")))?;

    Ok(content)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Test: sign() rejects invalid signer_id with lowercase letters.
    /// 测试：sign() 拒绝包含小写字母的无效 signer_id。
    #[test]
    fn test_sign_rejects_lowercase_signer_id() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let ucx_path = tmp.path().join("test.ucx");
        let key_path = tmp.path().join("key.pem");
        let cert_path = tmp.path().join("cert.pem");

        // Create dummy files so the file-not-found check doesn't fire first.
        // 创建虚拟文件，确保不会先触发文件不存在的检查。
        std::fs::write(&ucx_path, b"dummy").unwrap();
        std::fs::write(&key_path, b"dummy").unwrap();
        std::fs::write(&cert_path, b"dummy").unwrap();

        // Lowercase signer_id should be rejected before any I/O.
        // 小写 signer_id 应在任何 I/O 之前被拒绝。
        let result = sign(&ucx_path, &key_path, &cert_path, "author");
        assert!(result.is_err(), "lowercase signer_id should be rejected");

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("A-Z"),
            "error should mention allowed characters, got: {err_msg}"
        );
    }

    /// Test: sign() rejects signer_id with special characters.
    /// 测试：sign() 拒绝包含特殊字符的 signer_id。
    #[test]
    fn test_sign_rejects_special_char_signer_id() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let ucx_path = tmp.path().join("test.ucx");
        let key_path = tmp.path().join("key.pem");
        let cert_path = tmp.path().join("cert.pem");
        std::fs::write(&ucx_path, b"dummy").unwrap();
        std::fs::write(&key_path, b"dummy").unwrap();
        std::fs::write(&cert_path, b"dummy").unwrap();

        // Special characters should be rejected.
        // 特殊字符应被拒绝。
        for bad_id in &["AUTHOR@1", "MY-SIGNER", "A B", "", "A".repeat(33).as_str()] {
            let result = sign(&ucx_path, &key_path, &cert_path, bad_id);
            assert!(result.is_err(), "signer_id '{bad_id}' should be rejected");
        }
    }

    /// Test: validate_signer_id accepts valid IDs.
    /// 测试：validate_signer_id 接受有效的 ID。
    #[test]
    fn test_validate_signer_id_accepts_valid() {
        // Valid signer IDs should be accepted.
        // 有效的签名者 ID 应被接受。
        for valid_id in &["AUTHOR", "SIGNER_1", "A", "MY_ORG_2026", "X1_Y2_Z3"] {
            assert!(
                validate_signer_id(valid_id).is_ok(),
                "signer_id '{valid_id}' should be accepted"
            );
        }
    }

    /// Test: keygen() creates valid key files.
    /// 测试：keygen() 创建有效的密钥文件。
    #[test]
    fn test_keygen_creates_key_files() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let key_path = tmp.path().join("test_key.pem");

        // Generate keys.
        // 生成密钥。
        keygen(&key_path).expect("keygen should succeed");

        // Verify private key file exists and is loadable.
        // 验证私钥文件存在且可加载。
        assert!(key_path.exists(), "private key file should exist");
        let loaded = keys::load_private_key(&key_path);
        assert!(loaded.is_ok(), "private key should be loadable");

        // Verify public key file exists and is loadable.
        // 验证公钥文件存在且可加载。
        let pub_path = key_path.with_extension("pem.pub");
        assert!(pub_path.exists(), "public key file should exist");
        let loaded_pub = keys::load_public_key(&pub_path);
        assert!(loaded_pub.is_ok(), "public key should be loadable");

        // Verify the public key matches the private key's derived public key.
        // 验证公钥与私钥导出的公钥一致。
        let signing_key = loaded.unwrap();
        let verifying_key = loaded_pub.unwrap();
        assert_eq!(
            signing_key.verifying_key().to_bytes(),
            verifying_key.to_bytes(),
            "public key must match the signing key's verifying key"
        );
    }

    /// Test: create_cert() creates a valid certificate file.
    /// 测试：create_cert() 创建有效的证书文件。
    #[test]
    fn test_create_cert_creates_valid_cert() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let key_path = tmp.path().join("key.pem");
        let cert_path = tmp.path().join("cert.pem");

        // First, generate a key pair.
        // 首先，生成密钥对。
        keygen(&key_path).expect("keygen should succeed");

        // Create a certificate.
        // 创建证书。
        create_cert(&key_path, "Test Signer CN", 365, &cert_path)
            .expect("create_cert should succeed");

        // Verify the certificate file exists and is loadable.
        // 验证证书文件存在且可加载。
        assert!(cert_path.exists(), "certificate file should exist");
        let cert_der = cert::load_certificate(&cert_path);
        assert!(cert_der.is_ok(), "certificate should be loadable");

        // Verify the CN matches what we provided.
        // 验证 CN 与我们提供的值一致。
        let cn = cert::cert_subject_cn(&cert_der.unwrap()).expect("CN extraction should succeed");
        assert_eq!(cn, "Test Signer CN", "CN must match the provided value");
    }

    /// Test: complete sign flow — init → build → sign → verify file still opens.
    /// 测试：完整签名流程 — 初始化 → 构建 → 签名 → 验证文件仍可打开。
    #[test]
    fn test_complete_sign_flow() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("sign-test-project");

        // --- Step A: Initialize a minimal UCX project ---
        // 步骤 A：初始化一个最小 UCX 项目。
        let init_options = ucx_init::InitOptions {
            name: "签名测试".to_string(),
            author: "测试作者".to_string(),
            language: "zh-CN".to_string(),
            no_git: true, // Skip git for test speed. / 跳过 git 以加速测试。
            ..Default::default()
        };
        ucx_init::init(&project_dir, &init_options).expect("init should succeed");

        // --- Step B: Build the UCX file ---
        // 步骤 B：构建 UCX 文件。
        let build_options = ucx_build::BuildOptions::default();
        let ucx_path =
            ucx_build::build(&project_dir, &build_options).expect("build should succeed");
        assert!(ucx_path.exists(), "built .ucx file should exist");

        // --- Step C: Generate key pair and certificate ---
        // 步骤 C：生成密钥对和证书。
        let key_path = tmp.path().join("signer.pem");
        let cert_path = tmp.path().join("signer.cert.pem");
        keygen(&key_path).expect("keygen should succeed");
        create_cert(&key_path, "Test Signer", 365, &cert_path).expect("create_cert should succeed");

        // --- Step D: Sign the UCX file ---
        // 步骤 D：签名 UCX 文件。
        sign(&ucx_path, &key_path, &cert_path, "AUTHOR").expect("sign should succeed");

        // --- Step E: Verify the signed file can still be opened by ucx_parse ---
        // 步骤 E：验证签名后的文件仍可被 ucx_parse 打开。
        let archive = ucx_parse::open(&ucx_path);
        assert!(
            archive.is_ok(),
            "signed UCX file should still be parseable: {:?}",
            archive.err()
        );

        let archive = archive.unwrap();
        assert_eq!(
            archive.codex().title.main,
            "签名测试",
            "codex title must be preserved after signing"
        );

        // --- Step F: Verify the signed file contains signature entries ---
        // 步骤 F：验证签名后的文件包含签名条目。
        let files = archive.list_files();
        assert!(
            files.iter().any(|f| f == "META-INF/signatures/AUTHOR.SF"),
            "signed archive should contain AUTHOR.SF"
        );
        assert!(
            files.iter().any(|f| f == "META-INF/signatures/AUTHOR.EC"),
            "signed archive should contain AUTHOR.EC"
        );
        assert!(
            files.iter().any(|f| f == "META-INF/certs/AUTHOR.cert.pem"),
            "signed archive should contain AUTHOR.cert.pem"
        );

        // --- Step G: Verify the signing block exists in the binary data ---
        // 步骤 G：验证签名块存在于二进制数据中。
        let final_data = std::fs::read(&ucx_path).unwrap();
        let block_result = zip_binary::find_signing_block(&final_data)
            .expect("find_signing_block should not error");
        assert!(
            block_result.is_some(),
            "signed file must contain a Layer 2 signing block"
        );
    }
}
