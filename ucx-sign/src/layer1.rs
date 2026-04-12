//! Layer 1: JAR-style Signature File (SF) Generation and Ed25519 Signing.
//!
//! This module implements the first layer of the UCX dual-layer signing system,
//! modeled after the JAR signing specification:
//!
//! 1. **Generate SF**: Compute BLAKE3 digests of `MANIFEST.MF` content and
//!    produce a Signature File (`.SF`) in RFC 822 header format.
//! 2. **Sign SF**: Sign the `.SF` content with an Ed25519 private key,
//!    producing a binary signature blob (`.EC`) that bundles the signature
//!    and the signer's X.509 certificate.
//! 3. **Verify SF**: Parse the `.EC` blob, extract the certificate and
//!    signature, and verify the Ed25519 signature against the `.SF` content.
//! 4. **Parse SF**: Extract structured fields from the `.SF` text.
//!
//! Layer 1: JAR 式签名文件（SF）生成与 Ed25519 签名。
//!
//! 本模块实现 UCX 双层签名体系的第一层，参照 JAR 签名规范：
//!
//! 1. **生成 SF**：计算 `MANIFEST.MF` 内容的 BLAKE3 摘要，
//!    生成 RFC 822 头格式的签名文件（`.SF`）。
//! 2. **签署 SF**：用 Ed25519 私钥签署 `.SF` 内容，
//!    生成包含签名和签名者 X.509 证书的二进制签名块（`.EC`）。
//! 3. **验证 SF**：解析 `.EC` 块，提取证书和签名，
//!    验证 Ed25519 签名是否与 `.SF` 内容匹配。
//! 4. **解析 SF**：从 `.SF` 文本中提取结构化字段。

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use ed25519_dalek::{Signer, Verifier, VerifyingKey};

use crate::SignError;

// =============================================================================
// Constants / 常量
// =============================================================================

/// Signature algorithm identifier for Ed25519 + BLAKE3.
/// Ed25519 + BLAKE3 的签名算法标识符。
const ALGORITHM_ID_ED25519_BLAKE3: u32 = 0x0001;

/// Size of an Ed25519 signature in bytes.
/// Ed25519 签名的字节大小。
const ED25519_SIGNATURE_LEN: usize = 64;

// =============================================================================
// Public data types / 公开数据类型
// =============================================================================

/// Result of verifying an SF signature (.EC blob).
///
/// SF 签名验证结果（.EC 块）。
#[derive(Debug)]
pub struct VerifySfResult {
    /// Whether the signature is valid.
    /// 签名是否有效。
    pub valid: bool,

    /// Signature algorithm identifier extracted from the blob.
    /// 从签名块中提取的签名算法标识符。
    pub algorithm_id: u32,

    /// DER-encoded X.509 certificate extracted from the blob.
    /// 从签名块中提取的 DER 编码 X.509 证书。
    pub cert_der: Vec<u8>,
}

/// Parsed contents of a Signature File (.SF).
///
/// 签名文件（.SF）的解析内容。
#[derive(Debug)]
pub struct SfData {
    /// Value of `Signature-Version` header.
    /// `Signature-Version` 头的值。
    pub signature_version: String,

    /// Value of `Hash-Algorithm` header.
    /// `Hash-Algorithm` 头的值。
    pub hash_algorithm: String,

    /// Base64-encoded BLAKE3 digest of the entire MANIFEST.MF.
    /// 整个 MANIFEST.MF 的 BLAKE3 摘要（Base64 编码）。
    pub manifest_digest: String,

    /// Base64-encoded BLAKE3 digest of the main attributes section
    /// (content before the first blank line) of MANIFEST.MF.
    /// MANIFEST.MF 主属性段（第一个空行之前的内容）的 BLAKE3 摘要（Base64 编码）。
    pub manifest_main_attr_digest: String,
}

// =============================================================================
// SF generation / SF 生成
// =============================================================================

