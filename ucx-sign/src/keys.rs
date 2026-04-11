//! Ed25519 Key Generation and PEM I/O.
//!
//! This module provides functions for:
//! - Generating Ed25519 signing key pairs
//! - Saving / loading private keys in PKCS#8 PEM format
//! - Saving / loading public keys in SPKI PEM format
//!
//! Ed25519 密钥生成与 PEM 读写。
//!
//! 本模块提供以下功能：
//! - 生成 Ed25519 签名密钥对
//! - 以 PKCS#8 PEM 格式保存/加载私钥
//! - 以 SPKI PEM 格式保存/加载公钥

use std::path::Path;

use ed25519_dalek::{SigningKey, VerifyingKey};
use pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding};

use crate::SignError;

// =============================================================================
// Key generation / 密钥生成
// =============================================================================

/// Generate a new Ed25519 signing key pair.
///
/// Uses the operating system's cryptographically secure random number generator
/// (`OsRng`) to generate a fresh Ed25519 key pair.
///
/// 生成新的 Ed25519 签名密钥对。
/// 使用操作系统的密码学安全随机数生成器（`OsRng`）生成全新的 Ed25519 密钥对。
///
/// # Returns / 返回
///
/// A tuple `(SigningKey, VerifyingKey)` on success.
/// 成功时返回 `(SigningKey, VerifyingKey)` 元组。
///
/// # Errors / 错误
///
/// Returns `SignError::KeyGenError` if key generation fails.
/// 密钥生成失败时返回 `SignError::KeyGenError`。
pub fn generate_ed25519_keypair() -> Result<(SigningKey, VerifyingKey), SignError> {
    // Generate a random signing key using the OS RNG.
    // 使用操作系统随机数生成器生成签名密钥。
    let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);

    // Derive the corresponding verifying (public) key.
    // 导出对应的验证（公）钥。
    let verifying_key = signing_key.verifying_key();

    Ok((signing_key, verifying_key))
}

// =============================================================================
// Private key I/O / 私钥读写
// =============================================================================

/// Save a private key to a PEM file in PKCS#8 format.
///
/// The output file will contain a PEM block starting with
/// `-----BEGIN PRIVATE KEY-----`.
///
/// 将私钥以 PKCS#8 格式保存到 PEM 文件。
/// 输出文件将包含以 `-----BEGIN PRIVATE KEY-----` 开头的 PEM 块。
///
/// # Arguments / 参数
///
/// * `key`  - The Ed25519 signing key to save.
///   要保存的 Ed25519 签名密钥。
/// * `path` - Destination file path.
///   目标文件路径。
///
/// # Errors / 错误
///
/// Returns `SignError::PemError` if PEM encoding fails.
/// Returns `SignError::Io` if file writing fails.
/// PEM 编码失败返回 `SignError::PemError`，文件写入失败返回 `SignError::Io`。
pub fn save_private_key(key: &SigningKey, path: &Path) -> Result<(), SignError> {
    // Encode the signing key as a PKCS#8 PEM string.
    // 将签名密钥编码为 PKCS#8 PEM 字符串。
    let pem_string = key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| SignError::PemError(format!("failed to encode private key to PEM: {e}")))?;

    // Write the PEM string to the specified file.
    // 将 PEM 字符串写入指定文件。
    std::fs::write(path, pem_string.as_bytes())?;

    Ok(())
}

