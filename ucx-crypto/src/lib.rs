//! UCX Encryption & Decryption Module.
//!
//! This module provides chapter-level, resource-level, and paragraph-level
//! encryption for UCX files. It supports:
//! - AES-256-GCM (recommended)
//! - ChaCha20-Poly1305
//! - AES-256-CBC with HMAC-SHA256 (Encrypt-then-MAC)
//!
//! It also handles the UCXE binary format, key derivation (Argon2id / PBKDF2),
//! and chunked encryption for large files (>64 MiB).
//!
//! UCX 加密与解密模块。
//! 提供章节级、资源级和段落级加密。支持：
//! - AES-256-GCM（推荐）
//! - ChaCha20-Poly1305
//! - AES-256-CBC + HMAC-SHA256（Encrypt-then-MAC）
//!
//! 同时处理 UCXE 二进制格式、密钥派生（Argon2id / PBKDF2）
//! 和大文件分块加密（>64 MiB）。

use std::path::Path;
use thiserror::Error;
use zeroize::Zeroizing;

// =============================================================================
// Sub-modules / 子模块
// =============================================================================

/// UCXE binary format read/write.
/// UCXE 二进制格式的序列化与反序列化。
pub mod format;

/// AES-256-GCM encryption engine.
/// AES-256-GCM 加密引擎。
pub mod aes_gcm;

/// ChaCha20-Poly1305 encryption engine.
/// ChaCha20-Poly1305 加密引擎。
pub mod chacha20;

/// AES-256-CBC with HMAC-SHA256 (Encrypt-then-MAC) engine.
/// AES-256-CBC + HMAC-SHA256（Encrypt-then-MAC）加密引擎。
pub mod aes_cbc;

/// Key derivation functions (Argon2id / PBKDF2).
/// 密钥派生函数（Argon2id / PBKDF2）。
pub mod kdf;

/// Chunked encryption for large files (>64 MiB).
/// 大文件分块加密（>64 MiB）。
pub mod chunked;

/// Paragraph-level encryption for `<encrypted>` tags.
/// 段落级加密，用于 `<encrypted>` 标签。
pub mod paragraph;

// =============================================================================
// Constants / 常量
// =============================================================================

/// Magic number for the UCXE encrypted file format.
/// UCXE 加密文件格式的魔数。
///
/// ASCII: "UCXE", Hex: 55 43 58 45
pub const UCXE_MAGIC: [u8; 4] = [0x55, 0x43, 0x58, 0x45];

/// Current UCXE format version.
/// 当前 UCXE 格式版本。
pub const UCXE_FORMAT_VERSION: u8 = 0x01;

// =============================================================================
// Error types / 错误类型
// =============================================================================

/// Errors that can occur during encryption/decryption operations.
///
/// 加密/解密操作过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Unsupported encryption algorithm.
    /// 不支持的加密算法。
    #[error("unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),

    /// Invalid UCXE file format.
    /// 无效的 UCXE 文件格式。
    #[error("invalid UCXE format: {0}")]
    InvalidFormat(String),

    /// Authentication tag verification failed (data may be tampered).
    /// 认证标签验证失败（数据可能被篡改）。
    #[error("authentication failed")]
    AuthenticationFailed,

    /// Key derivation error.
    /// 密钥派生错误。
    #[error("key derivation error: {0}")]
    KeyDerivation(String),

    /// I/O error during crypto operations.
    /// 加密操作过程中的 I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// KDF parameters fall outside the allowed security / resource bounds.
    /// 内部包含诊断字符串，供日志或诊断使用 —— 公开 API 在解密路径中
    /// 会通过 `into_public_decrypt_error` 统一映射为 `DecryptionFailed`。
    ///
    /// KDF 参数超出安全或资源上下限。
    #[error("weak or out-of-range KDF parameters: {0}")]
    WeakKdfParameters(String),

    /// Generic, opaque decryption failure. Returned by the public `decrypt*`
    /// entry points so attackers cannot distinguish between wrong-key,
    /// tampered-ciphertext, or malformed-format errors (防 Oracle 攻击)。
    ///
    /// 通用、不透明的解密失败。由公开 `decrypt*` 入口统一返回，
    /// 攻击者无法区分错误密钥、篡改密文或格式错误（防止信息泄漏 oracle）。
    #[error("decryption failed")]
    DecryptionFailed,
}

impl CryptoError {
    /// Map any error into the opaque `DecryptionFailed` variant used by the
    /// public decrypt API. I/O errors are preserved because they indicate
    /// environmental problems rather than ciphertext tampering.
    ///
    /// 将任意错误统一映射为公开 decrypt API 使用的不透明 `DecryptionFailed`。
    /// I/O 错误保持原样 —— 它反映的是环境问题而非密文被篡改。
    #[allow(dead_code)] // used by public decrypt API after CRYPTO-6 lands
    pub(crate) fn into_public_decrypt_error(self) -> CryptoError {
        match self {
            CryptoError::Io(_) => self,
            _ => CryptoError::DecryptionFailed,
        }
    }
}

// =============================================================================
// Types / 类型定义
// =============================================================================

/// Supported encryption algorithms.
///
/// 支持的加密算法。
///
/// Each variant corresponds to an Algorithm ID byte in the UCXE header:
/// - `0x01` = AES-256-GCM
/// - `0x02` = AES-256-CBC (Encrypt-then-MAC)
/// - `0x03` = ChaCha20-Poly1305
///
/// 每个变体对应 UCXE 头部中的算法 ID 字节。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// AES-256-GCM (recommended).
    /// AES-256-GCM（推荐）。
    Aes256Gcm = 0x01,

    /// AES-256-CBC with HMAC-SHA256 (Encrypt-then-MAC).
    /// AES-256-CBC + HMAC-SHA256（Encrypt-then-MAC）。
    Aes256Cbc = 0x02,

    /// ChaCha20-Poly1305.
    ChaCha20Poly1305 = 0x03,
}