/// Generate a Signature File (.SF) from MANIFEST.MF content.
///
/// Computes the BLAKE3 digest of the entire MANIFEST.MF and of its main
/// attributes section (everything before the first blank line), then formats
/// the result as an RFC 822-style header block.
///
/// 从 MANIFEST.MF 内容生成签名文件（.SF）。
/// 计算整个 MANIFEST.MF 的 BLAKE3 摘要以及其主属性段（第一个空行之前的所有内容）的摘要，
/// 然后以 RFC 822 头格式输出结果。
///
/// # Arguments / 参数
///
/// * `manifest_content` - Raw bytes of the MANIFEST.MF file.
///   MANIFEST.MF 文件的原始字节。
/// * `ucx_version` - UCX format version string (e.g. "1.0").
///   UCX 格式版本字符串（如 "1.0"）。
///
/// # Returns / 返回
///
/// The SF content as a UTF-8 string.
/// SF 内容（UTF-8 字符串）。
pub fn generate_sf(manifest_content: &[u8], ucx_version: &str) -> String {
    // --- Compute BLAKE3 digest of the entire MANIFEST.MF ---
    // 计算整个 MANIFEST.MF 的 BLAKE3 摘要。
    let full_hash = blake3::hash(manifest_content);
    let full_digest_b64 = BASE64_STANDARD.encode(full_hash.as_bytes());

    // --- Extract the main attributes section ---
    // 提取主属性段。
    // The main attributes section is everything before the first blank line.
    // A blank line is defined as "\r\n\r\n" or "\n\n".
    // 主属性段是第一个空行之前的所有内容。
    // 空行定义为 "\r\n\r\n" 或 "\n\n"。
    let main_attr_bytes = extract_main_attributes(manifest_content);

    // --- Compute BLAKE3 digest of the main attributes section ---
    // 计算主属性段的 BLAKE3 摘要。
    let main_attr_hash = blake3::hash(main_attr_bytes);
    let main_attr_digest_b64 = BASE64_STANDARD.encode(main_attr_hash.as_bytes());

    // --- Build the SF content string ---
    // 构建 SF 内容字符串。
    format!(
        "Signature-Version: 1.0\n\
         UCX-Version: {ucx_version}\n\
         Hash-Algorithm: BLAKE3\n\
         Created-By: unicodex {version}\n\
         \n\
         BLAKE3-Digest-Manifest: {full_digest_b64}\n\
         BLAKE3-Digest-Manifest-Main-Attr: {main_attr_digest_b64}\n",
        version = env!("CARGO_PKG_VERSION"),
    )
}

// =============================================================================
// SF signing / SF 签署
// =============================================================================