/// Load a private key from a PEM file in PKCS#8 format.
///
/// The file must contain a PEM block starting with
/// `-----BEGIN PRIVATE KEY-----`.
///
/// 从 PKCS#8 格式的 PEM 文件加载私钥。
/// 文件必须包含以 `-----BEGIN PRIVATE KEY-----` 开头的 PEM 块。
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
pub fn load_private_key(path: &Path) -> Result<SigningKey, SignError> {
    // Check if the file exists before attempting to read.
    // 在尝试读取之前检查文件是否存在。
    if !path.exists() {
        return Err(SignError::KeyNotFound(format!(
            "private key file not found: {}",
            path.display()
        )));
    }

    // Read the PEM file contents.
    // 读取 PEM 文件内容。
    let pem_contents = std::fs::read_to_string(path)?;

    // Parse the PKCS#8 PEM string into a SigningKey.
    // 将 PKCS#8 PEM 字符串解析为 SigningKey。
    let signing_key = SigningKey::from_pkcs8_pem(&pem_contents)
        .map_err(|e| SignError::PemError(format!("failed to decode private key from PEM: {e}")))?;

    Ok(signing_key)
}

// =============================================================================
// Public key I/O / 公钥读写
// =============================================================================

/// Save a public key to a PEM file in SPKI (Subject Public Key Info) format.
///
/// The output file will contain a PEM block starting with
/// `-----BEGIN PUBLIC KEY-----`.
///
/// 将公钥以 SPKI（主体公钥信息）格式保存到 PEM 文件。
/// 输出文件将包含以 `-----BEGIN PUBLIC KEY-----` 开头的 PEM 块。
///
/// # Arguments / 参数
///
/// * `key`  - The Ed25519 verifying key to save.
///   要保存的 Ed25519 验证密钥。
/// * `path` - Destination file path.
///   目标文件路径。
///
/// # Errors / 错误
///
/// Returns `SignError::PemError` if PEM encoding fails.
/// Returns `SignError::Io` if file writing fails.
/// PEM 编码失败返回 `SignError::PemError`，文件写入失败返回 `SignError::Io`。
pub fn save_public_key(key: &VerifyingKey, path: &Path) -> Result<(), SignError> {
    // Encode the verifying key as an SPKI PEM string.
    // 将验证密钥编码为 SPKI PEM 字符串。
    let pem_string = key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| SignError::PemError(format!("failed to encode public key to PEM: {e}")))?;

    // Write the PEM string to the specified file.
    // 将 PEM 字符串写入指定文件。
    std::fs::write(path, pem_string.as_bytes())?;

    Ok(())
}

