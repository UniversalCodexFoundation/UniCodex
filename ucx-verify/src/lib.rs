//! UCX Signature & Integrity Verification Module.
//!
//! This module verifies UCX file integrity and digital signatures,
//! supporting both Layer 1 (JAR-style) and Layer 2 (APK v2-style) signatures.
//!
//! Verification flow:
//! 1. Read the UCX file into memory.
//! 2. **Layer 2**: Find the signing block, parse signer entries, recompute
//!    the protected-content digest, and verify the Ed25519 signature.
//! 3. **Layer 1**: Open as ZIP, find `META-INF/signatures/*.SF` and `*.EC`,
//!    verify SF digest against MANIFEST.MF, verify EC signature against SF.
//! 4. Combine results into a [`VerifyReport`].
//!
//! UCX 签名与完整性验证模块。
//! 验证 UCX 文件的完整性和数字签名，
//! 支持 Layer 1（JAR 式）和 Layer 2（APK v2 式）签名。
//!
//! 验证流程：
//! 1. 将 UCX 文件读入内存。
//! 2. **Layer 2**：查找签名块，解析签名者条目，重新计算
//!    受保护内容摘要，并验证 Ed25519 签名。
//! 3. **Layer 1**：以 ZIP 打开，查找 `META-INF/signatures/*.SF` 和 `*.EC`，
//!    验证 SF 摘要与 MANIFEST.MF 一致，验证 EC 签名与 SF 一致。
//! 4. 将结果汇总为 [`VerifyReport`]。

use std::io::Read;

use thiserror::Error;

// =============================================================================
// Error types / 错误类型
// =============================================================================

/// Errors that can occur during signature verification.
///
/// 签名验证过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum VerifyError {
    /// Signature block not found in the UCX archive.
    /// UCX 归档中找不到签名块。
    #[error("signing block not found")]
    SigningBlockNotFound,

    /// Signature verification failed.
    /// 签名验证失败。
    #[error("signature verification failed: {0}")]
    SignatureInvalid(String),

    /// Certificate chain validation failed.
    /// 证书链验证失败。
    #[error("certificate chain validation failed: {0}")]
    CertificateInvalid(String),

    /// File hash mismatch detected.
    /// 检测到文件哈希不匹配。
    #[error("file hash mismatch: {path} (expected {expected}, got {actual})")]
    HashMismatch {
        /// The file path within the archive.
        /// 归档内的文件路径。
        path: String,
        /// The expected hash value from MANIFEST.MF.
        /// MANIFEST.MF 中记录的期望哈希值。
        expected: String,
        /// The actual computed hash value.
        /// 实际计算的哈希值。
        actual: String,
    },

    /// I/O error during verification.
    /// 验证过程中的 I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The file is not a valid UCX/ZIP archive.
    /// 文件不是有效的 UCX/ZIP 归档。
    #[error("invalid UCX file: {0}")]
    InvalidFile(String),
}

// =============================================================================
// Result types / 结果类型
// =============================================================================

/// Overall verification status.
///
/// 总体验证状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyStatus {
    /// All checks passed — both Layer 1 and Layer 2 valid.
    /// 所有检查通过 — Layer 1 和 Layer 2 均有效。
    Valid,

    /// Only one layer passed (partial verification).
    /// 仅一层通过（部分验证）。
    ValidWithWarnings,

    /// Verification failed — signatures found but invalid.
    /// 验证失败 — 找到签名但无效。
    Invalid,

    /// No signatures found (unsigned file).
    /// 未找到签名（未签名文件）。
    Unsigned,
}

/// Detailed verification report.
///
/// 详细的验证报告。
#[derive(Debug, Clone)]
pub struct VerifyReport {
    /// Overall verification status.
    /// 总体验证状态。
    pub status: VerifyStatus,

    /// Layer 1 verification result (if signatures found).
    /// Layer 1 验证结果（如找到签名）。
    pub layer1: Option<Layer1Result>,

    /// Layer 2 verification result (if signing block found).
    /// Layer 2 验证结果（如找到签名块）。
    pub layer2: Option<Layer2Result>,

    /// Per-signer verification information.
    /// 各签名者验证信息。
    pub signers: Vec<SignerInfo>,
}

/// Layer 1 (JAR-style) verification result.
///
/// Layer 1（JAR 式）验证结果。
#[derive(Debug, Clone)]
pub struct Layer1Result {
    /// Whether all Layer 1 signatures are valid.
    /// 所有 Layer 1 签名是否有效。
    pub valid: bool,

    /// Number of signers found in the archive.
    /// 归档中找到的签名者数量。
    pub signer_count: usize,

    /// Textual description of the verification result.
    /// 验证结果的文字描述。
    pub details: String,
}

/// Layer 2 (APK v2-style) verification result.
///
/// Layer 2（APK v2 式）验证结果。
#[derive(Debug, Clone)]
pub struct Layer2Result {
    /// Whether the Layer 2 signature is valid.
    /// Layer 2 签名是否有效。
    pub valid: bool,

