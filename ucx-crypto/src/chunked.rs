//! Chunked encryption for large files (>64 MiB).
//! 大文件分块加密（>64 MiB）。
//!
//! When a file exceeds 64 MiB, it is split into 1 MiB chunks,
//! each encrypted independently with a derived nonce:
//!   nonce = base_nonce XOR uint96_le(chunk_index)
//!
//! Only AEAD algorithms (AES-256-GCM and ChaCha20-Poly1305) are supported.
//! AES-256-CBC is NOT supported for chunked encryption because CBC mode
//! does not provide built-in per-chunk authentication.
//!
//! 当文件超过 64 MiB 时，按 1 MiB 分块，每块独立加密，
//! 使用派生 nonce：nonce = base_nonce XOR uint96_le(chunk_index)。
//!
//! 仅支持 AEAD 算法（AES-256-GCM 和 ChaCha20-Poly1305）。
//! 不支持 AES-256-CBC 分块加密，因为 CBC 模式不提供内建的逐块认证。

use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead};
use chacha20poly1305::ChaCha20Poly1305;

use crate::{Algorithm, CryptoError};

// =============================================================================
// Constants / 常量
// =============================================================================

/// Chunk size: 1 MiB (1,048,576 bytes).
/// 分块大小：1 MiB（1,048,576 字节）。
pub const CHUNK_SIZE: usize = 1_048_576;

/// Threshold for chunked encryption: 64 MiB (67,108,864 bytes).
/// 分块加密阈值：64 MiB（67,108,864 字节）。
pub const CHUNKED_THRESHOLD: u64 = 67_108_864;

/// Authentication tag size in bytes (both GCM and Poly1305 use 16 bytes).
/// 认证标签大小（字节），GCM 和 Poly1305 均为 16 字节。
const TAG_SIZE: usize = 16;

// =============================================================================
// Types / 类型定义
// =============================================================================

/// A single encrypted chunk with its ciphertext and authentication tag.
/// 单个加密分块，包含密文和认证标签。
#[derive(Debug, Clone)]
pub struct ChunkData {
    /// The encrypted data for this chunk (without the tag).
    /// 该分块的加密数据（不含标签）。
    pub ciphertext: Vec<u8>,

    /// The 16-byte authentication tag (GCM or Poly1305).
    /// 16 字节认证标签（GCM 或 Poly1305）。
    pub tag: [u8; 16],
}

/// Result of chunked encryption, containing all encrypted chunks.
/// 分块加密的结果，包含所有加密分块。
#[derive(Debug, Clone)]
pub struct ChunkedCiphertext {
    /// Total number of chunks.
    /// 分块总数。
    pub chunk_count: u32,

    /// The encrypted chunks, in order.
    /// 按顺序排列的加密分块。
    pub chunks: Vec<ChunkData>,
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Compute chunk-specific nonce: base_nonce XOR uint96_le(chunk_index).
/// 计算分块专用 nonce：base_nonce XOR uint96_le(chunk_index)。
///
/// Only the low 4 bytes are XORed, which supports up to 2^32 chunks (= 4 PiB).
/// 仅对低 4 字节执行 XOR，支持最多 2^32 个分块（= 4 PiB）。
fn derive_chunk_nonce(base_nonce: &[u8; 12], chunk_index: u32) -> [u8; 12] {
    let mut nonce = *base_nonce;
    let index_bytes = chunk_index.to_le_bytes(); // 4 bytes, little-endian
    // XOR the low 4 bytes of the nonce with the chunk index.
    // 将 nonce 的低 4 字节与 chunk index 进行 XOR。
    for i in 0..4 {
        nonce[i] ^= index_bytes[i];
    }
    nonce
}

/// Internal: encrypt plaintext with AES-256-GCM using a specific nonce.
/// 内部函数：使用指定 nonce 进行 AES-256-GCM 加密。
///
/// Returns `(ciphertext_without_tag, tag)` on success.
/// 成功返回 `(不含标签的密文, 标签)`。
fn encrypt_aes_gcm_with_nonce(
    key: &[u8; 32],
    nonce: &[u8; 12],
    plaintext: &[u8],
) -> Result<(Vec<u8>, [u8; 16]), CryptoError> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce: aes_gcm::Nonce<_> = (*nonce).into();