impl Algorithm {
    /// Convert a `u8` byte to an `Algorithm` variant.
    /// Returns `None` if the byte does not correspond to a known algorithm.
    ///
    /// 将 `u8` 字节转换为 `Algorithm` 枚举变体。
    /// 如果字节不对应任何已知算法，则返回 `None`。
    ///
    /// # Examples
    ///
    /// ```
    /// use ucx_crypto::Algorithm;
    /// assert_eq!(Algorithm::from_u8(0x01), Some(Algorithm::Aes256Gcm));
    /// assert_eq!(Algorithm::from_u8(0xFF), None);
    /// ```
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Aes256Gcm),
            0x02 => Some(Self::Aes256Cbc),
            0x03 => Some(Self::ChaCha20Poly1305),
            _ => None,
        }
    }

    /// Convert an `Algorithm` variant to its `u8` byte representation.
    ///
    /// 将 `Algorithm` 枚举变体转换为 `u8` 字节表示。
    ///
    /// # Examples
    ///
    /// ```
    /// use ucx_crypto::Algorithm;
    /// assert_eq!(Algorithm::Aes256Gcm.to_u8(), 0x01);
    /// ```
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// Supported key derivation functions.
///
/// 支持的密钥派生函数。
///
/// Each variant corresponds to a KDF ID byte in the UCXE header:
/// - `0x00` = None (key provided directly)
/// - `0x01` = Argon2id
/// - `0x02` = PBKDF2-HMAC-SHA256
///
/// 每个变体对应 UCXE 头部中的 KDF ID 字节。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kdf {
    /// No KDF — key is provided directly.
    /// 无 KDF —— 密钥直接提供。
    None = 0x00,

    /// Argon2id (recommended).
    /// Argon2id（推荐）。
    Argon2id = 0x01,

    /// PBKDF2-HMAC-SHA256.
    Pbkdf2HmacSha256 = 0x02,
}

impl Kdf {
    /// Convert a `u8` byte to a `Kdf` variant.
    /// Returns `None` if the byte does not correspond to a known KDF.
    ///
    /// 将 `u8` 字节转换为 `Kdf` 枚举变体。
    /// 如果字节不对应任何已知 KDF，则返回 `None`。
    ///
    /// # Examples
    ///
    /// ```
    /// use ucx_crypto::Kdf;
    /// assert_eq!(Kdf::from_u8(0x00), Some(Kdf::None));
    /// assert_eq!(Kdf::from_u8(0xFF), None);
    /// ```
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::None),
            0x01 => Some(Self::Argon2id),
            0x02 => Some(Self::Pbkdf2HmacSha256),
            _ => None,
        }
    }

    /// Convert a `Kdf` variant to its `u8` byte representation.
    ///
    /// 将 `Kdf` 枚举变体转换为 `u8` 字节表示。
    ///
    /// # Examples
    ///
    /// ```
    /// use ucx_crypto::Kdf;
    /// assert_eq!(Kdf::None.to_u8(), 0x00);
    /// ```
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Encrypt a file using the specified algorithm.
///
/// Reads plaintext from the source, encrypts it, and writes the UCXE
/// formatted ciphertext to the destination.
///
/// **Note**: AES-256-CBC is not supported in direct-key mode because it
/// requires two independent 32-byte keys (64 bytes total). Use
/// `encrypt_with_passphrase()` for AES-256-CBC, which derives 64 bytes via KDF.
///
/// 使用指定算法加密文件。
/// 从源读取明文，加密后将 UCXE 格式的密文写入目标。
///
/// **注意**：AES-256-CBC 在直接密钥模式下不受支持，因为它需要两个独立的
/// 32 字节密钥（共 64 字节）。请使用 `encrypt_with_passphrase()` 进行
/// AES-256-CBC 加密，该函数通过 KDF 派生 64 字节密钥。
///
/// # Arguments / 参数
///
/// * `source`    - Path to the plaintext file / 明文文件路径。
/// * `dest`      - Path for the encrypted output / 加密输出文件路径。
/// * `key`       - The encryption key (32 bytes) / 加密密钥（32 字节）。
/// * `algorithm` - The encryption algorithm to use / 使用的加密算法。
///
/// # Returns / 返回
///
/// Returns `Ok(())` on success, or a `CryptoError` on failure.
/// 成功返回 `Ok(())`，失败返回 `CryptoError`。
pub fn encrypt(
    source: &Path,
    dest: &Path,
    key: &[u8; 32],
    algorithm: Algorithm,
) -> Result<(), CryptoError> {
    // AES-256-CBC requires 64-byte key (enc_key + mac_key), reject in direct-key mode.
    // AES-256-CBC 需要 64 字节密钥（enc_key + mac_key），直接密钥模式不支持。
    if algorithm == Algorithm::Aes256Cbc {
        return Err(CryptoError::UnsupportedAlgorithm(
            "AES-256-CBC requires 64-byte key (enc_key + mac_key), use encrypt_with_passphrase()"
                .into(),
        ));
    }

    // Step 1: Read plaintext from source file.
    // 第 1 步：从源文件读取明文。
    let plaintext = std::fs::read(source)?;

    // Step 2: Determine if chunked encryption is needed (>64 MiB).
    // 第 2 步：判断是否需要分块加密（>64 MiB）。
    let is_chunked = plaintext.len() > chunked::CHUNKED_THRESHOLD as usize;

    // Step 3–5: Encrypt and build UCXE structure.
    // 第 3–5 步：加密并构建 UCXE 结构。
    let ucxe = encrypt_to_ucxe(key, &plaintext, algorithm, is_chunked, Kdf::None, &format::KdfParams::None, &[])?;

    // Step 6: Serialize and write to destination.
    // 第 6 步：序列化并写入目标文件。
    let bytes = format::serialize_ucxe(&ucxe)?;
    std::fs::write(dest, bytes)?;

    Ok(())
}

