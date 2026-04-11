//! Self-Signed X.509 Certificate Generation.
//!
//! This module provides functions for:
//! - Creating self-signed X.509 v3 certificates for Ed25519 key pairs
//! - Saving / loading certificates in PEM format
//! - Computing certificate fingerprints (BLAKE3, SHA-256)
//! - Extracting subject CN from certificates
//!
//! 自签名 X.509 证书生成。
//!
//! 本模块提供以下功能：
//! - 为 Ed25519 密钥对创建自签名 X.509 v3 证书
//! - 以 PEM 格式保存/加载证书
//! - 计算证书指纹（BLAKE3、SHA-256）
//! - 从证书中提取主体 CN

use std::path::Path;

use der::Decode;
use ed25519_dalek::SigningKey;
use pkcs8::EncodePrivateKey;
use rustls_pki_types::PrivatePkcs8KeyDer;
use sha2::{Digest, Sha256};

use crate::SignError;

// =============================================================================
// Certificate options / 证书选项
// =============================================================================

/// Options for creating a self-signed certificate.
///
/// 创建自签名证书的选项。
pub struct CertOptions {
    /// Common Name (CN) field in the certificate subject.
    /// 证书主体中的通用名称（CN）字段。
    pub common_name: String,

    /// Validity period in days (default: 365).
    /// 有效期（天数，默认 365）。
    pub days_valid: u32,

    /// Organization (O) field in the certificate subject (optional).
    /// 证书主体中的组织（O）字段（可选）。
    pub organization: Option<String>,
}

impl Default for CertOptions {
    /// Create default certificate options with placeholder values.
    /// 使用占位值创建默认的证书选项。
    fn default() -> Self {
        Self {
            common_name: "Unicodex Signer".to_string(),
            days_valid: 365,
            organization: None,
        }
    }
}

// =============================================================================
// Certificate generation / 证书生成
// =============================================================================

/// Create a self-signed X.509 v3 certificate for an Ed25519 key pair.
///
/// The certificate includes:
/// - Key Usage: digitalSignature
/// - Extended Key Usage: codeSigning (OID 1.3.6.1.5.5.7.3.3)
/// - Basic Constraints: CA:FALSE
/// - Subject: CN=<common_name>, O=<organization if provided>
///
/// 为 Ed25519 密钥对创建自签名 X.509 v3 证书。
///
/// 证书包含：
/// - 密钥用途：digitalSignature
/// - 扩展密钥用途：codeSigning（OID 1.3.6.1.5.5.7.3.3）
/// - 基本约束：CA:FALSE
/// - 主体：CN=<common_name>, O=<organization if provided>
///
/// # Arguments / 参数
///
/// * `signing_key` - The Ed25519 signing key to create the certificate for.
///   要为其创建证书的 Ed25519 签名密钥。
/// * `options`     - Certificate options (CN, validity, organization).
///   证书选项（CN、有效期、组织）。
///
/// # Returns / 返回
///
/// DER-encoded certificate bytes on success.
/// 成功时返回 DER 编码的证书字节。
///
/// # Errors / 错误
///
/// Returns `SignError::CertificateError` if certificate generation fails.
/// 证书生成失败时返回 `SignError::CertificateError`。
pub fn create_self_signed_cert(
    signing_key: &SigningKey,
    options: &CertOptions,
) -> Result<Vec<u8>, SignError> {
    // --- Convert the ed25519-dalek key to PKCS#8 DER for rcgen ---
    // 将 ed25519-dalek 密钥转换为 PKCS#8 DER 格式，供 rcgen 使用。
    let pkcs8_der = signing_key
        .to_pkcs8_der()
        .map_err(|e| SignError::CertificateError(format!("failed to encode key to PKCS#8 DER: {e}")))?;

    // Wrap in PrivatePkcs8KeyDer for rcgen compatibility.
    // 包装为 PrivatePkcs8KeyDer 以兼容 rcgen。
    let pkcs8_key_der = PrivatePkcs8KeyDer::from(pkcs8_der.as_bytes().to_vec());

    // --- Create an rcgen KeyPair from the PKCS#8 DER bytes ---
    // 从 PKCS#8 DER 字节创建 rcgen KeyPair。
    let rcgen_key_pair = rcgen::KeyPair::from_pkcs8_der_and_sign_algo(
        &pkcs8_key_der,
        &rcgen::PKCS_ED25519,
    )
    .map_err(|e| SignError::CertificateError(format!("failed to create rcgen KeyPair: {e}")))?;

    // --- Configure certificate parameters ---
    // 配置证书参数。
    let mut params = rcgen::CertificateParams::default();

    // Set the distinguished name (subject).
    // 设置可分辨名称（主体）。
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, &options.common_name);
    if let Some(ref org) = options.organization {
        dn.push(rcgen::DnType::OrganizationName, org);
    }
    params.distinguished_name = dn;

    // Set validity period.
    // 设置有效期。
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + time::Duration::days(i64::from(options.days_valid));

    // Set Basic Constraints: CA:FALSE.
    // 设置基本约束：CA:FALSE。
    params.is_ca = rcgen::IsCa::NoCa;

    // Set Key Usage: digitalSignature.
    // 设置密钥用途：digitalSignature。
    params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];

    // Set Extended Key Usage: codeSigning.
    // 设置扩展密钥用途：codeSigning。
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::CodeSigning];

    // --- Generate the self-signed certificate ---
    // 生成自签名证书。
    let cert = params
        .self_signed(&rcgen_key_pair)
        .map_err(|e| SignError::CertificateError(format!("failed to generate self-signed certificate: {e}")))?;

    // Return DER-encoded certificate bytes.
    // 返回 DER 编码的证书字节。
    Ok(cert.der().to_vec())
}