    // aes-gcm returns ciphertext || tag (tag is last 16 bytes).
    // aes-gcm 返回 密文 || 标签（标签在最后 16 字节）。
    let combined = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| CryptoError::InvalidFormat("AES-256-GCM chunk encryption failed".into()))?;

    // Split ciphertext and tag.
    // 分离密文和标签。
    let ct_len = combined.len() - TAG_SIZE;
    let ciphertext = combined[..ct_len].to_vec();
    let mut tag = [0u8; TAG_SIZE];
    tag.copy_from_slice(&combined[ct_len..]);

    Ok((ciphertext, tag))
}

/// Internal: decrypt ciphertext with AES-256-GCM using a specific nonce.
/// 内部函数：使用指定 nonce 进行 AES-256-GCM 解密。
fn decrypt_aes_gcm_with_nonce(
    key: &[u8; 32],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    tag: &[u8; 16],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce: aes_gcm::Nonce<_> = (*nonce).into();

    // Reassemble ciphertext || tag for aes-gcm.
    // 重新拼接 密文 || 标签 供 aes-gcm 使用。
    let mut combined = Vec::with_capacity(ciphertext.len() + TAG_SIZE);
    combined.extend_from_slice(ciphertext);
    combined.extend_from_slice(tag);

    cipher
        .decrypt(&nonce, combined.as_ref())
        .map_err(|_| CryptoError::AuthenticationFailed)
}

/// Internal: encrypt plaintext with ChaCha20-Poly1305 using a specific nonce.
/// 内部函数：使用指定 nonce 进行 ChaCha20-Poly1305 加密。
///
/// Returns `(ciphertext_without_tag, tag)` on success.
/// 成功返回 `(不含标签的密文, 标签)`。
fn encrypt_chacha20_with_nonce(
    key: &[u8; 32],
    nonce: &[u8; 12],
    plaintext: &[u8],
) -> Result<(Vec<u8>, [u8; 16]), CryptoError> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = chacha20poly1305::Nonce::from(*nonce);

    // chacha20poly1305 returns ciphertext || tag (tag is last 16 bytes).
    // chacha20poly1305 返回 密文 || 标签（标签在最后 16 字节）。
    let combined = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| {
            CryptoError::InvalidFormat("ChaCha20-Poly1305 chunk encryption failed".into())
        })?;

    // Split ciphertext and tag.
    // 分离密文和标签。
    let ct_len = combined.len() - TAG_SIZE;
    let ciphertext = combined[..ct_len].to_vec();
    let mut tag = [0u8; TAG_SIZE];
    tag.copy_from_slice(&combined[ct_len..]);

    Ok((ciphertext, tag))
}

