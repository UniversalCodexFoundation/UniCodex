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

    // Restrict the file permissions so that only the current user can read/write.
    // This is best-effort: failure is logged but does not abort the save —
    // callers should treat a warning here as a security advisory.
    //
    // 限制文件权限为仅当前用户可读写。这是"尽力而为"：失败仅记录，不中断保存 —
    // 调用方应将此处的警告视为安全提示。
    if let Err(e) = restrict_private_key_permissions(path) {
        tracing::warn!(
            path = %path.display(),
            error = %e,
            "failed to restrict private key permissions — key may be readable by other users"
        );
    }

    Ok(())
}

/// Restrict permissions on a private key file so that only the current user
/// can read / write it.
///
/// - On Unix: `chmod 0600`.
/// - On Windows: `icacls <path> /inheritance:r /grant:r "<CurrentUser>:(R,W)"`
///   — breaks inheritance (so the default `Authenticated Users:M` ACE is no
///   longer inherited) and grants only the current user Read+Write.
///
/// This hardens freshly-written key files. Best-effort: if the OS command
/// fails we surface the error to the caller, but `save_private_key` treats
/// it as a warning rather than an error.
///
/// 限制私钥文件权限为仅当前用户可读写。
///
/// - 在 Unix 上：`chmod 0600`。
/// - 在 Windows 上：`icacls <path> /inheritance:r /grant:r "<CurrentUser>:(R,W)"`
///   — 断开继承（使默认的 `Authenticated Users:M` 继承条目失效），并仅授予
///   当前用户 Read+Write 权限。
///
/// 这项加固作用于刚写入的密钥文件。"尽力而为"：若 OS 命令失败会将错误返回
/// 给调用方，但 `save_private_key` 将其视为警告而非错误。
fn restrict_private_key_permissions(path: &Path) -> Result<(), SignError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // 0o600 = owner: rw, group: -, other: -
        // 0o600 = 所有者：rw，组：-，其他：-
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
        Ok(())
    }

    #[cfg(windows)]
    {
        // Resolve the current user's principal name.
        // The USERDOMAIN / USERNAME pair is set on all interactive Windows sessions;
        // we fall back to %USERNAME% alone if USERDOMAIN is missing (rare).
        //
        // 解析当前用户的身份名。USERDOMAIN / USERNAME 在所有交互式
        // Windows 会话中都会设置；若 USERDOMAIN 缺失（罕见）则回退到
        // 仅使用 %USERNAME%。
        let username = std::env::var("USERNAME").map_err(|e| {
            SignError::Io(std::io::Error::other(format!(
                "USERNAME env var not set: {e}"
            )))
        })?;
        let principal = match std::env::var("USERDOMAIN") {
            Ok(domain) if !domain.is_empty() => format!("{domain}\\{username}"),
            _ => username,
        };

        // Use icacls to:
        //   /inheritance:r      — remove all inherited ACEs (including the
        //                         default "Authenticated Users:M" that Windows
        //                         grants on files under %USERPROFILE%).
        //   /grant:r "user:(R,W)" — replace (rather than add) ACEs so the
        //                           current user gets exactly R+W.
        //
        // 使用 icacls：
        //   /inheritance:r        — 移除所有继承的 ACE（包括 Windows 默认
        //                           在 %USERPROFILE% 下文件上授予的
        //                           "Authenticated Users:M"）。
        //   /grant:r "user:(R,W)" — 替换（而非追加）ACE，使当前用户仅获 R+W。
        let output = std::process::Command::new("icacls")
            .arg(path.as_os_str())
            .arg("/inheritance:r")
            .arg("/grant:r")
            .arg(format!("{principal}:(R,W)"))
            .output()
            .map_err(|e| {
                SignError::Io(std::io::Error::new(
                    e.kind(),
                    format!("failed to spawn icacls: {e}"),
                ))
            })?;
        if !output.status.success() {
            // Fall back to Rust's cross-platform read-only flag. It's not as
            // strong as ACL tightening, but at least prevents accidental
            // overwrites by the current process; the principal-set attempt
            // above is what blocks other users.
            //
            // 回退到 Rust 的跨平台只读标志。它没有 ACL 收紧那么强，但至少
            // 能阻止当前进程的意外覆盖；阻断其他用户访问靠的是上面的
            // principal-set 尝试。
            let mut perms = std::fs::metadata(path)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(true);
            std::fs::set_permissions(path, perms)?;
            return Err(SignError::Io(std::io::Error::other(format!(
                "icacls returned non-zero status ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ))));
        }
        Ok(())
    }

    // On other (non-Unix, non-Windows) targets, we can only toggle the
    // cross-platform readonly bit — not a security control, but it prevents
    // accidental overwrites.
    //
    // 在其他（非 Unix、非 Windows）目标上，我们只能切换跨平台 readonly 位 ——
    // 这并非安全控制，但能防止意外覆盖。
    #[cfg(not(any(unix, windows)))]
    {
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(path, perms)?;
        Ok(())
    }
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
        let (signing_key, verifying_key) =
            generate_ed25519_keypair().expect("key generation should succeed");

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
        let (signing_key, _) = generate_ed25519_keypair().expect("key generation should succeed");

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
        let (_, verifying_key) = generate_ed25519_keypair().expect("key generation should succeed");

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

        let (signing_key, verifying_key) =
            generate_ed25519_keypair().expect("key generation should succeed");

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

    /// Test (Unix): save_private_key sets the file mode to 0o600.
    /// 测试 (Unix)：save_private_key 将文件权限设为 0o600。
    #[cfg(unix)]
    #[test]
    fn test_save_private_key_sets_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let (signing_key, _) = generate_ed25519_keypair().expect("key generation should succeed");

        let tmp_dir = TempDir::new().expect("failed to create temp dir");
        let key_path = tmp_dir.path().join("sig5.pem");

        save_private_key(&signing_key, &key_path).expect("save_private_key should succeed");

        let mode = std::fs::metadata(&key_path)
            .expect("metadata should succeed")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(
            mode, 0o600,
            "private key file must be mode 0600 after save_private_key, got {mode:o}"
        );
    }

    /// Test (Windows): save_private_key invokes icacls and the file remains
    /// readable by the current user (we can load it back).
    ///
    /// We cannot easily assert "other users cannot read it" from within this
    /// test process because it IS the current user. The key property we
    /// check is: the hardening step did not corrupt the file and the current
    /// user still has Read access.
    ///
    /// 测试 (Windows)：save_private_key 调用 icacls 后文件仍可被当前用户读取。
    /// 本测试进程本身就是当前用户，无法直接断言"其他用户不能读"；我们检查的
    /// 关键性质是：加固步骤未损坏文件，当前用户仍有 Read 权限。
    #[cfg(windows)]
    #[test]
    fn test_save_private_key_current_user_can_still_read_on_windows() {
        let (signing_key, _) = generate_ed25519_keypair().expect("key generation should succeed");

        let tmp_dir = TempDir::new().expect("failed to create temp dir");
        let key_path = tmp_dir.path().join("sig5.pem");

        save_private_key(&signing_key, &key_path).expect("save_private_key should succeed");

        // The file must still exist and be readable by us.
        // 文件应仍然存在，且可由当前用户读取。
        assert!(key_path.exists(), "key file must exist after save");
        let loaded = load_private_key(&key_path)
            .expect("current user must still be able to load the key after ACL hardening");
        assert_eq!(
            loaded.to_bytes(),
            signing_key.to_bytes(),
            "loaded key must match the original"
        );
    }

    /// Test: PEM output starts with the correct header.
    /// 测试：PEM 输出以正确的头部开始。
    #[test]
    fn test_pem_output_starts_with_correct_header() {
        let (signing_key, _) = generate_ed25519_keypair().expect("key generation should succeed");

        // Encode private key to PKCS#8 PEM.
        // 将私钥编码为 PKCS#8 PEM。
        let pem_string = signing_key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("PEM encoding should succeed");

        // Verify PEM header.
        // 验证 PEM 头部。
        assert!(
            pem_string
                .as_str()
                .starts_with("-----BEGIN PRIVATE KEY-----"),
            "PEM output must start with '-----BEGIN PRIVATE KEY-----', got: {}",
            &pem_string.as_str()[..50]
        );
    }
}