    /// Textual description of the verification result.
    /// 验证结果的文字描述。
    pub details: String,
}

/// Information about an individual signer found during verification.
///
/// 验证过程中发现的各签名者信息。
#[derive(Debug, Clone)]
pub struct SignerInfo {
    /// Signer identifier (e.g., "AUTHOR"), derived from the SF filename stem.
    /// 签名者标识（如 "AUTHOR"），从 SF 文件名词干提取。
    pub signer_id: String,

    /// Subject Common Name (CN) from the signer's certificate.
    /// 签名者证书的主体通用名称（CN）。
    pub subject_cn: String,

    /// Certificate type: "self-signed" or "CA-issued".
    /// 证书类型："self-signed" 或 "CA-issued"。
    pub cert_type: String,

    /// BLAKE3 fingerprint of the signer's certificate (hex string).
    /// 签名者证书的 BLAKE3 指纹（十六进制字符串）。
    pub fingerprint_blake3: String,

    /// Whether this signer's Layer 1 signature is valid.
    /// 此签名者的 Layer 1 签名是否有效。
    pub layer1_valid: bool,

    /// Whether this signer's Layer 2 signature is valid.
    /// 此签名者的 Layer 2 签名是否有效。
    pub layer2_valid: bool,
}

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Verify a UCX file's integrity and signatures.
///
/// Performs a comprehensive verification including:
/// 1. Layer 2 (APK v2-style) signing block verification.
/// 2. Layer 1 (JAR-style) SF/EC signature verification.
/// 3. MANIFEST.MF digest comparison.
/// 4. Certificate information extraction.
///
/// Returns a detailed [`VerifyReport`].
///
/// 验证 UCX 文件的完整性和签名。
/// 执行全面验证，包括：
/// 1. Layer 2（APK v2 式）签名块验证。
/// 2. Layer 1（JAR 式）SF/EC 签名验证。
/// 3. MANIFEST.MF 摘要比对。
/// 4. 证书信息提取。
///
/// 返回详细的 [`VerifyReport`]。
///
/// # Arguments / 参数
///
/// * `path` - Path to the `.ucx` file.
///   `.ucx` 文件路径。
///
/// # Errors / 错误
///
/// Returns `VerifyError::Io` on I/O failure.
/// Returns `VerifyError::InvalidFile` if the file is not a valid UCX/ZIP archive.
/// I/O 失败返回 `VerifyError::Io`，文件不是有效 UCX/ZIP 归档返回 `VerifyError::InvalidFile`。
pub fn verify(path: &std::path::Path) -> Result<VerifyReport, VerifyError> {
    // -------------------------------------------------------------------------
    // Step 1: Read the entire file into memory.
    // 步骤 1：将整个文件读入内存。
    // -------------------------------------------------------------------------
    let file_data = std::fs::read(path)?;

    // -------------------------------------------------------------------------
    // Step 2: Validate that this is a valid ZIP archive.
    // 步骤 2：验证这是一个有效的 ZIP 归档。
    // -------------------------------------------------------------------------
    let cursor = std::io::Cursor::new(&file_data);
    if zip::ZipArchive::new(cursor).is_err() {
        return Err(VerifyError::InvalidFile(
            "file is not a valid ZIP/UCX archive".to_string(),
        ));
    }

    // -------------------------------------------------------------------------
    // Step 3: Layer 2 verification — check for signing block.
    // 步骤 3：Layer 2 验证 — 检查签名块。
    // -------------------------------------------------------------------------
    let (layer2_result, layer2_signers) = verify_layer2(&file_data);

    // -------------------------------------------------------------------------
    // Step 4: Layer 1 verification — check for SF/EC files in ZIP.
    // 步骤 4：Layer 1 验证 — 检查 ZIP 中的 SF/EC 文件。
    // -------------------------------------------------------------------------
    let (layer1_result, layer1_signers) = verify_layer1(&file_data);

    // -------------------------------------------------------------------------
    // Step 5: Merge signer information from both layers.
    // 步骤 5：合并两层的签名者信息。
    // -------------------------------------------------------------------------
    let signers = merge_signer_info(&layer1_signers, &layer2_signers);

    // -------------------------------------------------------------------------
    // Step 6: Determine overall verification status.
    // 步骤 6：确定总体验证状态。
    // -------------------------------------------------------------------------
    let has_layer1 = layer1_result.is_some();
    let has_layer2 = layer2_result.is_some();
    let layer1_valid = layer1_result.as_ref().is_some_and(|r| r.valid);
    let layer2_valid = layer2_result.as_ref().is_some_and(|r| r.valid);

    let status = if !has_layer1 && !has_layer2 {
        // No signatures found at all.
        // 完全没有找到签名。
        VerifyStatus::Unsigned
    } else if layer1_valid && layer2_valid {
        // Both layers pass → fully verified.
        // 两层都通过 → 完全验证。
        VerifyStatus::Valid
    } else if layer1_valid || layer2_valid {
        // Only one layer passes → partial.
        // 仅一层通过 → 部分验证。
        VerifyStatus::ValidWithWarnings
    } else {
        // Signatures found but none valid → invalid.
        // 找到签名但都无效 → 无效。
        VerifyStatus::Invalid
    };

    Ok(VerifyReport {
        status,
        layer1: layer1_result,
        layer2: layer2_result,
        signers,
    })
}