/// Internal: decrypt ciphertext with ChaCha20-Poly1305 using a specific nonce.
/// 内部函数：使用指定 nonce 进行 ChaCha20-Poly1305 解密。
fn decrypt_chacha20_with_nonce(
    key: &[u8; 32],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    tag: &[u8; 16],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = chacha20poly1305::Nonce::from(*nonce);

    // Reassemble ciphertext || tag for chacha20poly1305.
    // 重新拼接 密文 || 标签 供 chacha20poly1305 使用。
    let mut combined = Vec::with_capacity(ciphertext.len() + TAG_SIZE);
    combined.extend_from_slice(ciphertext);
    combined.extend_from_slice(tag);

    cipher
        .decrypt(&nonce, combined.as_ref())
        .map_err(|_| CryptoError::AuthenticationFailed)
}

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Encrypt plaintext in chunks using the specified AEAD algorithm.
/// 使用指定的 AEAD 算法对明文进行分块加密。
///
/// The plaintext is split into `CHUNK_SIZE` (1 MiB) chunks. Each chunk is
/// encrypted with a nonce derived from `base_nonce XOR chunk_index`.
///
/// 明文按 `CHUNK_SIZE`（1 MiB）分块。每块使用
/// `base_nonce XOR chunk_index` 派生的 nonce 加密。
///
/// # Supported algorithms / 支持的算法
///
/// - `Algorithm::Aes256Gcm`
/// - `Algorithm::ChaCha20Poly1305`
///
/// `Algorithm::Aes256Cbc` is **not** supported and will return an error.
/// `Algorithm::Aes256Cbc` **不**被支持，会返回错误。
///
/// # Arguments / 参数
///
/// * `key`        - 256-bit encryption key / 256 位加密密钥
/// * `base_nonce` - 12-byte base nonce (chunk nonces are derived from this) /
///   12 字节基础 nonce（分块 nonce 由此派生）
/// * `plaintext`  - Data to encrypt / 待加密数据
/// * `algorithm`  - The AEAD algorithm to use / 使用的 AEAD 算法
///
/// # Returns / 返回
///
/// A `ChunkedCiphertext` containing all encrypted chunks on success.
/// 成功返回包含所有加密分块的 `ChunkedCiphertext`。
pub fn encrypt_chunked(
    key: &[u8; 32],
    base_nonce: &[u8; 12],
    plaintext: &[u8],
    algorithm: Algorithm,
) -> Result<ChunkedCiphertext, CryptoError> {
    // AES-256-CBC does not support chunked encryption (no per-chunk authentication).
    // AES-256-CBC 不支持分块加密（无逐块认证）。
    if algorithm == Algorithm::Aes256Cbc {
        return Err(CryptoError::UnsupportedAlgorithm(
            "AES-256-CBC does not support chunked encryption".into(),
        ));
    }

    // Split plaintext into CHUNK_SIZE chunks (last chunk may be smaller).
    // 将明文按 CHUNK_SIZE 分块（最后一块可能较小）。
    let chunks_iter: Vec<&[u8]> = plaintext.chunks(CHUNK_SIZE).collect();
    let chunk_count = chunks_iter.len() as u32;

    let mut encrypted_chunks = Vec::with_capacity(chunks_iter.len());

    for (index, chunk) in chunks_iter.iter().enumerate() {
        // Derive per-chunk nonce.
        // 派生逐块 nonce。
        let chunk_nonce = derive_chunk_nonce(base_nonce, index as u32);

        // Encrypt chunk using the appropriate algorithm.
        // 使用对应算法加密分块。
        let (ciphertext, tag) = match algorithm {
            Algorithm::Aes256Gcm => encrypt_aes_gcm_with_nonce(key, &chunk_nonce, chunk)?,
            Algorithm::ChaCha20Poly1305 => {
                encrypt_chacha20_with_nonce(key, &chunk_nonce, chunk)?
            }
            // Already checked above, but exhaustive match required.
            // 上面已检查，但需要穷举匹配。
            Algorithm::Aes256Cbc => unreachable!(),
        };

        encrypted_chunks.push(ChunkData { ciphertext, tag });
    }

    Ok(ChunkedCiphertext {
        chunk_count,
        chunks: encrypted_chunks,
    })
}

/// Decrypt chunked ciphertext back to plaintext.
/// 将分块密文解密还原为明文。
///
/// Each chunk is decrypted with its derived nonce and its authentication tag
/// is verified. If any chunk fails authentication, the entire operation fails.
///
/// 每块使用其派生 nonce 解密，并验证认证标签。
/// 如果任何分块认证失败，整个操作将失败。
///
/// # Arguments / 参数
///
/// * `key`        - 256-bit decryption key / 256 位解密密钥
/// * `base_nonce` - 12-byte base nonce (same as used during encryption) /
///   12 字节基础 nonce（与加密时相同）
/// * `chunked`    - The chunked ciphertext to decrypt / 待解密的分块密文
/// * `algorithm`  - The AEAD algorithm used during encryption / 加密时使用的 AEAD 算法
///
/// # Returns / 返回
///
/// The reassembled plaintext on success.
/// 成功返回重组后的明文。
pub fn decrypt_chunked(
    key: &[u8; 32],
    base_nonce: &[u8; 12],
    chunked: &ChunkedCiphertext,
    algorithm: Algorithm,
) -> Result<Vec<u8>, CryptoError> {
    // AES-256-CBC does not support chunked encryption.
    // AES-256-CBC 不支持分块加密。
    if algorithm == Algorithm::Aes256Cbc {
        return Err(CryptoError::UnsupportedAlgorithm(
            "AES-256-CBC does not support chunked decryption".into(),
        ));
    }

    let mut plaintext = Vec::new();

    for (index, chunk) in chunked.chunks.iter().enumerate() {
        // Derive per-chunk nonce (must match encryption).
        // 派生逐块 nonce（必须与加密时一致）。
        let chunk_nonce = derive_chunk_nonce(base_nonce, index as u32);

        // Decrypt chunk using the appropriate algorithm.
        // 使用对应算法解密分块。
        let decrypted = match algorithm {
            Algorithm::Aes256Gcm => {
                decrypt_aes_gcm_with_nonce(key, &chunk_nonce, &chunk.ciphertext, &chunk.tag)?
            }
            Algorithm::ChaCha20Poly1305 => {
                decrypt_chacha20_with_nonce(key, &chunk_nonce, &chunk.ciphertext, &chunk.tag)?
            }
            Algorithm::Aes256Cbc => unreachable!(),
        };

        plaintext.extend_from_slice(&decrypted);
    }

    Ok(plaintext)
}

