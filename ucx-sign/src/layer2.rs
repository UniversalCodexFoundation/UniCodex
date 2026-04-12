//! Layer 2 UCX Signing Block Implementation.
//!
//! This module implements the APK v2-style signing block for UCX files.
//! It provides functions for:
//! - Computing chunked content digests using BLAKE3
//! - Computing protected content digests (excluding the signing block itself)
//! - Building and parsing the binary signing block structure
//! - Building signer data blobs with Ed25519 signatures
//!
//! Layer 2 UCX 签名块实现。
//!
//! 本模块实现 UCX 文件的 APK v2 式签名块，提供以下功能：
//! - 使用 BLAKE3 计算分块内容摘要
//! - 计算受保护内容摘要（排除签名块本身）
//! - 构建和解析签名块二进制结构
//! - 构建包含 Ed25519 签名的签名者数据

use ed25519_dalek::{Signer, SigningKey};

use crate::SignError;
use crate::zip_binary;

// =============================================================================
// Constants / 常量
// =============================================================================

/// Chunk size for content digest: 1 MiB (1,048,576 bytes).
/// 内容摘要的分块大小：1 MiB（1,048,576 字节）。
const CHUNK_SIZE: usize = 1024 * 1024;

/// Prefix byte for individual chunk digests (0xa5).
/// 单个分块摘要的前缀字节（0xa5）。
const CHUNK_DIGEST_PREFIX: u8 = 0xa5;

/// Prefix byte for the top-level digest (0x5a).
/// 顶层摘要的前缀字节（0x5a）。
const TOP_DIGEST_PREFIX: u8 = 0x5a;

/// Algorithm ID for Ed25519 + BLAKE3 digest scheme.
/// Ed25519 + BLAKE3 摘要方案的算法 ID。
const ALGORITHM_ID_ED25519_BLAKE3: u32 = 0x0001;

// =============================================================================
// Content digest / 内容摘要
// =============================================================================

/// Compute the chunked content digest for Layer 2 signing.
///
/// Algorithm (from spec):
/// 1. Split data into 1 MiB chunks (last chunk may be smaller)
/// 2. `chunk_digest[i] = BLAKE3(0xa5 || u32_le(chunk_len) || chunk[i])`
/// 3. `top_digest = BLAKE3(0x5a || u32_le(chunk_count) || chunk_digest[0] || ...)`
///
/// 计算 Layer 2 签名的分块内容摘要。
///
/// 算法（来自规范）：
/// 1. 将数据分割为 1 MiB 的块（最后一块可能更小）
/// 2. `chunk_digest[i] = BLAKE3(0xa5 || u32_le(chunk_len) || chunk[i])`
/// 3. `top_digest = BLAKE3(0x5a || u32_le(chunk_count) || chunk_digest[0] || ...)`
///
/// # Arguments / 参数
///
/// * `data` - The raw byte data to digest.
///   要计算摘要的原始字节数据。
///
/// # Returns / 返回
///
/// A 32-byte BLAKE3 digest.
/// 32 字节的 BLAKE3 摘要。
pub fn compute_chunked_digest(data: &[u8]) -> [u8; 32] {
    // Split data into chunks of CHUNK_SIZE bytes.
    // 将数据分割为 CHUNK_SIZE 字节的块。
    let chunks: Vec<&[u8]> = if data.is_empty() {
        // Empty data produces a single empty chunk.
        // 空数据产生一个空块。
        vec![&[]]
    } else {
        data.chunks(CHUNK_SIZE).collect()
    };

    let chunk_count = chunks.len() as u32;

    // Compute per-chunk digests.
    // 计算每个分块的摘要。
    let mut chunk_digests = Vec::with_capacity(chunks.len() * 32);
    for chunk in &chunks {
        // chunk_digest = BLAKE3(0xa5 || u32_le(chunk_len) || chunk_data)
        let chunk_len = chunk.len() as u32;
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[CHUNK_DIGEST_PREFIX]);
        hasher.update(&chunk_len.to_le_bytes());
        hasher.update(chunk);
        let digest = hasher.finalize();
        chunk_digests.extend_from_slice(digest.as_bytes());
    }

    // Compute the top-level digest.
    // 计算顶层摘要。
    // top_digest = BLAKE3(0x5a || u32_le(chunk_count) || all_chunk_digests)
    let mut top_hasher = blake3::Hasher::new();
    top_hasher.update(&[TOP_DIGEST_PREFIX]);
    top_hasher.update(&chunk_count.to_le_bytes());
    top_hasher.update(&chunk_digests);
    let top_digest = top_hasher.finalize();

    *top_digest.as_bytes()
}

