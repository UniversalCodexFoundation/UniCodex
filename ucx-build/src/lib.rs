//! UCX Build & Pack Module.
//!
//! This module packages project directories into `.ucx` archive files.
//! It handles TOML-to-JSON metadata conversion, MANIFEST.MF generation,
//! ZIP archive creation, and optional auto-versioning/auto-signing.
//!
//! UCX 构建与打包模块。
//! 将项目目录打包为 `.ucx` 归档文件。
//! 处理 TOML→JSON 元数据转换、MANIFEST.MF 生成、
//! ZIP 归档创建，以及可选的自动版本/自动签名。

use thiserror::Error;

// =============================================================================
// Error types / 错误类型
// =============================================================================

/// Errors that can occur during UCX file building.
///
/// UCX 文件构建过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum BuildError {
    /// Project configuration file not found.
    /// 找不到项目配置文件。
    #[error("project configuration not found: {0}")]
    ConfigNotFound(String),

    /// Invalid project structure.
    /// 项目结构无效。
    #[error("invalid project structure: {0}")]
    InvalidStructure(String),

    /// I/O error during build.
    /// 构建过程中的 I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// ZIP archive creation error.
    /// ZIP 归档创建错误。
    #[error("ZIP archive error: {0}")]
    Zip(String),

    /// Metadata serialization error.
    /// 元数据序列化错误。
    #[error("serialization error: {0}")]
    Serialization(String),
}

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Build a UCX file from the project at the given path.
///
/// Reads `unicodex.toml`, converts metadata, collects content files,
/// generates MANIFEST.MF, and creates the `.ucx` ZIP archive.
///
/// 从指定路径的项目构建 UCX 文件。
/// 读取 `unicodex.toml`，转换元数据，收集内容文件，
/// 生成 MANIFEST.MF，并创建 `.ucx` ZIP 归档。
///
/// # Arguments / 参数
///
/// * `project_path` - Path to the project root directory.
///                    项目根目录路径。
/// * `output_path`  - Path for the output `.ucx` file.
///                    输出 `.ucx` 文件路径。
///
/// # Returns / 返回
///
/// Returns `Ok(())` on success, or a `BuildError` on failure.
/// 成功返回 `Ok(())`，失败返回 `BuildError`。
pub fn build(
    _project_path: &std::path::Path,
    _output_path: &std::path::Path,
) -> Result<(), BuildError> {
    // TODO: Implement UCX build.
    // TODO: 实现 UCX 构建。
    //
    // Steps / 步骤:
    // 1. Read and parse unicodex.toml.
    //    读取并解析 unicodex.toml。
    // 2. Convert TOML metadata to JSON (codex.json, struct.json).
    //    将 TOML 元数据转换为 JSON。
    // 3. Collect all content and asset files.
    //    收集所有内容和资源文件。
    // 4. Compute BLAKE3 hashes for all files.
    //    计算所有文件的 BLAKE3 哈希。
    // 5. Generate MANIFEST.MF.
    //    生成 MANIFEST.MF。
    // 6. Create ZIP archive with mimetype as first entry (uncompressed).
    //    创建 ZIP 归档（mimetype 为第一个条目，不压缩）。
    // 7. Optional: Run ucx-version auto.
    //    可选：执行自动版本生成。
    // 8. Optional: Run ucx-sign if configured.
    //    可选：按配置执行自动签名。
    todo!("ucx-build: build not yet implemented")
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {
        assert!(true);
    }
}