// =============================================================================
// Layer 2 verification / Layer 2 验证
// =============================================================================

/// Verify Layer 2 (APK v2-style signing block).
///
/// Returns the verification result and a list of Layer 2 signer info entries.
/// If no signing block is found, returns `(None, vec![])`.
///
/// 验证 Layer 2（APK v2 式签名块）。
/// 返回验证结果和 Layer 2 签名者信息列表。
/// 如果未找到签名块，返回 `(None, vec![])`。
fn verify_layer2(file_data: &[u8]) -> (Option<Layer2Result>, Vec<Layer2SignerInfo>) {
    // Try to find the signing block in the raw binary data.
    // 尝试在原始二进制数据中查找签名块。
    let block_result = match ucx_sign::zip_binary::find_signing_block(file_data) {
        Ok(Some((block_data, _offset))) => block_data,
        Ok(None) => {
            // No signing block found — file has no Layer 2 signature.
            // 未找到签名块 — 文件没有 Layer 2 签名。
            return (None, vec![]);
        }
        Err(e) => {
            // Error while searching for signing block — treat as invalid.
            // 搜索签名块时出错 — 视为无效。
            return (
                Some(Layer2Result {
                    valid: false,
                    details: format!("error locating signing block: {e}"),
                }),
                vec![],
            );
        }
    };

    // Parse the signing block to extract signer entries.
    // 解析签名块以提取签名者条目。
    let signer_entries = match ucx_sign::layer2::parse_signing_block(&block_result) {
        Ok(entries) => entries,
        Err(e) => {
            return (
                Some(Layer2Result {
                    valid: false,
                    details: format!("failed to parse signing block: {e}"),
                }),
                vec![],
            );
        }
    };

    if signer_entries.is_empty() {
        return (
            Some(Layer2Result {
                valid: false,
                details: "signing block contains no signer entries".to_string(),
            }),
            vec![],
        );
    }

    // Recompute the protected content digest.
    // 重新计算受保护内容摘要。
    //
    // To recompute the digest, we need the ZIP data *without* the signing block.
    // The signing block sits between entries and CD. We need to reconstruct
    // the original ZIP layout: entries + CD + EOCD.
    // 要重新计算摘要，需要*不含*签名块的 ZIP 数据。
    // 签名块位于条目和 CD 之间。需要重建原始 ZIP 布局：条目 + CD + EOCD。
    let recomputed_digest = match recompute_protected_digest(file_data) {
        Ok(digest) => digest,
        Err(e) => {
            return (
                Some(Layer2Result {
                    valid: false,
                    details: format!("failed to recompute protected digest: {e}"),
                }),
                vec![],
            );
        }
    };

    // Verify each signer entry's signature.
    // 验证每个签名者条目的签名。
    let mut all_valid = true;
    let mut details_parts = Vec::new();
    let mut layer2_signers = Vec::new();

    for (i, entry) in signer_entries.iter().enumerate() {
        // Check that the stored digest matches the recomputed one.
        // 检查存储的摘要与重新计算的是否一致。
        let digest_matches = entry.digest == recomputed_digest;

        // Verify the Ed25519 signature over the signed_data.
        // 验证 signed_data 上的 Ed25519 签名。
        let sig_valid = verify_layer2_signature(entry);

        let entry_valid = digest_matches && sig_valid;
        if !entry_valid {
            all_valid = false;
        }

        // Extract certificate info for this signer.
        // 提取此签名者的证书信息。
        let cert_cn = ucx_sign::cert::cert_subject_cn(&entry.cert_der)
            .unwrap_or_else(|_| "<unknown>".to_string());
        let cert_fingerprint = ucx_sign::cert::cert_fingerprint_blake3(&entry.cert_der);

        layer2_signers.push(Layer2SignerInfo {
            cert_cn,
            fingerprint_blake3: cert_fingerprint,
            cert_der: entry.cert_der.clone(),
            valid: entry_valid,
        });

        details_parts.push(format!(
            "signer[{i}]: digest_match={digest_matches}, sig_valid={sig_valid}"
        ));
    }

    let details = format!(
        "{} signer(s) in signing block; {}",
        signer_entries.len(),
        details_parts.join("; ")
    );

    (
        Some(Layer2Result {
            valid: all_valid,
            details,
        }),
        layer2_signers,
    )
}

