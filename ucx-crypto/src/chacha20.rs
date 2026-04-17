//! ChaCha20-Poly1305 encryption engine.
//! ChaCha20-Poly1305 加密引擎。
//!
//! This module provides authenticated encryption and decryption using
//! the ChaCha20-Poly1305 AEAD construction (RFC 8439).
//!
//! 本模块提供基于 ChaCha20-Poly1305 AEAD 构造（RFC 8439）的
//! 认证加密与解密功能。

use chacha20poly1305::{
    ChaCha20Poly1305,
    aead::{Aead, KeyInit, Payload},
};
use rand::RngCore;

use crate::CryptoError;

/// Nonce size in bytes (96-bit).
/// Nonce 大小，单位为字节（96 位）。
pub const NONCE_SIZE: usize = 12;

/// Poly1305 authentication tag size in bytes (128-bit).
/// Poly1305 认证标签大小，单位为字节（128 位）。
pub const TAG_SIZE: usize = 16;

/// Key size in bytes (256-bit).
/// 密钥大小，单位为字节（256 位）。
pub const KEY_SIZE: usize = 32;

/// Encrypt plaintext using ChaCha20-Poly1305 AEAD.
///
/// Generates a random 12-byte nonce via `OsRng`, encrypts the plaintext,
/// and returns the ciphertext, nonce, and authentication tag separately.
///
/// 使用 ChaCha20-Poly1305 AEAD 加密明文。
/// 通过 `OsRng` 生成随机 12 字节 nonce，加密明文，
/// 并分别返回密文、nonce 和认证标签。
///
/// # Arguments / 参数
///
/// * `key`       - 256-bit encryption key / 256 位加密密钥
/// * `plaintext` - Data to encrypt / 待加密数据
///
/// # Returns / 返回
///
/// Result type for encrypt: (ciphertext, nonce, tag).
/// encrypt 的返回类型：(密文, nonce, 标签)。
pub type EncryptResult = (Vec<u8>, [u8; 12], [u8; 16]);