/// Sign SF file content with an Ed25519 private key.
///
/// Produces a binary blob containing:
/// - `signature_algorithm_id`: `u32` LE — algorithm identifier (0x0001 for Ed25519+BLAKE3)
/// - `signature_length`: `u32` LE — length of the signature (always 64 for Ed25519)
/// - `signature_bytes`: `[u8; 64]` — the Ed25519 signature over the SF content
/// - `cert_length`: `u32` LE — length of the certificate DER bytes
/// - `cert_der`: `[u8]` — the signer's X.509 certificate in DER encoding
///
/// 用 Ed25519 私钥签署 SF 文件内容。
///
/// 生成的二进制块包含：
/// - `signature_algorithm_id`: `u32` 小端 — 算法标识符（Ed25519+BLAKE3 为 0x0001）
/// - `signature_length`: `u32` 小端 — 签名长度（Ed25519 固定为 64）
/// - `signature_bytes`: `[u8; 64]` — SF 内容的 Ed25519 签名
/// - `cert_length`: `u32` 小端 — 证书 DER 字节长度
/// - `cert_der`: `[u8]` — 签名者的 X.509 证书（DER 编码）
///
/// # Arguments / 参数
///
/// * `sf_content`  - Raw bytes of the SF file to sign.
///   要签署的 SF 文件原始字节。
/// * `signing_key` - Ed25519 private key used for signing.
///   用于签名的 Ed25519 私钥。
/// * `cert_der`    - DER-encoded X.509 certificate of the signer.
///   签名者的 DER 编码 X.509 证书。
///
/// # Returns / 返回
///
/// Binary signature blob on success.
/// 成功时返回二进制签名块。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if the Ed25519 signing operation fails.
/// 签名操作失败时返回 `SignError::SigningFailed`。
pub fn sign_sf(
    sf_content: &[u8],
    signing_key: &ed25519_dalek::SigningKey,
    cert_der: &[u8],
) -> Result<Vec<u8>, SignError> {
    // --- Sign the SF content with Ed25519 ---
    // 用 Ed25519 签署 SF 内容。
    let signature = signing_key.try_sign(sf_content).map_err(|e| {
        SignError::SigningFailed(format!("Ed25519 signing failed: {e}"))
    })?;
    let sig_bytes = signature.to_bytes();

    // --- Build the binary blob ---
    // 构建二进制块。
    // Total size: 4 (algo id) + 4 (sig len) + 64 (sig) + 4 (cert len) + cert_der.len()
    // 总大小：4（算法 ID）+ 4（签名长度）+ 64（签名）+ 4（证书长度）+ cert_der.len()
    let total_len = 4 + 4 + ED25519_SIGNATURE_LEN + 4 + cert_der.len();
    let mut blob = Vec::with_capacity(total_len);

    // Write signature algorithm id (u32 LE).
    // 写入签名算法 ID（u32 小端）。
    blob.extend_from_slice(&ALGORITHM_ID_ED25519_BLAKE3.to_le_bytes());

    // Write signature length (u32 LE).
    // 写入签名长度（u32 小端）。
    blob.extend_from_slice(&(ED25519_SIGNATURE_LEN as u32).to_le_bytes());

    // Write signature bytes.
    // 写入签名字节。
    blob.extend_from_slice(&sig_bytes);

    // Write certificate length (u32 LE).
    // 写入证书长度（u32 小端）。
    blob.extend_from_slice(&(cert_der.len() as u32).to_le_bytes());

    // Write certificate DER bytes.
    // 写入证书 DER 字节。
    blob.extend_from_slice(cert_der);

    Ok(blob)
}

// =============================================================================
// SF verification / SF 验证
// =============================================================================

