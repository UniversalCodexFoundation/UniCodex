//! Paragraph-level encryption for UCX content files.
//! UCX 内容文件的段落级加密。
//!
//! Used inside `<encrypted>` tags in .ucxc and .mdx formats.
//! The encrypted content is stored as Base64-encoded binary:
//!   IV/Nonce (12 bytes) || Ciphertext || Auth Tag (16 bytes)
//!
//! 用于 .ucxc 和 .mdx 格式中的 `<encrypted>` 标签。
//! 加密内容以 Base64 编码的二进制数据存储：
//!   IV/Nonce (12 字节) || 密文 || 认证标签 (16 字节)

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

use crate::{Algorithm, CryptoError};

// =============================================================================
// Constants / 常量
// =============================================================================

/// Nonce/IV size in bytes (96-bit, shared by AES-GCM and ChaCha20-Poly1305).
/// Nonce/IV 大小（字节），96 位，AES-GCM 和 ChaCha20-Poly1305 共用。
const NONCE_SIZE: usize = 12;

/// Authentication tag size in bytes (128-bit).
/// 认证标签大小（字节），128 位。
const TAG_SIZE: usize = 16;

/// Minimum binary payload size: nonce (12) + tag (16) = 28 bytes.
/// No ciphertext means the original plaintext was empty, which is still valid.
///
/// 最小二进制负载大小：nonce (12) + tag (16) = 28 字节。
/// 无密文表示原始明文为空，这仍然是有效的。
const MIN_PAYLOAD_SIZE: usize = NONCE_SIZE + TAG_SIZE;

// =============================================================================
// Parsed structures / 解析结构
// =============================================================================

/// Parsed `<encrypted>` block from a .ucxc content file.
/// 从 .ucxc 内容文件解析的 `<encrypted>` 块。
///
/// This struct captures the information embedded in an `<encrypted>` XML tag,
/// including the algorithm, key access methods, and the Base64-encoded content.
///
/// 此结构体捕获嵌入在 `<encrypted>` XML 标签中的信息，
/// 包括算法、密钥访问方法和 Base64 编码的内容。
#[derive(Debug, Clone)]
pub struct EncryptedBlock {
    /// Algorithm specified in the tag attribute (e.g. "aes-256-gcm").
    /// 标签属性中指定的算法（例如 "aes-256-gcm"）。
    pub algorithm: String,

    /// Key access methods from `<key-access>` child elements.
    /// 来自 `<key-access>` 子元素的密钥访问方法。
    pub key_accesses: Vec<KeyAccessInfo>,

    /// Base64-encoded encrypted content from `<content>` element.
    /// 来自 `<content>` 元素的 Base64 编码加密内容。
    pub content_base64: String,
}

/// Key access information within an `<encrypted>` block.
/// `<encrypted>` 块内的密钥访问信息。
///
/// Describes how a reader can obtain the decryption key (e.g. password-based,
/// certificate-based, or token-based access).
///
/// 描述读者如何获取解密密钥（例如基于密码、基于证书或基于令牌的访问）。
#[derive(Debug, Clone)]
pub struct KeyAccessInfo {
    /// Access method type (e.g. "password", "certificate", "token").
    /// 访问方法类型（例如 "password"、"certificate"、"token"）。
    pub method: String,

