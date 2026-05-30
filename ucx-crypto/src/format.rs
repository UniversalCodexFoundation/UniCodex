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

use crate::{Algorithm, CryptoError, Kdf, UCXE_FORMAT_VERSION, UCXE_MAGIC};
use std::io::{Cursor, Read, Write};

// =============================================================================
// Length-field upper bounds / 长度字段上限常量
// =============================================================================
//
// These caps harden `read_ucxe` against maliciously-crafted or corrupted input
// where the declared length far exceeds the remaining readable bytes. Without
// caps, a `Vec::with_capacity(huge_value)` call can trigger a panic/OOM before
// `read_exact` fails. Each cap reflects the realistic maximum for that field.
//
// 这些上限用于防范恶意构造或损坏的输入 —— 声明长度远超剩余可读字节时，
// 未加校验的 `Vec::with_capacity(huge_value)` 会在 `read_exact` 失败前触发
// panic/OOM。每个上限都反映该字段在正常场景下的合理最大值。

/// Maximum allowed salt length (bytes). UCX 规范规定 salt 固定 16 B，
/// 上限放宽至 1 KiB 以容错但仍能阻断攻击者构造的巨大值。
pub(crate) const MAX_SALT_LEN: usize = 1024;

/// Maximum allowed IV/nonce length (bytes). 常见 AEAD nonce 12 B，
/// AES-CBC IV 16 B，上限 256 B 足以容错。
pub(crate) const MAX_IV_LEN: usize = 256;

/// Absolute hard cap for ciphertext length: 16 GiB.
/// UCXE 文件理论上不会大于 16 GiB；任何超过该值的声明都视为恶意或损坏。
/// 绝对硬上限，密文长度 16 GiB。
pub(crate) const MAX_CIPHERTEXT_LEN: u64 = 16 * 1024 * 1024 * 1024;

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

    /// Whether the ciphertext uses chunked encryption.
    /// Stored in the flags byte (offset 7): bit 0 = chunked.
    ///
    /// 密文是否使用了分块加密。
    /// 存储在 flags 字节（offset 7）中：bit 0 = 分块模式。
    pub chunked: bool,

    /// Raw flags byte read from the on-disk UCXE header (offset 7).
    ///
    /// When constructing a *new* header for encryption, this should be set
    /// to `0x00` (or `0x01` for chunked) so that reserved bits are zero.
    /// When *reading* an existing file, this preserves the exact byte from
    /// disk — including any reserved bits — so that `to_bytes()` feeds the
    /// authentic value into the AEAD AAD. If an attacker flips a reserved
    /// bit, the AAD will differ from what was bound at encryption time and
    /// tag verification will fail.
    ///
    /// 从磁盘 UCXE 头部（offset 7）读取的原始 flags 字节。
    /// 新建头部加密时应设为 `0x00`（分块时 `0x01`），确保保留位为零。
    /// 读取已有文件时保留磁盘上的精确字节（包括保留位），使 `to_bytes()`
    /// 向 AEAD AAD 提供真实值。攻击者翻转保留位后，AAD 与加密时绑定的值
    /// 不同，tag 校验将失败。
    pub raw_flags: u8,
}