/// Decrypt a UCXE formatted file.
///
/// Reads the encrypted file, verifies the authentication tag,
/// and returns the decrypted plaintext.
///
/// If the file was encrypted with a KDF (passphrase mode), this function
/// returns an error — use `decrypt_with_passphrase()` instead.
///
/// 解密 UCXE 格式文件。
/// 读取加密文件，验证认证标签，并返回解密后的明文。
///
/// 如果文件使用 KDF（口令模式）加密，此函数会返回错误
/// —— 请改用 `decrypt_with_passphrase()`。
///
/// # Arguments / 参数
///
/// * `source` - Path to the UCXE encrypted file / UCXE 加密文件路径。
/// * `key`    - The decryption key (32 bytes) / 解密密钥（32 字节）。
///
/// # Returns / 返回
///
/// Returns the decrypted plaintext bytes, or a `CryptoError` on failure.
/// 返回解密后的明文字节，或在失败时返回 `CryptoError`。
pub fn decrypt(
    source: &Path,
    key: &[u8; 32],
) -> Result<Vec<u8>, CryptoError> {
    // I/O errors surface as-is (they reflect the caller's environment, not a
    // ciphertext-content issue). All cryptographic / parsing errors collapse
    // into the opaque `DecryptionFailed` variant so attackers cannot learn
    // whether it was a wrong key, a tampered ciphertext, or a malformed file.
    //
    // I/O 错误按原样上报（反映调用方环境问题）；加密 / 解析相关错误统一
    // 塌缩为不透明的 `DecryptionFailed`，避免通过错误文本泄漏是密钥错、
    // 密文被篡改还是格式错误。
    decrypt_internal(source, key).map_err(CryptoError::into_public_decrypt_error)
}

/// Internal implementation of `decrypt` that returns the real error kind.
/// Kept private so callers cannot depend on the exact error discrimination.
///
/// 内部实现，返回真实错误类型；保持私有，使外部调用方无法依赖具体错误变体。
fn decrypt_internal(
    source: &Path,
    key: &[u8; 32],
) -> Result<Vec<u8>, CryptoError> {
    // Step 1: Read the UCXE file bytes.
    // 第 1 步：读取 UCXE 文件字节。
    let data = std::fs::read(source)?;

    // Step 2: Parse the UCXE binary format.
    // 第 2 步：解析 UCXE 二进制格式。
    let ucxe = format::parse_ucxe(&data)?;

    // Step 3: Reject KDF-protected files (need passphrase).
    // 第 3 步：拒绝 KDF 保护的文件（需要口令）。
    if ucxe.header.kdf != Kdf::None {
        return Err(CryptoError::KeyDerivation(
            "file was encrypted with a passphrase, use decrypt_with_passphrase() / \
             文件使用口令加密，请使用 decrypt_with_passphrase()"
                .into(),
        ));
    }

    // Step 4: Decrypt based on algorithm and chunked mode.
    // 第 4 步：根据算法和分块模式解密。
    decrypt_ucxe_payload(key, &ucxe)
}

/// Encrypt a file using a passphrase (with KDF key derivation).
///
/// Generates a random salt, derives the encryption key(s) from the passphrase
/// using the specified KDF, encrypts the plaintext, and writes a UCXE file
/// containing the KDF parameters so the file can be decrypted with only the
/// passphrase.
///
/// 使用口令加密文件（通过 KDF 密钥派生）。
/// 生成随机盐值，通过指定 KDF 从口令派生加密密钥，加密明文，
/// 并写入包含 KDF 参数的 UCXE 文件，使得仅需口令即可解密。
///
/// # Arguments / 参数
///
/// * `source`    - Path to the plaintext file / 明文文件路径。
/// * `dest`      - Path for the encrypted output / 加密输出文件路径。
/// * `passphrase` - The passphrase string / 口令字符串。
/// * `algorithm` - The encryption algorithm to use / 使用的加密算法。
/// * `kdf_type`  - The KDF to use (Argon2id or Pbkdf2) / 使用的 KDF。
///
/// # Returns / 返回
///
/// Returns `Ok(())` on success, or a `CryptoError` on failure.
/// 成功返回 `Ok(())`，失败返回 `CryptoError`。
pub fn encrypt_with_passphrase(
    source: &Path,
    dest: &Path,
    passphrase: &str,
    algorithm: Algorithm,
    kdf_type: Kdf,
) -> Result<(), CryptoError> {
    // Use default KDF parameters.
    // 使用默认 KDF 参数。
    let kdf_params = default_kdf_params(kdf_type)?;
    encrypt_with_passphrase_and_params(source, dest, passphrase, algorithm, kdf_type, &kdf_params)
}

/// Internal: encrypt with custom KDF parameters (used by tests for faster execution).
///
/// 内部函数：使用自定义 KDF 参数加密（测试中使用以加速执行）。
fn encrypt_with_passphrase_and_params(
    source: &Path,
    dest: &Path,
    passphrase: &str,
    algorithm: Algorithm,
    kdf_type: Kdf,
    kdf_params: &format::KdfParams,
) -> Result<(), CryptoError> {
    // Step 1: Generate random salt.
    // 第 1 步：生成随机盐值。
    let salt = kdf::generate_salt();

    // Step 2: Determine output length — AES-CBC needs 64 bytes, others need 32.
    // 第 2 步：确定输出长度 —— AES-CBC 需要 64 字节，其他需要 32 字节。
    let output_len = if algorithm == Algorithm::Aes256Cbc { 64 } else { 32 };

    // Step 3: Derive key(s) from passphrase.
    // 第 3 步：从口令派生密钥。
    let derived = kdf::derive_key(passphrase.as_bytes(), &salt, kdf_type, kdf_params, output_len)?;

    // Step 4: Read plaintext.
    // 第 4 步：读取明文。
    let plaintext = std::fs::read(source)?;

    // Step 5: Determine if chunked mode is needed.
    // 第 5 步：判断是否需要分块模式。
    // AES-CBC does not support chunked mode, so skip for CBC even if large.
    // AES-CBC 不支持分块模式，因此即使文件很大也不使用分块。
    let is_chunked = algorithm != Algorithm::Aes256Cbc
        && plaintext.len() > chunked::CHUNKED_THRESHOLD as usize;

    // Step 6: Extract the 32-byte encryption key (first 32 bytes of derived output).
    // 第 6 步：提取 32 字节加密密钥（派生输出的前 32 字节）。
    // Zeroizing 包装确保密钥在 drop 时被安全清零。
    let key: Zeroizing<[u8; 32]> = Zeroizing::new(
        derived[..32].try_into().map_err(|_| {
            CryptoError::KeyDerivation("derived key shorter than 32 bytes / 派生密钥不足 32 字节".into())
        })?,
    );

    // Step 7: Encrypt and build UCXE. For AES-CBC, pass the full 64-byte key via special path.
    // 第 7 步：加密并构建 UCXE。对于 AES-CBC，通过特殊路径传递完整 64 字节密钥。
    let ucxe = if algorithm == Algorithm::Aes256Cbc {
        encrypt_aes_cbc_to_ucxe(&derived, &plaintext, kdf_type, kdf_params, &salt)?
    } else {
        encrypt_to_ucxe(&key, &plaintext, algorithm, is_chunked, kdf_type, kdf_params, &salt)?
    };

    // Step 8: Serialize and write.
    // 第 8 步：序列化并写入。
    let bytes = format::serialize_ucxe(&ucxe)?;
    std::fs::write(dest, bytes)?;

    Ok(())
}

