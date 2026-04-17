//! AES-256-GCM encryption engine.
//! AES-256-GCM 加密引擎。
//!
//! Provides authenticated encryption using AES-256 in GCM mode.
//! Nonce is 96 bits (12 bytes), generated via CSPRNG.
//! Auth tag is 128 bits (16 bytes).
//!
//! 使用 AES-256 GCM 模式提供认证加密。
//! Nonce 为 96 位（12 字节），通过 CSPRNG 生成。
//! 认证标签为 128 位（16 字节）。

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use rand::RngCore;

use crate::CryptoError;

/// AES-256-GCM nonce size in bytes (96 bits).
/// AES-256-GCM nonce 大小（字节），96 位。
pub const NONCE_SIZE: usize = 12;

/// AES-256-GCM authentication tag size in bytes (128 bits).
/// AES-256-GCM 认证标签大小（字节），128 位。
pub const TAG_SIZE: usize = 16;

/// AES-256-GCM key size in bytes (256 bits).
/// AES-256-GCM 密钥大小（字节），256 位。
pub const KEY_SIZE: usize = 32;

/// Encrypt result: (ciphertext, nonce, authentication tag).
/// 加密结果：(密文, nonce, 认证标签)。
pub type EncryptResult = (Vec<u8>, [u8; NONCE_SIZE], [u8; TAG_SIZE]);