/// Compute the protected content digest for signing.
///
/// The protected content consists of three sections:
/// - Section 1: ZIP entries (bytes `0..cd_offset`, local file data)
/// - Section 3: Central Directory (bytes `cd_offset..eocd_offset`)
/// - Section 4: EOCD (bytes `eocd_offset..end`)
///
/// The signing block itself (Section 2) is excluded from the digest.
///
/// 计算用于签名的受保护内容摘要。
///
/// 受保护内容由三个部分组成：
/// - 第 1 部分：ZIP 条目（字节 `0..cd_offset`，本地文件数据）
/// - 第 3 部分：中央目录（字节 `cd_offset..eocd_offset`）
/// - 第 4 部分：EOCD（字节 `eocd_offset..末尾`）
///
/// 签名块本身（第 2 部分）被排除在摘要之外。
///
/// # Arguments / 参数
///
/// * `zip_data`    - The complete ZIP file data (without signing block).
///   完整的 ZIP 文件数据（不含签名块）。
/// * `cd_offset`   - Offset of the Central Directory.
///   中央目录的偏移。
/// * `eocd_offset` - Offset of the EOCD record.
///   EOCD 记录的偏移。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if offsets are invalid.
/// 偏移无效时返回 `SignError::SigningFailed`。
pub fn compute_protected_digest(
    zip_data: &[u8],
    cd_offset: usize,
    eocd_offset: usize,
) -> Result<[u8; 32], SignError> {
    // Validate that offsets are in order and within bounds.
    // 验证偏移是有序且在合法范围内的。
    if cd_offset > eocd_offset || eocd_offset > zip_data.len() {
        return Err(SignError::SigningFailed(format!(
            "invalid offsets: cd_offset={cd_offset}, eocd_offset={eocd_offset}, data_len={}",
            zip_data.len()
        )));
    }

    // Concatenate: Section 1 (entries) + Section 3 (CD) + Section 4 (EOCD).
    // 拼接：第 1 部分（条目） + 第 3 部分（CD） + 第 4 部分（EOCD）。
    let section1 = &zip_data[..cd_offset];
    let section3 = &zip_data[cd_offset..eocd_offset];
    let section4 = &zip_data[eocd_offset..];

    let mut protected = Vec::with_capacity(section1.len() + section3.len() + section4.len());
    protected.extend_from_slice(section1);
    protected.extend_from_slice(section3);
    protected.extend_from_slice(section4);

    // Compute the chunked digest of the protected content.
    // 计算受保护内容的分块摘要。
    Ok(compute_chunked_digest(&protected))
}

// =============================================================================
// Signer data / 签名者数据
// =============================================================================

/// A parsed signer entry extracted from a UCX Signing Block.
///
/// 从 UCX 签名块中解析出的签名者条目。
#[derive(Debug, Clone)]
pub struct SignerEntry {
    /// Signature algorithm ID (e.g., 0x0001 for Ed25519 + BLAKE3).
    /// 签名算法 ID（如 0x0001 表示 Ed25519 + BLAKE3）。
    pub algorithm_id: u32,

    /// Content digest bytes (32 bytes for BLAKE3).
    /// 内容摘要字节（BLAKE3 为 32 字节）。
    pub digest: Vec<u8>,

    /// Ed25519 signature bytes (64 bytes).
    /// Ed25519 签名字节（64 字节）。
    pub signature: Vec<u8>,

    /// Ed25519 public key bytes (32 bytes).
    /// Ed25519 公钥字节（32 字节）。
    pub public_key: Vec<u8>,

    /// DER-encoded X.509 certificate bytes.
    /// DER 编码的 X.509 证书字节。
    pub cert_der: Vec<u8>,
}