/// Load a public key from a PEM file in SPKI format.
///
/// The file must contain a PEM block starting with
/// `-----BEGIN PUBLIC KEY-----`.
///
/// 从 SPKI 格式的 PEM 文件加载公钥。
/// 文件必须包含以 `-----BEGIN PUBLIC KEY-----` 开头的 PEM 块。
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
pub fn load_public_key(path: &Path) -> Result<VerifyingKey, SignError> {
    // Check if the file exists before attempting to read.
    // 在尝试读取之前检查文件是否存在。
    if !path.exists() {
        return Err(SignError::KeyNotFound(format!(
            "public key file not found: {}",
            path.display()
        )));
    }

    // Read the PEM file contents.
    // 读取 PEM 文件内容。
    let pem_contents = std::fs::read_to_string(path)?;

    // Parse the SPKI PEM string into a VerifyingKey.
    // 将 SPKI PEM 字符串解析为 VerifyingKey。
    let verifying_key = VerifyingKey::from_public_key_pem(&pem_contents)
        .map_err(|e| SignError::PemError(format!("failed to decode public key from PEM: {e}")))?;

    Ok(verifying_key)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;
    use pkcs8::EncodePrivateKey;
    use tempfile::TempDir;

    /// Test: generate_ed25519_keypair returns a valid key pair.
    /// 测试：generate_ed25519_keypair 返回有效的密钥对。
    #[test]
    fn test_generate_ed25519_keypair_returns_valid_keys() {
        let (signing_key, verifying_key) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        // The verifying key derived from the signing key should match.
        // 从签名密钥导出的验证密钥应当一致。
        assert_eq!(
            signing_key.verifying_key(),
            verifying_key,
            "verifying key must match the signing key's derived verifying key"
        );
    }

    /// Test: save_private_key + load_private_key roundtrip.
    /// 测试：save_private_key + load_private_key 往返一致。
    #[test]
    fn test_private_key_save_load_roundtrip() {
        let (signing_key, _) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        // Create a temporary directory for the test.
        // 为测试创建临时目录。
        let tmp_dir = TempDir::new().expect("failed to create temp dir");
        let key_path = tmp_dir.path().join("private.pem");

        // Save and reload the private key.
        // 保存并重新加载私钥。
        save_private_key(&signing_key, &key_path).expect("save_private_key should succeed");
        let loaded_key = load_private_key(&key_path).expect("load_private_key should succeed");

        // The loaded key should be identical to the original.
        // 加载的密钥应与原始密钥相同。
        assert_eq!(
            signing_key.to_bytes(),
            loaded_key.to_bytes(),
            "loaded private key must match the original"
        );
    }

    /// Test: save_public_key + load_public_key roundtrip.
    /// 测试：save_public_key + load_public_key 往返一致。
    #[test]
    fn test_public_key_save_load_roundtrip() {
        let (_, verifying_key) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        // Create a temporary directory for the test.
        // 为测试创建临时目录。
        let tmp_dir = TempDir::new().expect("failed to create temp dir");
        let key_path = tmp_dir.path().join("public.pem");

        // Save and reload the public key.
        // 保存并重新加载公钥。
        save_public_key(&verifying_key, &key_path).expect("save_public_key should succeed");
        let loaded_key = load_public_key(&key_path).expect("load_public_key should succeed");

        // The loaded key should be identical to the original.
        // 加载的密钥应与原始密钥相同。
        assert_eq!(
            verifying_key.to_bytes(),
            loaded_key.to_bytes(),
            "loaded public key must match the original"
        );
    }

    /// Test: load_private_key on a nonexistent file returns an error.
    /// 测试：对不存在的文件调用 load_private_key 应返回错误。
    #[test]
    fn test_load_private_key_nonexistent_file_returns_error() {
        let result = load_private_key(Path::new("/nonexistent/path/key.pem"));

        assert!(result.is_err(), "loading from a nonexistent path must fail");
        match result.unwrap_err() {
            SignError::KeyNotFound(msg) => {
                assert!(
                    msg.contains("not found"),
                    "error message should mention 'not found': {msg}"
                );
            }
            other => panic!("expected KeyNotFound error, got: {other:?}"),
        }
    }

    /// Test: sign and verify with generated keys (basic Ed25519).
    /// 测试：使用生成的密钥进行签名和验证（基础 Ed25519）。
    #[test]
    fn test_sign_and_verify_with_generated_keys() {
        use ed25519_dalek::Verifier;

        let (signing_key, verifying_key) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        // Sign a test message.
        // 对测试消息签名。
        let message = b"Unicodex test message for Ed25519 signing";
        let signature = signing_key.sign(message);

        // Verify the signature with the public key.
        // 使用公钥验证签名。
        assert!(
            verifying_key.verify(message, &signature).is_ok(),
            "signature verification must succeed for valid key pair"
        );

        // Verify that a wrong message fails verification.
        // 验证错误消息应当失败。
        let wrong_message = b"tampered message";
        assert!(
            verifying_key.verify(wrong_message, &signature).is_err(),
            "signature verification must fail for wrong message"
        );
    }

    /// Test: PEM output starts with the correct header.
    /// 测试：PEM 输出以正确的头部开始。
    #[test]
    fn test_pem_output_starts_with_correct_header() {
        let (signing_key, _) = generate_ed25519_keypair()
            .expect("key generation should succeed");

        // Encode private key to PKCS#8 PEM.
        // 将私钥编码为 PKCS#8 PEM。
        let pem_string = signing_key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("PEM encoding should succeed");

        // Verify PEM header.
        // 验证 PEM 头部。
        assert!(
            pem_string.as_str().starts_with("-----BEGIN PRIVATE KEY-----"),
            "PEM output must start with '-----BEGIN PRIVATE KEY-----', got: {}",
            &pem_string.as_str()[..50]
        );
    }
}