    /// Additional attributes for this access method.
    /// 此访问方法的附加属性。
    pub attributes: std::collections::HashMap<String, String>,
}

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Encrypt plaintext for paragraph-level embedding.
/// Returns Base64-encoded string: IV(12) || ciphertext || tag(16).
///
/// 加密明文用于段落级嵌入。
/// 返回 Base64 编码字符串：IV(12) || 密文 || tag(16)。
///
/// Only AEAD algorithms (AES-256-GCM, ChaCha20-Poly1305) are supported.
/// AES-256-CBC is not supported for paragraph-level encryption because
/// it lacks built-in authenticated encryption, and paragraph-level use
/// requires compact, self-contained ciphertext with integrity guarantees.
///
/// 仅支持 AEAD 算法（AES-256-GCM、ChaCha20-Poly1305）。
/// AES-256-CBC 不支持段落级加密，因为它缺乏内建的认证加密，
/// 而段落级使用要求紧凑、自包含且具有完整性保证的密文。
///
/// # Arguments / 参数
///
/// * `plaintext`  - The text to encrypt / 待加密的文本
/// * `key`        - 256-bit encryption key / 256 位加密密钥
/// * `algorithm`  - The AEAD algorithm to use / 要使用的 AEAD 算法
///
/// # Returns / 返回
///
/// Base64-encoded string on success, or `CryptoError` on failure.
/// 成功返回 Base64 编码字符串，失败返回 `CryptoError`。
///
/// # Errors / 错误
///
/// * `CryptoError::UnsupportedAlgorithm` — if AES-256-CBC is specified.
///   如果指定了 AES-256-CBC。
pub fn encrypt_paragraph(
    plaintext: &str,
    key: &[u8; 32],
    algorithm: Algorithm,
) -> Result<String, CryptoError> {
    // 1. Reject AES-256-CBC — it is not an AEAD and cannot provide
    //    compact authenticated ciphertext for paragraph embedding.
    //    拒绝 AES-256-CBC —— 它不是 AEAD，无法为段落嵌入提供
    //    紧凑的认证密文。
    if algorithm == Algorithm::Aes256Cbc {
        return Err(CryptoError::UnsupportedAlgorithm(
            "AES-256-CBC is not supported for paragraph-level encryption".to_string(),
        ));
    }

    // 2. Encrypt using the appropriate AEAD engine.
    //    Each engine returns (ciphertext, nonce, tag) separately.
    //    使用相应的 AEAD 引擎加密。
    //    每个引擎分别返回 (密文, nonce, tag)。
    let (ciphertext, nonce, tag) = match algorithm {
        Algorithm::Aes256Gcm => crate::aes_gcm::encrypt(key, plaintext.as_bytes())?,
        Algorithm::ChaCha20Poly1305 => crate::chacha20::encrypt(key, plaintext.as_bytes())?,
        // Unreachable because we already rejected CBC above.
        // 不可达，因为上面已经拒绝了 CBC。
        Algorithm::Aes256Cbc => unreachable!(),
    };

    // 3. Assemble the binary payload: nonce(12B) || ciphertext || tag(16B).
    //    组装二进制负载：nonce(12B) || 密文 || tag(16B)。
    let mut payload = Vec::with_capacity(NONCE_SIZE + ciphertext.len() + TAG_SIZE);
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&ciphertext);
    payload.extend_from_slice(&tag);

    // 4. Base64-encode the payload using standard encoding (with padding).
    //    使用标准编码（带填充）对负载进行 Base64 编码。
    Ok(BASE64_STANDARD.encode(&payload))
}