/// Serialize `ChunkedCiphertext` to bytes for embedding in UCXE after the header.
/// 将 `ChunkedCiphertext` 序列化为字节，用于嵌入 UCXE 头部之后。
///
/// Binary format (all integers are little-endian):
/// ```text
/// [chunk_count: u32 LE]
/// For each chunk:
///   [chunk_ciphertext_size: u32 LE]
///   [ciphertext: chunk_ciphertext_size bytes]
///   [tag: 16 bytes]
/// ```
///
/// 二进制格式（所有整数为小端序）：
/// ```text
/// [chunk_count: u32 LE]
/// 每个分块：
///   [chunk_ciphertext_size: u32 LE]
///   [ciphertext: chunk_ciphertext_size 字节]
///   [tag: 16 字节]
/// ```
pub fn serialize_chunks(chunked: &ChunkedCiphertext) -> Vec<u8> {
    // Pre-calculate total size for efficient allocation.
    // 预计算总大小以高效分配。
    let total_size: usize = 4 // chunk_count
        + chunked.chunks.iter().map(|c| 4 + c.ciphertext.len() + TAG_SIZE).sum::<usize>();

    let mut buf = Vec::with_capacity(total_size);

    // Write chunk count (u32 LE).
    // 写入分块数量（u32 小端序）。
    buf.extend_from_slice(&chunked.chunk_count.to_le_bytes());

    // Write each chunk: [size: u32 LE][ciphertext][tag: 16B].
    // 写入每个分块：[大小: u32 LE][密文][标签: 16B]。
    for chunk in &chunked.chunks {
        let ct_size = chunk.ciphertext.len() as u32;
        buf.extend_from_slice(&ct_size.to_le_bytes());
        buf.extend_from_slice(&chunk.ciphertext);
        buf.extend_from_slice(&chunk.tag);
    }

    buf
}