/// Build a signer data blob for the signing block.
///
/// Format:
/// - `signed_data_length`: u32 LE
/// - `signed_data`:
///   - `digest_algorithm_id`: u32 LE (0x0001 for Ed25519+BLAKE3)
///   - `digest`: [u8; 32]
///   - `cert_length`: u32 LE
///   - `cert_der`: [u8]
/// - `signature_algorithm_id`: u32 LE (0x0001)
/// - `signature_length`: u32 LE
/// - `signature`: [u8; 64]
/// - `public_key_length`: u32 LE
/// - `public_key`: [u8; 32]
///
/// 构建签名块中的签名者数据。
///
/// # Arguments / 参数
///
/// * `digest`      - The 32-byte content digest to sign.
///   要签名的 32 字节内容摘要。
/// * `signing_key` - The Ed25519 signing key.
///   Ed25519 签名密钥。
/// * `cert_der`    - DER-encoded X.509 certificate.
///   DER 编码的 X.509 证书。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if the signing operation fails.
/// 签名操作失败时返回 `SignError::SigningFailed`。
pub fn build_signer_data(
    digest: &[u8; 32],
    signing_key: &SigningKey,
    cert_der: &[u8],
) -> Result<Vec<u8>, SignError> {
    // --- Build signed_data ---
    // 构建 signed_data。
    //
    // signed_data = digest_algorithm_id(4) + digest(32) + cert_length(4) + cert_der(N)
    let signed_data_len = 4 + 32 + 4 + cert_der.len();
    let mut signed_data = Vec::with_capacity(signed_data_len);
    signed_data.extend_from_slice(&ALGORITHM_ID_ED25519_BLAKE3.to_le_bytes()); // digest_algorithm_id
    signed_data.extend_from_slice(digest);                                      // digest
    signed_data.extend_from_slice(&(cert_der.len() as u32).to_le_bytes());      // cert_length
    signed_data.extend_from_slice(cert_der);                                    // cert_der

    // --- Sign the signed_data ---
    // 对 signed_data 进行签名。
    let signature = signing_key.sign(&signed_data);
    let signature_bytes = signature.to_bytes();

    // --- Get the public key ---
    // 获取公钥。
    let public_key = signing_key.verifying_key();
    let public_key_bytes = public_key.to_bytes();

    // --- Assemble the complete signer data blob ---
    // 组装完整的签名者数据。
    //
    // Layout:
    //   signed_data_length(4) + signed_data(N)
    //   + signature_algorithm_id(4) + signature_length(4) + signature(64)
    //   + public_key_length(4) + public_key(32)
    let total_len = 4 + signed_data.len() + 4 + 4 + 64 + 4 + 32;
    let mut blob = Vec::with_capacity(total_len);

    // signed_data_length + signed_data
    blob.extend_from_slice(&(signed_data.len() as u32).to_le_bytes());
    blob.extend_from_slice(&signed_data);

    // signature_algorithm_id
    blob.extend_from_slice(&ALGORITHM_ID_ED25519_BLAKE3.to_le_bytes());

    // signature_length + signature
    blob.extend_from_slice(&(signature_bytes.len() as u32).to_le_bytes());
    blob.extend_from_slice(&signature_bytes);

    // public_key_length + public_key
    blob.extend_from_slice(&(public_key_bytes.len() as u32).to_le_bytes());
    blob.extend_from_slice(&public_key_bytes);

    Ok(blob)
}

// =============================================================================
// Signing block construction / 签名块构建
// =============================================================================

