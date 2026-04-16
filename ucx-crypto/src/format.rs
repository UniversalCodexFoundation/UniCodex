//! UCXE binary format serialization and deserialization.
//! UCXE 二进制格式的序列化与反序列化。
//!
//! This module handles reading and writing the UCXE encrypted file format.
//! The binary layout is:
//!
//! 本模块负责 UCXE 加密文件格式的读写。二进制布局如下：
//!
//! ```text
//! [Magic: 4B "UCXE"][Version: 1B][AlgoID: 1B][KdfID: 1B][Reserved: 1B 0x00]
//! --- Only when KDF != None / 仅当 KDF != None ---
//! Argon2id params: [memory_cost_kib: u32 LE][time_cost: u32 LE][parallelism: u32 LE] (12B)
//! PBKDF2 params:   [iterations: u32 LE] (4B)
//! ---
//! [Salt Length: u16 LE][Salt: N bytes]
//! [IV Length: u16 LE][IV: N bytes]
//! [Ciphertext Length: u64 LE][Ciphertext: N bytes]
//! [Auth Tag: M bytes] (GCM=16, CBC/HMAC=32, ChaCha20=16, no length prefix)
//! ```

use std::io::{Read, Write, Cursor};
use crate::{Algorithm, CryptoError, Kdf, UCXE_FORMAT_VERSION, UCXE_MAGIC};

// =============================================================================
// Types / 类型定义
// =============================================================================

/// UCXE file header containing format version, algorithm, and KDF identifiers.
///
/// UCXE 文件头，包含格式版本、加密算法和 KDF 标识。
#[derive(Debug, Clone)]
pub struct UcxeHeader {
    /// Format version byte (currently 0x01).
    /// 格式版本字节（当前为 0x01）。
    pub format_version: u8,

    /// Encryption algorithm used for this file.
    /// 此文件使用的加密算法。
    pub algorithm: Algorithm,

    /// Key derivation function used for this file.
    /// 此文件使用的密钥派生函数。
    pub kdf: Kdf,
}

/// Key derivation function parameters stored in the UCXE file.
///
/// 存储在 UCXE 文件中的密钥派生函数参数。
#[derive(Debug, Clone)]
pub enum KdfParams {
    /// No KDF — key is provided directly.
    /// 无 KDF —— 密钥直接提供。
    None,

    /// Argon2id parameters.
    /// Argon2id 参数。
    Argon2id {
        /// Memory cost in KiB.
        /// 内存开销（KiB）。
        memory_cost_kib: u32,

        /// Number of iterations (time cost).
        /// 迭代次数（时间开销）。
        time_cost: u32,

        /// Degree of parallelism.
        /// 并行度。
        parallelism: u32,
    },

    /// PBKDF2-HMAC-SHA256 parameters.
    /// PBKDF2-HMAC-SHA256 参数。
    Pbkdf2 {
        /// Number of iterations.
        /// 迭代次数。
        iterations: u32,
    },
}

/// Complete representation of a UCXE encrypted file.
///
/// UCXE 加密文件的完整表示。
#[derive(Debug, Clone)]
pub struct UcxeFile {
    /// File header (version, algorithm, KDF).
    /// 文件头（版本、算法、KDF）。
    pub header: UcxeHeader,

    /// KDF parameters (if any).
    /// KDF 参数（如有）。
    pub kdf_params: KdfParams,

    /// Salt bytes (may be empty if no KDF).
    /// 盐值字节（无 KDF 时可为空）。
    pub salt: Vec<u8>,

    /// Initialization vector / nonce bytes.
    /// 初始化向量 / nonce 字节。
    pub iv: Vec<u8>,

    /// Encrypted ciphertext bytes.
    /// 加密后的密文字节。
    pub ciphertext: Vec<u8>,

    /// Authentication tag bytes.
    /// 认证标签字节。
    pub tag: Vec<u8>,
}

// =============================================================================
// Public functions / 公开函数
// =============================================================================