/// Decrypt a UCXE file using a passphrase.
///
/// Reads the UCXE file, extracts KDF parameters, derives the key from
/// the passphrase, and decrypts the ciphertext.
///
/// 使用口令解密 UCXE 文件。
/// 读取 UCXE 文件，提取 KDF 参数，从口令派生密钥，并解密密文。
///
/// # Arguments / 参数
///
/// * `source`     - Path to the UCXE encrypted file / UCXE 加密文件路径。
/// * `passphrase` - The passphrase string / 口令字符串。
///
/// # Returns / 返回
///
/// Returns the decrypted plaintext bytes, or a `CryptoError` on failure.
/// 返回解密后的明文字节，或在失败时返回 `CryptoError`。
pub fn decrypt_with_passphrase(
    source: &Path,
    passphrase: &str,
) -> Result<Vec<u8>, CryptoError> {
    // Public API: collapse all crypto/parsing errors to `DecryptionFailed`.
    // I/O errors are preserved.
    // 公开 API：将所有加密/解析错误统一塌缩为 `DecryptionFailed`；I/O 错误保留。
    decrypt_with_passphrase_internal(source, passphrase)
        .map_err(CryptoError::into_public_decrypt_error)
}

/// Internal implementation of `decrypt_with_passphrase`.
/// 内部实现 —— 保留真实错误类型供本 crate 内部诊断使用。
fn decrypt_with_passphrase_internal(
    source: &Path,
    passphrase: &str,
) -> Result<Vec<u8>, CryptoError> {
    // Step 1: Read and parse UCXE.
    // 第 1 步：读取并解析 UCXE。
    let data = std::fs::read(source)?;
    let ucxe = format::parse_ucxe(&data)?;

    // Step 2: Ensure a KDF is specified.
    // 第 2 步：确保指定了 KDF。
    if ucxe.header.kdf == Kdf::None {
        return Err(CryptoError::KeyDerivation(
            "file was not encrypted with a passphrase (KDF=None) / \
             文件未使用口令加密（KDF=None）"
                .into(),
        ));
    }

    // Step 3: Determine output length based on algorithm.
    // 第 3 步：根据算法确定输出长度。
    let output_len = if ucxe.header.algorithm == Algorithm::Aes256Cbc { 64 } else { 32 };

    // Step 4: Convert salt to fixed-size array.
    // 第 4 步：将盐值转换为固定大小数组。
    let salt: [u8; 16] = ucxe.salt.clone().try_into().map_err(|_| {
        CryptoError::InvalidFormat(format!(
            "salt length must be 16, got {} / 盐值长度必须为 16，实际为 {}",
            ucxe.salt.len(),
            ucxe.salt.len()
        ))
    })?;

    // Step 5: Derive key(s) from passphrase.
    // 第 5 步：从口令派生密钥。
    let derived = kdf::derive_key(
        passphrase.as_bytes(),
        &salt,
        ucxe.header.kdf,
        &ucxe.kdf_params,
        output_len,
    )?;

    // Step 6: Decrypt based on algorithm.
    // 第 6 步：根据算法解密。
    if ucxe.header.algorithm == Algorithm::Aes256Cbc {
        decrypt_aes_cbc_payload(&derived, &ucxe)
    } else {
        let key: Zeroizing<[u8; 32]> = Zeroizing::new(
            derived[..32].try_into().map_err(|_| {
                CryptoError::KeyDerivation("derived key shorter than 32 bytes / 派生密钥不足 32 字节".into())
            })?,
        );
        decrypt_ucxe_payload(&key, &ucxe)
    }
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Build default KDF parameters for the given KDF type.
/// 为给定 KDF 类型构建默认参数。
fn default_kdf_params(kdf_type: Kdf) -> Result<format::KdfParams, CryptoError> {
    match kdf_type {
        Kdf::None => Err(CryptoError::KeyDerivation(
            "KDF type cannot be None for passphrase mode / 口令模式下 KDF 类型不能为 None".into(),
        )),
        Kdf::Argon2id => Ok(format::KdfParams::Argon2id {
            memory_cost_kib: kdf::ARGON2ID_DEFAULT_MEMORY_KIB,
            time_cost: kdf::ARGON2ID_DEFAULT_TIME_COST,
            parallelism: kdf::ARGON2ID_DEFAULT_PARALLELISM,
        }),
        Kdf::Pbkdf2HmacSha256 => Ok(format::KdfParams::Pbkdf2 {
            iterations: kdf::PBKDF2_DEFAULT_ITERATIONS,
        }),
    }
}

/// Core encryption logic: encrypt plaintext and build a UcxeFile struct.
/// Works for AEAD algorithms (AES-GCM, ChaCha20-Poly1305).
///
/// 核心加密逻辑：加密明文并构建 UcxeFile 结构体。
/// 适用于 AEAD 算法（AES-GCM, ChaCha20-Poly1305）。
fn encrypt_to_ucxe(
    key: &[u8; 32],
    plaintext: &[u8],
    algorithm: Algorithm,
    is_chunked: bool,
    kdf_type: Kdf,
    kdf_params: &format::KdfParams,
    salt: &[u8],
) -> Result<format::UcxeFile, CryptoError> {
    // Build the header first, then compute the AEAD AAD from
    // `header || kdf_params || salt`. Binding the full tuple into every
    // AEAD tag prevents an attacker from swapping any of these bytes
    // (algorithm, KDF choice, Argon2id/PBKDF2 parameters, salt, chunked
    // flag) without the decrypt side detecting it.
    //
    // 先构建 header，再基于 `header || kdf_params || salt` 组合成 AEAD AAD。
    // 将完整元组绑入每个 AEAD 标签，可阻止攻击者在不被解密端察觉的情况下
    // 篡改任何字段（算法、KDF 选择、Argon2id/PBKDF2 参数、盐值、分块标志）。
    let header = format::UcxeHeader {
        format_version: UCXE_FORMAT_VERSION,
        algorithm,
        kdf: kdf_type,
        chunked: is_chunked,
    };
    let aad = format::build_aead_aad(&header, kdf_params, salt);

    if is_chunked {
        // Chunked mode: generate base nonce, encrypt in chunks, serialize.
        // 分块模式：生成基础 nonce，分块加密，序列化。
        let mut base_nonce = [0u8; 12];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut base_nonce);

        let chunked_ct =
            chunked::encrypt_chunked(key, &base_nonce, plaintext, algorithm, &aad)?;
        let serialized = chunked::serialize_chunks(&chunked_ct);

        Ok(format::UcxeFile {
            header,
            kdf_params: kdf_params.clone(),
            salt: salt.to_vec(),
            iv: base_nonce.to_vec(),
            ciphertext: serialized,
            // Chunked mode: no top-level tag (each chunk has its own tag).
            // 分块模式：无顶层标签（每个分块有自己的标签）。
            tag: vec![0u8; format::tag_length(algorithm)],
        })
    } else {
        // Non-chunked mode: encrypt the entire plaintext at once.
        // 非分块模式：一次性加密全部明文。
        let (ciphertext, nonce, tag) = match algorithm {
            Algorithm::Aes256Gcm => aes_gcm::encrypt(key, plaintext, &aad)?,
            Algorithm::ChaCha20Poly1305 => chacha20::encrypt(key, plaintext, &aad)?,
            Algorithm::Aes256Cbc => {
                return Err(CryptoError::UnsupportedAlgorithm(
                    "AES-256-CBC should use encrypt_aes_cbc_to_ucxe() / \
                     AES-256-CBC 应使用 encrypt_aes_cbc_to_ucxe()"
                        .into(),
                ));
            }
        };

        Ok(format::UcxeFile {
            header,
            kdf_params: kdf_params.clone(),
            salt: salt.to_vec(),
            iv: nonce.to_vec(),
            ciphertext,
            tag: tag.to_vec(),
        })
    }
}