/// Recompute the protected content digest for Layer 2 verification.
///
/// Strips the signing block from the file data to reconstruct the original
/// ZIP layout, then computes the digest over the protected content.
///
/// 为 Layer 2 验证重新计算受保护内容摘要。
/// 从文件数据中剥离签名块以重建原始 ZIP 布局，
/// 然后计算受保护内容的摘要。
fn recompute_protected_digest(file_data: &[u8]) -> Result<Vec<u8>, String> {
    // Find the EOCD in the signed data.
    // 在签名数据中查找 EOCD。
    let eocd_offset = ucx_sign::zip_binary::find_eocd(file_data)
        .map_err(|e| format!("failed to find EOCD: {e}"))?;
    let cd_offset_in_signed = ucx_sign::zip_binary::get_cd_offset(file_data, eocd_offset)
        .map_err(|e| format!("failed to get CD offset: {e}"))? as usize;

    // The signing block sits between the entries and the CD.
    // Find the signing block to determine its boundaries.
    // 签名块位于条目和 CD 之间。
    // 查找签名块以确定其边界。
    let (block_data, block_offset) = ucx_sign::zip_binary::find_signing_block(file_data)
        .map_err(|e| format!("failed to find signing block: {e}"))?
        .ok_or("no signing block found")?;

    let block_size = block_data.len();

    // Reconstruct the original ZIP data (without the signing block):
    //   entries (0..block_offset) + CD+EOCD (cd_offset_in_signed..end)
    // 重建原始 ZIP 数据（不含签名块）：
    //   条目 (0..block_offset) + CD+EOCD (cd_offset_in_signed..end)
    let entries = &file_data[..block_offset];
    let cd_and_eocd = &file_data[cd_offset_in_signed..];

    let mut original_zip = Vec::with_capacity(entries.len() + cd_and_eocd.len());
    original_zip.extend_from_slice(entries);
    original_zip.extend_from_slice(cd_and_eocd);

    // Fix the EOCD's CD offset in the reconstructed data.
    // 修正重建数据中 EOCD 的 CD 偏移。
    let new_eocd_offset = eocd_offset - block_size;
    let new_cd_offset = (cd_offset_in_signed - block_size) as u32;
    ucx_sign::zip_binary::set_cd_offset(&mut original_zip, new_eocd_offset, new_cd_offset);

    // Compute the protected content digest on the reconstructed ZIP.
    // 在重建的 ZIP 上计算受保护内容摘要。
    let cd_offset_original = new_cd_offset as usize;
    let digest = ucx_sign::layer2::compute_protected_digest(
        &original_zip,
        cd_offset_original,
        new_eocd_offset,
    )
    .map_err(|e| format!("failed to compute protected digest: {e}"))?;

    Ok(digest.to_vec())
}

/// Verify a single Layer 2 signer entry's Ed25519 signature.
///
/// Reconstructs the signed_data from the entry and verifies the signature
/// using the public key embedded in the entry.
///
/// 验证单个 Layer 2 签名者条目的 Ed25519 签名。
/// 从条目中重建 signed_data 并使用条目中嵌入的公钥验证签名。
fn verify_layer2_signature(entry: &ucx_sign::layer2::SignerEntry) -> bool {
    // Reconstruct signed_data: algorithm_id(4) + digest(32) + cert_length(4) + cert_der
    // 重建 signed_data：algorithm_id(4) + digest(32) + cert_length(4) + cert_der
    let mut signed_data = Vec::new();
    signed_data.extend_from_slice(&entry.algorithm_id.to_le_bytes());
    signed_data.extend_from_slice(&entry.digest);
    signed_data.extend_from_slice(&(entry.cert_der.len() as u32).to_le_bytes());
    signed_data.extend_from_slice(&entry.cert_der);

    // Parse the public key.
    // 解析公钥。
    let pk_bytes: [u8; 32] = match entry.public_key.as_slice().try_into() {
        Ok(b) => b,
        Err(_) => return false,
    };
    let verifying_key = match ed25519_dalek::VerifyingKey::from_bytes(&pk_bytes) {
        Ok(k) => k,
        Err(_) => return false,
    };

    // Parse the signature.
    // 解析签名。
    let sig_bytes: [u8; 64] = match entry.signature.as_slice().try_into() {
        Ok(b) => b,
        Err(_) => return false,
    };
    let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);

    // Verify.
    // 验证。
    use ed25519_dalek::Verifier;
    verifying_key.verify(&signed_data, &signature).is_ok()
}

/// Internal Layer 2 signer info used for merging.
///
/// 用于合并的内部 Layer 2 签名者信息。
struct Layer2SignerInfo {
    /// Subject CN from the certificate.
    /// 证书中的主体 CN。
    cert_cn: String,

    /// BLAKE3 fingerprint of the certificate.
    /// 证书的 BLAKE3 指纹。
    fingerprint_blake3: String,