/// Returns the authentication tag length in bytes for the given algorithm.
///
/// 根据算法返回 Auth Tag 的字节长度。
///
/// - AES-256-GCM:        16 bytes
/// - AES-256-CBC + HMAC: 32 bytes (HMAC-SHA256)
/// - ChaCha20-Poly1305:  16 bytes
pub fn tag_length(algorithm: Algorithm) -> usize {
    match algorithm {
        // GCM produces a 128-bit (16-byte) authentication tag.
        // GCM 产生 128 位（16 字节）认证标签。
        Algorithm::Aes256Gcm => 16,

        // CBC + HMAC-SHA256 produces a 256-bit (32-byte) MAC.
        // CBC + HMAC-SHA256 产生 256 位（32 字节）MAC。
        Algorithm::Aes256Cbc => 32,

        // Poly1305 produces a 128-bit (16-byte) authentication tag.
        // Poly1305 产生 128 位（16 字节）认证标签。
        Algorithm::ChaCha20Poly1305 => 16,
    }
}

/// Write a complete UCXE file to the given writer.
///
/// 将完整的 UCXE 文件数据写入给定的 writer。
///
/// # Arguments / 参数
///
/// * `writer` - Any type implementing `Write` (file, buffer, etc.).
///   任何实现了 `Write` 的类型（文件、缓冲区等）。
/// * `file`   - The UCXE file structure to serialize.
///   要序列化的 UCXE 文件结构。
///
/// # Errors / 错误
///
/// Returns `CryptoError::Io` if any write operation fails.
/// 如果任何写入操作失败则返回 `CryptoError::Io`。
pub fn write_ucxe(writer: &mut impl Write, file: &UcxeFile) -> Result<(), CryptoError> {
    // Step 1: Write the 4-byte magic number "UCXE".
    // 第 1 步：写入 4 字节魔数 "UCXE"。
    writer.write_all(&UCXE_MAGIC)?;

    // Step 2: Write header bytes — version, algorithm ID, KDF ID, reserved.
    // 第 2 步：写入头部字节 —— 版本、算法 ID、KDF ID、保留位。
    writer.write_all(&[
        file.header.format_version,
        file.header.algorithm.to_u8(),
        file.header.kdf.to_u8(),
        0x00, // Reserved byte / 保留字节
    ])?;

    // Step 3: Write KDF parameters (if KDF is not None).
    // 第 3 步：写入 KDF 参数（如果 KDF 不为 None）。
    match &file.kdf_params {
        KdfParams::None => {
            // No KDF parameters to write.
            // 无 KDF 参数需要写入。
        }
        KdfParams::Argon2id {
            memory_cost_kib,
            time_cost,
            parallelism,
        } => {
            // Write Argon2id parameters as three u32 little-endian values (12 bytes total).
            // 以三个 u32 小端序值写入 Argon2id 参数（共 12 字节）。
            writer.write_all(&memory_cost_kib.to_le_bytes())?;
            writer.write_all(&time_cost.to_le_bytes())?;
            writer.write_all(&parallelism.to_le_bytes())?;
        }
        KdfParams::Pbkdf2 { iterations } => {
            // Write PBKDF2 iteration count as a single u32 little-endian value (4 bytes).
            // 以单个 u32 小端序值写入 PBKDF2 迭代次数（4 字节）。
            writer.write_all(&iterations.to_le_bytes())?;
        }
    }

    // Step 4: Write salt — length prefix (u16 LE) followed by salt bytes.
    // 第 4 步：写入盐值 —— 长度前缀（u16 LE）加盐值字节。
    writer.write_all(&(file.salt.len() as u16).to_le_bytes())?;
    writer.write_all(&file.salt)?;

    // Step 5: Write IV — length prefix (u16 LE) followed by IV bytes.
    // 第 5 步：写入 IV —— 长度前缀（u16 LE）加 IV 字节。
    writer.write_all(&(file.iv.len() as u16).to_le_bytes())?;
    writer.write_all(&file.iv)?;

    // Step 6: Write ciphertext — length prefix (u64 LE) followed by ciphertext bytes.
    // 第 6 步：写入密文 —— 长度前缀（u64 LE）加密文字节。
    writer.write_all(&(file.ciphertext.len() as u64).to_le_bytes())?;
    writer.write_all(&file.ciphertext)?;

    // Step 7: Write authentication tag (no length prefix — length is implied by algorithm).
    // 第 7 步：写入认证标签（无长度前缀 —— 长度由算法隐含决定）。
    writer.write_all(&file.tag)?;

    Ok(())
}