/// A tuple of `(ciphertext, nonce, tag)` on success.
/// 成功时返回 `(密文, nonce, 标签)` 元组。
pub fn encrypt(
    key: &[u8; 32],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<EncryptResult, CryptoError> {
    // 1. Generate a random 12-byte nonce using CSPRNG.
    //    使用 CSPRNG 生成随机 12 字节 nonce。
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = chacha20poly1305::Nonce::from(nonce_bytes);

    // 2. Create the cipher instance from the key.
    //    使用密钥创建加密器实例。
    let cipher = ChaCha20Poly1305::new(key.into());

    // 3. Encrypt with AAD bound into the Poly1305 tag.
    //    使用 Payload 将 AAD 绑入 Poly1305 认证标签。
    let combined = cipher
        .encrypt(&nonce, Payload { msg: plaintext, aad })
        .map_err(|_| CryptoError::AuthenticationFailed)?;

    // 4. Split ciphertext and tag.
    //    分离密文和标签。
    let ct_len = combined.len() - TAG_SIZE;
    let ciphertext = combined[..ct_len].to_vec();
    let mut tag = [0u8; TAG_SIZE];
    tag.copy_from_slice(&combined[ct_len..]);

    Ok((ciphertext, nonce_bytes, tag))
}

/// Decrypt ciphertext using ChaCha20-Poly1305 AEAD.
///
/// Reassembles the ciphertext and tag, then decrypts and verifies
/// authenticity in a single operation.
///
/// 使用 ChaCha20-Poly1305 AEAD 解密密文。
/// 重新组合密文和标签，然后在单次操作中解密并验证真实性。
///
/// # Arguments / 参数
///
/// * `key`        - 256-bit decryption key / 256 位解密密钥
/// * `nonce`      - The 12-byte nonce used during encryption / 加密时使用的 12 字节 nonce
/// * `ciphertext` - The encrypted data (without tag) / 加密数据（不含标签）
/// * `tag`        - The 16-byte authentication tag / 16 字节认证标签
///
/// # Returns / 返回
///
/// The decrypted plaintext on success, or `CryptoError::AuthenticationFailed`
/// if the tag does not verify.
/// 成功时返回解密后的明文，标签验证失败则返回 `CryptoError::AuthenticationFailed`。
pub fn decrypt(
    key: &[u8; 32],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    tag: &[u8; 16],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    // 1. Reassemble ciphertext || tag (the format expected by the AEAD).
    //    重新拼接 密文 || 标签（AEAD 期望的格式）。
    let mut combined = Vec::with_capacity(ciphertext.len() + TAG_SIZE);
    combined.extend_from_slice(ciphertext);
    combined.extend_from_slice(tag);

    // 2. Create the cipher instance.
    //    创建加密器实例。
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = chacha20poly1305::Nonce::from(*nonce);

    // 3. Decrypt and verify the authentication tag with matching AAD.
    //    解密并验证认证标签，AAD 必须与加密时一致。
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: combined.as_ref(),
                aad,
            },
        )
        .map_err(|_| CryptoError::AuthenticationFailed)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Empty AAD for tests that don't exercise AAD binding.
    // 测试中不关心 AAD 绑定时使用的空 AAD。
    const EMPTY_AAD: &[u8] = b"";

    /// Round-trip: encrypt then decrypt should return the original plaintext.
    /// 往返测试：加密后解密应返回原始明文。
    #[test]
    fn round_trip() {
        let key = [0x42u8; 32];
        let plaintext = b"Hello, ChaCha20-Poly1305!";

        let (ciphertext, nonce, tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();
        let decrypted = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// Decryption with a wrong key should fail with AuthenticationFailed.
    /// 使用错误密钥解密应返回 AuthenticationFailed 错误。
    #[test]
    fn wrong_key() {
        let key = [0x42u8; 32];
        let wrong_key = [0x99u8; 32];
        let plaintext = b"secret data";

        let (ciphertext, nonce, tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();
        let result = decrypt(&wrong_key, &nonce, &ciphertext, &tag, EMPTY_AAD);

        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), CryptoError::AuthenticationFailed),
            "expected AuthenticationFailed error"
        );
    }

    /// Tampered ciphertext should fail authentication.
    /// 篡改密文应导致认证失败。
    #[test]
    fn tampered_ciphertext() {
        let key = [0x42u8; 32];
        let plaintext = b"tamper test data";

        let (mut ciphertext, nonce, tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();

        // Flip one bit in the ciphertext.
        // 翻转密文中的一个比特。
        ciphertext[0] ^= 0x01;

        let result = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD);
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::AuthenticationFailed
        ));
    }

    /// Empty plaintext should encrypt and decrypt successfully.
    /// 空明文应能成功加密和解密。
    #[test]
    fn empty_plaintext() {
        let key = [0x42u8; 32];
        let plaintext = b"";

        let (ciphertext, nonce, tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();

        // Ciphertext should be empty (no payload), but tag should exist.
        // 密文应为空（无有效载荷），但标签应存在。
        assert!(ciphertext.is_empty());
        assert_eq!(tag.len(), TAG_SIZE);

        let decrypted = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD).unwrap();
        assert!(decrypted.is_empty());
    }

    /// Tampered tag should fail authentication.
    /// 篡改 tag 应导致认证失败。
    #[test]
    fn tampered_tag() {
        let key = [0x42u8; 32];
        let plaintext = b"tag integrity check";

        let (ciphertext, nonce, mut tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();

        // Flip one bit in the tag.
        // 翻转 tag 中的一个比特。
        tag[0] ^= 0x01;

        let result = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD);
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::AuthenticationFailed
        ));
    }

    /// Two encryptions of the same plaintext should produce different nonces.
    /// 对同一明文的两次加密应产生不同的 nonce。
    #[test]
    fn nonce_uniqueness() {
        let key = [0x42u8; 32];
        let plaintext = b"nonce uniqueness test";

        let (_, nonce1, _) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();
        let (_, nonce2, _) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();

        // Nonces should be different (collision probability is negligible).
        // Nonce 应不同（碰撞概率可忽略不计）。
        assert_ne!(nonce1, nonce2);
    }

    /// Large plaintext (1 MB) should encrypt and decrypt successfully.
    /// 大明文（1 MB）应能正常加密和解密。
    #[test]
    fn large_plaintext() {
        let key = [0x42u8; 32];
        let plaintext = vec![0xABu8; 1024 * 1024]; // 1 MB

        let (ciphertext, nonce, tag) = encrypt(&key, &plaintext, EMPTY_AAD).unwrap();
        let decrypted = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// AAD mismatch between encrypt and decrypt must fail authentication.
    /// 加密与解密 AAD 不一致必须导致认证失败。
    #[test]
    fn aad_mismatch_fails() {
        let key = [0x42u8; 32];
        let plaintext = b"aad-bound data";

        let (ciphertext, nonce, tag) = encrypt(&key, plaintext, b"hdr-A").unwrap();
        let result = decrypt(&key, &nonce, &ciphertext, &tag, b"hdr-B");
        assert!(matches!(result, Err(CryptoError::AuthenticationFailed)));
    }
}