/// Encrypt plaintext using AES-256-CBC + HMAC-SHA256 (requires 64-byte key).
///
/// AES-256-CBC 加密（需要 64 字节密钥：前 32 字节 enc_key + 后 32 字节 mac_key）。
fn encrypt_aes_cbc_to_ucxe(
    derived: &[u8],
    plaintext: &[u8],
    kdf_type: Kdf,
    kdf_params: &format::KdfParams,
    salt: &[u8],
) -> Result<format::UcxeFile, CryptoError> {
    // Split derived key into enc_key (first 32B) and mac_key (last 32B).
    // 将派生密钥分为 enc_key（前 32B）和 mac_key（后 32B）。
    // Zeroizing 包装确保密钥在 drop 时被安全清零。
    let enc_key: Zeroizing<[u8; 32]> = Zeroizing::new(
        derived[..32].try_into().map_err(|_| {
            CryptoError::KeyDerivation("derived key shorter than 64 bytes / 派生密钥不足 64 字节".into())
        })?,
    );
    let mac_key: Zeroizing<[u8; 32]> = Zeroizing::new(
        derived[32..64].try_into().map_err(|_| {
            CryptoError::KeyDerivation("derived key shorter than 64 bytes / 派生密钥不足 64 字节".into())
        })?,
    );

    // Build header first so we can pass its serialized bytes as HMAC AAD.
    // 先构建 header，以便把其序列化字节作为 HMAC 的 AAD。
    let header = format::UcxeHeader {
        format_version: UCXE_FORMAT_VERSION,
        algorithm: Algorithm::Aes256Cbc,
        kdf: kdf_type,
        chunked: false,
    };
    let aad = header.to_bytes();

    let (ciphertext, iv, hmac_tag) =
        aes_cbc::encrypt(&enc_key, &mac_key, plaintext, &aad)?;

    Ok(format::UcxeFile {
        header,
        kdf_params: kdf_params.clone(),
        salt: salt.to_vec(),
        iv: iv.to_vec(),
        ciphertext,
        tag: hmac_tag.to_vec(),
    })
}

/// Core decryption logic for AEAD algorithms (AES-GCM, ChaCha20-Poly1305).
///
/// AEAD 算法（AES-GCM, ChaCha20-Poly1305）的核心解密逻辑。
fn decrypt_ucxe_payload(
    key: &[u8; 32],
    ucxe: &format::UcxeFile,
) -> Result<Vec<u8>, CryptoError> {
    let algorithm = ucxe.header.algorithm;

    // Recompute the AAD from the parsed header, KDF parameters, and salt.
    // Must match the bytes bound at encrypt time (see `encrypt_to_ucxe`);
    // any discrepancy will cause AEAD tag verification to fail with
    // `AuthenticationFailed`, correctly flagging header / KDF / salt tampering.
    //
    // 从解析出的 header、KDF 参数和盐值重建 AAD。必须与加密端绑入的字节一致
    //（参见 `encrypt_to_ucxe`）；若任一字节不匹配，AEAD 标签验证会失败并
    // 返回 `AuthenticationFailed`，正确识别 header / KDF / salt 的篡改。
    let aad = format::build_aead_aad(&ucxe.header, &ucxe.kdf_params, &ucxe.salt);

    if ucxe.header.chunked {
        // Chunked mode: deserialize chunks, then decrypt.
        // 分块模式：反序列化分块，然后解密。
        let nonce: [u8; 12] = ucxe.iv.clone().try_into().map_err(|_| {
            CryptoError::InvalidFormat(format!(
                "IV length must be 12 for chunked AEAD, got {} / \
                 分块 AEAD 的 IV 长度必须为 12，实际为 {}",
                ucxe.iv.len(),
                ucxe.iv.len()
            ))
        })?;

        let chunked_ct = chunked::deserialize_chunks(&ucxe.ciphertext, algorithm)?;
        chunked::decrypt_chunked(key, &nonce, &chunked_ct, algorithm, &aad)
    } else {
        // Non-chunked mode: decrypt directly.
        // 非分块模式：直接解密。
        match algorithm {
            Algorithm::Aes256Gcm => {
                let nonce: [u8; 12] = ucxe.iv.clone().try_into().map_err(|_| {
                    CryptoError::InvalidFormat("AES-GCM nonce must be 12 bytes".into())
                })?;
                let tag: [u8; 16] = ucxe.tag.clone().try_into().map_err(|_| {
                    CryptoError::InvalidFormat("AES-GCM tag must be 16 bytes".into())
                })?;
                aes_gcm::decrypt(key, &nonce, &ucxe.ciphertext, &tag, &aad)
            }
            Algorithm::ChaCha20Poly1305 => {
                let nonce: [u8; 12] = ucxe.iv.clone().try_into().map_err(|_| {
                    CryptoError::InvalidFormat("ChaCha20 nonce must be 12 bytes".into())
                })?;
                let tag: [u8; 16] = ucxe.tag.clone().try_into().map_err(|_| {
                    CryptoError::InvalidFormat("ChaCha20 tag must be 16 bytes".into())
                })?;
                chacha20::decrypt(key, &nonce, &ucxe.ciphertext, &tag, &aad)
            }
            Algorithm::Aes256Cbc => Err(CryptoError::UnsupportedAlgorithm(
                "AES-256-CBC requires passphrase mode for decryption / \
                 AES-256-CBC 解密需要口令模式"
                    .into(),
            )),
        }
    }
}