/// Encrypt plaintext using AES-256-GCM.
/// 使用 AES-256-GCM 加密明文。
///
/// Generates a random 12-byte nonce via CSPRNG (OsRng).
/// Returns `(ciphertext, nonce, tag)` on success.
/// The ciphertext does NOT include the tag — they are returned separately.
///
/// 通过 CSPRNG (OsRng) 生成随机 12 字节 nonce。
/// 成功返回 `(密文, nonce, tag)`。
/// 密文不包含 tag —— 二者分开返回。
pub fn encrypt(
    key: &[u8; KEY_SIZE],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<EncryptResult, CryptoError> {
    // 1. Create cipher instance from key.
    //    根据密钥创建 cipher 实例。
    let cipher = Aes256Gcm::new(key.into());

    // 2. Generate 12-byte random nonce via CSPRNG.
    //    通过 CSPRNG 生成 12 字节随机 nonce。
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce: Nonce<_> = nonce_bytes.into();

    // 3. Encrypt with Payload { msg, aad } so the AAD is bound into the tag.
    //    使用 Payload { msg, aad } 将 AAD 绑入认证标签。
    let ciphertext_with_tag = cipher
        .encrypt(&nonce, Payload { msg: plaintext, aad })
        .map_err(|_| CryptoError::InvalidFormat("AES-256-GCM encryption failed".to_string()))?;

    // 4. Split ciphertext and tag (last 16 bytes are tag).
    //    分离密文和 tag（最后 16 字节为 tag）。
    let ct_len = ciphertext_with_tag.len() - TAG_SIZE;
    let ciphertext = ciphertext_with_tag[..ct_len].to_vec();
    let mut tag = [0u8; TAG_SIZE];
    tag.copy_from_slice(&ciphertext_with_tag[ct_len..]);

    Ok((ciphertext, nonce_bytes, tag))
}

/// Decrypt ciphertext using AES-256-GCM.
/// 使用 AES-256-GCM 解密密文。
///
/// Verifies the authentication tag before returning plaintext.
/// Returns `CryptoError::AuthenticationFailed` if tag verification fails.
///
/// 在返回明文前验证认证标签。
/// 标签验证失败时返回 `CryptoError::AuthenticationFailed`。
pub fn decrypt(
    key: &[u8; KEY_SIZE],
    nonce: &[u8; NONCE_SIZE],
    ciphertext: &[u8],
    tag: &[u8; TAG_SIZE],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    // 1. Create cipher instance from key.
    //    根据密钥创建 cipher 实例。
    let cipher = Aes256Gcm::new(key.into());

    // 2. Reassemble ciphertext || tag (aes-gcm expects postfix tag).
    //    重新拼接 ciphertext || tag（aes-gcm 期望后缀 tag）。
    let mut ciphertext_with_tag = Vec::with_capacity(ciphertext.len() + TAG_SIZE);
    ciphertext_with_tag.extend_from_slice(ciphertext);
    ciphertext_with_tag.extend_from_slice(tag);

    // 3. Decrypt and verify authentication tag with the same AAD that was
    //    bound during encryption. Any AAD mismatch = AuthenticationFailed.
    //    解密并验证认证标签，AAD 必须与加密时一致。
    let nonce: Nonce<_> = (*nonce).into();
    let plaintext = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext_with_tag.as_ref(),
                aad,
            },
        )
        .map_err(|_| CryptoError::AuthenticationFailed)?;

    Ok(plaintext)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Empty AAD used by tests that don't care about AAD binding.
    // 测试中不关心 AAD 绑定时使用的空 AAD。
    const EMPTY_AAD: &[u8] = b"";

    /// Round-trip: encrypt then decrypt should return original plaintext.
    /// 往返测试：加密后解密应返回原始明文。
    #[test]
    fn test_encrypt_decrypt_round_trip() {
        let key = [0x42u8; KEY_SIZE];
        let plaintext = b"Hello, Unicodex!";

        let (ciphertext, nonce, tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();
        let decrypted = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// Decryption with a wrong key should fail with AuthenticationFailed.
    /// 使用错误密钥解密应返回 AuthenticationFailed。
    #[test]
    fn test_wrong_key_fails() {
        let key = [0x42u8; KEY_SIZE];
        let wrong_key = [0x00u8; KEY_SIZE];
        let plaintext = b"secret data";

        let (ciphertext, nonce, tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();
        let result = decrypt(&wrong_key, &nonce, &ciphertext, &tag, EMPTY_AAD);

        assert!(matches!(result, Err(CryptoError::AuthenticationFailed)));
    }

    /// Tampered ciphertext should fail authentication.
    /// 篡改密文应导致认证失败。
    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = [0x42u8; KEY_SIZE];
        let plaintext = b"integrity check";

        let (mut ciphertext, nonce, tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();

        // Flip a bit in the ciphertext.
        // 翻转密文中的一个比特。
        ciphertext[0] ^= 0xFF;

        let result = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD);
        assert!(matches!(result, Err(CryptoError::AuthenticationFailed)));
    }

    /// Tampered tag should fail authentication.
    /// 篡改 tag 应导致认证失败。
    #[test]
    fn test_tampered_tag_fails() {
        let key = [0x42u8; KEY_SIZE];
        let plaintext = b"tag check";

        let (ciphertext, nonce, mut tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();

        // Flip a bit in the tag.
        // 翻转 tag 中的一个比特。
        tag[0] ^= 0xFF;

        let result = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD);
        assert!(matches!(result, Err(CryptoError::AuthenticationFailed)));
    }

    /// Empty plaintext should encrypt and decrypt successfully.
    /// 空明文应能正常加密和解密。
    #[test]
    fn test_empty_plaintext() {
        let key = [0x42u8; KEY_SIZE];
        let plaintext = b"";

        let (ciphertext, nonce, tag) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();

        // Ciphertext should be empty (only tag is separate).
        // 密文应为空（tag 单独分离）。
        assert!(ciphertext.is_empty());

        let decrypted = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    /// Large plaintext (1 MB) should encrypt and decrypt successfully.
    /// 大明文（1 MB）应能正常加密和解密。
    #[test]
    fn test_large_plaintext() {
        let key = [0x42u8; KEY_SIZE];
        let plaintext = vec![0xABu8; 1024 * 1024]; // 1 MB

        let (ciphertext, nonce, tag) = encrypt(&key, &plaintext, EMPTY_AAD).unwrap();
        let decrypted = decrypt(&key, &nonce, &ciphertext, &tag, EMPTY_AAD).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// Two encryptions of the same plaintext should produce different nonces.
    /// 对同一明文的两次加密应产生不同的 nonce。
    #[test]
    fn test_nonce_uniqueness() {
        let key = [0x42u8; KEY_SIZE];
        let plaintext = b"nonce test";

        let (_, nonce1, _) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();
        let (_, nonce2, _) = encrypt(&key, plaintext, EMPTY_AAD).unwrap();

        // Nonces should be different (probability of collision is negligible).
        // Nonce 应不同（碰撞概率可忽略不计）。
        assert_ne!(nonce1, nonce2);
    }

    /// AAD mismatch between encrypt and decrypt must fail authentication.
    /// 加密与解密 AAD 不一致必须导致认证失败。
    #[test]
    fn test_aad_mismatch_fails() {
        let key = [0x42u8; KEY_SIZE];
        let plaintext = b"aad-bound data";

        let (ciphertext, nonce, tag) = encrypt(&key, plaintext, b"header-A").unwrap();

        // Same key/nonce/tag but different AAD → must fail.
        // 相同的 key/nonce/tag 但 AAD 不同 → 必须失败。
        let result = decrypt(&key, &nonce, &ciphertext, &tag, b"header-B");
        assert!(matches!(result, Err(CryptoError::AuthenticationFailed)));

        // Same AAD should succeed.
        // 相同 AAD 必须成功。
        let ok = decrypt(&key, &nonce, &ciphertext, &tag, b"header-A").unwrap();
        assert_eq!(ok, plaintext);
    }
}
