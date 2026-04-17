//! Key derivation functions (KDF) for UCX encryption.
//! UCX 加密的密钥派生函数。
//!
//! Supports:
//! - Argon2id (recommended): memory-hard, resistant to GPU/ASIC attacks
//! - PBKDF2-HMAC-SHA256: widely compatible fallback
//!
//! 支持：
//! - Argon2id（推荐）：内存困难型，抵抗 GPU/ASIC 攻击
//! - PBKDF2-HMAC-SHA256：广泛兼容的备选方案

use crate::CryptoError;
use zeroize::Zeroizing;

// =============================================================================
// Default Parameters / 默认参数
// =============================================================================

/// Default Argon2id memory cost: 64 MiB (65536 KiB).
/// Argon2id 默认内存开销：64 MiB（65536 KiB）。
pub const ARGON2ID_DEFAULT_MEMORY_KIB: u32 = 65536;

/// Default Argon2id time cost (iterations): 3.
/// Argon2id 默认时间开销（迭代次数）：3。
pub const ARGON2ID_DEFAULT_TIME_COST: u32 = 3;

/// Default Argon2id parallelism: 4.
/// Argon2id 默认并行度：4。
pub const ARGON2ID_DEFAULT_PARALLELISM: u32 = 4;

/// Default PBKDF2 iterations: 600,000.
/// PBKDF2 默认迭代次数：600,000。
pub const PBKDF2_DEFAULT_ITERATIONS: u32 = 600_000;

/// Salt size in bytes (128 bits).
/// 盐值大小（128 位 = 16 字节）。
pub const SALT_SIZE: usize = 16;

// =============================================================================
// KDF Parameter Bounds / KDF 参数上下限
// =============================================================================
//
// OWASP 2023 recommends Argon2id: m ≥ 19 MiB, t ≥ 2, p ≥ 1.
// We additionally cap time/memory to prevent DoS via maliciously-large params.
//
// OWASP 2023 推荐 Argon2id：m ≥ 19 MiB, t ≥ 2, p ≥ 1。
// 额外设置上限以防范恶意超大参数导致的拒绝服务攻击。

/// Argon2id minimum memory cost (KiB). OWASP 2023 推荐最低 19 MiB。
pub const ARGON2ID_MIN_MEMORY_KIB: u32 = 19_456;

/// Argon2id maximum memory cost (KiB). 4 GiB 作为硬上限，防 DoS。
pub const ARGON2ID_MAX_MEMORY_KIB: u32 = 4_194_304;

/// Argon2id minimum time cost (iterations).
pub const ARGON2ID_MIN_TIME_COST: u32 = 2;

/// Argon2id maximum time cost (iterations). 上限防 DoS。
pub const ARGON2ID_MAX_TIME_COST: u32 = 100;

/// Argon2id minimum parallelism.
pub const ARGON2ID_MIN_PARALLELISM: u32 = 1;

/// PBKDF2 minimum iteration count. OWASP 2023 推荐 ≥ 600,000；
/// 此处取保守下限 100,000 以兼容旧文件但仍阻断极弱参数。
pub const PBKDF2_MIN_ITERATIONS: u32 = 100_000;

/// PBKDF2 maximum iteration count. 上限防 DoS。
pub const PBKDF2_MAX_ITERATIONS: u32 = 10_000_000;

