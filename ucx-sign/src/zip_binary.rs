//! Low-Level ZIP Binary Operations for UCX Signing Block.
//!
//! This module provides functions for manipulating ZIP archives at the binary
//! level, specifically for inserting and extracting UCX Signing Blocks.
//!
//! The UCX Signing Block is placed between the ZIP entries (local file data)
//! and the Central Directory, similar to the APK Signing Block in Android.
//!
//! UCX 签名块的底层 ZIP 二进制操作。
//!
//! 本模块提供在二进制层面操作 ZIP 归档的功能，
//! 专门用于插入和提取 UCX 签名块。
//!
//! UCX 签名块位于 ZIP 条目（本地文件数据）和中央目录之间，
//! 类似于 Android 的 APK 签名块。

use crate::SignError;

// =============================================================================
// Constants / 常量
// =============================================================================

/// Magic bytes for the UCX Signing Block.
/// UCX 签名块的 magic 字节。
pub const UCX_SIGNING_BLOCK_MAGIC: &[u8; 16] = b"UCX Sig Block 1\0";

/// Pair ID for UCX Signature Scheme v1.
/// UCX 签名方案 v1 的 Pair ID。
pub const PAIR_ID_UCX_SIG_V1: u32 = 0x55435801;

/// ZIP End of Central Directory signature bytes: PK\x05\x06.
/// ZIP 中央目录结束记录签名字节：PK\x05\x06。
const EOCD_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x05, 0x06];

/// Minimum size of an EOCD record (without comment).
/// EOCD 记录的最小大小（不含注释）。
const EOCD_MIN_SIZE: usize = 22;

/// Maximum size of an EOCD record (with maximum-length comment of 65535 bytes).
/// EOCD 记录的最大大小（含最大长度 65535 字节的注释）。
const EOCD_MAX_SIZE: usize = 22 + 65535;

// =============================================================================
// EOCD operations / EOCD 操作
// =============================================================================

/// Find the End of Central Directory record in a ZIP file.
/// Returns the offset of the EOCD record from the start of the file.
///
/// The search starts from the end of the file and scans backwards, because
/// the EOCD is always at the end and may contain a variable-length comment.
///
/// 在 ZIP 文件中定位中央目录结束记录（EOCD）。
/// 返回 EOCD 记录相对于文件起始的偏移量。
///
/// 搜索从文件末尾开始向前扫描，因为 EOCD 总是位于文件末尾，
/// 且可能包含变长注释。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if:
/// - The data is too small to contain a valid ZIP file
/// - No EOCD signature is found
///
/// 在以下情况返回 `SignError::SigningFailed`：
/// - 数据太小，无法构成有效的 ZIP 文件
/// - 未找到 EOCD 签名
pub fn find_eocd(data: &[u8]) -> Result<usize, SignError> {
    // A valid ZIP file must be at least EOCD_MIN_SIZE bytes.
    // 有效的 ZIP 文件至少需要 EOCD_MIN_SIZE 字节。
    if data.len() < EOCD_MIN_SIZE {
        return Err(SignError::SigningFailed(
            "data too small to contain a valid ZIP EOCD record".to_string(),
        ));
    }

    // Calculate the maximum distance to search back from the end.
    // 计算从文件末尾向前搜索的最大距离。
    let search_limit = data.len().min(EOCD_MAX_SIZE);

    // Scan backwards from the end of the file looking for EOCD_SIGNATURE.
    // 从文件末尾向前扫描，寻找 EOCD_SIGNATURE。
    for offset in (data.len() - search_limit)..=(data.len() - EOCD_MIN_SIZE) {
        if data[offset..offset + 4] == EOCD_SIGNATURE {
            // Verify: the comment length field at offset+20 must be consistent
            // with the remaining data after the EOCD header.
            // 验证：偏移 +20 处的注释长度字段必须与 EOCD 头部之后的剩余数据一致。
            let comment_len =
                u16::from_le_bytes([data[offset + 20], data[offset + 21]]) as usize;
            if offset + EOCD_MIN_SIZE + comment_len == data.len() {
                return Ok(offset);
            }
        }
    }

    Err(SignError::SigningFailed(
        "EOCD signature not found in ZIP data".to_string(),
    ))
}