/// Deserialize `ChunkedCiphertext` from bytes.
/// 从字节反序列化 `ChunkedCiphertext`。
///
/// # Arguments / 参数
///
/// * `data`      - Serialized bytes (as produced by `serialize_chunks`) /
///   序列化字节（由 `serialize_chunks` 生成）
/// * `algorithm` - The algorithm used (for validation, CBC is rejected) /
///   使用的算法（用于验证，CBC 会被拒绝）
///
/// # Returns / 返回
///
/// A `ChunkedCiphertext` on success.
/// 成功返回 `ChunkedCiphertext`。
pub fn deserialize_chunks(
    data: &[u8],
    algorithm: Algorithm,
) -> Result<ChunkedCiphertext, CryptoError> {
    // AES-256-CBC is not supported for chunked operations.
    // AES-256-CBC 不支持分块操作。
    if algorithm == Algorithm::Aes256Cbc {
        return Err(CryptoError::UnsupportedAlgorithm(
            "AES-256-CBC does not support chunked encryption".into(),
        ));
    }

    // Need at least 4 bytes for chunk_count.
    // 至少需要 4 字节来读取 chunk_count。
    if data.len() < 4 {
        return Err(CryptoError::InvalidFormat(
            "chunked data too short: missing chunk_count".into(),
        ));
    }

    let chunk_count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let mut offset = 4usize;
    let mut chunks = Vec::with_capacity(chunk_count as usize);

    for i in 0..chunk_count {
        // Read ciphertext size (u32 LE).
        // 读取密文大小（u32 小端序）。
        if offset + 4 > data.len() {
            return Err(CryptoError::InvalidFormat(format!(
                "chunked data truncated at chunk {} ciphertext size",
                i
            )));
        }
        let ct_size =
            u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
                as usize;
        offset += 4;

        // Read ciphertext bytes.
        // 读取密文字节。
        if offset + ct_size > data.len() {
            return Err(CryptoError::InvalidFormat(format!(
                "chunked data truncated at chunk {} ciphertext",
                i
            )));
        }
        let ciphertext = data[offset..offset + ct_size].to_vec();
        offset += ct_size;

        // Read 16-byte authentication tag.
        // 读取 16 字节认证标签。
        if offset + TAG_SIZE > data.len() {
            return Err(CryptoError::InvalidFormat(format!(
                "chunked data truncated at chunk {} tag",
                i
            )));
        }
        let mut tag = [0u8; TAG_SIZE];
        tag.copy_from_slice(&data[offset..offset + TAG_SIZE]);
        offset += TAG_SIZE;

        chunks.push(ChunkData { ciphertext, tag });
    }

    Ok(ChunkedCiphertext {
        chunk_count,
        chunks,
    })
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test nonce derivation: index 0 should return base_nonce unchanged,
    /// index > 0 should XOR the low bytes.
    /// 测试 nonce 派生：index 0 应返回不变的 base_nonce，
    /// index > 0 应对低位字节执行 XOR。
    #[test]
    fn test_derive_chunk_nonce() {
        let base = [0x00u8; 12];

        // Index 0: nonce should equal base_nonce (XOR with 0 is identity).
        // 索引 0：nonce 应等于 base_nonce（与 0 异或为恒等运算）。
        let n0 = derive_chunk_nonce(&base, 0);
        assert_eq!(n0, base);

        // Index 1: low byte should be 0x01.
        // 索引 1：低位字节应为 0x01。
        let n1 = derive_chunk_nonce(&base, 1);
        assert_eq!(n1[0], 0x01);
        assert_eq!(n1[1..], [0u8; 11]);

        // Index 256: second byte should be 0x01 (little-endian).
        // 索引 256：第二个字节应为 0x01（小端序）。
        let n256 = derive_chunk_nonce(&base, 256);
        assert_eq!(n256[0], 0x00);
        assert_eq!(n256[1], 0x01);

        // Verify XOR with non-zero base nonce.
        // 验证与非零 base nonce 的 XOR。
        let base2 = [0xFF, 0x00, 0xAA, 0x55, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let n1_b2 = derive_chunk_nonce(&base2, 1);
        // 0xFF XOR 0x01 = 0xFE
        assert_eq!(n1_b2[0], 0xFE);
        assert_eq!(n1_b2[1], 0x00);
        assert_eq!(n1_b2[2], 0xAA);
        assert_eq!(n1_b2[3], 0x55);
    }

    /// Small file (less than CHUNK_SIZE): should produce exactly 1 chunk.
    /// 小文件（小于 CHUNK_SIZE）：应产生恰好 1 个分块。
    #[test]
    fn test_small_file_single_chunk() {
        let key = [0x42u8; 32];
        let nonce = [0x01u8; 12];
        let plaintext = b"Hello, chunked encryption!";

        let chunked = encrypt_chunked(&key, &nonce, plaintext, Algorithm::Aes256Gcm).unwrap();

        // Should be exactly 1 chunk.
        // 应恰好有 1 个分块。
        assert_eq!(chunked.chunk_count, 1);
        assert_eq!(chunked.chunks.len(), 1);

        // Decrypt and verify round-trip.
        // 解密并验证往返正确。
        let decrypted = decrypt_chunked(&key, &nonce, &chunked, Algorithm::Aes256Gcm).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    /// Multi-chunk round trip: 2.5 MiB data should produce 3 chunks.
    /// 多分块往返：2.5 MiB 数据应产生 3 个分块。
    #[test]
    fn test_multi_chunk_round_trip() {
        let key = [0x42u8; 32];
        let nonce = [0xABu8; 12];
        // 2.5 MiB = 2 full chunks + 1 half chunk
        // 2.5 MiB = 2 个完整块 + 1 个半块
        let plaintext = vec![0xCDu8; CHUNK_SIZE * 2 + CHUNK_SIZE / 2];

        let chunked = encrypt_chunked(&key, &nonce, &plaintext, Algorithm::Aes256Gcm).unwrap();

        assert_eq!(chunked.chunk_count, 3);
        assert_eq!(chunked.chunks.len(), 3);

        let decrypted = decrypt_chunked(&key, &nonce, &chunked, Algorithm::Aes256Gcm).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    /// Exact chunk boundary: 2 MiB should produce exactly 2 chunks.
    /// 恰好在分块边界：2 MiB 应产生恰好 2 个分块。
    #[test]
    fn test_exact_chunk_boundary() {
        let key = [0x42u8; 32];
        let nonce = [0x00u8; 12];
        // Exactly 2 MiB = 2 chunks, no remainder.
        // 恰好 2 MiB = 2 个分块，无余数。
        let plaintext = vec![0xEFu8; CHUNK_SIZE * 2];

        let chunked = encrypt_chunked(&key, &nonce, &plaintext, Algorithm::Aes256Gcm).unwrap();

        assert_eq!(chunked.chunk_count, 2);
        assert_eq!(chunked.chunks.len(), 2);

        let decrypted = decrypt_chunked(&key, &nonce, &chunked, Algorithm::Aes256Gcm).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    /// Tampering with a chunk's ciphertext should cause decryption to fail.
    /// 篡改某分块的密文应导致解密失败。
    #[test]
    fn test_tampered_chunk_fails() {
        let key = [0x42u8; 32];
        let nonce = [0x01u8; 12];
        let plaintext = vec![0xAAu8; CHUNK_SIZE * 2];

        let mut chunked = encrypt_chunked(&key, &nonce, &plaintext, Algorithm::Aes256Gcm).unwrap();

        // Tamper with the second chunk's ciphertext.
        // 篡改第二个分块的密文。
        chunked.chunks[1].ciphertext[0] ^= 0xFF;

        let result = decrypt_chunked(&key, &nonce, &chunked, Algorithm::Aes256Gcm);
        assert!(
            matches!(result, Err(CryptoError::AuthenticationFailed)),
            "tampered chunk should fail authentication"
        );
    }

    /// ChaCha20-Poly1305 chunked encryption round trip.
    /// ChaCha20-Poly1305 分块加密往返测试。
    #[test]
    fn test_chacha20_chunked() {
        let key = [0x99u8; 32];
        let nonce = [0x55u8; 12];
        // 1.5 MiB = 2 chunks (1 full + 1 half)
        // 1.5 MiB = 2 个分块（1 个完整 + 1 个半块）
        let plaintext = vec![0xBBu8; CHUNK_SIZE + CHUNK_SIZE / 2];

        let chunked =
            encrypt_chunked(&key, &nonce, &plaintext, Algorithm::ChaCha20Poly1305).unwrap();

        assert_eq!(chunked.chunk_count, 2);

        let decrypted =
            decrypt_chunked(&key, &nonce, &chunked, Algorithm::ChaCha20Poly1305).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    /// AES-256-CBC should be rejected for chunked encryption.
    /// AES-256-CBC 分块加密应被拒绝。
    #[test]
    fn test_aes_cbc_rejected() {
        let key = [0x42u8; 32];
        let nonce = [0x00u8; 12];
        let plaintext = b"should not encrypt";

        let result = encrypt_chunked(&key, &nonce, plaintext, Algorithm::Aes256Cbc);
        assert!(
            matches!(result, Err(CryptoError::UnsupportedAlgorithm(_))),
            "AES-256-CBC should be rejected for chunked encryption"
        );
    }

    /// Serialize and deserialize round trip.
    /// 序列化和反序列化往返测试。
    #[test]
    fn test_serialize_deserialize_round_trip() {
        let key = [0x42u8; 32];
        let nonce = [0xABu8; 12];
        let plaintext = vec![0xCDu8; CHUNK_SIZE + 100];

        let chunked = encrypt_chunked(&key, &nonce, &plaintext, Algorithm::Aes256Gcm).unwrap();

        // Serialize then deserialize.
        // 序列化然后反序列化。
        let serialized = serialize_chunks(&chunked);
        let deserialized = deserialize_chunks(&serialized, Algorithm::Aes256Gcm).unwrap();

        assert_eq!(deserialized.chunk_count, chunked.chunk_count);
        assert_eq!(deserialized.chunks.len(), chunked.chunks.len());

        // Verify the deserialized data can still be decrypted.
        // 验证反序列化后的数据仍可解密。
        let decrypted =
            decrypt_chunked(&key, &nonce, &deserialized, Algorithm::Aes256Gcm).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