/// Build the complete UCX Signing Block.
///
/// Block format:
/// ```text
/// [size_of_block: u64 LE]           <-- total size excluding this leading u64
/// [pair_size: u64 LE][pair_id: u32 LE][signers_data...]
/// [size_of_block: u64 LE]           <-- repeated (same value)
/// [magic: 16 bytes]                 <-- UCX_SIGNING_BLOCK_MAGIC
/// ```
///
/// `size_of_block` = len(pair_block) + 8(trailing size) + 16(magic).
///
/// 构建完整的 UCX 签名块。
///
/// # Arguments / 参数
///
/// * `signers_data` - The serialized signer data bytes.
///   序列化的签名者数据字节。
///
/// # Returns / 返回
///
/// The complete signing block bytes, ready for insertion into a ZIP file.
/// 完整的签名块字节，可直接插入 ZIP 文件。
pub fn build_signing_block(signers_data: &[u8]) -> Vec<u8> {
    // --- Build the pair block ---
    // 构建 pair 块。
    //
    // pair_block = pair_size(u64) + pair_id(u32) + signers_data
    // pair_size = 4(pair_id) + len(signers_data)
    let pair_size = (4 + signers_data.len()) as u64;
    let mut pair_block = Vec::with_capacity(8 + 4 + signers_data.len());
    pair_block.extend_from_slice(&pair_size.to_le_bytes());              // pair_size
    pair_block.extend_from_slice(&zip_binary::PAIR_ID_UCX_SIG_V1.to_le_bytes()); // pair_id
    pair_block.extend_from_slice(signers_data);                          // signers_data

    // --- Calculate size_of_block ---
    // 计算 size_of_block。
    //
    // size_of_block = len(pair_block) + 8(trailing size) + 16(magic)
    let size_of_block = (pair_block.len() + 8 + 16) as u64;

    // --- Assemble the complete block ---
    // 组装完整的块。
    let total_len = 8 + pair_block.len() + 8 + 16;
    let mut block = Vec::with_capacity(total_len);
    block.extend_from_slice(&size_of_block.to_le_bytes());               // size_of_block (leading)
    block.extend_from_slice(&pair_block);                                // pair_block
    block.extend_from_slice(&size_of_block.to_le_bytes());               // size_of_block (trailing)
    block.extend_from_slice(zip_binary::UCX_SIGNING_BLOCK_MAGIC);        // magic

    block
}

// =============================================================================
// Signing block parsing / 签名块解析
// =============================================================================

/// Parse a UCX Signing Block and extract signer entries.
///
/// Expected format (from `build_signing_block`):
/// ```text
/// [size_of_block: u64 LE]
/// [pair_size: u64 LE][pair_id: u32 LE][signers_data...]
/// [size_of_block: u64 LE]
/// [magic: 16 bytes]
/// ```
///
/// The signers_data is parsed according to the `build_signer_data` format.
///
/// 解析 UCX 签名块并提取签名者条目。
///
/// # Arguments / 参数
///
/// * `block_data` - The raw signing block bytes.
///   原始签名块字节。
///
/// # Returns / 返回
///
/// A vector of `SignerEntry` structs on success.
/// 成功时返回 `SignerEntry` 结构体向量。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if the block format is invalid.
/// 签名块格式无效时返回 `SignError::SigningFailed`。
pub fn parse_signing_block(block_data: &[u8]) -> Result<Vec<SignerEntry>, SignError> {
    // Minimum block size: 8(size) + 8(pair_size) + 4(pair_id) + 8(size) + 16(magic) = 44
    // 最小块大小：8(大小) + 8(pair_size) + 4(pair_id) + 8(大小) + 16(magic) = 44
    if block_data.len() < 44 {
        return Err(SignError::SigningFailed(
            "signing block too small to be valid".to_string(),
        ));
    }

    // Verify the magic at the end.
    // 验证末尾的 magic。
    let magic_start = block_data.len() - 16;
    if &block_data[magic_start..] != zip_binary::UCX_SIGNING_BLOCK_MAGIC {
        return Err(SignError::SigningFailed(
            "signing block magic mismatch".to_string(),
        ));
    }

    // Read the leading size_of_block.
    // 读取开头的 size_of_block。
    let mut pos = 0;
    let _size_of_block = read_u64_le(block_data, &mut pos)?;

    // Parse pairs until we reach the trailing size_of_block.
    // 解析 pair，直到到达尾部的 size_of_block。
    let pairs_end = magic_start - 8; // Position of trailing size_of_block.
                                      // 尾部 size_of_block 的位置。

    let mut entries = Vec::new();

    while pos < pairs_end {
        // Read pair_size and pair_id.
        // 读取 pair_size 和 pair_id。
        let pair_size = read_u64_le(block_data, &mut pos)? as usize;
        if pair_size < 4 {
            return Err(SignError::SigningFailed(
                "pair_size too small (must be at least 4 for pair_id)".to_string(),
            ));
        }

        let pair_id = read_u32_le(block_data, &mut pos)?;
        let signers_data_len = pair_size - 4;

        if pos + signers_data_len > pairs_end {
            return Err(SignError::SigningFailed(
                "pair data exceeds block bounds".to_string(),
            ));
        }

        // Only parse known pair IDs.
        // 仅解析已知的 pair ID。
        if pair_id == zip_binary::PAIR_ID_UCX_SIG_V1 {
            let signer_data = &block_data[pos..pos + signers_data_len];
            let entry = parse_signer_data(signer_data)?;
            entries.push(entry);
        }

        pos += signers_data_len;
    }

    Ok(entries)
}