/// Read the Central Directory offset from the EOCD record.
///
/// EOCD layout (22 bytes minimum):
///   signature(4) + disk_number(2) + disk_cd(2) + entries_disk(2)
///   + entries_total(2) + cd_size(4) + cd_offset(4) + comment_len(2)
///
/// The Central Directory offset is at bytes 16..20 from the EOCD start.
///
/// 从 EOCD 记录中读取中央目录偏移。
///
/// EOCD 布局（最小 22 字节）：
///   签名(4) + 磁盘号(2) + CD 磁盘号(2) + 本盘条目数(2)
///   + 总条目数(2) + CD 大小(4) + CD 偏移(4) + 注释长度(2)
///
/// 中央目录偏移位于 EOCD 起始的第 16..20 字节。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if the data slice is too short.
/// 数据切片过短时返回 `SignError::SigningFailed`。
pub fn get_cd_offset(data: &[u8], eocd_offset: usize) -> Result<u32, SignError> {
    // Ensure there are enough bytes to read the CD offset field.
    // 确保有足够的字节来读取 CD 偏移字段。
    if data.len() < eocd_offset + 20 {
        return Err(SignError::SigningFailed(
            "data too short to read CD offset from EOCD".to_string(),
        ));
    }

    // Read the 4-byte little-endian CD offset at EOCD + 16.
    // 在 EOCD + 16 处读取 4 字节小端序 CD 偏移。
    let offset_bytes = &data[eocd_offset + 16..eocd_offset + 20];
    Ok(u32::from_le_bytes([
        offset_bytes[0],
        offset_bytes[1],
        offset_bytes[2],
        offset_bytes[3],
    ]))
}

/// Set the Central Directory offset in the EOCD record.
///
/// Writes a new 4-byte little-endian value at EOCD + 16.
///
/// 设置 EOCD 记录中的中央目录偏移。
/// 在 EOCD + 16 处写入新的 4 字节小端序值。
///
/// # Panics
///
/// Panics if the data slice is too short (caller must ensure valid offset).
/// 数据切片过短时 panic（调用方必须确保偏移有效）。
pub fn set_cd_offset(data: &mut [u8], eocd_offset: usize, new_offset: u32) {
    let bytes = new_offset.to_le_bytes();
    data[eocd_offset + 16..eocd_offset + 20].copy_from_slice(&bytes);
}

/// Read the Central Directory size from EOCD.
///
/// The CD size is at bytes 12..16 from the EOCD start.
///
/// 从 EOCD 读取中央目录大小。
/// CD 大小位于 EOCD 起始的第 12..16 字节。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if the data slice is too short.
/// 数据切片过短时返回 `SignError::SigningFailed`。
pub fn get_cd_size(data: &[u8], eocd_offset: usize) -> Result<u32, SignError> {
    // Ensure there are enough bytes to read the CD size field.
    // 确保有足够的字节来读取 CD 大小字段。
    if data.len() < eocd_offset + 16 {
        return Err(SignError::SigningFailed(
            "data too short to read CD size from EOCD".to_string(),
        ));
    }

    // Read the 4-byte little-endian CD size at EOCD + 12.
    // 在 EOCD + 12 处读取 4 字节小端序 CD 大小。
    let size_bytes = &data[eocd_offset + 12..eocd_offset + 16];
    Ok(u32::from_le_bytes([
        size_bytes[0],
        size_bytes[1],
        size_bytes[2],
        size_bytes[3],
    ]))
}

// =============================================================================
// Signing block insertion / 签名块插入
// =============================================================================