// =============================================================================
// Certificate I/O / 证书读写
// =============================================================================

/// Save a certificate to a PEM file.
///
/// The output file will contain a PEM block starting with
/// `-----BEGIN CERTIFICATE-----`.
///
/// 将证书保存到 PEM 文件。
/// 输出文件将包含以 `-----BEGIN CERTIFICATE-----` 开头的 PEM 块。
///
/// # Arguments / 参数
///
/// * `cert_der` - DER-encoded certificate bytes.
///   DER 编码的证书字节。
/// * `path`     - Destination file path.
///   目标文件路径。
///
/// # Errors / 错误
///
/// Returns `SignError::PemError` if PEM encoding fails.
/// Returns `SignError::Io` if file writing fails.
/// PEM 编码失败返回 `SignError::PemError`，文件写入失败返回 `SignError::Io`。
pub fn save_certificate(cert_der: &[u8], path: &Path) -> Result<(), SignError> {
    // Encode the DER bytes as a PEM CERTIFICATE block.
    // 将 DER 字节编码为 PEM CERTIFICATE 块。
    let pem_string = pem_rfc7468::encode_string("CERTIFICATE", pem_rfc7468::LineEnding::LF, cert_der)
        .map_err(|e| SignError::PemError(format!("failed to encode certificate to PEM: {e}")))?;

    // Write the PEM string to the specified file.
    // 将 PEM 字符串写入指定文件。
    std::fs::write(path, pem_string.as_bytes())?;

    Ok(())
}

/// Load a certificate from a PEM file. Returns DER-encoded bytes.
///
/// The file must contain a PEM block starting with
/// `-----BEGIN CERTIFICATE-----`.
///
/// 从 PEM 文件加载证书。返回 DER 编码的字节。
/// 文件必须包含以 `-----BEGIN CERTIFICATE-----` 开头的 PEM 块。
///
/// # Arguments / 参数
///
/// * `path` - Path to the PEM file.
///   PEM 文件路径。
///
/// # Errors / 错误
///
/// Returns `SignError::KeyNotFound` if the file does not exist.
/// Returns `SignError::PemError` if PEM decoding fails.
/// 文件不存在返回 `SignError::KeyNotFound`，PEM 解码失败返回 `SignError::PemError`。
pub fn load_certificate(path: &Path) -> Result<Vec<u8>, SignError> {
    // Check if the file exists before attempting to read.
    // 在尝试读取之前检查文件是否存在。
    if !path.exists() {
        return Err(SignError::KeyNotFound(format!(
            "certificate file not found: {}",
            path.display()
        )));
    }

    // Read the PEM file contents.
    // 读取 PEM 文件内容。
    let pem_contents = std::fs::read_to_string(path)?;

    // Decode the PEM block to get the DER bytes.
    // 解码 PEM 块以获取 DER 字节。
    let (label, der_bytes) = pem_rfc7468::decode_vec(pem_contents.as_bytes())
        .map_err(|e| SignError::PemError(format!("failed to decode certificate PEM: {e}")))?;

    // Verify the PEM label is "CERTIFICATE".
    // 验证 PEM 标签为 "CERTIFICATE"。
    if label != "CERTIFICATE" {
        return Err(SignError::PemError(format!(
            "expected PEM label 'CERTIFICATE', got '{label}'"
        )));
    }

    Ok(der_bytes)
}

// =============================================================================
// Fingerprints / 指纹
// =============================================================================