/// Parse a single signer data blob into a `SignerEntry`.
///
/// Format (matches `build_signer_data`):
/// - signed_data_length: u32 LE
/// - signed_data:
///   - digest_algorithm_id: u32 LE
///   - digest: [u8; 32]
///   - cert_length: u32 LE
///   - cert_der: [u8]
/// - signature_algorithm_id: u32 LE
/// - signature_length: u32 LE
/// - signature: [u8; 64]
/// - public_key_length: u32 LE
/// - public_key: [u8; 32]
///
/// 将单个签名者数据解析为 `SignerEntry`。
fn parse_signer_data(data: &[u8]) -> Result<SignerEntry, SignError> {
    let mut pos = 0;

    // --- Read signed_data ---
    // 读取 signed_data。
    let signed_data_len = read_u32_le(data, &mut pos)? as usize;
    if pos + signed_data_len > data.len() {
        return Err(SignError::SigningFailed(
            "signed_data_length exceeds signer data bounds".to_string(),
        ));
    }

    // Parse fields within signed_data.
    // 解析 signed_data 中的字段。
    let signed_data_start = pos;
    let algorithm_id = read_u32_le(data, &mut pos)?;

    // Read digest (32 bytes).
    // 读取摘要（32 字节）。
    if pos + 32 > signed_data_start + signed_data_len {
        return Err(SignError::SigningFailed(
            "not enough data for digest in signed_data".to_string(),
        ));
    }
    let digest = data[pos..pos + 32].to_vec();
    pos += 32;

    // Read cert_length and cert_der.
    // 读取证书长度和证书 DER。
    let cert_length = read_u32_le(data, &mut pos)? as usize;
    if pos + cert_length > signed_data_start + signed_data_len {
        return Err(SignError::SigningFailed(
            "cert_length exceeds signed_data bounds".to_string(),
        ));
    }
    let cert_der = data[pos..pos + cert_length].to_vec();
    pos = signed_data_start + signed_data_len;

    // --- Read signature ---
    // 读取签名。
    let _sig_algorithm_id = read_u32_le(data, &mut pos)?;
    let sig_length = read_u32_le(data, &mut pos)? as usize;
    if pos + sig_length > data.len() {
        return Err(SignError::SigningFailed(
            "signature_length exceeds signer data bounds".to_string(),
        ));
    }
    let signature = data[pos..pos + sig_length].to_vec();
    pos += sig_length;

    // --- Read public_key ---
    // 读取公钥。
    let pk_length = read_u32_le(data, &mut pos)? as usize;
    if pos + pk_length > data.len() {
        return Err(SignError::SigningFailed(
            "public_key_length exceeds signer data bounds".to_string(),
        ));
    }
    let public_key = data[pos..pos + pk_length].to_vec();

    Ok(SignerEntry {
        algorithm_id,
        digest,
        signature,
        public_key,
        cert_der,
    })
}

// =============================================================================
// Binary read helpers / 二进制读取辅助函数
// =============================================================================

/// Read a u32 little-endian value from `data` at `pos`, advancing `pos` by 4.
///
/// 从 `data` 的 `pos` 位置读取 u32 小端序值，并将 `pos` 前移 4 字节。
fn read_u32_le(data: &[u8], pos: &mut usize) -> Result<u32, SignError> {
    if *pos + 4 > data.len() {
        return Err(SignError::SigningFailed(format!(
            "unexpected end of data reading u32 at offset {pos}",
        )));
    }
    let value = u32::from_le_bytes([
        data[*pos],
        data[*pos + 1],
        data[*pos + 2],
        data[*pos + 3],
    ]);
    *pos += 4;
    Ok(value)
}