/// Decrypt AES-256-CBC payload using 64-byte derived key.
///
/// 使用 64 字节派生密钥解密 AES-256-CBC 载荷。
fn decrypt_aes_cbc_payload(
    derived: &[u8],
    ucxe: &format::UcxeFile,
) -> Result<Vec<u8>, CryptoError> {
    // Zeroizing 包装确保密钥在 drop 时被安全清零。
    let enc_key: Zeroizing<[u8; 32]> = Zeroizing::new(
        derived[..32].try_into().map_err(|_| {
            CryptoError::KeyDerivation("derived key shorter than 64 bytes / 派生密钥不足 64 字节".into())
        })?,
    );
    let mac_key: Zeroizing<[u8; 32]> = Zeroizing::new(
        derived[32..64].try_into().map_err(|_| {
            CryptoError::KeyDerivation("derived key shorter than 64 bytes / 派生密钥不足 64 字节".into())
        })?,
    );

    let iv: [u8; 16] = ucxe.iv.clone().try_into().map_err(|_| {
        CryptoError::InvalidFormat("AES-CBC IV must be 16 bytes".into())
    })?;
    let hmac_tag: [u8; 32] = ucxe.tag.clone().try_into().map_err(|_| {
        CryptoError::InvalidFormat("AES-CBC HMAC tag must be 32 bytes".into())
    })?;

    // Recompute AAD from the parsed header so HMAC verification also covers
    // the UCXE header bytes; any header tamper → MAC mismatch.
    // 从解析出的 header 重建 AAD，使 HMAC 同时覆盖 UCXE 头部；
    // 头部任一字节被篡改都会导致 MAC 校验失败。
    let aad = ucxe.header.to_bytes();
    aes_cbc::decrypt(&enc_key, &mac_key, &iv, &ucxe.ciphertext, &hmac_tag, &aad)
}