/// Compute the BLAKE3 fingerprint of a DER-encoded certificate.
///
/// Returns the fingerprint as a lowercase hexadecimal string.
///
/// 计算 DER 编码证书的 BLAKE3 指纹。
/// 返回小写十六进制字符串形式的指纹。
///
/// # Arguments / 参数
///
/// * `cert_der` - DER-encoded certificate bytes.
///   DER 编码的证书字节。
pub fn cert_fingerprint_blake3(cert_der: &[u8]) -> String {
    // Compute the BLAKE3 hash of the DER bytes.
    // 计算 DER 字节的 BLAKE3 哈希。
    let hash = blake3::hash(cert_der);
    hash.to_hex().to_string()
}

/// Compute the SHA-256 fingerprint of a DER-encoded certificate.
///
/// Returns the fingerprint as a lowercase hexadecimal string.
///
/// 计算 DER 编码证书的 SHA-256 指纹。
/// 返回小写十六进制字符串形式的指纹。
///
/// # Arguments / 参数
///
/// * `cert_der` - DER-encoded certificate bytes.
///   DER 编码的证书字节。
pub fn cert_fingerprint_sha256(cert_der: &[u8]) -> String {
    // Compute the SHA-256 hash of the DER bytes.
    // 计算 DER 字节的 SHA-256 哈希。
    let hash = Sha256::digest(cert_der);

    // Convert to lowercase hex string.
    // 转换为小写十六进制字符串。
    hex_encode(&hash)
}

/// Encode bytes as a lowercase hexadecimal string.
///
/// 将字节编码为小写十六进制字符串。
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// =============================================================================
// Certificate inspection / 证书检查
// =============================================================================

