//! AES-256-CBC with HMAC-SHA256 (Encrypt-then-MAC) engine.
//! AES-256-CBC + HMAC-SHA256（Encrypt-then-MAC）加密引擎。
//!
//! This module provides authenticated encryption using AES-256-CBC for
//! confidentiality and HMAC-SHA256 for integrity (Encrypt-then-MAC pattern).
//!
//! 本模块使用 AES-256-CBC 提供机密性，HMAC-SHA256 提供完整性
//! （Encrypt-then-MAC 模式）。
//!
//! ## Security notes / 安全说明
//!
//! - HMAC is computed over `aad || iv || ciphertext`. Including the `aad`
//!   (typically the UCXE header bytes) prevents both IV-manipulation and
//!   header-byte-swap attacks: an attacker cannot change the algorithm ID
//!   or KDF ID in transit without breaking the MAC.
//!   HMAC 对 `aad || iv || ciphertext` 计算。包含 `aad`（通常为 UCXE 头部
//!   字节）同时防御 IV 篡改与头部字节交换攻击 —— 攻击者无法在不破坏
//!   MAC 的前提下修改算法 ID 或 KDF ID。
//! - Decryption always verifies HMAC **before** decrypting to prevent
//!   Padding Oracle attacks.
//!   解密时始终**先验证 HMAC 再解密**，以防止 Padding Oracle 攻击。
//! - HMAC comparison uses constant-time equality to prevent timing attacks.
//!   HMAC 比较使用常量时间比较，防止时序攻击。

use aes::Aes256;
use cbc::cipher::{BlockModeEncrypt, BlockModeDecrypt, KeyIvInit, block_padding::Pkcs7};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

use crate::CryptoError;

/// HMAC-SHA256 type alias.
/// HMAC-SHA256 类型别名。
type HmacSha256 = Hmac<Sha256>;

/// CBC encryptor type alias (AES-256 + PKCS#7 padding).
/// CBC 加密器类型别名（AES-256 + PKCS#7 填充）。
type Aes256CbcEnc = cbc::Encryptor<Aes256>;

/// CBC decryptor type alias (AES-256 + PKCS#7 padding).
/// CBC 解密器类型别名（AES-256 + PKCS#7 填充）。
type Aes256CbcDec = cbc::Decryptor<Aes256>;

/// IV (Initialization Vector) size in bytes (128-bit, one AES block).
/// IV（初始化向量）大小，单位为字节（128 位，一个 AES 块）。
pub const IV_SIZE: usize = 16;

/// HMAC-SHA256 tag size in bytes (256-bit).
/// HMAC-SHA256 标签大小，单位为字节（256 位）。
pub const TAG_SIZE: usize = 32;

/// Encryption key size in bytes (256-bit).
/// 加密密钥大小，单位为字节（256 位）。
///
/// Note: This scheme requires **two** independent 32-byte keys:
/// one for AES-256-CBC encryption and one for HMAC-SHA256 MAC.
///
/// 注意：此方案需要**两个**独立的 32 字节密钥：
/// 一个用于 AES-256-CBC 加密，一个用于 HMAC-SHA256 MAC。
pub const KEY_SIZE: usize = 32;

/// Encrypt plaintext using AES-256-CBC + HMAC-SHA256 (Encrypt-then-MAC).
///
/// Flow:
/// 1. Generate random 16-byte IV via `OsRng`.
/// 2. PKCS#7 pad plaintext, then AES-256-CBC encrypt.
/// 3. Compute HMAC-SHA256 over `iv || ciphertext`.
/// 4. Return `(ciphertext, iv, hmac_tag)`.
///
/// 使用 AES-256-CBC + HMAC-SHA256 加密明文（Encrypt-then-MAC）。
///
/// 流程：
/// 1. 通过 `OsRng` 生成随机 16 字节 IV。
/// 2. PKCS#7 填充明文，然后 AES-256-CBC 加密。
/// 3. 对 `iv || ciphertext` 计算 HMAC-SHA256。
/// 4. 返回 `(密文, iv, hmac_tag)`。
///
/// # Arguments / 参数
///
/// * `enc_key` - 256-bit AES encryption key / 256 位 AES 加密密钥
/// * `mac_key` - 256-bit HMAC-SHA256 key / 256 位 HMAC-SHA256 密钥
/// * `plaintext` - Data to encrypt / 待加密数据
///
/// # Returns / 返回
///
/// Result type for encrypt: (ciphertext, iv, hmac_tag).
/// encrypt 的返回类型：(密文, iv, hmac_tag)。
pub type EncryptResult = (Vec<u8>, [u8; 16], [u8; 32]);