/// Validate KDF parameters against minimum security and maximum resource bounds.
///
/// Called by `format::parse_ucxe` immediately after the KDF parameters are
/// decoded. Rejecting out-of-range values at parse time prevents both:
/// 1. Weak-parameter attacks (too low → password cracking trivial).
/// 2. Resource-exhaustion attacks (too high → DoS during decryption).
///
/// 在 `format::parse_ucxe` 解析完 KDF 参数后调用。
/// 解析阶段就拒绝越界值可同时防范：
/// 1. 弱参数攻击（过低会使口令爆破变得容易）；
/// 2. 资源耗尽攻击（过高会在解密时触发 DoS）。
///
/// # Arguments / 参数
///
/// * `kdf`        - The declared KDF variant / 声明的 KDF 类型。
/// * `kdf_params` - The decoded parameter struct / 解码后的参数结构。
///
/// # Errors / 错误
///
/// Returns `CryptoError::WeakKdfParameters` if any parameter is out of bounds.
/// 任一参数越界则返回 `CryptoError::WeakKdfParameters`。
pub fn validate_kdf_params(
    kdf: crate::Kdf,
    kdf_params: &crate::format::KdfParams,
) -> Result<(), CryptoError> {
    match (kdf, kdf_params) {
        // KDF None → params must also be None, no bounds to check.
        // KDF 为 None 时参数也应为 None，无上下限校验。
        (crate::Kdf::None, crate::format::KdfParams::None) => Ok(()),
        // Mismatch between KDF id and params variant: reject as weak/malformed.
        // KDF ID 与参数变体不一致：按非法处理。
        (crate::Kdf::None, _) | (_, crate::format::KdfParams::None) => {
            Err(CryptoError::WeakKdfParameters(
                "KDF id and parameter variant mismatch / KDF ID 与参数变体不一致".into(),
            ))
        }
        (
            crate::Kdf::Argon2id,
            crate::format::KdfParams::Argon2id {
                memory_cost_kib,
                time_cost,
                parallelism,
            },
        ) => {
            if *memory_cost_kib < ARGON2ID_MIN_MEMORY_KIB {
                return Err(CryptoError::WeakKdfParameters(format!(
                    "Argon2id memory {memory_cost_kib} KiB < min {ARGON2ID_MIN_MEMORY_KIB} KiB"
                )));
            }
            if *memory_cost_kib > ARGON2ID_MAX_MEMORY_KIB {
                return Err(CryptoError::WeakKdfParameters(format!(
                    "Argon2id memory {memory_cost_kib} KiB > max {ARGON2ID_MAX_MEMORY_KIB} KiB"
                )));
            }
            if *time_cost < ARGON2ID_MIN_TIME_COST {
                return Err(CryptoError::WeakKdfParameters(format!(
                    "Argon2id time {time_cost} < min {ARGON2ID_MIN_TIME_COST}"
                )));
            }
            if *time_cost > ARGON2ID_MAX_TIME_COST {
                return Err(CryptoError::WeakKdfParameters(format!(
                    "Argon2id time {time_cost} > max {ARGON2ID_MAX_TIME_COST}"
                )));
            }
            if *parallelism < ARGON2ID_MIN_PARALLELISM {
                return Err(CryptoError::WeakKdfParameters(format!(
                    "Argon2id parallelism {parallelism} < min {ARGON2ID_MIN_PARALLELISM}"
                )));
            }
            Ok(())
        }
        (crate::Kdf::Pbkdf2HmacSha256, crate::format::KdfParams::Pbkdf2 { iterations }) => {
            if *iterations < PBKDF2_MIN_ITERATIONS {
                return Err(CryptoError::WeakKdfParameters(format!(
                    "PBKDF2 iterations {iterations} < min {PBKDF2_MIN_ITERATIONS}"
                )));
            }
            if *iterations > PBKDF2_MAX_ITERATIONS {
                return Err(CryptoError::WeakKdfParameters(format!(
                    "PBKDF2 iterations {iterations} > max {PBKDF2_MAX_ITERATIONS}"
                )));
            }
            Ok(())
        }
        // Any other (kdf, params) pairing is malformed.
        // 其他不匹配组合视为非法。
        _ => Err(CryptoError::WeakKdfParameters(
            "KDF id and parameter variant mismatch / KDF ID 与参数变体不一致".into(),
        )),
    }
}

// =============================================================================
// Public Functions / 公开函数
// =============================================================================

/// Generate a random 16-byte salt using CSPRNG.
/// 使用 CSPRNG 生成 16 字节随机盐值。
///
/// Uses the operating system's cryptographically secure random number generator
/// (`OsRng`) to ensure high-quality entropy.
///
/// 使用操作系统的密码学安全随机数生成器（`OsRng`）确保高质量熵源。
///
/// # Returns / 返回
///
/// A 16-byte array filled with cryptographically secure random bytes.
/// 一个填满密码学安全随机字节的 16 字节数组。
pub fn generate_salt() -> [u8; SALT_SIZE] {
    use rand::RngCore;

    let mut salt = [0u8; SALT_SIZE];
    // OsRng delegates to the OS CSPRNG (e.g., /dev/urandom, BCryptGenRandom).
    // OsRng 委托给操作系统 CSPRNG（如 /dev/urandom、BCryptGenRandom）。
    rand::rngs::OsRng.fill_bytes(&mut salt);
    salt
}