/// Verify an SF signature against the given SF content.
///
/// Parses the binary `.EC` blob produced by [`sign_sf`], extracts the
/// Ed25519 signature and the X.509 certificate, then verifies the signature
/// using the public key embedded in the certificate.
///
/// 验证 SF 签名是否与给定的 SF 内容匹配。
/// 解析由 [`sign_sf`] 生成的二进制 `.EC` 块，提取 Ed25519 签名和 X.509 证书，
/// 然后使用证书中嵌入的公钥验证签名。
///
/// # Arguments / 参数
///
/// * `sf_content` - Raw bytes of the SF file that was signed.
///   被签署的 SF 文件原始字节。
/// * `ec_data`    - Binary blob produced by [`sign_sf`].
///   由 [`sign_sf`] 生成的二进制块。
///
/// # Returns / 返回
///
/// A [`VerifySfResult`] indicating validity, algorithm, and the embedded certificate.
/// 返回 [`VerifySfResult`]，表明有效性、算法和嵌入的证书。
///
/// # Errors / 错误
///
/// Returns `SignError::InvalidKey` if the blob is malformed or the
/// public key cannot be extracted from the certificate.
/// 如果签名块格式错误或无法从证书中提取公钥，返回 `SignError::InvalidKey`。
pub fn verify_sf_signature(
    sf_content: &[u8],
    ec_data: &[u8],
) -> Result<VerifySfResult, SignError> {
    // --- Parse the binary blob ---
    // 解析二进制块。

    // Minimum blob size: 4 (algo) + 4 (sig len) + 0 (sig) + 4 (cert len) = 12
    // 最小块大小：4（算法）+ 4（签名长度）+ 0（签名）+ 4（证书长度）= 12
    if ec_data.len() < 12 {
        return Err(SignError::InvalidKey(
            "EC blob too short: must be at least 12 bytes".to_string(),
        ));
    }

    // Read signature algorithm id (u32 LE).
    // 读取签名算法 ID（u32 小端）。
    let algorithm_id = u32::from_le_bytes(
        ec_data[0..4]
            .try_into()
            .expect("slice is exactly 4 bytes"),
    );

    // Read signature length (u32 LE).
    // 读取签名长度（u32 小端）。
    let sig_len = u32::from_le_bytes(
        ec_data[4..8]
            .try_into()
            .expect("slice is exactly 4 bytes"),
    ) as usize;

    // Validate that the blob contains enough data for the signature.
    // 验证块中包含足够的签名数据。
    let sig_end = 8 + sig_len;
    if ec_data.len() < sig_end + 4 {
        return Err(SignError::InvalidKey(format!(
            "EC blob truncated: expected at least {} bytes for signature + cert_length, got {}",
            sig_end + 4,
            ec_data.len()
        )));
    }

    // Extract signature bytes.
    // 提取签名字节。
    let sig_bytes = &ec_data[8..sig_end];

    // Read certificate length (u32 LE).
    // 读取证书长度（u32 小端）。
    let cert_len = u32::from_le_bytes(
        ec_data[sig_end..sig_end + 4]
            .try_into()
            .expect("slice is exactly 4 bytes"),
    ) as usize;

    // Validate that the blob contains enough data for the certificate.
    // 验证块中包含足够的证书数据。
    let cert_start = sig_end + 4;
    let cert_end = cert_start + cert_len;
    if ec_data.len() < cert_end {
        return Err(SignError::InvalidKey(format!(
            "EC blob truncated: expected {} bytes for certificate, got {}",
            cert_len,
            ec_data.len() - cert_start
        )));
    }

    // Extract certificate DER bytes.
    // 提取证书 DER 字节。
    let cert_der = ec_data[cert_start..cert_end].to_vec();

    // --- Extract the public key from the certificate ---
    // 从证书中提取公钥。
    let verifying_key = extract_verifying_key_from_cert_der(&cert_der)?;

    // --- Parse the Ed25519 signature ---
    // 解析 Ed25519 签名。
    let signature = ed25519_dalek::Signature::from_bytes(
        sig_bytes
            .try_into()
            .map_err(|_| {
                SignError::InvalidKey(format!(
                    "invalid Ed25519 signature length: expected {ED25519_SIGNATURE_LEN}, got {}",
                    sig_bytes.len()
                ))
            })?,
    );

    // --- Verify the signature ---
    // 验证签名。
    let valid = verifying_key.verify(sf_content, &signature).is_ok();

    Ok(VerifySfResult {
        valid,
        algorithm_id,
        cert_der,
    })
}

// =============================================================================
// SF parsing / SF 解析
// =============================================================================