    /// DER-encoded certificate bytes.
    /// DER 编码的证书字节。
    cert_der: Vec<u8>,

    /// Whether this signer's Layer 2 verification passed.
    /// 此签名者的 Layer 2 验证是否通过。
    valid: bool,
}

// =============================================================================
// Layer 1 verification / Layer 1 验证
// =============================================================================

/// Verify Layer 1 (JAR-style SF/EC signatures).
///
/// Iterates over all `META-INF/signatures/*.SF` entries, finds the matching
/// `.EC` file, reads MANIFEST.MF, and verifies both the SF digest and the
/// EC signature.
///
/// 验证 Layer 1（JAR 式 SF/EC 签名）。
/// 遍历所有 `META-INF/signatures/*.SF` 条目，查找匹配的 `.EC` 文件，
/// 读取 MANIFEST.MF，并验证 SF 摘要和 EC 签名。
fn verify_layer1(file_data: &[u8]) -> (Option<Layer1Result>, Vec<Layer1SignerInfo>) {
    let cursor = std::io::Cursor::new(file_data);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(_) => return (None, vec![]),
    };

    // Collect all file names in the archive.
    // 收集归档中的所有文件名。
    let file_names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .collect();

    // Find all SF files under META-INF/signatures/.
    // 查找 META-INF/signatures/ 下的所有 SF 文件。
    let sf_files: Vec<String> = file_names
        .iter()
        .filter(|name| {
            name.starts_with("META-INF/signatures/") && name.ends_with(".SF")
        })
        .cloned()
        .collect();

    if sf_files.is_empty() {
        // No Layer 1 signatures found.
        // 未找到 Layer 1 签名。
        return (None, vec![]);
    }

    // Read MANIFEST.MF content (needed for all signers).
    // 读取 MANIFEST.MF 内容（所有签名者都需要）。
    let manifest_content = match read_zip_entry(&mut archive, "META-INF/MANIFEST.MF") {
        Ok(content) => content,
        Err(e) => {
            return (
                Some(Layer1Result {
                    valid: false,
                    signer_count: sf_files.len(),
                    details: format!("failed to read MANIFEST.MF: {e}"),
                }),
                vec![],
            );
        }
    };

    // Verify each signer.
    // 验证每个签名者。
    let mut all_valid = true;
    let mut details_parts = Vec::new();
    let mut layer1_signers = Vec::new();

    for sf_path in &sf_files {
        // Extract signer_id from the filename stem.
        // 从文件名词干提取 signer_id。
        // e.g., "META-INF/signatures/AUTHOR.SF" -> "AUTHOR"
        let signer_id = sf_path
            .strip_prefix("META-INF/signatures/")
            .unwrap_or(sf_path)
            .strip_suffix(".SF")
            .unwrap_or(sf_path)
            .to_string();

        // Construct the expected EC path.
        // 构建预期的 EC 路径。
        let ec_path = format!("META-INF/signatures/{signer_id}.EC");

        // Read SF content.
        // 读取 SF 内容。
        let sf_content = match read_zip_entry(&mut archive, sf_path) {
            Ok(c) => c,
            Err(e) => {
                all_valid = false;
                details_parts.push(format!("{signer_id}: failed to read SF: {e}"));
                continue;
            }
        };

        // Read EC content.
        // 读取 EC 内容。
        let ec_content = match read_zip_entry(&mut archive, &ec_path) {
            Ok(c) => c,
            Err(e) => {
                all_valid = false;
                details_parts.push(format!("{signer_id}: failed to read EC: {e}"));
                continue;
            }
        };

        // Parse the SF to get the manifest digest.
        // 解析 SF 以获取 manifest 摘要。
        let sf_text = match std::str::from_utf8(&sf_content) {
            Ok(s) => s,
            Err(e) => {
                all_valid = false;
                details_parts.push(format!("{signer_id}: SF is not valid UTF-8: {e}"));
                continue;
            }
        };

        let sf_data = match ucx_sign::layer1::parse_sf(sf_text) {
            Ok(d) => d,
            Err(e) => {
                all_valid = false;
                details_parts.push(format!("{signer_id}: failed to parse SF: {e}"));
                continue;
            }
        };

        // Verify the manifest digest in the SF matches the actual MANIFEST.MF hash.
        // 验证 SF 中的 manifest 摘要与实际 MANIFEST.MF 哈希一致。
        let actual_manifest_hash = blake3::hash(&manifest_content);
        let actual_manifest_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, actual_manifest_hash.as_bytes());
        let digest_matches = sf_data.manifest_digest == actual_manifest_b64;

        // Verify the EC signature against the SF content.
        // 验证 EC 签名与 SF 内容一致。
        let verify_result = ucx_sign::layer1::verify_sf_signature(&sf_content, &ec_content);
        let (sig_valid, cert_der) = match verify_result {
            Ok(r) => (r.valid, r.cert_der),
            Err(e) => {
                all_valid = false;
                details_parts.push(format!(
                    "{signer_id}: EC verification error: {e}"
                ));
                continue;
            }
        };

        let entry_valid = digest_matches && sig_valid;
        if !entry_valid {
            all_valid = false;
        }

        // Extract certificate information.
        // 提取证书信息。
        let cert_cn = ucx_sign::cert::cert_subject_cn(&cert_der)
            .unwrap_or_else(|_| "<unknown>".to_string());
        let cert_fingerprint = ucx_sign::cert::cert_fingerprint_blake3(&cert_der);

        // Determine certificate type (self-signed vs CA-issued).
        // 判断证书类型（自签名 vs CA 签发）。
        let cert_type = determine_cert_type(&cert_der);

        layer1_signers.push(Layer1SignerInfo {
            signer_id: signer_id.clone(),
            cert_cn,
            cert_type,
            fingerprint_blake3: cert_fingerprint,
            valid: entry_valid,
        });

        details_parts.push(format!(
            "{signer_id}: digest_match={digest_matches}, sig_valid={sig_valid}"
        ));
    }

    let details = format!(
        "{} signer(s) found; {}",
        sf_files.len(),
        details_parts.join("; ")
    );

    (
        Some(Layer1Result {
            valid: all_valid,
            signer_count: sf_files.len(),
            details,
        }),
        layer1_signers,
    )
}