/// Decrypt a Base64-encoded paragraph ciphertext.
/// Expects format: IV(12) || ciphertext || tag(16).
///
/// 解密 Base64 编码的段落密文。
/// 期望格式：IV(12) || 密文 || tag(16)。
///
/// # Arguments / 参数
///
/// * `base64_content` - Base64-encoded encrypted payload / Base64 编码的加密负载
/// * `key`            - 256-bit decryption key / 256 位解密密钥
/// * `algorithm`      - The AEAD algorithm used for encryption / 加密时使用的 AEAD 算法
///
/// # Returns / 返回
///
/// Decrypted plaintext string on success, or `CryptoError` on failure.
/// 成功返回解密后的明文字符串，失败返回 `CryptoError`。
///
/// # Errors / 错误
///
/// * `CryptoError::UnsupportedAlgorithm` — if AES-256-CBC is specified.
/// * `CryptoError::InvalidFormat` — if Base64 decoding fails, payload is too short,
///   or decrypted bytes are not valid UTF-8.
/// * `CryptoError::AuthenticationFailed` — if the authentication tag does not verify.
pub fn decrypt_paragraph(
    base64_content: &str,
    key: &[u8; 32],
    algorithm: Algorithm,
) -> Result<String, CryptoError> {
    // 1. Reject AES-256-CBC.
    //    拒绝 AES-256-CBC。
    if algorithm == Algorithm::Aes256Cbc {
        return Err(CryptoError::UnsupportedAlgorithm(
            "AES-256-CBC is not supported for paragraph-level encryption".to_string(),
        ));
    }

    // 2. Base64-decode the input.
    //    Base64 解码输入。
    let payload = BASE64_STANDARD
        .decode(base64_content)
        .map_err(|e| CryptoError::InvalidFormat(format!("Base64 decode failed: {e}")))?;

    // 3. Validate minimum payload size: nonce(12) + tag(16) = 28 bytes.
    //    验证最小负载大小：nonce(12) + tag(16) = 28 字节。
    if payload.len() < MIN_PAYLOAD_SIZE {
        return Err(CryptoError::InvalidFormat(format!(
            "payload too short: {} bytes, minimum is {} bytes",
            payload.len(),
            MIN_PAYLOAD_SIZE,
        )));
    }

    // 4. Split the payload into nonce, ciphertext, and tag.
    //    将负载分割为 nonce、密文和 tag。
    let nonce: [u8; NONCE_SIZE] = payload[..NONCE_SIZE]
        .try_into()
        .expect("nonce slice length is guaranteed to be NONCE_SIZE");
    let tag_start = payload.len() - TAG_SIZE;
    let ciphertext = &payload[NONCE_SIZE..tag_start];
    let tag: [u8; TAG_SIZE] = payload[tag_start..]
        .try_into()
        .expect("tag slice length is guaranteed to be TAG_SIZE");

    // 5. Decrypt using the appropriate AEAD engine.
    //    使用相应的 AEAD 引擎解密。
    let plaintext_bytes = match algorithm {
        Algorithm::Aes256Gcm => crate::aes_gcm::decrypt(key, &nonce, ciphertext, &tag)?,
        Algorithm::ChaCha20Poly1305 => {
            crate::chacha20::decrypt(key, &nonce, ciphertext, &tag)?
        }
        Algorithm::Aes256Cbc => unreachable!(),
    };

    // 6. Convert decrypted bytes to UTF-8 string.
    //    将解密后的字节转换为 UTF-8 字符串。
    String::from_utf8(plaintext_bytes).map_err(|e| {
        CryptoError::InvalidFormat(format!("decrypted data is not valid UTF-8: {e}"))
    })
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// AES-256-GCM paragraph encrypt/decrypt round-trip.
    /// AES-256-GCM 段落级加密/解密往返测试。
    #[test]
    fn test_encrypt_decrypt_paragraph_aes_gcm() {
        let key = [0x42u8; 32];
        let plaintext = "这是一段需要加密的小说内容。Hello, Unicodex!";

        let encrypted = encrypt_paragraph(plaintext, &key, Algorithm::Aes256Gcm).unwrap();
        let decrypted = decrypt_paragraph(&encrypted, &key, Algorithm::Aes256Gcm).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// ChaCha20-Poly1305 paragraph encrypt/decrypt round-trip.
    /// ChaCha20-Poly1305 段落级加密/解密往返测试。
    #[test]
    fn test_encrypt_decrypt_paragraph_chacha20() {
        let key = [0x7Fu8; 32];
        let plaintext = "Chapter 3: The Secret\n秘密章节的内容...";

        let encrypted =
            encrypt_paragraph(plaintext, &key, Algorithm::ChaCha20Poly1305).unwrap();
        let decrypted =
            decrypt_paragraph(&encrypted, &key, Algorithm::ChaCha20Poly1305).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// AES-256-CBC should be rejected for paragraph-level encryption.
    /// AES-256-CBC 应在段落级加密中被拒绝。
    #[test]
    fn test_aes_cbc_rejected() {
        let key = [0x42u8; 32];

        // Encrypt should fail.
        // 加密应失败。
        let enc_result = encrypt_paragraph("test", &key, Algorithm::Aes256Cbc);
        assert!(
            matches!(&enc_result, Err(CryptoError::UnsupportedAlgorithm(_))),
            "expected UnsupportedAlgorithm, got: {enc_result:?}"
        );

        // Decrypt should also fail.
        // 解密也应失败。
        let dec_result = decrypt_paragraph("AAAA", &key, Algorithm::Aes256Cbc);
        assert!(
            matches!(&dec_result, Err(CryptoError::UnsupportedAlgorithm(_))),
            "expected UnsupportedAlgorithm, got: {dec_result:?}"
        );
    }

    /// Verify the encrypted output is valid Base64 and the decoded
    /// binary has the correct structure: nonce(12) + ciphertext + tag(16).
    ///
    /// 验证加密输出是有效的 Base64，且解码后的二进制数据
    /// 具有正确的结构：nonce(12) + 密文 + tag(16)。
    #[test]
    fn test_base64_format_correct() {
        let key = [0x42u8; 32];
        let plaintext = "format check";

        let encrypted = encrypt_paragraph(plaintext, &key, Algorithm::Aes256Gcm).unwrap();

        // Decode should succeed.
        // 解码应成功。
        let decoded = BASE64_STANDARD.decode(&encrypted).unwrap();

        // Total length = 12 (nonce) + plaintext.len() (ciphertext for GCM is same size) + 16 (tag).
        // 总长度 = 12 (nonce) + plaintext.len()（GCM 密文与明文等长） + 16 (tag)。
        let expected_len = NONCE_SIZE + plaintext.len() + TAG_SIZE;
        assert_eq!(
            decoded.len(),
            expected_len,
            "decoded length {} != expected {}",
            decoded.len(),
            expected_len
        );
    }

    /// Invalid Base64 input should return an InvalidFormat error.
    /// 无效的 Base64 输入应返回 InvalidFormat 错误。
    #[test]
    fn test_invalid_base64_rejected() {
        let key = [0x42u8; 32];

        // "!!!" is not valid Base64.
        // "!!!" 不是有效的 Base64。
        let result = decrypt_paragraph("!!not-base64!!", &key, Algorithm::Aes256Gcm);
        assert!(
            matches!(&result, Err(CryptoError::InvalidFormat(_))),
            "expected InvalidFormat, got: {result:?}"
        );
    }

    /// Base64 data that decodes to fewer than 28 bytes should be rejected.
    /// 解码后少于 28 字节的 Base64 数据应被拒绝。
    #[test]
    fn test_too_short_data_rejected() {
        let key = [0x42u8; 32];

        // 10 bytes encoded to Base64 — well under the 28-byte minimum.
        // 10 字节编码为 Base64 —— 远低于 28 字节的最小值。
        let short_payload = BASE64_STANDARD.encode([0u8; 10]);
        let result = decrypt_paragraph(&short_payload, &key, Algorithm::Aes256Gcm);
        assert!(
            matches!(&result, Err(CryptoError::InvalidFormat(_))),
            "expected InvalidFormat for short payload, got: {result:?}"
        );
    }
}