/// Insert the UCX Signing Block between ZIP entries and Central Directory.
/// Returns the new file data with the signing block inserted and offsets corrected.
///
/// The insertion process:
/// 1. Find EOCD -> get Central Directory offset
/// 2. Split file at CD offset: entries_data | cd_and_eocd
/// 3. Insert signing block between entries and CD
/// 4. Update EOCD's CD offset (add signing_block.len())
///
/// Note: Local header offsets stored in CD entries do NOT need updating because
/// the signing block is inserted AFTER all local file data. The local headers
/// remain at the same absolute positions.
///
/// 在 ZIP 条目和中央目录之间插入 UCX 签名块。
/// 返回插入签名块并修正偏移后的新文件数据。
///
/// 插入过程：
/// 1. 查找 EOCD -> 获取中央目录偏移
/// 2. 在 CD 偏移处分割文件：条目数据 | CD 和 EOCD
/// 3. 在条目和 CD 之间插入签名块
/// 4. 更新 EOCD 的 CD 偏移（加上 signing_block.len()）
///
/// 注意：CD 条目中存储的本地头偏移不需要更新，因为签名块是在所有
/// 本地文件数据之后插入的。本地头保持在相同的绝对位置。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if:
/// - EOCD is not found
/// - CD offset is invalid
///
/// 在以下情况返回 `SignError::SigningFailed`：
/// - 未找到 EOCD
/// - CD 偏移无效
pub fn insert_signing_block(
    zip_data: &[u8],
    signing_block: &[u8],
) -> Result<Vec<u8>, SignError> {
    // Step 1: Find EOCD and get the CD offset.
    // 步骤 1：查找 EOCD 并获取 CD 偏移。
    let eocd_offset = find_eocd(zip_data)?;
    let cd_offset = get_cd_offset(zip_data, eocd_offset)? as usize;

    // Validate that the CD offset is within bounds.
    // 验证 CD 偏移在合法范围内。
    if cd_offset > zip_data.len() {
        return Err(SignError::SigningFailed(format!(
            "CD offset ({cd_offset}) exceeds file size ({})",
            zip_data.len()
        )));
    }

    // Step 2: Split the file at the CD offset.
    // 步骤 2：在 CD 偏移处分割文件。
    let entries_data = &zip_data[..cd_offset];
    let cd_and_eocd = &zip_data[cd_offset..];

    // Step 3: Build the new file: entries + signing_block + cd_and_eocd.
    // 步骤 3：构建新文件：条目 + 签名块 + CD 和 EOCD。
    let new_size = entries_data.len() + signing_block.len() + cd_and_eocd.len();
    let mut result = Vec::with_capacity(new_size);
    result.extend_from_slice(entries_data);
    result.extend_from_slice(signing_block);
    result.extend_from_slice(cd_and_eocd);

    // Step 4: Fix the EOCD's CD offset in the new data.
    // 步骤 4：修正新数据中 EOCD 的 CD 偏移。
    //
    // The new EOCD position = original EOCD offset + signing block size.
    // 新 EOCD 位置 = 原始 EOCD 偏移 + 签名块大小。
    let new_eocd_offset = eocd_offset + signing_block.len();
    let new_cd_offset = (cd_offset + signing_block.len()) as u32;
    set_cd_offset(&mut result, new_eocd_offset, new_cd_offset);

    Ok(result)
}

// =============================================================================
// Signing block extraction / 签名块提取
// =============================================================================