/// Internal Layer 1 signer info used for merging.
///
/// 用于合并的内部 Layer 1 签名者信息。
struct Layer1SignerInfo {
    /// Signer identifier (e.g., "AUTHOR").
    /// 签名者标识（如 "AUTHOR"）。
    signer_id: String,

    /// Subject CN from the certificate.
    /// 证书中的主体 CN。
    cert_cn: String,

    /// Certificate type: "self-signed" or "CA-issued".
    /// 证书类型："self-signed" 或 "CA-issued"。
    cert_type: String,

    /// BLAKE3 fingerprint of the certificate.
    /// 证书的 BLAKE3 指纹。
    fingerprint_blake3: String,

    /// Whether this signer's Layer 1 verification passed.
    /// 此签名者的 Layer 1 验证是否通过。
    valid: bool,
}

// =============================================================================
// Merging / 合并
// =============================================================================

/// Merge Layer 1 and Layer 2 signer information into a single list.
///
/// Matches signers by their certificate BLAKE3 fingerprint. Layer 1 signers
/// have `signer_id` from the filename; Layer 2 signers are matched if their
/// certificate fingerprint matches a Layer 1 signer.
///
/// 将 Layer 1 和 Layer 2 的签名者信息合并为一个列表。
/// 通过证书 BLAKE3 指纹匹配签名者。Layer 1 签名者的 `signer_id` 来自文件名；
/// 如果 Layer 2 签名者的证书指纹与某个 Layer 1 签名者匹配，则关联。
fn merge_signer_info(
    layer1_signers: &[Layer1SignerInfo],
    layer2_signers: &[Layer2SignerInfo],
) -> Vec<SignerInfo> {
    let mut result = Vec::new();

    // Start with Layer 1 signers as the base.
    // 以 Layer 1 签名者为基础。
    for l1 in layer1_signers {
        // Try to find a matching Layer 2 signer by fingerprint.
        // 尝试通过指纹查找匹配的 Layer 2 签名者。
        let l2_match = layer2_signers
            .iter()
            .find(|l2| l2.fingerprint_blake3 == l1.fingerprint_blake3);

        result.push(SignerInfo {
            signer_id: l1.signer_id.clone(),
            subject_cn: l1.cert_cn.clone(),
            cert_type: l1.cert_type.clone(),
            fingerprint_blake3: l1.fingerprint_blake3.clone(),
            layer1_valid: l1.valid,
            layer2_valid: l2_match.is_some_and(|l2| l2.valid),
        });
    }

    // Add any Layer 2 signers that were not matched with Layer 1.
    // 添加未与 Layer 1 匹配的 Layer 2 签名者。
    for l2 in layer2_signers {
        let already_merged = result
            .iter()
            .any(|s| s.fingerprint_blake3 == l2.fingerprint_blake3);
        if !already_merged {
            result.push(SignerInfo {
                signer_id: "<layer2-only>".to_string(),
                subject_cn: l2.cert_cn.clone(),
                cert_type: determine_cert_type(&l2.cert_der),
                fingerprint_blake3: l2.fingerprint_blake3.clone(),
                layer1_valid: false,
                layer2_valid: l2.valid,
            });
        }
    }

    result
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Read a single entry from a ZIP archive by name.
///
/// 按名称从 ZIP 归档中读取单个条目。
fn read_zip_entry(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    name: &str,
) -> Result<Vec<u8>, String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|e| format!("entry '{name}' not found: {e}"))?;

    let mut content = Vec::with_capacity(entry.size() as usize);
    entry
        .read_to_end(&mut content)
        .map_err(|e| format!("failed to read entry '{name}': {e}"))?;

    Ok(content)
}