/// Read and parse a complete UCXE file from the given reader.
///
/// 从给定的 reader 读取并解析完整的 UCXE 文件。
///
/// # Arguments / 参数
///
/// * `reader` - Any type implementing `Read` (file, cursor, etc.).
///   任何实现了 `Read` 的类型（文件、游标等）。
///
/// # Errors / 错误
///
/// - `CryptoError::InvalidFormat` if magic, version, or KDF ID is invalid.
///   如果魔数、版本或 KDF ID 无效则返回 `CryptoError::InvalidFormat`。
/// - `CryptoError::UnsupportedAlgorithm` if the algorithm ID is unknown.
///   如果算法 ID 未知则返回 `CryptoError::UnsupportedAlgorithm`。
/// - `CryptoError::Io` if any read operation fails.
///   如果任何读取操作失败则返回 `CryptoError::Io`。
pub fn read_ucxe(reader: &mut impl Read) -> Result<UcxeFile, CryptoError> {
    // Step 1: Read and verify the 4-byte magic number.
    // 第 1 步：读取并验证 4 字节魔数。
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if magic != UCXE_MAGIC {
        return Err(CryptoError::InvalidFormat(format!(
            "invalid magic number: expected {:?}, got {:?}",
            UCXE_MAGIC, magic
        )));
    }

    // Step 2: Read header bytes — version, algorithm ID, KDF ID, reserved.
    // 第 2 步：读取头部字节 —— 版本、算法 ID、KDF ID、保留位。
    let mut header_bytes = [0u8; 4];
    reader.read_exact(&mut header_bytes)?;
    let version = header_bytes[0];
    let algo_id = header_bytes[1];
    let kdf_id = header_bytes[2];
    // header_bytes[3] is reserved / header_bytes[3] 为保留位

    // Step 3: Validate format version.
    // 第 3 步：验证格式版本。
    if version != UCXE_FORMAT_VERSION {
        return Err(CryptoError::InvalidFormat(format!(
            "unsupported format version: expected {:#04X}, got {:#04X}",
            UCXE_FORMAT_VERSION, version
        )));
    }

    // Step 4: Parse algorithm ID.
    // 第 4 步：解析算法 ID。
    let algorithm = Algorithm::from_u8(algo_id).ok_or_else(|| {
        CryptoError::UnsupportedAlgorithm(format!("unknown algorithm ID: {:#04X}", algo_id))
    })?;

    // Step 5: Parse KDF ID.
    // 第 5 步：解析 KDF ID。
    let kdf = Kdf::from_u8(kdf_id).ok_or_else(|| {
        CryptoError::InvalidFormat(format!("unknown KDF ID: {:#04X}", kdf_id))
    })?;

    // Step 6: Read KDF parameters based on the KDF type.
    // 第 6 步：根据 KDF 类型读取 KDF 参数。
    let kdf_params = match kdf {
        Kdf::None => KdfParams::None,
        Kdf::Argon2id => {
            // Read three u32 LE values: memory_cost_kib, time_cost, parallelism.
            // 读取三个 u32 LE 值：memory_cost_kib、time_cost、parallelism。
            let mut buf = [0u8; 12];
            reader.read_exact(&mut buf)?;
            KdfParams::Argon2id {
                memory_cost_kib: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
                time_cost: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
                parallelism: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            }
        }
        Kdf::Pbkdf2HmacSha256 => {
            // Read one u32 LE value: iterations.
            // 读取一个 u32 LE 值：iterations。
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            KdfParams::Pbkdf2 {
                iterations: u32::from_le_bytes(buf),
            }
        }
    };

    // Step 7: Read salt — u16 LE length prefix followed by salt bytes.
    // 第 7 步：读取盐值 —— u16 LE 长度前缀加盐值字节。
    let mut len_buf = [0u8; 2];
    reader.read_exact(&mut len_buf)?;
    let salt_len = u16::from_le_bytes(len_buf) as usize;
    let mut salt = vec![0u8; salt_len];
    reader.read_exact(&mut salt)?;

    // Step 8: Read IV — u16 LE length prefix followed by IV bytes.
    // 第 8 步：读取 IV —— u16 LE 长度前缀加 IV 字节。
    reader.read_exact(&mut len_buf)?;
    let iv_len = u16::from_le_bytes(len_buf) as usize;
    let mut iv = vec![0u8; iv_len];
    reader.read_exact(&mut iv)?;

    // Step 9: Read ciphertext — u64 LE length prefix followed by ciphertext bytes.
    // 第 9 步：读取密文 —— u64 LE 长度前缀加密文字节。
    let mut ct_len_buf = [0u8; 8];
    reader.read_exact(&mut ct_len_buf)?;
    let ciphertext_len = u64::from_le_bytes(ct_len_buf) as usize;
    let mut ciphertext = vec![0u8; ciphertext_len];
    reader.read_exact(&mut ciphertext)?;

    // Step 10: Read authentication tag — length determined by algorithm (no length prefix).
    // 第 10 步：读取认证标签 —— 长度由算法决定（无长度前缀）。
    let tlen = tag_length(algorithm);
    let mut tag = vec![0u8; tlen];
    reader.read_exact(&mut tag)?;

    Ok(UcxeFile {
        header: UcxeHeader {
            format_version: version,
            algorithm,
            kdf,
        },
        kdf_params,
        salt,
        iv,
        ciphertext,
        tag,
    })
}