/// Parse SF content and extract structured fields.
///
/// Expects the SF content to follow the RFC 822-style header format produced
/// by [`generate_sf`].
///
/// 解析 SF 内容并提取结构化字段。
/// SF 内容应遵循 [`generate_sf`] 生成的 RFC 822 头格式。
///
/// # Arguments / 参数
///
/// * `sf_content` - The SF content as a UTF-8 string.
///   SF 内容（UTF-8 字符串）。
///
/// # Returns / 返回
///
/// Parsed [`SfData`] on success.
/// 成功时返回解析后的 [`SfData`]。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if required headers are missing.
/// 缺少必需的头时返回 `SignError::SigningFailed`。
pub fn parse_sf(sf_content: &str) -> Result<SfData, SignError> {
    // Helper closure: find a header value by key name.
    // 辅助闭包：按键名查找头部值。
    let find_header = |key: &str| -> Result<String, SignError> {
        for line in sf_content.lines() {
            // Split at the first colon to get key-value pair, and check for a match.
            // 在第一个冒号处分割以获取键值对，并检查是否匹配。
            if let Some((k, v)) = line.split_once(':')
                && k.trim().eq_ignore_ascii_case(key)
            {
                return Ok(v.trim().to_string());
            }
        }
        Err(SignError::SigningFailed(format!(
            "missing required SF header: {key}"
        )))
    };

    Ok(SfData {
        signature_version: find_header("Signature-Version")?,
        hash_algorithm: find_header("Hash-Algorithm")?,
        manifest_digest: find_header("BLAKE3-Digest-Manifest")?,
        manifest_main_attr_digest: find_header("BLAKE3-Digest-Manifest-Main-Attr")?,
    })
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Extract the main attributes section from MANIFEST.MF content.
///
/// The main attributes section is defined as everything before the first
/// blank line. A blank line is "\n\n" (Unix) or "\r\n\r\n" (Windows).
/// If no blank line is found, the entire content is treated as the main
/// attributes section.
///
/// 从 MANIFEST.MF 内容中提取主属性段。
/// 主属性段定义为第一个空行之前的所有内容。
/// 空行为 "\n\n"（Unix）或 "\r\n\r\n"（Windows）。
/// 如果未找到空行，则整个内容视为主属性段。
fn extract_main_attributes(content: &[u8]) -> &[u8] {
    // Search for the first occurrence of "\r\n\r\n" (Windows line endings).
    // 搜索第一个 "\r\n\r\n"（Windows 换行符）出现的位置。
    if let Some(pos) = find_subsequence(content, b"\r\n\r\n") {
        // Include the first "\r\n" as the line terminator of the last header line.
        // 包含第一个 "\r\n" 作为最后一行头的行终止符。
        return &content[..pos + 2];
    }

    // Search for the first occurrence of "\n\n" (Unix line endings).
    // 搜索第一个 "\n\n"（Unix 换行符）出现的位置。
    if let Some(pos) = find_subsequence(content, b"\n\n") {
        // Include the first "\n" as the line terminator of the last header line.
        // 包含第一个 "\n" 作为最后一行头的行终止符。
        return &content[..pos + 1];
    }

    // No blank line found — treat the entire content as the main section.
    // 未找到空行 — 将整个内容视为主属性段。
    content
}

/// Find the first occurrence of a byte subsequence in a byte slice.
///
/// Returns the starting index, or `None` if not found.
///
/// 在字节切片中查找子序列的第一个出现位置。
/// 找到返回起始索引，否则返回 `None`。
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Extract an Ed25519 verifying key from a DER-encoded X.509 certificate.
///
/// Parses the certificate, locates the Subject Public Key Info (SPKI),
/// and interprets the raw public key bytes as an Ed25519 verifying key.
///
/// 从 DER 编码的 X.509 证书中提取 Ed25519 验证密钥。
/// 解析证书，定位主体公钥信息（SPKI），
/// 并将原始公钥字节解释为 Ed25519 验证密钥。
fn extract_verifying_key_from_cert_der(cert_der: &[u8]) -> Result<VerifyingKey, SignError> {
    use der::Decode;

    // Parse the DER-encoded X.509 certificate.
    // 解析 DER 编码的 X.509 证书。
    let cert = x509_cert::Certificate::from_der(cert_der).map_err(|e| {
        SignError::InvalidKey(format!("failed to parse X.509 certificate: {e}"))
    })?;

    // Extract the raw public key bits from the SubjectPublicKeyInfo.
    // 从 SubjectPublicKeyInfo 中提取原始公钥位。
    let spki = &cert.tbs_certificate.subject_public_key_info;
    let pk_bytes = spki
        .subject_public_key
        .as_bytes()
        .ok_or_else(|| {
            SignError::InvalidKey(
                "subject public key bit string is not byte-aligned".to_string(),
            )
        })?;

    // Ed25519 public keys are exactly 32 bytes.
    // Ed25519 公钥恰好为 32 字节。
    let pk_array: [u8; 32] = pk_bytes.try_into().map_err(|_| {
        SignError::InvalidKey(format!(
            "invalid Ed25519 public key length: expected 32 bytes, got {}",
            pk_bytes.len()
        ))
    })?;

    // Construct the VerifyingKey from the raw bytes.
    // 从原始字节构建 VerifyingKey。
    VerifyingKey::from_bytes(&pk_array).map_err(|e| {
        SignError::InvalidKey(format!("invalid Ed25519 public key: {e}"))
    })
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cert::{CertOptions, create_self_signed_cert};
    use crate::keys::generate_ed25519_keypair;

    /// Test: generate_sf produces correct format with expected headers.
    /// 测试：generate_sf 生成的格式正确且包含预期的头部。
    #[test]
    fn test_generate_sf_correct_format() {
        let manifest = b"Manifest-Version: 1.0\nCreated-By: unicodex\n\nName: chapter1.txt\nBLAKE3-Digest: abc123\n";
        let sf = generate_sf(manifest, "1.0");

        // Verify required headers are present.
        // 验证必需的头部存在。
        assert!(
            sf.contains("Signature-Version: 1.0"),
            "SF must contain Signature-Version header"
        );
        assert!(
            sf.contains("UCX-Version: 1.0"),
            "SF must contain UCX-Version header"
        );
        assert!(
            sf.contains("Hash-Algorithm: BLAKE3"),
            "SF must contain Hash-Algorithm header"
        );
        assert!(
            sf.contains("Created-By: unicodex"),
            "SF must contain Created-By header"
        );
        assert!(
            sf.contains("BLAKE3-Digest-Manifest:"),
            "SF must contain BLAKE3-Digest-Manifest header"
        );
        assert!(
            sf.contains("BLAKE3-Digest-Manifest-Main-Attr:"),
            "SF must contain BLAKE3-Digest-Manifest-Main-Attr header"
        );
    }

    /// Test: generate_sf computes correct BLAKE3 digest (verify against manual hash).
    /// 测试：generate_sf 计算出正确的 BLAKE3 摘要（与手动计算的哈希对比验证）。
    #[test]
    fn test_generate_sf_correct_blake3_digest() {
        let manifest = b"Manifest-Version: 1.0\nCreated-By: test\n";
        let sf = generate_sf(manifest, "1.0");

        // Manually compute the expected BLAKE3 digest of the entire manifest.
        // 手动计算整个 manifest 的预期 BLAKE3 摘要。
        let expected_hash = blake3::hash(manifest);
        let expected_b64 = BASE64_STANDARD.encode(expected_hash.as_bytes());

        // Extract the BLAKE3-Digest-Manifest value from the SF output.
        // 从 SF 输出中提取 BLAKE3-Digest-Manifest 的值。
        let parsed = parse_sf(&sf).expect("parse_sf should succeed");

        assert_eq!(
            parsed.manifest_digest, expected_b64,
            "BLAKE3-Digest-Manifest must match the manually computed digest"
        );
    }

    /// Test: sign_sf + verify_sf_signature roundtrip succeeds.
    /// 测试：sign_sf + verify_sf_signature 往返验证成功。
    #[test]
    fn test_sign_verify_roundtrip() {
        // Generate key pair and certificate.
        // 生成密钥对和证书。
        let (signing_key, _) =
            generate_ed25519_keypair().expect("key generation should succeed");
        let cert_der = create_self_signed_cert(
            &signing_key,
            &CertOptions {
                common_name: "Layer1 Test".to_string(),
                days_valid: 30,
                organization: None,
            },
        )
        .expect("certificate generation should succeed");

        // Generate and sign an SF.
        // 生成并签署 SF。
        let manifest = b"Manifest-Version: 1.0\nTest: value\n";
        let sf = generate_sf(manifest, "1.0");
        let ec_blob =
            sign_sf(sf.as_bytes(), &signing_key, &cert_der).expect("sign_sf should succeed");

        // Verify the signature.
        // 验证签名。
        let result = verify_sf_signature(sf.as_bytes(), &ec_blob)
            .expect("verify_sf_signature should succeed");

        assert!(result.valid, "signature must be valid for matching content");
        assert_eq!(
            result.algorithm_id, ALGORITHM_ID_ED25519_BLAKE3,
            "algorithm id must be Ed25519+BLAKE3 (0x0001)"
        );
        assert_eq!(
            result.cert_der, cert_der,
            "certificate DER must match the original"
        );
    }

    /// Test: verify_sf_signature fails with tampered SF content.
    /// 测试：篡改 SF 内容后 verify_sf_signature 应验证失败。
    #[test]
    fn test_verify_fails_with_tampered_content() {
        // Generate key pair and certificate.
        // 生成密钥对和证书。
        let (signing_key, _) =
            generate_ed25519_keypair().expect("key generation should succeed");
        let cert_der = create_self_signed_cert(
            &signing_key,
            &CertOptions {
                common_name: "Tamper Test".to_string(),
                days_valid: 30,
                organization: None,
            },
        )
        .expect("certificate generation should succeed");

        // Sign original SF content.
        // 签署原始 SF 内容。
        let sf_original = b"Signature-Version: 1.0\nOriginal: true\n";
        let ec_blob = sign_sf(sf_original, &signing_key, &cert_der)
            .expect("sign_sf should succeed");

        // Verify with tampered content.
        // 用篡改的内容验证。
        let sf_tampered = b"Signature-Version: 1.0\nOriginal: false\n";
        let result = verify_sf_signature(sf_tampered, &ec_blob)
            .expect("verify_sf_signature should succeed (returns valid=false, not error)");

        assert!(
            !result.valid,
            "signature must be invalid for tampered content"
        );
    }

    /// Test: parse_sf extracts correct fields.
    /// 测试：parse_sf 正确提取各字段。
    #[test]
    fn test_parse_sf_extracts_fields() {
        let sf_text = "Signature-Version: 1.0\n\
                        UCX-Version: 1.0\n\
                        Hash-Algorithm: BLAKE3\n\
                        Created-By: unicodex 0.2.0-alpha.1\n\
                        \n\
                        BLAKE3-Digest-Manifest: AAAA\n\
                        BLAKE3-Digest-Manifest-Main-Attr: BBBB\n";

        let data = parse_sf(sf_text).expect("parse_sf should succeed");

        assert_eq!(data.signature_version, "1.0", "Signature-Version must be 1.0");
        assert_eq!(data.hash_algorithm, "BLAKE3", "Hash-Algorithm must be BLAKE3");
        assert_eq!(data.manifest_digest, "AAAA", "manifest digest must match");
        assert_eq!(
            data.manifest_main_attr_digest, "BBBB",
            "manifest main attr digest must match"
        );
    }

    /// Test: SF main attr digest only covers content before first blank line.
    /// 测试：SF 主属性摘要仅覆盖第一个空行之前的内容。
    #[test]
    fn test_main_attr_digest_before_blank_line() {
        // Build a manifest with a clear separation between main section and entries.
        // 构建一个主属性段和条目之间有明确分隔的 manifest。
        let main_section = "Manifest-Version: 1.0\nCreated-By: unicodex\n";
        let entry_section = "Name: chapter1.txt\nBLAKE3-Digest: abc123\n";
        let manifest = format!("{main_section}\n{entry_section}");

        let sf = generate_sf(manifest.as_bytes(), "1.0");
        let parsed = parse_sf(&sf).expect("parse_sf should succeed");

        // Manually compute the digest of just the main attributes section.
        // The main section ends with the trailing "\n" before the blank "\n".
        // 手动计算仅主属性段的摘要。
        // 主属性段以尾部 "\n"（空行 "\n" 之前）结束。
        let expected_main_hash = blake3::hash(main_section.as_bytes());
        let expected_main_b64 = BASE64_STANDARD.encode(expected_main_hash.as_bytes());

        assert_eq!(
            parsed.manifest_main_attr_digest, expected_main_b64,
            "main attr digest must only cover content before the first blank line"
        );

        // Also verify the full digest covers the entire manifest.
        // 同时验证完整摘要覆盖整个 manifest。
        let expected_full_hash = blake3::hash(manifest.as_bytes());
        let expected_full_b64 = BASE64_STANDARD.encode(expected_full_hash.as_bytes());

        assert_eq!(
            parsed.manifest_digest, expected_full_b64,
            "full manifest digest must cover the entire manifest content"
        );
    }
}