/// Read a u64 little-endian value from `data` at `pos`, advancing `pos` by 8.
///
/// 从 `data` 的 `pos` 位置读取 u64 小端序值，并将 `pos` 前移 8 字节。
fn read_u64_le(data: &[u8], pos: &mut usize) -> Result<u64, SignError> {
    if *pos + 8 > data.len() {
        return Err(SignError::SigningFailed(format!(
            "unexpected end of data reading u64 at offset {pos}",
        )));
    }
    let value = u64::from_le_bytes([
        data[*pos],
        data[*pos + 1],
        data[*pos + 2],
        data[*pos + 3],
        data[*pos + 4],
        data[*pos + 5],
        data[*pos + 6],
        data[*pos + 7],
    ]);
    *pos += 8;
    Ok(value)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cert::{CertOptions, create_self_signed_cert};
    use crate::keys::generate_ed25519_keypair;

    /// Test: compute_chunked_digest on small data (< 1 MiB, single chunk).
    /// 测试：对小数据（< 1 MiB，单块）计算分块摘要。
    #[test]
    fn test_chunked_digest_small_data() {
        let data = b"Hello, Unicodex! This is a small test.";

        let digest1 = compute_chunked_digest(data);
        let digest2 = compute_chunked_digest(data);

        // Digest must be deterministic.
        // 摘要必须是确定性的。
        assert_eq!(digest1, digest2, "digest must be deterministic");

        // Digest must be 32 bytes (BLAKE3 output).
        // 摘要必须是 32 字节（BLAKE3 输出）。
        assert_eq!(digest1.len(), 32, "digest must be 32 bytes");

        // Different data must produce a different digest.
        // 不同数据必须产生不同的摘要。
        let other_digest = compute_chunked_digest(b"different data");
        assert_ne!(
            digest1, other_digest,
            "different data must produce different digests"
        );
    }

    /// Test: compute_chunked_digest on data exactly 1 MiB.
    /// 测试：对恰好 1 MiB 的数据计算分块摘要。
    #[test]
    fn test_chunked_digest_exact_1mib() {
        // Create exactly 1 MiB of data.
        // 创建恰好 1 MiB 的数据。
        let data = vec![0xABu8; CHUNK_SIZE];

        let digest = compute_chunked_digest(&data);
        assert_eq!(digest.len(), 32, "digest must be 32 bytes");

        // Verify it produces a single chunk:
        // manually compute what we expect.
        // 验证它产生单个块：手动计算期望值。
        let mut chunk_hasher = blake3::Hasher::new();
        chunk_hasher.update(&[CHUNK_DIGEST_PREFIX]);
        chunk_hasher.update(&(CHUNK_SIZE as u32).to_le_bytes());
        chunk_hasher.update(&data);
        let expected_chunk_digest = chunk_hasher.finalize();

        let mut top_hasher = blake3::Hasher::new();
        top_hasher.update(&[TOP_DIGEST_PREFIX]);
        top_hasher.update(&1u32.to_le_bytes()); // 1 chunk
        top_hasher.update(expected_chunk_digest.as_bytes());
        let expected_top = top_hasher.finalize();

        assert_eq!(
            digest,
            *expected_top.as_bytes(),
            "digest for exactly 1 MiB must match manual calculation"
        );
    }

    /// Test: compute_chunked_digest on data > 1 MiB (multiple chunks).
    /// 测试：对大于 1 MiB 的数据（多块）计算分块摘要。
    #[test]
    fn test_chunked_digest_multiple_chunks() {
        // Create 2.5 MiB of data (3 chunks: 1 MiB, 1 MiB, 0.5 MiB).
        // 创建 2.5 MiB 的数据（3 块：1 MiB、1 MiB、0.5 MiB）。
        let data = vec![0xCDu8; CHUNK_SIZE * 2 + CHUNK_SIZE / 2];

        let digest = compute_chunked_digest(&data);
        assert_eq!(digest.len(), 32, "digest must be 32 bytes");

        // Verify by manual computation.
        // 通过手动计算验证。
        let chunks: Vec<&[u8]> = data.chunks(CHUNK_SIZE).collect();
        assert_eq!(chunks.len(), 3, "should have 3 chunks");

        let mut all_chunk_digests = Vec::new();
        for chunk in &chunks {
            let mut h = blake3::Hasher::new();
            h.update(&[CHUNK_DIGEST_PREFIX]);
            h.update(&(chunk.len() as u32).to_le_bytes());
            h.update(chunk);
            all_chunk_digests.extend_from_slice(h.finalize().as_bytes());
        }

        let mut top_h = blake3::Hasher::new();
        top_h.update(&[TOP_DIGEST_PREFIX]);
        top_h.update(&3u32.to_le_bytes());
        top_h.update(&all_chunk_digests);
        let expected = top_h.finalize();

        assert_eq!(
            digest,
            *expected.as_bytes(),
            "multi-chunk digest must match manual calculation"
        );
    }

    /// Test: build_signing_block + parse_signing_block roundtrip.
    /// 测试：build_signing_block + parse_signing_block 往返一致。
    #[test]
    fn test_signing_block_build_parse_roundtrip() {
        // Generate a key pair and certificate for testing.
        // 生成用于测试的密钥对和证书。
        let (signing_key, _) =
            generate_ed25519_keypair().expect("key generation should succeed");

        let cert_der = create_self_signed_cert(
            &signing_key,
            &CertOptions {
                common_name: "Block Roundtrip Test".to_string(),
                days_valid: 30,
                organization: None,
            },
        )
        .expect("cert generation should succeed");

        let digest = compute_chunked_digest(b"test data for roundtrip");

        // Build signer data.
        // 构建签名者数据。
        let signer_data = build_signer_data(&digest, &signing_key, &cert_der)
            .expect("build_signer_data should succeed");

        // Build the complete signing block.
        // 构建完整的签名块。
        let block = build_signing_block(&signer_data);

        // Parse the block back.
        // 解析回签名块。
        let entries =
            parse_signing_block(&block).expect("parse_signing_block should succeed");

        // Should have exactly 1 signer entry.
        // 应有恰好 1 个签名者条目。
        assert_eq!(entries.len(), 1, "should parse exactly 1 signer entry");

        let entry = &entries[0];

        // Verify the parsed fields match the originals.
        // 验证解析出的字段与原始值一致。
        assert_eq!(
            entry.algorithm_id, ALGORITHM_ID_ED25519_BLAKE3,
            "algorithm_id must match"
        );
        assert_eq!(entry.digest, digest.to_vec(), "digest must match");
        assert_eq!(entry.cert_der, cert_der, "cert_der must match");
        assert_eq!(
            entry.public_key,
            signing_key.verifying_key().to_bytes().to_vec(),
            "public_key must match"
        );
        assert_eq!(entry.signature.len(), 64, "signature must be 64 bytes");
    }

    /// Test: build_signer_data roundtrip with parse.
    /// 测试：build_signer_data 构建的数据可被正确解析。
    #[test]
    fn test_signer_data_build_parse_roundtrip() {
        let (signing_key, _) =
            generate_ed25519_keypair().expect("key generation should succeed");

        let cert_der = create_self_signed_cert(
            &signing_key,
            &CertOptions {
                common_name: "Signer Data Test".to_string(),
                days_valid: 30,
                organization: Some("Test Org".to_string()),
            },
        )
        .expect("cert generation should succeed");

        let digest = compute_chunked_digest(b"signer data test content");

        // Build signer data.
        // 构建签名者数据。
        let signer_data = build_signer_data(&digest, &signing_key, &cert_der)
            .expect("build_signer_data should succeed");

        // Parse it directly.
        // 直接解析。
        let entry = parse_signer_data(&signer_data)
            .expect("parse_signer_data should succeed");

        // Verify all fields.
        // 验证所有字段。
        assert_eq!(entry.algorithm_id, ALGORITHM_ID_ED25519_BLAKE3);
        assert_eq!(entry.digest, digest.to_vec());
        assert_eq!(entry.cert_der, cert_der);
        assert_eq!(entry.signature.len(), 64);
        assert_eq!(
            entry.public_key,
            signing_key.verifying_key().to_bytes().to_vec()
        );

        // Verify the signature is valid.
        // 验证签名是否有效。
        let verifying_key = signing_key.verifying_key();
        let sig_bytes: [u8; 64] = entry
            .signature
            .as_slice()
            .try_into()
            .expect("signature should be 64 bytes");
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);

        // Reconstruct signed_data to verify the signature.
        // 重建 signed_data 以验证签名。
        let mut signed_data = Vec::new();
        signed_data.extend_from_slice(&ALGORITHM_ID_ED25519_BLAKE3.to_le_bytes());
        signed_data.extend_from_slice(&digest);
        signed_data.extend_from_slice(&(cert_der.len() as u32).to_le_bytes());
        signed_data.extend_from_slice(&cert_der);

        use ed25519_dalek::Verifier;
        assert!(
            verifying_key.verify(&signed_data, &signature).is_ok(),
            "signature verification must succeed"
        );
    }
}