/// Convenience function: parse a UCXE file from a byte slice.
///
/// 便捷方法：从字节切片解析 UCXE 文件。
///
/// # Arguments / 参数
///
/// * `data` - The complete UCXE file as a byte slice.
///   完整的 UCXE 文件字节切片。
pub fn parse_ucxe(data: &[u8]) -> Result<UcxeFile, CryptoError> {
    let mut cursor = Cursor::new(data);
    read_ucxe(&mut cursor)
}

/// Convenience function: serialize a UCXE file to a byte vector.
///
/// 便捷方法：将 UCXE 文件序列化为字节向量。
///
/// # Arguments / 参数
///
/// * `file` - The UCXE file structure to serialize.
///   要序列化的 UCXE 文件结构。
pub fn serialize_ucxe(file: &UcxeFile) -> Result<Vec<u8>, CryptoError> {
    let mut buf = Vec::new();
    write_ucxe(&mut buf, file)?;
    Ok(buf)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a sample UcxeFile for testing.
    /// 辅助函数：创建用于测试的示例 UcxeFile。
    fn make_test_file(algorithm: Algorithm, kdf: Kdf, kdf_params: KdfParams) -> UcxeFile {
        let tlen = tag_length(algorithm);
        UcxeFile {
            header: UcxeHeader {
                format_version: UCXE_FORMAT_VERSION,
                algorithm,
                kdf,
            },
            kdf_params,
            salt: vec![0xAA; 16],
            iv: vec![0xBB; 12],
            ciphertext: vec![0xCC; 64],
            tag: vec![0xDD; tlen],
        }
    }

    /// Helper: assert that two UcxeFile instances are equal in all fields.
    /// 辅助函数：断言两个 UcxeFile 实例的所有字段相等。
    fn assert_files_equal(a: &UcxeFile, b: &UcxeFile) {
        assert_eq!(a.header.format_version, b.header.format_version);
        assert_eq!(a.header.algorithm, b.header.algorithm);
        assert_eq!(a.header.kdf, b.header.kdf);
        assert_eq!(a.salt, b.salt);
        assert_eq!(a.iv, b.iv);
        assert_eq!(a.ciphertext, b.ciphertext);
        assert_eq!(a.tag, b.tag);

        // Compare KDF params by matching variants.
        // 通过匹配变体比较 KDF 参数。
        match (&a.kdf_params, &b.kdf_params) {
            (KdfParams::None, KdfParams::None) => {}
            (
                KdfParams::Argon2id {
                    memory_cost_kib: m1,
                    time_cost: t1,
                    parallelism: p1,
                },
                KdfParams::Argon2id {
                    memory_cost_kib: m2,
                    time_cost: t2,
                    parallelism: p2,
                },
            ) => {
                assert_eq!(m1, m2);
                assert_eq!(t1, t2);
                assert_eq!(p1, p2);
            }
            (KdfParams::Pbkdf2 { iterations: i1 }, KdfParams::Pbkdf2 { iterations: i2 }) => {
                assert_eq!(i1, i2);
            }
            _ => panic!("KDF params variant mismatch"),
        }
    }

    // =========================================================================
    // Test 1: AES-256-GCM + No KDF round-trip.
    // 测试 1：AES-256-GCM + 无 KDF 往返。
    // =========================================================================
    #[test]
    fn test_ucxe_round_trip_aes_gcm() {
        let original = make_test_file(Algorithm::Aes256Gcm, Kdf::None, KdfParams::None);
        let bytes = serialize_ucxe(&original).expect("serialize should succeed");
        let parsed = parse_ucxe(&bytes).expect("parse should succeed");
        assert_files_equal(&original, &parsed);
    }

    // =========================================================================
    // Test 2: ChaCha20-Poly1305 + No KDF round-trip.
    // 测试 2：ChaCha20-Poly1305 + 无 KDF 往返。
    // =========================================================================
    #[test]
    fn test_ucxe_round_trip_chacha20() {
        let original =
            make_test_file(Algorithm::ChaCha20Poly1305, Kdf::None, KdfParams::None);
        let bytes = serialize_ucxe(&original).expect("serialize should succeed");
        let parsed = parse_ucxe(&bytes).expect("parse should succeed");
        assert_files_equal(&original, &parsed);
    }

    // =========================================================================
    // Test 3: AES-256-CBC + No KDF round-trip (32-byte tag).
    // 测试 3：AES-256-CBC + 无 KDF 往返（32 字节 tag）。
    // =========================================================================
    #[test]
    fn test_ucxe_round_trip_aes_cbc() {
        let original = make_test_file(Algorithm::Aes256Cbc, Kdf::None, KdfParams::None);

        // Verify CBC tag is 32 bytes.
        // 验证 CBC tag 为 32 字节。
        assert_eq!(original.tag.len(), 32);

        let bytes = serialize_ucxe(&original).expect("serialize should succeed");
        let parsed = parse_ucxe(&bytes).expect("parse should succeed");
        assert_files_equal(&original, &parsed);
    }

    // =========================================================================
    // Test 4: AES-256-GCM + Argon2id KDF round-trip.
    // 测试 4：AES-256-GCM + Argon2id KDF 往返。
    // =========================================================================
    #[test]
    fn test_ucxe_round_trip_argon2id() {
        let original = make_test_file(
            Algorithm::Aes256Gcm,
            Kdf::Argon2id,
            KdfParams::Argon2id {
                memory_cost_kib: 65536,
                time_cost: 3,
                parallelism: 4,
            },
        );
        let bytes = serialize_ucxe(&original).expect("serialize should succeed");
        let parsed = parse_ucxe(&bytes).expect("parse should succeed");
        assert_files_equal(&original, &parsed);
    }

    // =========================================================================
    // Test 5: AES-256-GCM + PBKDF2 KDF round-trip.
    // 测试 5：AES-256-GCM + PBKDF2 KDF 往返。
    // =========================================================================
    #[test]
    fn test_ucxe_round_trip_pbkdf2() {
        let original = make_test_file(
            Algorithm::Aes256Gcm,
            Kdf::Pbkdf2HmacSha256,
            KdfParams::Pbkdf2 { iterations: 600_000 },
        );
        let bytes = serialize_ucxe(&original).expect("serialize should succeed");
        let parsed = parse_ucxe(&bytes).expect("parse should succeed");
        assert_files_equal(&original, &parsed);
    }

    // =========================================================================
    // Test 6: Invalid magic number should return InvalidFormat.
    // 测试 6：无效魔数应返回 InvalidFormat。
    // =========================================================================
    #[test]
    fn test_invalid_magic() {
        // Construct bytes with wrong magic "BADX" instead of "UCXE".
        // 构造魔数为 "BADX" 而非 "UCXE" 的字节。
        let mut data = vec![b'B', b'A', b'D', b'X'];
        data.extend_from_slice(&[0x01, 0x01, 0x00, 0x00]); // version, algo, kdf, reserved
        data.extend_from_slice(&[0x00, 0x00]); // salt_len = 0
        data.extend_from_slice(&[0x00, 0x00]); // iv_len = 0
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // ct_len = 0
        data.extend_from_slice(&[0x00; 16]); // tag (GCM = 16 bytes)

        let result = parse_ucxe(&data);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("invalid magic"),
            "error should mention 'invalid magic', got: {err_msg}"
        );
    }

    // =========================================================================
    // Test 7: Unsupported version (0x02) should return InvalidFormat.
    // 测试 7：不支持的版本（0x02）应返回 InvalidFormat。
    // =========================================================================
    #[test]
    fn test_unsupported_version() {
        // Construct bytes with correct magic but version = 0x02.
        // 构造具有正确魔数但版本 = 0x02 的字节。
        let mut data = Vec::new();
        data.extend_from_slice(&UCXE_MAGIC);
        data.extend_from_slice(&[0x02, 0x01, 0x00, 0x00]); // version=2, algo=GCM, kdf=None, reserved
        data.extend_from_slice(&[0x00, 0x00]); // salt_len = 0
        data.extend_from_slice(&[0x00, 0x00]); // iv_len = 0
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // ct_len = 0
        data.extend_from_slice(&[0x00; 16]); // tag (GCM = 16 bytes)

        let result = parse_ucxe(&data);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("version"),
            "error should mention 'version', got: {err_msg}"
        );
    }

    // =========================================================================
    // Test 8: Verify tag_length values for all three algorithms.
    // 测试 8：验证三种算法的 tag_length 值。
    // =========================================================================
    #[test]
    fn test_tag_length_values() {
        assert_eq!(tag_length(Algorithm::Aes256Gcm), 16);
        assert_eq!(tag_length(Algorithm::Aes256Cbc), 32);
        assert_eq!(tag_length(Algorithm::ChaCha20Poly1305), 16);
    }

    // =========================================================================
    // Test 9: Empty ciphertext should round-trip correctly.
    // 测试 9：空密文应能正确往返。
    // =========================================================================
    #[test]
    fn test_empty_ciphertext() {
        let original = UcxeFile {
            header: UcxeHeader {
                format_version: UCXE_FORMAT_VERSION,
                algorithm: Algorithm::Aes256Gcm,
                kdf: Kdf::None,
            },
            kdf_params: KdfParams::None,
            salt: vec![],
            iv: vec![0xBB; 12],
            ciphertext: vec![], // Empty ciphertext / 空密文
            tag: vec![0xDD; 16],
        };

        let bytes = serialize_ucxe(&original).expect("serialize should succeed");
        let parsed = parse_ucxe(&bytes).expect("parse should succeed");
        assert_files_equal(&original, &parsed);
        assert!(parsed.ciphertext.is_empty());
    }

    // =========================================================================
    // Test 10: Large ciphertext (1 MiB) should round-trip correctly.
    // 测试 10：大密文（1 MiB）应能正确往返。
    // =========================================================================
    #[test]
    fn test_large_ciphertext() {
        let large_ct = vec![0x42u8; 1024 * 1024]; // 1 MiB
        let original = UcxeFile {
            header: UcxeHeader {
                format_version: UCXE_FORMAT_VERSION,
                algorithm: Algorithm::ChaCha20Poly1305,
                kdf: Kdf::None,
            },
            kdf_params: KdfParams::None,
            salt: vec![0xAA; 32],
            iv: vec![0xBB; 12],
            ciphertext: large_ct,
            tag: vec![0xDD; 16],
        };

        let bytes = serialize_ucxe(&original).expect("serialize should succeed");
        let parsed = parse_ucxe(&bytes).expect("parse should succeed");
        assert_files_equal(&original, &parsed);

        // Verify the ciphertext length is exactly 1 MiB.
        // 验证密文长度恰好为 1 MiB。
        assert_eq!(parsed.ciphertext.len(), 1024 * 1024);
    }
}