/// Derive a 32-byte key using Argon2id.
/// 使用 Argon2id 派生 32 字节密钥。
///
/// Argon2id is a memory-hard KDF that combines Argon2i (data-independent memory
/// access, side-channel resistant) and Argon2d (data-dependent, GPU resistant).
/// It is the recommended KDF for password-based key derivation.
///
/// Argon2id 是一种内存困难型 KDF，结合了 Argon2i（数据无关内存访问，抗侧信道）
/// 和 Argon2d（数据相关，抗 GPU）。它是推荐的基于口令的密钥派生函数。
///
/// # Arguments / 参数
///
/// * `passphrase`      - The user's passphrase (must not be empty).
///   用户口令（不能为空）。
/// * `salt`            - A 16-byte random salt.
///   16 字节随机盐值。
/// * `memory_cost_kib` - Memory usage in KiB (e.g., 65536 = 64 MiB).
///   内存开销，单位 KiB（例如 65536 = 64 MiB）。
/// * `time_cost`       - Number of iterations.
///   迭代次数。
/// * `parallelism`     - Degree of parallelism (number of lanes).
///   并行度（通道数）。
///
/// # Errors / 错误
///
/// Returns `CryptoError::KeyDerivation` if:
/// - The passphrase is empty.
/// - Argon2id parameter construction or hashing fails.
///
/// 在以下情况返回 `CryptoError::KeyDerivation`：
/// - 口令为空。
/// - Argon2id 参数构造或哈希计算失败。
pub fn derive_argon2id(
    passphrase: &[u8],
    salt: &[u8; SALT_SIZE],
    memory_cost_kib: u32,
    time_cost: u32,
    parallelism: u32,
) -> Result<[u8; 32], CryptoError> {
    // Reject empty passphrases to prevent accidental unprotected encryption.
    // 拒绝空口令以防止意外的无保护加密。
    if passphrase.is_empty() {
        return Err(CryptoError::KeyDerivation(
            "passphrase must not be empty / 口令不能为空".to_string(),
        ));
    }

    // Build Argon2id parameters via ParamsBuilder.
    // 通过 ParamsBuilder 构建 Argon2id 参数。
    let params = argon2::ParamsBuilder::new()
        .m_cost(memory_cost_kib)
        .t_cost(time_cost)
        .p_cost(parallelism)
        .output_len(32)
        .build()
        .map_err(|e| CryptoError::KeyDerivation(format!("Argon2id params error: {e}")))?;

    // Create Argon2id instance with version 0x13 (v19).
    // 创建 Argon2id 实例，版本 0x13（v19）。
    let argon2 = argon2::Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        params,
    );

    // Derive the 32-byte key.
    // 派生 32 字节密钥。
    let mut output = [0u8; 32];
    argon2
        .hash_password_into(passphrase, salt, &mut output)
        .map_err(|e| CryptoError::KeyDerivation(format!("Argon2id hash error: {e}")))?;

    Ok(output)
}

/// Derive a 32-byte key using PBKDF2-HMAC-SHA256.
/// 使用 PBKDF2-HMAC-SHA256 派生 32 字节密钥。
///
/// PBKDF2 is a widely supported KDF that applies HMAC-SHA256 iteratively.
/// While not memory-hard like Argon2id, it provides broad compatibility
/// with existing systems and standards (e.g., NIST SP 800-132).
///
/// PBKDF2 是一种广泛支持的 KDF，通过迭代应用 HMAC-SHA256。
/// 虽然不像 Argon2id 那样具有内存困难性，但它提供了与现有系统和标准
/// （如 NIST SP 800-132）的广泛兼容性。
///
/// # Arguments / 参数
///
/// * `passphrase` - The user's passphrase (must not be empty).
///   用户口令（不能为空）。
/// * `salt`       - A 16-byte random salt.
///   16 字节随机盐值。
/// * `iterations` - Number of PBKDF2 iterations (e.g., 600,000).
///   PBKDF2 迭代次数（例如 600,000）。
///
/// # Errors / 错误
///
/// Returns `CryptoError::KeyDerivation` if the passphrase is empty.
/// 如果口令为空则返回 `CryptoError::KeyDerivation`。
pub fn derive_pbkdf2(
    passphrase: &[u8],
    salt: &[u8; SALT_SIZE],
    iterations: u32,
) -> Result<[u8; 32], CryptoError> {
    // Reject empty passphrases.
    // 拒绝空口令。
    if passphrase.is_empty() {
        return Err(CryptoError::KeyDerivation(
            "passphrase must not be empty / 口令不能为空".to_string(),
        ));
    }

    // Derive 32 bytes using PBKDF2-HMAC-SHA256.
    // 使用 PBKDF2-HMAC-SHA256 派生 32 字节。
    let mut output = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase, salt, iterations, &mut output);

    Ok(output)
}