impl UcxeHeader {
    /// Serialize the header as the exact 8-byte prefix used as AEAD AAD
    /// (Additional Authenticated Data): `magic[4] || version || algo || kdf || flags`.
    ///
    /// Binding this byte sequence into the AEAD tag prevents an attacker from
    /// swapping the algorithm/KDF/flags bytes without being detected — the
    /// decrypt side recomputes the same AAD and tag verification will fail
    /// if any header byte was tampered.
    ///
    /// 将头部序列化为作为 AEAD AAD（附加认证数据）的精确 8 字节前缀：
    /// `magic[4] || version || algo || kdf || flags`。
    /// 将该字节串绑入 AEAD 标签后，攻击者无法在不破坏认证的前提下篡改
    /// algorithm/KDF/flags 字节 —— 解密端会重算相同 AAD，任一字节被改
    /// 都会触发 tag 验证失败。
    pub fn to_bytes(&self) -> Vec<u8> {
        // Use the raw flags byte to preserve all bits (including reserved ones)
        // so that any tampering of the flags byte on disk will cause AAD mismatch.
        // 使用原始 flags 字节以保留所有位（含保留位），确保磁盘上 flags 字节的
        // 任何篡改都会导致 AAD 不匹配。
        let mut out = Vec::with_capacity(8);
        out.extend_from_slice(&UCXE_MAGIC);
        out.push(self.format_version);
        out.push(self.algorithm.to_u8());
        out.push(self.kdf.to_u8());
        out.push(self.raw_flags);
        out
    }
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

impl KdfParams {
    /// Serialize the KDF parameters as the exact byte sequence used as part
    /// of the AEAD AAD. The encoding mirrors `write_ucxe`'s Step 3:
    ///
    /// - `None`:     empty (0 bytes)
    /// - `Argon2id`: `memory_cost_kib || time_cost || parallelism` (3 × u32 LE = 12 B)
    /// - `Pbkdf2`:   `iterations` (u32 LE = 4 B)
    ///
    /// Keeping this in lock-step with the on-disk format ensures the AAD
    /// recomputed at decrypt time is byte-for-byte identical to what was
    /// bound into the tag at encrypt time.
    ///
    /// 将 KDF 参数序列化为 AEAD AAD 的组成字节串。编码方式与 `write_ucxe`
    /// 第 3 步一致：
    /// - `None`：空（0 字节）
    /// - `Argon2id`：`memory_cost_kib || time_cost || parallelism`（3 × u32 LE = 12 B）
    /// - `Pbkdf2`：`iterations`（u32 LE = 4 B）
    ///
    /// 保持与磁盘格式一致可以确保解密端重算出的 AAD 与加密端绑入标签的
    /// AAD 逐字节一致。
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            KdfParams::None => Vec::new(),
            KdfParams::Argon2id {
                memory_cost_kib,
                time_cost,
                parallelism,
            } => {
                let mut out = Vec::with_capacity(12);
                out.extend_from_slice(&memory_cost_kib.to_le_bytes());
                out.extend_from_slice(&time_cost.to_le_bytes());
                out.extend_from_slice(&parallelism.to_le_bytes());
                out
            }
            KdfParams::Pbkdf2 { iterations } => iterations.to_le_bytes().to_vec(),
        }
    }
}