/// A tuple of `(ciphertext, iv, hmac_tag)` on success.
/// 成功时返回 `(密文, iv, hmac_tag)` 元组。
pub fn encrypt(
    enc_key: &[u8; 32],
    mac_key: &[u8; 32],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<EncryptResult, CryptoError> {
    // 1. Generate a random 16-byte IV using CSPRNG.
    //    使用 CSPRNG 生成随机 16 字节 IV。
    let mut iv = [0u8; IV_SIZE];
    rand::rngs::OsRng.fill_bytes(&mut iv);

    // 2. AES-256-CBC encrypt with PKCS#7 padding.
    //    AES-256-CBC 加密，使用 PKCS#7 填充。
    let ciphertext = Aes256CbcEnc::new(enc_key.into(), &iv.into())
        .encrypt_padded_vec::<Pkcs7>(plaintext);

    // 3. Compute HMAC-SHA256 over (aad || iv || ciphertext). Including the
    //    aad covers the UCXE header so header-byte swaps are detected.
    //    对 (aad || iv || ciphertext) 计算 HMAC-SHA256。将 aad 纳入覆盖
    //    UCXE 头部，从而可检测头部字节交换攻击。
    let mut mac = HmacSha256::new_from_slice(mac_key)
        .expect("HMAC accepts any key size");
    mac.update(aad);
    mac.update(&iv);
    mac.update(&ciphertext);
    let hmac_result = mac.finalize().into_bytes();

    let mut hmac_tag = [0u8; TAG_SIZE];
    hmac_tag.copy_from_slice(&hmac_result);

    Ok((ciphertext, iv, hmac_tag))
}

/// Decrypt ciphertext using AES-256-CBC + HMAC-SHA256.
///
/// Flow (verify-before-decrypt to prevent Padding Oracle attacks):
/// 1. Recompute HMAC-SHA256 over `iv || ciphertext`.
/// 2. Constant-time compare with provided `hmac_tag`. Reject if mismatch.
/// 3. AES-256-CBC decrypt and PKCS#7 unpad.
///
/// 使用 AES-256-CBC + HMAC-SHA256 解密密文。
///
/// 流程（先验证再解密，防止 Padding Oracle 攻击）：
/// 1. 对 `iv || ciphertext` 重新计算 HMAC-SHA256。
/// 2. 与提供的 `hmac_tag` 进行常量时间比较。不匹配则拒绝。
/// 3. AES-256-CBC 解密并去除 PKCS#7 填充。
///
/// # Arguments / 参数
///
/// * `enc_key`    - 256-bit AES decryption key / 256 位 AES 解密密钥
/// * `mac_key`    - 256-bit HMAC-SHA256 key / 256 位 HMAC-SHA256 密钥
/// * `iv`         - The 16-byte IV used during encryption / 加密时使用的 16 字节 IV
/// * `ciphertext` - The encrypted data / 加密数据
/// * `hmac_tag`   - The 32-byte HMAC-SHA256 tag / 32 字节 HMAC-SHA256 标签
///
/// # Returns / 返回
///
/// The decrypted plaintext on success, or `CryptoError::AuthenticationFailed`
/// if the HMAC does not verify, or if PKCS#7 unpadding fails.
/// 成功时返回解密后的明文。HMAC 验证失败或 PKCS#7 去填充失败
/// 则返回 `CryptoError::AuthenticationFailed`。
pub fn decrypt(
    enc_key: &[u8; 32],
    mac_key: &[u8; 32],
    iv: &[u8; 16],
    ciphertext: &[u8],
    hmac_tag: &[u8; 32],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    // 1. Verify HMAC FIRST over (aad || iv || ciphertext) before any decrypt.
    //    首先对 (aad || iv || ciphertext) 验证 HMAC，再进行解密。
    let mut mac = HmacSha256::new_from_slice(mac_key)
        .expect("HMAC accepts any key size");
    mac.update(aad);
    mac.update(iv);
    mac.update(ciphertext);

    // Constant-time comparison via `verify_slice`.
    // 通过 `verify_slice` 进行常量时间比较。
    mac.verify_slice(hmac_tag)
        .map_err(|_| CryptoError::AuthenticationFailed)?;

    // 2. AES-256-CBC decrypt with PKCS#7 unpadding.
    //    AES-256-CBC 解密并去除 PKCS#7 填充。
    Aes256CbcDec::new(enc_key.into(), iv.into())
        .decrypt_padded_vec::<Pkcs7>(ciphertext)
        .map_err(|_| CryptoError::AuthenticationFailed)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip: encrypt then decrypt should return the original plaintext.
    /// 往返测试：加密后解密应返回原始明文。
    #[test]
    fn round_trip() {
        let enc_key = [0x42u8; 32];
        let mac_key = [0x43u8; 32];
        let plaintext = b"Hello, AES-256-CBC + HMAC-SHA256!";

        let (ciphertext, iv, hmac_tag) = encrypt(&enc_key, &mac_key, plaintext, b"").unwrap();
        let decrypted = decrypt(&enc_key, &mac_key, &iv, &ciphertext, &hmac_tag, b"").unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// Decryption with a wrong encryption key should fail.
    /// 使用错误的加密密钥解密应失败。
    #[test]
    fn wrong_key() {
        let enc_key = [0x42u8; 32];
        let mac_key = [0x43u8; 32];
        let wrong_enc_key = [0x99u8; 32];
        let plaintext = b"secret data";

        let (ciphertext, iv, hmac_tag) = encrypt(&enc_key, &mac_key, plaintext, b"").unwrap();

        // HMAC was computed with the correct mac_key, but enc_key is wrong.
        // The HMAC should still pass (it's independent of enc_key),
        // but the decrypted plaintext will be garbage and unpadding will fail.
        // HMAC 使用了正确的 mac_key 计算，但 enc_key 是错误的。
        // HMAC 仍应通过（它与 enc_key 无关），
        // 但解密结果将是乱码，去填充将失败。
        let result = decrypt(&wrong_enc_key, &mac_key, &iv, &ciphertext, &hmac_tag, b"");
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::AuthenticationFailed
        ));
    }

    /// Tampered HMAC tag should fail authentication before decryption.
    /// 篡改 HMAC 标签应在解密之前导致认证失败。
    #[test]
    fn tampered_hmac() {
        let enc_key = [0x42u8; 32];
        let mac_key = [0x43u8; 32];
        let plaintext = b"tamper test data";

        let (ciphertext, iv, mut hmac_tag) = encrypt(&enc_key, &mac_key, plaintext, b"").unwrap();

        // Flip one bit in the HMAC tag.
        // 翻转 HMAC 标签中的一个比特。
        hmac_tag[0] ^= 0x01;

        let result = decrypt(&enc_key, &mac_key, &iv, &ciphertext, &hmac_tag, b"");
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::AuthenticationFailed
        ));
    }

    /// Empty plaintext should encrypt and decrypt successfully.
    /// 空明文应能成功加密和解密。
    #[test]
    fn empty_plaintext() {
        let enc_key = [0x42u8; 32];
        let mac_key = [0x43u8; 32];
        let plaintext = b"";

        let (ciphertext, iv, hmac_tag) = encrypt(&enc_key, &mac_key, plaintext, b"").unwrap();

        // Even empty plaintext produces ciphertext due to PKCS#7 padding
        // (one full block of padding).
        // 即使是空明文，由于 PKCS#7 填充也会产生密文（一整块填充）。
        assert!(!ciphertext.is_empty());

        let decrypted = decrypt(&enc_key, &mac_key, &iv, &ciphertext, &hmac_tag, b"").unwrap();
        assert!(decrypted.is_empty());
    }

    /// Tampered ciphertext (not HMAC) should be detected by HMAC verification.
    /// 篡改密文（非 HMAC）应被 HMAC 验证检测到。
    #[test]
    fn tampered_ciphertext() {
        let enc_key = [0x42u8; 32];
        let mac_key = [0x43u8; 32];
        let plaintext = b"ciphertext tamper test";

        let (mut ciphertext, iv, hmac_tag) = encrypt(&enc_key, &mac_key, plaintext, b"").unwrap();

        // Flip one bit in the ciphertext body (not the HMAC tag).
        // 翻转密文主体中的一个比特（不是 HMAC 标签）。
        ciphertext[0] ^= 0x01;

        let result = decrypt(&enc_key, &mac_key, &iv, &ciphertext, &hmac_tag, b"");
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::AuthenticationFailed
        ));
    }

    /// Decryption with a wrong MAC key should fail with AuthenticationFailed.
    /// 使用错误的 MAC 密钥解密应返回 AuthenticationFailed 错误。
    #[test]
    fn wrong_mac_key() {
        let enc_key = [0x42u8; 32];
        let mac_key = [0x43u8; 32];
        let wrong_mac_key = [0x99u8; 32];
        let plaintext = b"mac key test data";

        let (ciphertext, iv, hmac_tag) = encrypt(&enc_key, &mac_key, plaintext, b"").unwrap();

        // Decrypt with incorrect mac_key — HMAC verification should fail.
        // 使用错误的 mac_key 解密 —— HMAC 验证应失败。
        let result = decrypt(&enc_key, &wrong_mac_key, &iv, &ciphertext, &hmac_tag, b"");
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::AuthenticationFailed
        ));
    }

    /// Large plaintext (1 MB) should encrypt and decrypt successfully.
    /// 大明文（1 MB）应能正常加密和解密。
    #[test]
    fn large_plaintext() {
        let enc_key = [0x42u8; 32];
        let mac_key = [0x43u8; 32];
        let plaintext = vec![0xABu8; 1024 * 1024]; // 1 MB

        let (ciphertext, iv, hmac_tag) = encrypt(&enc_key, &mac_key, &plaintext, b"").unwrap();
        let decrypted = decrypt(&enc_key, &mac_key, &iv, &ciphertext, &hmac_tag, b"").unwrap();

        assert_eq!(decrypted, plaintext);
    }

    /// AAD mismatch between encrypt and decrypt must fail HMAC verification.
    /// 加密与解密 AAD 不一致必须导致 HMAC 验证失败。
    #[test]
    fn aad_mismatch_fails() {
        let enc_key = [0x42u8; 32];
        let mac_key = [0x43u8; 32];
        let plaintext = b"aad bound to HMAC";

        let (ciphertext, iv, hmac_tag) =
            encrypt(&enc_key, &mac_key, plaintext, b"header-A").unwrap();

        // Same key/IV/tag/ciphertext but different AAD must be rejected.
        // 相同的 key/IV/tag/密文 但 AAD 不同必须被拒绝。
        let result = decrypt(&enc_key, &mac_key, &iv, &ciphertext, &hmac_tag, b"header-B");
        assert!(matches!(
            result.unwrap_err(),
            CryptoError::AuthenticationFailed
        ));

        // Same AAD succeeds.
        // 相同 AAD 必须成功。
        let ok = decrypt(&enc_key, &mac_key, &iv, &ciphertext, &hmac_tag, b"header-A").unwrap();
        assert_eq!(ok, plaintext);
    }
}