/// Determine whether a certificate is self-signed or CA-issued.
///
/// A simple heuristic: if the issuer and subject have the same CN, it's
/// considered self-signed. Otherwise it's CA-issued.
///
/// 判断证书是自签名还是 CA 签发。
/// 简单启发式：如果颁发者和主体有相同的 CN，则视为自签名。
fn determine_cert_type(cert_der: &[u8]) -> String {
    use der::Decode;

    let cert = match x509_cert::Certificate::from_der(cert_der) {
        Ok(c) => c,
        Err(_) => return "unknown".to_string(),
    };

    // Extract subject CN.
    // 提取主体 CN。
    let cn_oid = der::oid::ObjectIdentifier::new_unwrap("2.5.4.3");
    let subject_cn = extract_cn_from_name(&cert.tbs_certificate.subject, &cn_oid);
    let issuer_cn = extract_cn_from_name(&cert.tbs_certificate.issuer, &cn_oid);

    if subject_cn == issuer_cn {
        "self-signed".to_string()
    } else {
        "CA-issued".to_string()
    }
}

/// Extract the CN value from an X.500 Name.
///
/// 从 X.500 Name 中提取 CN 值。
fn extract_cn_from_name(
    name: &x509_cert::name::Name,
    cn_oid: &der::oid::ObjectIdentifier,
) -> Option<String> {
    for rdn in name.0.iter() {
        for atv in rdn.0.iter() {
            if atv.oid == *cn_oid
                && let Ok(s) = std::str::from_utf8(atv.value.value())
            {
                return Some(s.to_string());
            }
        }
    }
    None
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: VerifyStatus enum comparison works correctly.
    /// 测试：VerifyStatus 枚举比较正常工作。
    #[test]
    fn verify_status_equality() {
        assert_eq!(VerifyStatus::Valid, VerifyStatus::Valid);
        assert_ne!(VerifyStatus::Valid, VerifyStatus::Invalid);
        assert_ne!(VerifyStatus::Unsigned, VerifyStatus::ValidWithWarnings);
    }

    /// Helper: create a UCX project, build it, and return the .ucx file path.
    /// 辅助函数：创建 UCX 项目、构建，并返回 .ucx 文件路径。
    fn create_test_ucx(tmp: &tempfile::TempDir) -> std::path::PathBuf {
        let project_dir = tmp.path().join("verify-test-project");

        // Initialize a minimal UCX project.
        // 初始化最小 UCX 项目。
        let init_options = ucx_init::InitOptions {
            name: "验证测试".to_string(),
            author: "测试作者".to_string(),
            language: "zh-CN".to_string(),
            no_git: true,
            ..Default::default()
        };
        ucx_init::init(&project_dir, &init_options).expect("init should succeed");

        // Build the UCX file.
        // 构建 UCX 文件。
        let build_options = ucx_build::BuildOptions::default();
        ucx_build::build(&project_dir, &build_options).expect("build should succeed")
    }

    /// Helper: sign a UCX file and return key/cert paths.
    /// 辅助函数：签名 UCX 文件并返回密钥/证书路径。
    fn sign_test_ucx(
        tmp: &tempfile::TempDir,
        ucx_path: &std::path::Path,
    ) -> (std::path::PathBuf, std::path::PathBuf) {
        let key_path = tmp.path().join("signer.pem");
        let cert_path = tmp.path().join("signer.cert.pem");

        // Generate key pair and certificate.
        // 生成密钥对和证书。
        ucx_sign::keygen(&key_path).expect("keygen should succeed");
        ucx_sign::create_cert(&key_path, "Test Verify Signer", 365, &cert_path)
            .expect("create_cert should succeed");

        // Sign the UCX file.
        // 签名 UCX 文件。
        ucx_sign::sign(ucx_path, &key_path, &cert_path, "AUTHOR")
            .expect("sign should succeed");

        (key_path, cert_path)
    }

    /// Test 1: Unsigned file returns UNSIGNED status.
    /// 测试 1：未签名文件返回 UNSIGNED 状态。
    #[test]
    fn test_unsigned_file_returns_unsigned() {
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        let ucx_path = create_test_ucx(&tmp);

        let report = verify(&ucx_path).expect("verify should succeed");

        assert_eq!(
            report.status,
            VerifyStatus::Unsigned,
            "unsigned file must have Unsigned status"
        );
        assert!(report.layer1.is_none(), "no Layer 1 result for unsigned file");
        assert!(report.layer2.is_none(), "no Layer 2 result for unsigned file");
        assert!(report.signers.is_empty(), "no signers for unsigned file");
    }

    /// Test 2: Signed file returns VERIFIED (Valid) status.
    /// 测试 2：签名文件返回 VERIFIED (Valid) 状态。
    #[test]
    fn test_signed_file_returns_valid() {
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        let ucx_path = create_test_ucx(&tmp);
        sign_test_ucx(&tmp, &ucx_path);

        let report = verify(&ucx_path).expect("verify should succeed");

        assert_eq!(
            report.status,
            VerifyStatus::Valid,
            "signed file must have Valid status"
        );
        assert!(
            report.layer1.as_ref().is_some_and(|r| r.valid),
            "Layer 1 must be valid"
        );
        assert!(
            report.layer2.as_ref().is_some_and(|r| r.valid),
            "Layer 2 must be valid"
        );
        assert!(!report.signers.is_empty(), "must have at least one signer");
    }

    /// Test 3: Tampered content returns INVALID status.
    /// 测试 3：篡改内容后返回 INVALID 状态。
    #[test]
    fn test_tampered_content_returns_invalid() {
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        let ucx_path = create_test_ucx(&tmp);
        sign_test_ucx(&tmp, &ucx_path);

        // Read the signed file and tamper a byte in the middle.
        // 读取签名文件并在中间篡改一个字节。
        let mut data = std::fs::read(&ucx_path).expect("read should succeed");
        let mid = data.len() / 2;
        data[mid] ^= 0xFF; // Flip all bits of one byte. / 翻转一个字节的所有位。
        std::fs::write(&ucx_path, &data).expect("write should succeed");

        let result = verify(&ucx_path);

        // After tampering, verify should either return Invalid status
        // or return an error (if the ZIP is corrupted beyond repair).
        // 篡改后，verify 应返回 Invalid 状态或返回错误（如果 ZIP 损坏到无法修复）。
        match result {
            Ok(report) => {
                assert_ne!(
                    report.status,
                    VerifyStatus::Valid,
                    "tampered file must NOT have Valid status"
                );
            }
            Err(_) => {
                // An error (e.g., InvalidFile) is also acceptable for tampered data.
                // 对于篡改数据，错误（如 InvalidFile）也是可接受的。
            }
        }
    }

    /// Test 4: Signer info is correct.
    /// 测试 4：签名者信息正确。
    #[test]
    fn test_signer_info_is_correct() {
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        let ucx_path = create_test_ucx(&tmp);
        sign_test_ucx(&tmp, &ucx_path);

        let report = verify(&ucx_path).expect("verify should succeed");

        assert_eq!(report.signers.len(), 1, "must have exactly 1 signer");
        let signer = &report.signers[0];

        assert_eq!(
            signer.signer_id, "AUTHOR",
            "signer_id must be AUTHOR"
        );
        assert_eq!(
            signer.subject_cn, "Test Verify Signer",
            "subject_cn must match the certificate CN"
        );
        assert_eq!(
            signer.cert_type, "self-signed",
            "cert_type must be self-signed"
        );
        assert!(
            !signer.fingerprint_blake3.is_empty(),
            "fingerprint must not be empty"
        );
        assert_eq!(
            signer.fingerprint_blake3.len(),
            64,
            "BLAKE3 fingerprint must be 64 hex chars"
        );
        assert!(signer.layer1_valid, "Layer 1 must be valid for this signer");
        assert!(signer.layer2_valid, "Layer 2 must be valid for this signer");
    }

    /// Test 5: Multiple verification calls are consistent (idempotent).
    /// 测试 5：多次验证调用结果一致（幂等）。
    #[test]
    fn test_multiple_verify_calls_consistent() {
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        let ucx_path = create_test_ucx(&tmp);
        sign_test_ucx(&tmp, &ucx_path);

        let report1 = verify(&ucx_path).expect("first verify should succeed");
        let report2 = verify(&ucx_path).expect("second verify should succeed");

        assert_eq!(
            report1.status, report2.status,
            "verification status must be consistent across calls"
        );
        assert_eq!(
            report1.signers.len(),
            report2.signers.len(),
            "signer count must be consistent"
        );
        assert_eq!(
            report1.signers[0].fingerprint_blake3,
            report2.signers[0].fingerprint_blake3,
            "signer fingerprint must be consistent"
        );
    }

    /// Test 6: Non-UCX file returns VerifyError.
    /// 测试 6：非 UCX 文件返回 VerifyError。
    #[test]
    fn test_non_ucx_file_returns_error() {
        let tmp = tempfile::TempDir::new().expect("failed to create temp dir");
        let plain_file = tmp.path().join("not-a-ucx.txt");
        std::fs::write(&plain_file, b"this is just a plain text file")
            .expect("write should succeed");

        let result = verify(&plain_file);
        assert!(result.is_err(), "non-UCX file must return an error");

        match result.unwrap_err() {
            VerifyError::InvalidFile(_) => { /* expected */ }
            other => panic!(
                "expected VerifyError::InvalidFile, got: {other:?}"
            ),
        }
    }
}