/// Extract the subject Common Name (CN) from a DER-encoded certificate.
///
/// 从 DER 编码的证书中提取主体通用名称（CN）。
///
/// # Arguments / 参数
///
/// * `cert_der` - DER-encoded certificate bytes.
///   DER 编码的证书字节。
///
/// # Returns / 返回
///
/// The CN string on success.
/// 成功时返回 CN 字符串。
///
/// # Errors / 错误
///
/// Returns `SignError::CertificateError` if parsing fails or CN is not found.
/// 解析失败或未找到 CN 时返回 `SignError::CertificateError`。
pub fn cert_subject_cn(cert_der: &[u8]) -> Result<String, SignError> {
    // Parse the DER-encoded certificate using x509-cert.
    // 使用 x509-cert 解析 DER 编码的证书。
    let cert = x509_cert::Certificate::from_der(cert_der)
        .map_err(|e| SignError::CertificateError(format!("failed to parse certificate DER: {e}")))?;

    // The OID for CommonName is 2.5.4.3.
    // CommonName 的 OID 是 2.5.4.3。
    let cn_oid = der::oid::ObjectIdentifier::new_unwrap("2.5.4.3");

    // Iterate over RDN sequences in the subject to find the CN attribute.
    // 遍历主体中的 RDN 序列以查找 CN 属性。
    for rdn in cert.tbs_certificate.subject.0.iter() {
        for atv in rdn.0.iter() {
            if atv.oid == cn_oid {
                // Decode the CN value as a UTF-8 string.
                // 将 CN 值解码为 UTF-8 字符串。
                let cn_value = std::str::from_utf8(atv.value.value())
                    .map_err(|e| {
                        SignError::CertificateError(format!(
                            "CN value is not valid UTF-8: {e}"
                        ))
                    })?;
                return Ok(cn_value.to_string());
            }
        }
    }

    // CN not found in the certificate subject.
    // 证书主体中未找到 CN。
    Err(SignError::CertificateError(
        "Common Name (CN) not found in certificate subject".to_string(),
    ))
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::generate_ed25519_keypair;
    use tempfile::TempDir;

    /// Test: create_self_signed_cert generates valid DER bytes.
    /// 测试：create_self_signed_cert 生成有效的 DER 字节。
    #[test]
    fn test_create_self_signed_cert_generates_valid_der() {
        let (signing_key, _) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        let options = CertOptions {
            common_name: "Test Signer".to_string(),
            days_valid: 365,
            organization: None,
        };

        let cert_der = create_self_signed_cert(&signing_key, &options)
            .expect("certificate generation should succeed");

        // The DER bytes should not be empty.
        // DER 字节不应为空。
        assert!(!cert_der.is_empty(), "DER certificate must not be empty");

        // The DER should be parseable as an X.509 certificate.
        // DER 应当能被解析为 X.509 证书。
        let parsed = x509_cert::Certificate::from_der(&cert_der);
        assert!(parsed.is_ok(), "DER bytes must be a valid X.509 certificate");
    }

    /// Test: save + load certificate PEM roundtrip.
    /// 测试：save + load 证书 PEM 往返一致。
    #[test]
    fn test_certificate_save_load_roundtrip() {
        let (signing_key, _) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        let options = CertOptions {
            common_name: "Roundtrip Test".to_string(),
            days_valid: 30,
            organization: Some("Test Org".to_string()),
        };

        let cert_der = create_self_signed_cert(&signing_key, &options)
            .expect("certificate generation should succeed");

        // Create a temporary directory for the test.
        // 为测试创建临时目录。
        let tmp_dir = TempDir::new().expect("failed to create temp dir");
        let cert_path = tmp_dir.path().join("cert.pem");

        // Save and reload the certificate.
        // 保存并重新加载证书。
        save_certificate(&cert_der, &cert_path).expect("save_certificate should succeed");
        let loaded_der = load_certificate(&cert_path).expect("load_certificate should succeed");

        // The loaded DER should match the original.
        // 加载的 DER 应与原始一致。
        assert_eq!(
            cert_der, loaded_der,
            "loaded certificate DER must match the original"
        );
    }

    /// Test: fingerprint calculation is deterministic.
    /// 测试：指纹计算是确定性的。
    #[test]
    fn test_fingerprint_is_deterministic() {
        let (signing_key, _) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        let options = CertOptions {
            common_name: "Fingerprint Test".to_string(),
            days_valid: 365,
            organization: None,
        };

        let cert_der = create_self_signed_cert(&signing_key, &options)
            .expect("certificate generation should succeed");

        // Compute fingerprints twice; they should be identical.
        // 计算两次指纹；它们应当完全一致。
        let blake3_1 = cert_fingerprint_blake3(&cert_der);
        let blake3_2 = cert_fingerprint_blake3(&cert_der);
        assert_eq!(blake3_1, blake3_2, "BLAKE3 fingerprint must be deterministic");

        let sha256_1 = cert_fingerprint_sha256(&cert_der);
        let sha256_2 = cert_fingerprint_sha256(&cert_der);
        assert_eq!(sha256_1, sha256_2, "SHA-256 fingerprint must be deterministic");

        // Fingerprints should be non-empty hex strings.
        // 指纹应为非空的十六进制字符串。
        assert!(!blake3_1.is_empty(), "BLAKE3 fingerprint must not be empty");
        assert!(!sha256_1.is_empty(), "SHA-256 fingerprint must not be empty");

        // SHA-256 fingerprint should be 64 hex chars (256 bits / 4 bits per char).
        // SHA-256 指纹应为 64 个十六进制字符（256 位 / 每字符 4 位）。
        assert_eq!(sha256_1.len(), 64, "SHA-256 fingerprint must be 64 hex chars");

        // BLAKE3 fingerprint should be 64 hex chars (256 bits).
        // BLAKE3 指纹应为 64 个十六进制字符（256 位）。
        assert_eq!(blake3_1.len(), 64, "BLAKE3 fingerprint must be 64 hex chars");
    }

    /// Test: cert_subject_cn extracts the correct CN.
    /// 测试：cert_subject_cn 提取正确的 CN。
    #[test]
    fn test_cert_subject_cn_extracts_correct_cn() {
        let (signing_key, _) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        let cn = "My Unique Signer Name";
        let options = CertOptions {
            common_name: cn.to_string(),
            days_valid: 365,
            organization: None,
        };

        let cert_der = create_self_signed_cert(&signing_key, &options)
            .expect("certificate generation should succeed");

        let extracted_cn = cert_subject_cn(&cert_der)
            .expect("CN extraction should succeed");

        assert_eq!(
            extracted_cn, cn,
            "extracted CN must match the one used during creation"
        );
    }

    /// Test: certificate with organization includes the O field.
    /// 测试：带组织的证书包含 O 字段。
    #[test]
    fn test_certificate_with_organization() {
        let (signing_key, _) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        let options = CertOptions {
            common_name: "Org Test Signer".to_string(),
            days_valid: 365,
            organization: Some("Unicodex Foundation".to_string()),
        };

        let cert_der = create_self_signed_cert(&signing_key, &options)
            .expect("certificate generation should succeed");

        // Parse the certificate and check the Organization field.
        // 解析证书并检查组织字段。
        let cert = x509_cert::Certificate::from_der(&cert_der)
            .expect("certificate parsing should succeed");

        // OID for Organization is 2.5.4.10.
        // Organization 的 OID 是 2.5.4.10。
        let org_oid = der::oid::ObjectIdentifier::new_unwrap("2.5.4.10");

        let mut found_org = false;
        for rdn in cert.tbs_certificate.subject.0.iter() {
            for atv in rdn.0.iter() {
                if atv.oid == org_oid {
                    let org_value = std::str::from_utf8(atv.value.value())
                        .expect("O value should be valid UTF-8");
                    assert_eq!(
                        org_value, "Unicodex Foundation",
                        "Organization field must match"
                    );
                    found_org = true;
                }
            }
        }

        assert!(found_org, "certificate must include the Organization field");
    }
}