/// Build the AEAD Additional Authenticated Data (AAD) for a UCXE file.
///
/// The AAD is the concatenation of:
///
/// 1. `header.to_bytes()` — 8 B: magic(4) || format_version || algo || kdf || flags
/// 2. `kdf_params.to_bytes()` — 0 / 4 / 12 B depending on KDF
/// 3. `salt` — variable-length salt bytes
///
/// Binding these fields into the AEAD tag prevents an attacker from altering
/// any of them (algorithm, KDF choice / parameters, chunked flag, salt) without
/// invalidating the tag. Any single-bit flip in the header, KDF parameters, or
/// salt will cause `decrypt` to return `AuthenticationFailed`.
///
/// Both `encrypt_to_ucxe` / `decrypt_ucxe_payload` (and their CBC / chunked
/// siblings once applicable) must call this function with the same inputs so
/// that the encrypt-time and decrypt-time AADs are identical.
///
/// 为 UCXE 文件构建 AEAD 的附加认证数据（AAD）。
/// AAD 由以下部分拼接而成：
/// 1. `header.to_bytes()` — 8 B：magic(4) || format_version || algo || kdf || flags
/// 2. `kdf_params.to_bytes()` — 0 / 4 / 12 B，取决于 KDF
/// 3. `salt` — 变长盐值字节
///
/// 将这些字段绑入 AEAD 标签后，攻击者无法在不破坏认证的情况下篡改其中任何
/// 一个字段（算法、KDF 选择/参数、分块标志、盐值）。加密端与解密端必须以
/// 相同输入调用本函数，确保两端的 AAD 逐字节一致。
pub fn build_aead_aad(header: &UcxeHeader, kdf_params: &KdfParams, salt: &[u8]) -> Vec<u8> {
    let header_bytes = header.to_bytes();
    let kdf_bytes = kdf_params.to_bytes();

    let mut aad = Vec::with_capacity(header_bytes.len() + kdf_bytes.len() + salt.len());
    aad.extend_from_slice(&header_bytes);
    aad.extend_from_slice(&kdf_bytes);
    aad.extend_from_slice(salt);
    aad
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

    // Step 2: Write header bytes — version, algorithm ID, KDF ID, flags.
    // 第 2 步：写入头部字节 —— 版本、算法 ID、KDF ID、flags。
    //
    // Write the FULL `raw_flags` byte (the single source of truth), NOT a value
    // recomputed from `chunked`. `parse_ucxe` preserves the on-disk flags byte
    // (incl. reserved bits 1-7) in `raw_flags`, and `UcxeHeader::to_bytes()` uses
    // `raw_flags` to build the AAD (UCX-FORMAT §7.2/§7.6: the raw flags byte is
    // bound into the AAD). If write_ucxe instead emitted `if chunked {0x01} else
    // {0x00}` it would (a) drop reserved bits on a parse→write round-trip — making
    // serialize∘parse non-byte-identical and silently clearing reserved bits — and
    // (b) create two divergent sources for the same logical byte (disk vs AAD), a
    // latent inconsistency. `raw_flags` is kept consistent with `chunked` at
    // construction time (encrypt sets bit0 = chunked).
    // 写入**完整的** `raw_flags` 字节（唯一真值），而非从 `chunked` 重算的值。
    // `parse_ucxe` 把磁盘上的 flags 字节（含保留位 1-7）原样存入 `raw_flags`，且
    // `UcxeHeader::to_bytes()` 用 `raw_flags` 构造 AAD（UCX-FORMAT §7.2/§7.6：
    // 原始 flags 字节绑入 AAD）。若 write_ucxe 改用 `if chunked {0x01} else {0x00}`，
    // 会 (a) 在 parse→write 往返中丢弃保留位——使 serialize∘parse 非字节恒等并静默
    // 清除保留位；(b) 为同一逻辑字节制造两个分歧来源（磁盘 vs AAD），是潜在不一致。
    // `raw_flags` 在构造时与 `chunked` 保持一致（加密时 bit0 = chunked）。
    writer.write_all(&[
        file.header.format_version,
        file.header.algorithm.to_u8(),
        file.header.kdf.to_u8(),
        file.header.raw_flags,
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
    // Buffer the entire stream once so we can enforce per-field upper bounds
    // against the true total size before any large Vec allocation.
    //
    // 将整个流一次性缓冲，便于在任何大 Vec 分配之前
    // 用真实总字节数作为长度字段的真实上限。
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;
    parse_ucxe(&data)
}

/// Bounded UCXE parse implementation (used by both `read_ucxe` and `parse_ucxe`).
///
/// Enforces a hard cap on every length-prefixed field to prevent panic / OOM
/// caused by `Vec::with_capacity(huge)` when a malicious or truncated input
/// declares an absurd length. All caps are constants defined at the top of
/// this module (`MAX_SALT_LEN`, `MAX_IV_LEN`, `MAX_CIPHERTEXT_LEN`).
///
/// 边界安全版本的 UCXE 解析实现（供 `read_ucxe` 和 `parse_ucxe` 复用）。
/// 对每个带长度前缀的字段强制设置硬上限，防止恶意或截断输入通过声明
/// 巨大长度值触发 `Vec::with_capacity(huge)` panic/OOM。
fn parse_ucxe_bounded(data: &[u8]) -> Result<UcxeFile, CryptoError> {
    let total_len = data.len();
    let mut cursor = Cursor::new(data);

    // ------------------------------------------------------------------
    // Small helper closures. Returning closures as nested fns keeps the
    // boundary checks explicit and easy to audit field-by-field.
    // 内部小型闭包：保留逐字段的边界校验，便于审计。
    // ------------------------------------------------------------------

    /// Read exactly `n` bytes or return InvalidFormat on truncation.
    /// 读取恰好 `n` 字节；若截断则返回 InvalidFormat。
    fn read_fixed<const N: usize>(
        cur: &mut Cursor<&[u8]>,
        field: &'static str,
    ) -> Result<[u8; N], CryptoError> {
        let mut buf = [0u8; N];
        cur.read_exact(&mut buf)
            .map_err(|_| CryptoError::InvalidFormat(format!("truncated {field} / {field} 截断")))?;
        Ok(buf)
    }

    /// Read `n` bytes as Vec with length-cap check against `remaining`.
    /// 按声明长度读取字节，若长度超过剩余可读或硬上限则拒绝。
    fn read_vec(
        cur: &mut Cursor<&[u8]>,
        declared: usize,
        hard_cap: usize,
        remaining: usize,
        field: &'static str,
    ) -> Result<Vec<u8>, CryptoError> {
        // Reject values larger than the realistic upper bound for this field,
        // or larger than the bytes actually available in the input.
        // 声明值若超过该字段的合理上限或剩余可读字节，则拒绝。
        if declared > hard_cap {
            return Err(CryptoError::InvalidFormat(format!(
                "{field} too large: {declared} > {hard_cap} / {field} 长度过大"
            )));
        }
        if declared > remaining {
            return Err(CryptoError::InvalidFormat(format!(
                "{field} length {declared} exceeds remaining {remaining} bytes / \
                 {field} 长度超过剩余字节"
            )));
        }
        let mut buf = vec![0u8; declared];
        cur.read_exact(&mut buf).map_err(|_| {
            CryptoError::InvalidFormat(format!("truncated {field} body / {field} 主体截断"))
        })?;
        Ok(buf)
    }

    // Step 1: Magic number (4 B).
    // 第 1 步：4 字节魔数。
    let magic = read_fixed::<4>(&mut cursor, "magic")?;
    if magic != UCXE_MAGIC {
        return Err(CryptoError::InvalidFormat(format!(
            "invalid magic number: expected {:?}, got {:?}",
            UCXE_MAGIC, magic
        )));
    }

    // Step 2: Header (version, algo, kdf, flags) = 4 B.
    // 第 2 步：头部 4 字节（版本、算法、KDF、标志位）。
    let header_bytes = read_fixed::<4>(&mut cursor, "header")?;
    let version = header_bytes[0];
    let algo_id = header_bytes[1];
    let kdf_id = header_bytes[2];
    let chunked = (header_bytes[3] & 0x01) != 0;

    if version != UCXE_FORMAT_VERSION {
        return Err(CryptoError::InvalidFormat(format!(
            "unsupported format version: expected {:#04X}, got {:#04X}",
            UCXE_FORMAT_VERSION, version
        )));
    }
    let algorithm = Algorithm::from_u8(algo_id).ok_or_else(|| {
        CryptoError::UnsupportedAlgorithm(format!("unknown algorithm ID: {:#04X}", algo_id))
    })?;
    let kdf = Kdf::from_u8(kdf_id)
        .ok_or_else(|| CryptoError::InvalidFormat(format!("unknown KDF ID: {:#04X}", kdf_id)))?;

    // Step 3: KDF parameters (variable size by KDF type).
    // 第 3 步：KDF 参数（按 KDF 类型长度不同）。
    let kdf_params = match kdf {
        Kdf::None => KdfParams::None,
        Kdf::Argon2id => {
            let buf = read_fixed::<12>(&mut cursor, "Argon2id params")?;
            KdfParams::Argon2id {
                memory_cost_kib: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
                time_cost: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
                parallelism: u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            }
        }
        Kdf::Pbkdf2HmacSha256 => {
            let buf = read_fixed::<4>(&mut cursor, "PBKDF2 params")?;
            KdfParams::Pbkdf2 {
                iterations: u32::from_le_bytes(buf),
            }
        }
    };

    // Step 4: Validate KDF parameters are sane (neither too weak nor absurd).
    // 第 4 步：校验 KDF 参数处于合理区间（既不过弱也不过大）。
    crate::kdf::validate_kdf_params(kdf, &kdf_params)?;

    // Step 5: Salt (u16-prefixed, capped).
    // 第 5 步：盐值（u16 长度前缀，受上限约束）。
    let salt_len_buf = read_fixed::<2>(&mut cursor, "salt length")?;
    let salt_len = u16::from_le_bytes(salt_len_buf) as usize;
    let pos = cursor.position() as usize;
    let salt = read_vec(
        &mut cursor,
        salt_len,
        MAX_SALT_LEN,
        total_len.saturating_sub(pos),
        "salt",
    )?;

    // Step 6: IV (u16-prefixed, capped).
    // 第 6 步：IV（u16 长度前缀，受上限约束）。
    let iv_len_buf = read_fixed::<2>(&mut cursor, "IV length")?;
    let iv_len = u16::from_le_bytes(iv_len_buf) as usize;
    let pos = cursor.position() as usize;
    let iv = read_vec(
        &mut cursor,
        iv_len,
        MAX_IV_LEN,
        total_len.saturating_sub(pos),
        "IV",
    )?;

    // Step 7: Ciphertext (u64-prefixed, capped at 16 GiB or remaining bytes).
    // 第 7 步：密文（u64 长度前缀，上限 16 GiB 或剩余字节）。
    let ct_len_buf = read_fixed::<8>(&mut cursor, "ciphertext length")?;
    let ciphertext_len_u64 = u64::from_le_bytes(ct_len_buf);
    if ciphertext_len_u64 > MAX_CIPHERTEXT_LEN {
        return Err(CryptoError::InvalidFormat(format!(
            "ciphertext length {ciphertext_len_u64} exceeds hard cap {MAX_CIPHERTEXT_LEN} / \
             密文长度超过硬上限"
        )));
    }
    // Guard against u64 → usize narrowing on 32-bit platforms.
    // 防止在 32 位平台上 u64 → usize 溢出。
    let ciphertext_len: usize = ciphertext_len_u64.try_into().map_err(|_| {
        CryptoError::InvalidFormat(
            "ciphertext length exceeds platform usize / 密文长度超 usize".into(),
        )
    })?;
    let pos = cursor.position() as usize;
    let remaining_after_ct_hdr = total_len.saturating_sub(pos);
    // Remaining bytes must fit: ciphertext + fixed tag.
    // 剩余字节需能容纳密文 + 固定长度 tag。
    let tlen = tag_length(algorithm);
    if ciphertext_len
        .checked_add(tlen)
        .is_none_or(|need| need > remaining_after_ct_hdr)
    {
        return Err(CryptoError::InvalidFormat(format!(
            "ciphertext length {ciphertext_len} + tag {tlen} exceeds remaining {remaining_after_ct_hdr} bytes / \
             密文+tag 超过剩余字节"
        )));
    }
    let ciphertext = read_vec(
        &mut cursor,
        ciphertext_len,
        // Second cap: ciphertext alone cannot exceed the usize half of MAX.
        // Cast is safe because we already bounded via MAX_CIPHERTEXT_LEN above.
        // 第二重上限：usize 版本的 16 GiB。
        MAX_CIPHERTEXT_LEN.min(usize::MAX as u64) as usize,
        remaining_after_ct_hdr,
        "ciphertext",
    )?;

    // Step 8: Auth tag (fixed length determined by algorithm).
    // 第 8 步：认证标签（长度由算法决定，无长度前缀）。
    let pos = cursor.position() as usize;
    let tag = read_vec(
        &mut cursor,
        tlen,
        tlen,
        total_len.saturating_sub(pos),
        "auth tag",
    )?;

    Ok(UcxeFile {
        header: UcxeHeader {
            format_version: version,
            algorithm,
            kdf,
            chunked,
            // Preserve the original flags byte from disk so that to_bytes()
            // includes it in the AEAD AAD. Any tampering of reserved bits
            // will cause decryption to fail.
            // 保留磁盘上的原始 flags 字节，使 to_bytes() 将其纳入 AEAD AAD。
            // 攻击者篡改保留位后解密将失败。
            raw_flags: header_bytes[3],
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
    parse_ucxe_bounded(data)
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
                chunked: false,
                raw_flags: 0x00,
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
        assert_eq!(a.header.chunked, b.header.chunked);
        assert_eq!(a.header.raw_flags, b.header.raw_flags);
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
        let original = make_test_file(Algorithm::ChaCha20Poly1305, Kdf::None, KdfParams::None);
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
            KdfParams::Pbkdf2 {
                iterations: 600_000,
            },
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
                chunked: false,
                raw_flags: 0x00,
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
                chunked: false,
                raw_flags: 0x00,
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

    // =========================================================================
    // Test 11: Tampered KDF ID (Argon2id -> PBKDF2) should return Err, not panic.
    // 测试 11：篡改 KDF ID（Argon2id -> PBKDF2）应返回 Err 而非 panic。
    //
    // Attack model / 攻击模型：
    //   攻击者在合法 Argon2id-加密文件上把 KDF ID 字节改为 PBKDF2，使解析器
    //   按 4 字节读取 KDF 参数而非 12 字节，后续 salt_len / iv_len / ct_len
    //   被错位读到攻击者可控的巨大值，未加边界校验的版本会 panic。
    // =========================================================================
    #[test]
    fn test_tampered_kdf_id_no_panic() {
        // Build a valid Argon2id-encrypted UCXE (with OWASP-min params).
        // 构造一个合法 Argon2id UCXE（OWASP 最低参数）。
        let original = make_test_file(
            Algorithm::Aes256Gcm,
            Kdf::Argon2id,
            KdfParams::Argon2id {
                memory_cost_kib: crate::kdf::ARGON2ID_MIN_MEMORY_KIB,
                time_cost: crate::kdf::ARGON2ID_MIN_TIME_COST,
                parallelism: crate::kdf::ARGON2ID_MIN_PARALLELISM,
            },
        );
        let mut bytes = serialize_ucxe(&original).expect("serialize");
        // Flip KDF id byte (offset 6: magic[4] + version[1] + algo[1] + kdf).
        // 翻转第 6 字节 KDF ID，将 Argon2id(0x01) 改为 PBKDF2(0x02)。
        bytes[6] = 0x02;

        // Must return Err; must NOT panic from Vec::with_capacity.
        // 必须返回 Err；不得因 Vec::with_capacity 巨大值触发 panic。
        let result = parse_ucxe(&bytes);
        assert!(result.is_err(), "tampered KDF id must be rejected");
    }

    // =========================================================================
    // Test 12: ct_len = u64::MAX must return Err, not panic with OOM.
    // 测试 12：ct_len = u64::MAX 必须返回 Err，而非因 OOM panic。
    // =========================================================================
    #[test]
    fn test_ciphertext_length_u64_max_no_panic() {
        // Handcraft minimal UCXE bytes with ct_len = u64::MAX.
        // 手工构造 UCXE：ct_len = u64::MAX。
        let mut data = Vec::new();
        data.extend_from_slice(&UCXE_MAGIC);
        data.extend_from_slice(&[0x01, 0x01, 0x00, 0x00]); // version=1, algo=GCM, kdf=None
        data.extend_from_slice(&[0x00, 0x00]); // salt_len = 0
        data.extend_from_slice(&[0x00, 0x00]); // iv_len = 0
        data.extend_from_slice(&u64::MAX.to_le_bytes()); // ct_len = u64::MAX
        data.extend_from_slice(&[0x00; 16]); // fake tag (won't be reached)

        let result = parse_ucxe(&data);
        assert!(
            result.is_err(),
            "ciphertext length = u64::MAX must be rejected"
        );
    }

    // =========================================================================
    // Test 13: salt_len larger than remaining bytes must return Err.
    // 测试 13：salt_len 大于剩余字节时必须返回 Err。
    // =========================================================================
    #[test]
    fn test_salt_len_exceeds_remaining() {
        let mut data = Vec::new();
        data.extend_from_slice(&UCXE_MAGIC);
        data.extend_from_slice(&[0x01, 0x01, 0x00, 0x00]);
        // salt_len = 0xFFFF (65535) but only a few trailing bytes exist.
        // salt_len = 0xFFFF，但后续只剩少量字节。
        data.extend_from_slice(&0xFFFFu16.to_le_bytes());
        data.extend_from_slice(&[0u8; 8]); // short tail

        let result = parse_ucxe(&data);
        assert!(result.is_err(), "oversized salt_len must be rejected");
    }

    // =========================================================================
    // Test 14: iv_len beyond MAX_IV_LEN must return Err.
    // 测试 14：iv_len 超过 MAX_IV_LEN 时必须返回 Err。
    // =========================================================================
    #[test]
    fn test_iv_len_too_large() {
        let mut data = Vec::new();
        data.extend_from_slice(&UCXE_MAGIC);
        data.extend_from_slice(&[0x01, 0x01, 0x00, 0x00]);
        data.extend_from_slice(&[0x00, 0x00]); // salt_len = 0
        // iv_len = 1024 exceeds MAX_IV_LEN (256).
        // iv_len = 1024 超过 MAX_IV_LEN。
        data.extend_from_slice(&1024u16.to_le_bytes());
        data.extend_from_slice(&[0u8; 32]); // some trailing bytes

        let result = parse_ucxe(&data);
        assert!(result.is_err(), "iv_len above MAX_IV_LEN must be rejected");
    }

    // =========================================================================
    // Test 15: salt_len above MAX_SALT_LEN must return Err.
    // 测试 15：salt_len 超过 MAX_SALT_LEN 时必须返回 Err。
    // =========================================================================
    #[test]
    fn test_salt_len_above_cap() {
        let mut data = Vec::new();
        data.extend_from_slice(&UCXE_MAGIC);
        data.extend_from_slice(&[0x01, 0x01, 0x00, 0x00]);
        // salt_len = 2048 (> MAX_SALT_LEN = 1024) but buffer is oversized so
        // the check is strictly against the hard cap, not remaining bytes.
        // salt_len = 2048（超过 MAX_SALT_LEN = 1024）。
        data.extend_from_slice(&2048u16.to_le_bytes());
        data.extend_from_slice(&vec![0u8; 4096]); // enough tail

        let result = parse_ucxe(&data);
        assert!(
            result.is_err(),
            "salt_len above MAX_SALT_LEN must be rejected"
        );
    }
}