/// Check if a file is UCXE encrypted by examining its magic number.
///
/// 通过检查魔数判断文件是否为 UCXE 加密格式。
///
/// # Arguments / 参数
///
/// * `data` - The first few bytes of the file.
///   文件的前几个字节。
///
/// # Returns / 返回
///
/// Returns `true` if the data starts with the UCXE magic number.
/// 如果数据以 UCXE 魔数开头则返回 `true`。
pub fn is_encrypted(data: &[u8]) -> bool {
    data.len() >= 4 && data[0..4] == UCXE_MAGIC
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that `is_encrypted` correctly identifies UCXE magic.
    /// 测试 `is_encrypted` 是否正确识别 UCXE 魔数。
    #[test]
    fn test_is_encrypted() {
        // UCXE magic number should be detected.
        // 应检测到 UCXE 魔数。
        assert!(is_encrypted(b"UCXE\x01\x01\x01\x00"));

        // Non-UCXE data should not be detected.
        // 非 UCXE 数据不应被检测为加密。
        assert!(!is_encrypted(b"# Chapter 1\n"));

        // Empty data should not be detected.
        // 空数据不应被检测为加密。
        assert!(!is_encrypted(b""));
    }

    /// Verify the magic number constant matches "UCXE" in ASCII.
    /// 验证魔数常量与 ASCII "UCXE" 一致。
    #[test]
    fn test_magic_constant() {
        assert_eq!(&UCXE_MAGIC, b"UCXE");
    }

    /// Test `Algorithm::from_u8` / `to_u8` round-trip for all variants.
    /// 测试所有 `Algorithm` 变体的 `from_u8` / `to_u8` 往返转换。
    #[test]
    fn test_algorithm_from_to_u8() {
        for (byte, expected) in [
            (0x01u8, Algorithm::Aes256Gcm),
            (0x02, Algorithm::Aes256Cbc),
            (0x03, Algorithm::ChaCha20Poly1305),
        ] {
            let algo = Algorithm::from_u8(byte).expect("known algorithm byte");
            assert_eq!(algo, expected);
            assert_eq!(algo.to_u8(), byte);
        }

        assert!(Algorithm::from_u8(0x00).is_none());
        assert!(Algorithm::from_u8(0xFF).is_none());
    }

    /// Test `Kdf::from_u8` / `to_u8` round-trip for all variants.
    /// 测试所有 `Kdf` 变体的 `from_u8` / `to_u8` 往返转换。
    #[test]
    fn test_kdf_from_to_u8() {
        for (byte, expected) in [
            (0x00u8, Kdf::None),
            (0x01, Kdf::Argon2id),
            (0x02, Kdf::Pbkdf2HmacSha256),
        ] {
            let kdf = Kdf::from_u8(byte).expect("known KDF byte");
            assert_eq!(kdf, expected);
            assert_eq!(kdf.to_u8(), byte);
        }

        assert!(Kdf::from_u8(0x03).is_none());
        assert!(Kdf::from_u8(0xFF).is_none());
    }

    // =========================================================================
    // Integration tests for encrypt/decrypt API.
    // 加密/解密 API 集成测试。
    // =========================================================================

    /// Helper: write test plaintext to a temp file and return the paths.
    /// 辅助函数：将测试明文写入临时文件并返回路径。
    fn setup_temp_files(plaintext: &[u8]) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let src = dir.path().join("plain.txt");
        let dst = dir.path().join("encrypted.ucxe");
        std::fs::write(&src, plaintext).expect("failed to write plaintext");
        (dir, src, dst)
    }

    /// Test 1: AES-256-GCM file encrypt/decrypt round-trip.
    /// 测试 1：AES-256-GCM 文件加密/解密往返。
    #[test]
    fn test_encrypt_decrypt_aes_gcm() {
        let key = [0x42u8; 32];
        let plaintext = b"Hello, AES-GCM encryption!";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        encrypt(&src, &dst, &key, Algorithm::Aes256Gcm).expect("encrypt should succeed");
        let decrypted = decrypt(&dst, &key).expect("decrypt should succeed");

        assert_eq!(decrypted, plaintext);
    }

    /// Test 2: ChaCha20-Poly1305 file encrypt/decrypt round-trip.
    /// 测试 2：ChaCha20-Poly1305 文件加密/解密往返。
    #[test]
    fn test_encrypt_decrypt_chacha20() {
        let key = [0x99u8; 32];
        let plaintext = b"Hello, ChaCha20 encryption!";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        encrypt(&src, &dst, &key, Algorithm::ChaCha20Poly1305).expect("encrypt should succeed");
        let decrypted = decrypt(&dst, &key).expect("decrypt should succeed");

        assert_eq!(decrypted, plaintext);
    }

    /// Test 3: Argon2id passphrase mode encrypt/decrypt round-trip.
    /// Uses small Argon2id parameters for fast testing.
    ///
    /// 测试 3：Argon2id 口令模式加密/解密往返。
    /// 使用较小 Argon2id 参数以加速测试。
    #[test]
    fn test_encrypt_decrypt_passphrase_argon2id() {
        let plaintext = b"Secret data with Argon2id KDF";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        // Use OWASP-minimum Argon2id params: tests must pass parse-time validation.
        // 使用符合 OWASP 最低要求的 Argon2id 参数：测试必须能通过解析期校验。
        let kdf_params = format::KdfParams::Argon2id {
            memory_cost_kib: kdf::ARGON2ID_MIN_MEMORY_KIB,
            time_cost: kdf::ARGON2ID_MIN_TIME_COST,
            parallelism: kdf::ARGON2ID_MIN_PARALLELISM,
        };

        encrypt_with_passphrase_and_params(
            &src, &dst, "test-passphrase", Algorithm::Aes256Gcm, Kdf::Argon2id, &kdf_params,
        )
        .expect("encrypt should succeed");

        let decrypted = decrypt_with_passphrase(&dst, "test-passphrase")
            .expect("decrypt should succeed");

        assert_eq!(decrypted, plaintext);
    }

    /// Test 4: PBKDF2 passphrase mode encrypt/decrypt round-trip.
    /// Uses small iteration count for fast testing.
    ///
    /// 测试 4：PBKDF2 口令模式加密/解密往返。
    /// 使用较少迭代次数以加速测试。
    #[test]
    fn test_encrypt_decrypt_passphrase_pbkdf2() {
        let plaintext = b"Secret data with PBKDF2 KDF";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        let kdf_params = format::KdfParams::Pbkdf2 { iterations: kdf::PBKDF2_MIN_ITERATIONS };

        encrypt_with_passphrase_and_params(
            &src, &dst, "test-passphrase", Algorithm::ChaCha20Poly1305,
            Kdf::Pbkdf2HmacSha256, &kdf_params,
        )
        .expect("encrypt should succeed");

        let decrypted = decrypt_with_passphrase(&dst, "test-passphrase")
            .expect("decrypt should succeed");

        assert_eq!(decrypted, plaintext);
    }

    /// Test 5: Wrong key should fail with the opaque `DecryptionFailed` error
    /// whose display text is exactly `"decryption failed"` (no variant leakage).
    /// 测试 5：错误密钥应返回不透明的 `DecryptionFailed`，显示文本恰好为
    /// `"decryption failed"`（不泄漏错误类别）。
    #[test]
    fn test_wrong_key_fails() {
        let key = [0x42u8; 32];
        let wrong_key = [0x00u8; 32];
        let plaintext = b"sensitive data";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        encrypt(&src, &dst, &key, Algorithm::Aes256Gcm).expect("encrypt should succeed");

        let result = decrypt(&dst, &wrong_key);
        let err = result.expect_err("wrong key must fail decryption");
        assert!(
            matches!(err, CryptoError::DecryptionFailed),
            "public decrypt API must return opaque DecryptionFailed, got: {err:?}"
        );
        assert_eq!(format!("{err}"), "decryption failed");
    }

    /// Test 6: Wrong passphrase should fail decryption.
    /// 测试 6：错误口令应导致解密失败。
    #[test]
    fn test_wrong_passphrase_fails() {
        let plaintext = b"passphrase-protected data";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        let kdf_params = format::KdfParams::Pbkdf2 { iterations: kdf::PBKDF2_MIN_ITERATIONS };

        encrypt_with_passphrase_and_params(
            &src, &dst, "correct-passphrase", Algorithm::Aes256Gcm,
            Kdf::Pbkdf2HmacSha256, &kdf_params,
        )
        .expect("encrypt should succeed");

        let result = decrypt_with_passphrase(&dst, "wrong-passphrase");
        let err = result.expect_err("wrong passphrase must fail decryption");
        assert!(
            matches!(err, CryptoError::DecryptionFailed),
            "public API must return opaque DecryptionFailed, got: {err:?}"
        );
        assert_eq!(format!("{err}"), "decryption failed");
    }

    /// Test 7: AES-256-CBC direct key mode should be rejected.
    /// 测试 7：AES-256-CBC 直接密钥模式应被拒绝。
    #[test]
    fn test_aes_cbc_direct_key_rejected() {
        let key = [0x42u8; 32];
        let plaintext = b"should not encrypt";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        let result = encrypt(&src, &dst, &key, Algorithm::Aes256Cbc);
        assert!(
            matches!(result, Err(CryptoError::UnsupportedAlgorithm(_))),
            "AES-256-CBC direct key should be rejected"
        );
    }

    /// Test 8: AES-256-CBC passphrase mode should work (64-byte key via KDF).
    /// 测试 8：AES-256-CBC 口令模式应能工作（通过 KDF 生成 64 字节密钥）。
    #[test]
    fn test_aes_cbc_passphrase_mode() {
        let plaintext = b"AES-CBC with passphrase mode";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        let kdf_params = format::KdfParams::Pbkdf2 { iterations: kdf::PBKDF2_MIN_ITERATIONS };

        encrypt_with_passphrase_and_params(
            &src, &dst, "cbc-passphrase", Algorithm::Aes256Cbc,
            Kdf::Pbkdf2HmacSha256, &kdf_params,
        )
        .expect("encrypt should succeed");

        let decrypted = decrypt_with_passphrase(&dst, "cbc-passphrase")
            .expect("decrypt should succeed");

        assert_eq!(decrypted, plaintext);
    }

    /// Test: tampering the UCXE header's flags byte after encryption must
    /// cause decryption to fail — proves the header bytes are bound into AAD.
    ///
    /// 测试：加密后篡改 UCXE 头部 flags 字节必须导致解密失败 ——
    /// 证明头部字节已经绑定到 AEAD 的 AAD。
    #[test]
    fn test_header_tampering_fails_decryption() {
        let key = [0x42u8; 32];
        let plaintext = b"header-bound authenticated encryption";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        encrypt(&src, &dst, &key, Algorithm::Aes256Gcm).expect("encrypt");

        // File layout offsets: magic[4] + ver + algo + kdf + flags → flags at byte 7.
        // 文件布局偏移：魔数[4] + 版本 + 算法 + KDF + flags → flags 位于第 7 字节。
        let mut bytes = std::fs::read(&dst).unwrap();
        bytes[7] ^= 0x01; // flip the chunked flag bit
        std::fs::write(&dst, &bytes).unwrap();

        let result = decrypt(&dst, &key);
        assert!(
            result.is_err(),
            "header tampering must cause decryption failure (AAD binding active)"
        );
    }

    /// Test: tampering the encoded KDF parameters byte after encryption must
    /// cause decryption to fail — proves the KDF params are bound into AAD.
    ///
    /// For PBKDF2 the on-disk layout is:
    ///   magic(4) + version(1) + algo(1) + kdf(1) + flags(1) + iterations(u32 LE, 4 B)
    /// so byte 8 is the low byte of the iteration count. Flipping it both
    /// (a) causes a different key to be derived AND (b) desynchronizes AAD,
    /// so the AEAD tag must fail regardless of which check wins.
    ///
    /// 测试：加密后篡改编码后的 KDF 参数字节必须导致解密失败 ——
    /// 证明 KDF 参数已绑定到 AEAD 的 AAD。
    /// PBKDF2 的磁盘布局：魔数(4) + 版本(1) + 算法(1) + KDF(1) + flags(1)
    /// + 迭代次数(u32 LE, 4B)，因此第 8 字节是迭代次数低字节。翻转它既会
    /// 让派生密钥不同，也会让 AAD 不一致 —— 两条路径都会让 AEAD 失败。
    #[test]
    fn test_kdf_params_tampering_fails_decryption() {
        let plaintext = b"CRYPTO-2: kdf-params must be AAD-bound";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        let kdf_params = format::KdfParams::Pbkdf2 { iterations: kdf::PBKDF2_MIN_ITERATIONS };
        encrypt_with_passphrase_and_params(
            &src, &dst, "kdf-aad-passphrase", Algorithm::Aes256Gcm,
            Kdf::Pbkdf2HmacSha256, &kdf_params,
        )
        .expect("encrypt should succeed");

        // Flip one bit in the PBKDF2 iteration count (file byte 8).
        // 翻转 PBKDF2 迭代次数的一个比特（文件第 8 字节）。
        let mut bytes = std::fs::read(&dst).expect("read encrypted file");
        bytes[8] ^= 0x01;
        std::fs::write(&dst, &bytes).expect("write tampered file");

        let result = decrypt_with_passphrase(&dst, "kdf-aad-passphrase");
        assert!(
            matches!(result, Err(CryptoError::DecryptionFailed)),
            "tampered KDF params must fail decryption (opaque DecryptionFailed), got: {result:?}"
        );
    }

    /// Test: tampering the salt byte after encryption must cause decryption
    /// to fail — proves the salt is bound into AAD.
    ///
    /// PBKDF2 on-disk layout up to the salt:
    ///   magic(4) + header(4) + pbkdf2_params(4) + salt_len_prefix(u16 LE, 2 B)
    ///   + salt(...)
    /// so salt[0] lives at byte 14. Flipping it re-derives a different key
    /// and also desynchronizes AAD — either way the AEAD tag must reject.
    ///
    /// 测试：加密后篡改盐字节必须导致解密失败 —— 证明盐已经绑定到 AEAD
    /// 的 AAD。PBKDF2 盐值之前的布局：
    ///   魔数(4) + header(4) + pbkdf2 参数(4) + 盐长度前缀(u16 LE, 2B) + 盐...
    /// 因此 salt[0] 在第 14 字节。翻转它会派生出不同密钥，同时 AAD 也不
    /// 一致 —— 两条路径都会使 AEAD 标签被拒。
    #[test]
    fn test_salt_tampering_fails_decryption() {
        let plaintext = b"CRYPTO-2: salt must be AAD-bound";
        let (_dir, src, dst) = setup_temp_files(plaintext);

        let kdf_params = format::KdfParams::Pbkdf2 { iterations: kdf::PBKDF2_MIN_ITERATIONS };
        encrypt_with_passphrase_and_params(
            &src, &dst, "salt-aad-passphrase", Algorithm::Aes256Gcm,
            Kdf::Pbkdf2HmacSha256, &kdf_params,
        )
        .expect("encrypt should succeed");

        // Flip one bit in the first salt byte (file byte 14 for this layout).
        // 翻转第一个盐字节的一个比特（此布局下为文件第 14 字节）。
        let mut bytes = std::fs::read(&dst).expect("read encrypted file");
        bytes[14] ^= 0x01;
        std::fs::write(&dst, &bytes).expect("write tampered file");

        let result = decrypt_with_passphrase(&dst, "salt-aad-passphrase");
        assert!(
            matches!(result, Err(CryptoError::DecryptionFailed)),
            "tampered salt must fail decryption (opaque DecryptionFailed), got: {result:?}"
        );
    }
}