/// Find and extract the UCX Signing Block from signed ZIP data.
/// Returns `(signing_block_data, block_offset)` or `None` if not found.
///
/// The signing block is located between the ZIP entries and the Central Directory.
/// It is identified by the 16-byte magic at its end.
///
/// The block layout is:
/// ```text
/// [size_of_block: u64 LE]  <-- at block_offset
/// [pairs...]
/// [size_of_block: u64 LE]  <-- repeated before magic
/// [magic: 16 bytes]        <-- 16 bytes before CD start
/// ```
///
/// 从已签名的 ZIP 数据中查找并提取 UCX 签名块。
/// 返回 `(签名块数据, 块偏移)` 或 `None`（如果未找到）。
///
/// 签名块位于 ZIP 条目和中央目录之间，通过末尾的 16 字节 magic 来识别。
///
/// # Errors / 错误
///
/// Returns `SignError::SigningFailed` if the ZIP structure is invalid.
/// ZIP 结构无效时返回 `SignError::SigningFailed`。
pub fn find_signing_block(data: &[u8]) -> Result<Option<(Vec<u8>, usize)>, SignError> {
    // Step 1: Find EOCD and get the CD offset.
    // 步骤 1：查找 EOCD 并获取 CD 偏移。
    let eocd_offset = find_eocd(data)?;
    let cd_offset = get_cd_offset(data, eocd_offset)? as usize;

    // Step 2: Check if there is a magic marker just before the CD.
    // 步骤 2：检查 CD 之前是否有 magic 标记。
    //
    // The magic is the last 16 bytes of the signing block, placed
    // immediately before the Central Directory.
    // magic 是签名块的最后 16 字节，紧接在中央目录之前。
    if cd_offset < 16 + 8 {
        // Not enough room for magic + size_of_block.
        // 没有足够空间容纳 magic + size_of_block。
        return Ok(None);
    }

    let magic_start = cd_offset - 16;
    if &data[magic_start..cd_offset] != UCX_SIGNING_BLOCK_MAGIC {
        // No signing block magic found.
        // 未找到签名块 magic。
        return Ok(None);
    }

    // Step 3: Read the repeated size_of_block (u64 LE) just before the magic.
    // 步骤 3：读取 magic 之前的重复 size_of_block（u64 LE）。
    let size_end = magic_start;
    let size_start = size_end - 8;
    let block_size = u64::from_le_bytes([
        data[size_start],
        data[size_start + 1],
        data[size_start + 2],
        data[size_start + 3],
        data[size_start + 4],
        data[size_start + 5],
        data[size_start + 6],
        data[size_start + 7],
    ]) as usize;

    // Step 4: The block starts at: cd_offset - 8(size) - 16(magic) - block_size - 8(size_prefix).
    // Wait — let's re-derive the layout:
    //   [size_of_block: u64][pairs...][size_of_block: u64][magic: 16]
    // where size_of_block = len(pairs) + 8(trailing size) + 16(magic).
    //
    // So: block_start = cd_offset - 8 - block_size
    //     because the leading u64 stores the total: pairs_len + 8 + 16.
    //
    // 步骤 4：根据布局计算块起始位置。
    //   [size_of_block: u64][pairs...][size_of_block: u64][magic: 16]
    //   其中 size_of_block = pairs 长度 + 8(尾部大小) + 16(magic)。
    //   所以 block_start = cd_offset - 8 - block_size。
    if cd_offset < 8 + block_size {
        return Err(SignError::SigningFailed(
            "signing block size exceeds available data before Central Directory".to_string(),
        ));
    }

    let block_start = cd_offset - 8 - block_size;

    // Step 5: Read head size_of_block and verify it matches the tail value.
    // 步骤 5：读取头部 size_of_block 并验证与尾部一致。
    //
    // The first 8 bytes of the signing block are the leading size_of_block.
    // It must equal the trailing size_of_block for a well-formed block.
    // 签名块的前 8 个字节是开头的 size_of_block。
    // 对于格式正确的块，它必须与尾部的 size_of_block 相等。
    let head_offset = block_start;
    if head_offset + 8 > data.len() {
        return Err(SignError::SigningFailed(
            "signing block too small for head size_of_block".to_string(),
        ));
    }
    let head_size = u64::from_le_bytes([
        data[head_offset],
        data[head_offset + 1],
        data[head_offset + 2],
        data[head_offset + 3],
        data[head_offset + 4],
        data[head_offset + 5],
        data[head_offset + 6],
        data[head_offset + 7],
    ]) as usize;

    // The tail size was read in Step 3 as `block_size`.
    // 尾部大小在步骤 3 中以 `block_size` 读取。
    if head_size != block_size {
        return Err(SignError::SigningFailed(format!(
            "signing block size mismatch: head={head_size}, tail={block_size}"
        )));
    }

    // The total signing block bytes = from block_start to cd_offset.
    // 签名块总字节 = 从 block_start 到 cd_offset。
    let signing_block = data[block_start..cd_offset].to_vec();

    Ok(Some((signing_block, block_start)))
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: create a minimal ZIP file in memory using the zip crate.
    /// 辅助函数：使用 zip crate 在内存中创建一个最小的 ZIP 文件。
    fn create_test_zip() -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip_writer = zip::ZipWriter::new(&mut buf);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip_writer
                .start_file("hello.txt", options)
                .expect("start_file should succeed");
            zip_writer
                .write_all(b"Hello, Unicodex!")
                .expect("write_all should succeed");
            zip_writer.finish().expect("finish should succeed");
        }
        buf.into_inner()
    }

    /// Test: find_eocd on a real ZIP file created by the zip crate.
    /// 测试：在 zip crate 创建的真实 ZIP 文件上调用 find_eocd。
    #[test]
    fn test_find_eocd_on_real_zip() {
        let zip_data = create_test_zip();

        let eocd_offset = find_eocd(&zip_data).expect("find_eocd should succeed on valid ZIP");

        // The EOCD signature should be at the found offset.
        // 在找到的偏移处应有 EOCD 签名。
        assert_eq!(
            &zip_data[eocd_offset..eocd_offset + 4],
            &EOCD_SIGNATURE,
            "EOCD signature must be present at the found offset"
        );

        // CD offset should be valid (less than EOCD offset).
        // CD 偏移应有效（小于 EOCD 偏移）。
        let cd_offset = get_cd_offset(&zip_data, eocd_offset)
            .expect("get_cd_offset should succeed");
        assert!(
            (cd_offset as usize) <= eocd_offset,
            "CD offset must be <= EOCD offset"
        );
    }

    /// Test: insert_signing_block produces a valid ZIP that the zip crate can read.
    /// 测试：insert_signing_block 生成的 ZIP 仍可被 zip crate 正确读取。
    #[test]
    fn test_insert_signing_block_preserves_zip_validity() {
        let zip_data = create_test_zip();

        // Build a dummy signing block for testing.
        // 构建一个用于测试的虚拟签名块。
        let dummy_signers = b"test-signer-data";
        let signing_block = crate::layer2::build_signing_block(dummy_signers);

        // Insert the signing block.
        // 插入签名块。
        let signed_data = insert_signing_block(&zip_data, &signing_block)
            .expect("insert_signing_block should succeed");

        // The signed ZIP should be larger than the original.
        // 签名后的 ZIP 应比原始文件大。
        assert!(
            signed_data.len() > zip_data.len(),
            "signed data must be larger than original"
        );

        // The zip crate should still be able to read the archive.
        // zip crate 应仍能读取该归档。
        let cursor = std::io::Cursor::new(&signed_data);
        let mut archive = zip::ZipArchive::new(cursor)
            .expect("signed ZIP must be readable by zip crate");

        // The original file should still be accessible.
        // 原始文件应仍可访问。
        let mut file = archive
            .by_name("hello.txt")
            .expect("hello.txt must still exist in the archive");
        let mut contents = String::new();
        std::io::Read::read_to_string(&mut file, &mut contents)
            .expect("reading file contents should succeed");
        assert_eq!(
            contents, "Hello, Unicodex!",
            "file contents must be preserved after signing block insertion"
        );
    }

    /// Test: find_signing_block locates a previously inserted block.
    /// 测试：find_signing_block 能定位之前插入的签名块。
    #[test]
    fn test_find_signing_block_locates_inserted_block() {
        let zip_data = create_test_zip();

        // Build and insert a signing block.
        // 构建并插入签名块。
        let dummy_signers = b"find-me-signer-data";
        let signing_block = crate::layer2::build_signing_block(dummy_signers);
        let signed_data = insert_signing_block(&zip_data, &signing_block)
            .expect("insert_signing_block should succeed");

        // Find the signing block.
        // 查找签名块。
        let result = find_signing_block(&signed_data)
            .expect("find_signing_block should not error");
        assert!(result.is_some(), "signing block must be found in signed data");

        let (found_block, _block_offset) = result.unwrap();

        // The found block should match what we inserted.
        // 找到的块应与我们插入的一致。
        assert_eq!(
            found_block, signing_block,
            "found signing block must match the inserted one"
        );
    }

    /// Test: find_signing_block returns None for an unsigned ZIP.
    /// 测试：对未签名的 ZIP 调用 find_signing_block 应返回 None。
    #[test]
    fn test_find_signing_block_returns_none_for_unsigned_zip() {
        let zip_data = create_test_zip();

        let result = find_signing_block(&zip_data)
            .expect("find_signing_block should not error on unsigned ZIP");
        assert!(
            result.is_none(),
            "unsigned ZIP must not contain a signing block"
        );
    }
}