/// Derive key(s) using the specified KDF — unified entry point.
/// 使用指定 KDF 派生密钥 —— 统一入口函数。
///
/// This is the primary function for deriving encryption keys from a passphrase.
/// It dispatches to the appropriate KDF based on the `kdf` parameter and
/// supports variable-length output for different encryption algorithms:
/// - AES-256-GCM / ChaCha20-Poly1305: 32 bytes (encryption key only)
/// - AES-256-CBC (Encrypt-then-MAC): 64 bytes (32 enc_key + 32 mac_key)
///
/// 这是从口令派生加密密钥的主入口函数。根据 `kdf` 参数分派到相应的 KDF，
/// 并支持不同加密算法所需的可变长度输出：
/// - AES-256-GCM / ChaCha20-Poly1305：32 字节（仅加密密钥）
/// - AES-256-CBC（Encrypt-then-MAC）：64 字节（32 enc_key + 32 mac_key）
///
/// # Arguments / 参数
///
/// * `passphrase` - The user's passphrase (must not be empty).
///   用户口令（不能为空）。
/// * `salt`       - A 16-byte random salt.
///   16 字节随机盐值。
/// * `kdf`        - The KDF variant to use (`Argon2id` or `Pbkdf2HmacSha256`).
///   要使用的 KDF 变体（`Argon2id` 或 `Pbkdf2HmacSha256`）。
/// * `kdf_params` - KDF-specific parameters (must match the `kdf` variant).
///   KDF 特定参数（必须与 `kdf` 变体匹配）。
/// * `output_len` - Desired output length in bytes (typically 32 or 64).
///   期望的输出长度（字节），通常为 32 或 64。
///
/// # Errors / 错误
///
/// Returns `CryptoError::KeyDerivation` if:
/// - `kdf` is `Kdf::None` (no KDF to derive from).
/// - `kdf_params` variant does not match the `kdf` variant.
/// - The underlying KDF operation fails.
///
/// 在以下情况返回 `CryptoError::KeyDerivation`：
/// - `kdf` 为 `Kdf::None`（无 KDF 可用于派生）。
/// - `kdf_params` 变体与 `kdf` 变体不匹配。
/// - 底层 KDF 操作失败。
pub fn derive_key(
    passphrase: &[u8],
    salt: &[u8; SALT_SIZE],
    kdf: crate::Kdf,
    kdf_params: &crate::format::KdfParams,
    output_len: usize,
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    match kdf {
        // Kdf::None means the key is provided directly — no derivation needed.
        // Kdf::None 表示密钥直接提供 —— 不需要派生。
        crate::Kdf::None => Err(CryptoError::KeyDerivation(
            "KDF is None but derive_key was called / KDF 为 None 但调用了 derive_key".to_string(),
        )),

        // Argon2id: use ParamsBuilder to set the exact output length.
        // Argon2id：使用 ParamsBuilder 设置精确的输出长度。
        crate::Kdf::Argon2id => {
            // Ensure KdfParams matches the Argon2id variant.
            // 确保 KdfParams 与 Argon2id 变体匹配。
            let crate::format::KdfParams::Argon2id {
                memory_cost_kib,
                time_cost,
                parallelism,
            } = kdf_params
            else {
                return Err(CryptoError::KeyDerivation(
                    "KDF is Argon2id but params are not Argon2id / KDF 为 Argon2id 但参数不匹配"
                        .to_string(),
                ));
            };

            // Build params with the requested output length.
            // 使用请求的输出长度构建参数。
            let params = argon2::ParamsBuilder::new()
                .m_cost(*memory_cost_kib)
                .t_cost(*time_cost)
                .p_cost(*parallelism)
                .output_len(output_len)
                .build()
                .map_err(|e| CryptoError::KeyDerivation(format!("Argon2id params error: {e}")))?;

            let argon2 = argon2::Argon2::new(
                argon2::Algorithm::Argon2id,
                argon2::Version::V0x13,
                params,
            );

            // Zeroizing 包装确保密钥材料在 drop 时被安全清零。
            let mut output = Zeroizing::new(vec![0u8; output_len]);
            argon2
                .hash_password_into(passphrase, salt, &mut output)
                .map_err(|e| CryptoError::KeyDerivation(format!("Argon2id hash error: {e}")))?;

            Ok(output)
        }

        // PBKDF2-HMAC-SHA256: output length is controlled by the `res` buffer size.
        // PBKDF2-HMAC-SHA256：输出长度由 `res` 缓冲区大小控制。
        crate::Kdf::Pbkdf2HmacSha256 => {
            // Ensure KdfParams matches the Pbkdf2 variant.
            // 确保 KdfParams 与 Pbkdf2 变体匹配。
            let crate::format::KdfParams::Pbkdf2 { iterations } = kdf_params else {
                return Err(CryptoError::KeyDerivation(
                    "KDF is PBKDF2 but params are not Pbkdf2 / KDF 为 PBKDF2 但参数不匹配"
                        .to_string(),
                ));
            };

            // Reject empty passphrases.
            // 拒绝空口令。
            if passphrase.is_empty() {
                return Err(CryptoError::KeyDerivation(
                    "passphrase must not be empty / 口令不能为空".to_string(),
                ));
            }

            // Zeroizing 包装确保密钥材料在 drop 时被安全清零。
            let mut output = Zeroizing::new(vec![0u8; output_len]);
            pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase, salt, *iterations, &mut output);

            Ok(output)
        }
    }
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Use small Argon2id parameters for fast testing.
    // 使用较小的 Argon2id 参数以加速测试。
    const TEST_MEMORY_KIB: u32 = 256;
    const TEST_TIME_COST: u32 = 1;
    const TEST_PARALLELISM: u32 = 1;

    // Test PBKDF2 with reduced iterations for speed.
    // 使用较少的 PBKDF2 迭代次数以加速测试。
    const TEST_PBKDF2_ITERATIONS: u32 = 1000;

    /// Same passphrase + salt + params → same Argon2id key (determinism).
    /// 相同口令 + 盐值 + 参数 → 相同 Argon2id 密钥（确定性）。
    #[test]
    fn test_argon2id_derive_deterministic() {
        let passphrase = b"test-passphrase-123";
        let salt = [0xAA; SALT_SIZE];

        let key1 = derive_argon2id(passphrase, &salt, TEST_MEMORY_KIB, TEST_TIME_COST, TEST_PARALLELISM)
            .expect("first derivation should succeed");
        let key2 = derive_argon2id(passphrase, &salt, TEST_MEMORY_KIB, TEST_TIME_COST, TEST_PARALLELISM)
            .expect("second derivation should succeed");

        assert_eq!(key1, key2, "same inputs must produce the same Argon2id key");
    }

    /// Same passphrase + salt + iterations → same PBKDF2 key (determinism).
    /// 相同口令 + 盐值 + 迭代次数 → 相同 PBKDF2 密钥（确定性）。
    #[test]
    fn test_pbkdf2_derive_deterministic() {
        let passphrase = b"test-passphrase-456";
        let salt = [0xBB; SALT_SIZE];

        let key1 = derive_pbkdf2(passphrase, &salt, TEST_PBKDF2_ITERATIONS)
            .expect("first derivation should succeed");
        let key2 = derive_pbkdf2(passphrase, &salt, TEST_PBKDF2_ITERATIONS)
            .expect("second derivation should succeed");

        assert_eq!(key1, key2, "same inputs must produce the same PBKDF2 key");
    }

    /// Different passphrases must produce different keys.
    /// 不同口令必须产生不同密钥。
    #[test]
    fn test_different_passphrase_different_key() {
        let salt = [0xCC; SALT_SIZE];

        let key_a = derive_argon2id(b"password-A", &salt, TEST_MEMORY_KIB, TEST_TIME_COST, TEST_PARALLELISM)
            .expect("derive A should succeed");
        let key_b = derive_argon2id(b"password-B", &salt, TEST_MEMORY_KIB, TEST_TIME_COST, TEST_PARALLELISM)
            .expect("derive B should succeed");

        assert_ne!(key_a, key_b, "different passphrases must yield different keys");
    }

    /// Different salts must produce different keys.
    /// 不同盐值必须产生不同密钥。
    #[test]
    fn test_different_salt_different_key() {
        let passphrase = b"same-passphrase";
        let salt_a = [0x01; SALT_SIZE];
        let salt_b = [0x02; SALT_SIZE];

        let key_a = derive_pbkdf2(passphrase, &salt_a, TEST_PBKDF2_ITERATIONS)
            .expect("derive with salt A should succeed");
        let key_b = derive_pbkdf2(passphrase, &salt_b, TEST_PBKDF2_ITERATIONS)
            .expect("derive with salt B should succeed");

        assert_ne!(key_a, key_b, "different salts must yield different keys");
    }

    /// Empty passphrase must be rejected by both KDFs.
    /// 空口令必须被两个 KDF 拒绝。
    #[test]
    fn test_empty_passphrase_rejected() {
        let salt = [0xDD; SALT_SIZE];

        // Argon2id should reject empty passphrase.
        // Argon2id 应拒绝空口令。
        let result = derive_argon2id(b"", &salt, TEST_MEMORY_KIB, TEST_TIME_COST, TEST_PARALLELISM);
        assert!(result.is_err(), "Argon2id must reject empty passphrase");

        // PBKDF2 should reject empty passphrase.
        // PBKDF2 应拒绝空口令。
        let result = derive_pbkdf2(b"", &salt, TEST_PBKDF2_ITERATIONS);
        assert!(result.is_err(), "PBKDF2 must reject empty passphrase");
    }

    /// Two consecutive generate_salt() calls must produce different salts.
    /// 连续两次调用 generate_salt() 必须产生不同的盐值。
    #[test]
    fn test_salt_randomness() {
        let salt_a = generate_salt();
        let salt_b = generate_salt();

        // With 128-bit random values, collision probability is negligible.
        // 128 位随机值的碰撞概率可忽略不计。
        assert_ne!(salt_a, salt_b, "two random salts should differ");
    }

    /// Weak Argon2id params must be rejected by `validate_kdf_params`.
    /// 弱 Argon2id 参数必须被 `validate_kdf_params` 拒绝。
    #[test]
    fn test_validate_rejects_weak_argon2id() {
        // Memory below OWASP 2023 minimum → reject.
        // 内存低于 OWASP 2023 最低要求 → 拒绝。
        let weak = crate::format::KdfParams::Argon2id {
            memory_cost_kib: 1024,
            time_cost: 3,
            parallelism: 1,
        };
        assert!(matches!(
            validate_kdf_params(crate::Kdf::Argon2id, &weak),
            Err(CryptoError::WeakKdfParameters(_))
        ));

        // Time cost below minimum → reject.
        // time_cost 低于下限 → 拒绝。
        let weak_t = crate::format::KdfParams::Argon2id {
            memory_cost_kib: ARGON2ID_MIN_MEMORY_KIB,
            time_cost: 1,
            parallelism: 1,
        };
        assert!(matches!(
            validate_kdf_params(crate::Kdf::Argon2id, &weak_t),
            Err(CryptoError::WeakKdfParameters(_))
        ));
    }

    /// Extreme / out-of-range Argon2id params must be rejected.
    /// 极端（过大）Argon2id 参数必须被拒绝。
    #[test]
    fn test_validate_rejects_extreme_argon2id() {
        // Memory above hard cap → reject.
        // 内存超过硬上限 → 拒绝。
        let huge_m = crate::format::KdfParams::Argon2id {
            memory_cost_kib: ARGON2ID_MAX_MEMORY_KIB + 1,
            time_cost: 3,
            parallelism: 1,
        };
        assert!(matches!(
            validate_kdf_params(crate::Kdf::Argon2id, &huge_m),
            Err(CryptoError::WeakKdfParameters(_))
        ));

        // Time cost above hard cap → reject.
        // time_cost 超过硬上限 → 拒绝。
        let huge_t = crate::format::KdfParams::Argon2id {
            memory_cost_kib: ARGON2ID_MIN_MEMORY_KIB,
            time_cost: ARGON2ID_MAX_TIME_COST + 1,
            parallelism: 1,
        };
        assert!(matches!(
            validate_kdf_params(crate::Kdf::Argon2id, &huge_t),
            Err(CryptoError::WeakKdfParameters(_))
        ));
    }

    /// Weak / extreme PBKDF2 iterations must be rejected.
    /// 弱 / 极端 PBKDF2 迭代次数必须被拒绝。
    #[test]
    fn test_validate_rejects_weak_and_extreme_pbkdf2() {
        let weak = crate::format::KdfParams::Pbkdf2 { iterations: 1_000 };
        assert!(matches!(
            validate_kdf_params(crate::Kdf::Pbkdf2HmacSha256, &weak),
            Err(CryptoError::WeakKdfParameters(_))
        ));

        let huge = crate::format::KdfParams::Pbkdf2 {
            iterations: PBKDF2_MAX_ITERATIONS + 1,
        };
        assert!(matches!(
            validate_kdf_params(crate::Kdf::Pbkdf2HmacSha256, &huge),
            Err(CryptoError::WeakKdfParameters(_))
        ));
    }

    /// Sane Argon2id and PBKDF2 params must pass validation.
    /// 合理的 Argon2id 与 PBKDF2 参数必须通过校验。
    #[test]
    fn test_validate_accepts_sane_params() {
        let argon = crate::format::KdfParams::Argon2id {
            memory_cost_kib: ARGON2ID_MIN_MEMORY_KIB,
            time_cost: ARGON2ID_MIN_TIME_COST,
            parallelism: 1,
        };
        assert!(validate_kdf_params(crate::Kdf::Argon2id, &argon).is_ok());

        let pbkdf2 = crate::format::KdfParams::Pbkdf2 {
            iterations: PBKDF2_MIN_ITERATIONS,
        };
        assert!(validate_kdf_params(crate::Kdf::Pbkdf2HmacSha256, &pbkdf2).is_ok());
    }

    /// Mismatched KDF id and params variant must be rejected.
    /// KDF ID 与参数变体不一致必须被拒绝。
    #[test]
    fn test_validate_rejects_variant_mismatch() {
        let argon_params = crate::format::KdfParams::Argon2id {
            memory_cost_kib: ARGON2ID_MIN_MEMORY_KIB,
            time_cost: ARGON2ID_MIN_TIME_COST,
            parallelism: 1,
        };
        let result = validate_kdf_params(crate::Kdf::Pbkdf2HmacSha256, &argon_params);
        assert!(matches!(result, Err(CryptoError::WeakKdfParameters(_))));
    }

    /// derive_key() with output_len=64 should return 64 bytes.
    /// derive_key() 设置 output_len=64 时应返回 64 字节。
    #[test]
    fn test_derive_key_64_bytes() {
        let passphrase = b"test-passphrase-64";
        let salt = [0xEE; SALT_SIZE];

        // Test Argon2id with 64-byte output.
        // 测试 Argon2id 64 字节输出。
        let argon2_params = crate::format::KdfParams::Argon2id {
            memory_cost_kib: TEST_MEMORY_KIB,
            time_cost: TEST_TIME_COST,
            parallelism: TEST_PARALLELISM,
        };
        let result = derive_key(passphrase, &salt, crate::Kdf::Argon2id, &argon2_params, 64)
            .expect("Argon2id 64-byte derive should succeed");
        assert_eq!(result.len(), 64, "Argon2id output must be 64 bytes");

        // Test PBKDF2 with 64-byte output.
        // 测试 PBKDF2 64 字节输出。
        let pbkdf2_params = crate::format::KdfParams::Pbkdf2 {
            iterations: TEST_PBKDF2_ITERATIONS,
        };
        let result = derive_key(passphrase, &salt, crate::Kdf::Pbkdf2HmacSha256, &pbkdf2_params, 64)
            .expect("PBKDF2 64-byte derive should succeed");
        assert_eq!(result.len(), 64, "PBKDF2 output must be 64 bytes");
    }
}
