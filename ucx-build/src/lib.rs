//! UCX Build & Pack Module.
//!
//! This module packages project directories into `.ucx` archive files.
//! It handles TOML-to-JSON metadata conversion, MANIFEST.MF generation,
//! ZIP archive creation with correct entry ordering and compression strategies.
//!
//! UCX 构建与打包模块。
//! 将项目目录打包为 `.ucx` 归档文件。
//! 处理 TOML→JSON 元数据转换、MANIFEST.MF 生成、
//! ZIP 归档创建（含正确的条目顺序和压缩策略）。
//!
//! # Usage / 使用方法
//!
//! ```no_run
//! use std::path::Path;
//! use ucx_build::{BuildOptions, build};
//!
//! let options = BuildOptions {
//!     output_dir: None,   // defaults to "dist" / 默认 "dist"
//!     output_name: None,  // defaults to project dir name / 默认为项目目录名
//! };
//! let ucx_path = build(Path::new("./my-novel"), &options).unwrap();
//! println!("Built: {}", ucx_path.display());
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;
use tracing::info;

// --- Sub-modules / 子模块 ---
pub mod archive;
pub mod convert;

// --- Re-exports for convenience / 便捷重导出 ---
pub use archive::compression_method;
pub use convert::config_to_codex;

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

    /// Failed to parse the project configuration file.
    /// 解析项目配置文件失败。
    #[error("failed to parse configuration: {0}")]
    ConfigParse(String),

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
// BuildOptions / 构建选项
// =============================================================================

/// Options controlling the UCX build process.
///
/// 控制 UCX 构建过程的选项。
///
/// # Fields / 字段
///
/// - `output_dir` — Output directory for the `.ucx` file (default: "dist").
///   输出 `.ucx` 文件的目录（默认："dist"）。
/// - `output_name` — Base name for the output file without `.ucx` extension.
///   输出文件的基本名称，不含 `.ucx` 扩展名。
///   If `None`, uses the project directory name.
///   如果为 `None`，使用项目目录名称。
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    /// Output directory path (default: "dist" relative to project root).
    /// 输出目录路径（默认：相对于项目根目录的 "dist"）。
    pub output_dir: Option<PathBuf>,

    /// Output file name without the `.ucx` extension.
    /// 输出文件名（不含 `.ucx` 扩展名）。
    pub output_name: Option<String>,
}

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Build a UCX file from the project at the given path.
///
/// Reads `unicodex.toml`, converts metadata to `codex.json`, reads `struct.json`,
/// collects all content and asset files, generates MANIFEST.MF, and creates the
/// `.ucx` ZIP archive.
///
/// 从指定路径的项目构建 UCX 文件。
/// 读取 `unicodex.toml`，将元数据转换为 `codex.json`，读取 `struct.json`，
/// 收集所有内容和资源文件，生成 MANIFEST.MF，并创建 `.ucx` ZIP 归档。
///
/// # Arguments / 参数
///
/// * `project_path` - Path to the project root directory. / 项目根目录路径。
/// * `options` - Build options (output dir, file name). / 构建选项（输出目录、文件名）。
///
/// # Returns / 返回
///
/// The absolute path to the generated `.ucx` file.
/// 生成的 `.ucx` 文件的绝对路径。
///
/// # Errors / 错误
///
/// - [`BuildError::ConfigNotFound`] — if `unicodex.toml` does not exist.
/// - [`BuildError::ConfigParse`] — if `unicodex.toml` cannot be parsed.
/// - [`BuildError::InvalidStructure`] — if `content/struct.json` is missing.
/// - [`BuildError::Io`] — if any file operation fails.
/// - [`BuildError::Zip`] — if ZIP archive creation fails.
/// - [`BuildError::Serialization`] — if JSON serialization fails.
pub fn build(project_path: &Path, options: &BuildOptions) -> Result<PathBuf, BuildError> {
    // -------------------------------------------------------------------------
    // Step 1: Read and parse unicodex.toml.
    // 步骤 1：读取并解析 unicodex.toml。
    // -------------------------------------------------------------------------
    let config_path = project_path.join("unicodex.toml");
    if !config_path.exists() {
        return Err(BuildError::ConfigNotFound(
            config_path.display().to_string(),
        ));
    }

    let toml_content = fs::read_to_string(&config_path)?;
    let config: ucx_types::ProjectConfig =
        toml::from_str(&toml_content).map_err(|e| BuildError::ConfigParse(e.to_string()))?;

    info!(
        path = %config_path.display(),
        "Read project configuration / 已读取项目配置"
    );

    // -------------------------------------------------------------------------
    // Step 2: Convert ProjectConfig → Codex (TOML → codex.json).
    // 步骤 2：转换 ProjectConfig → Codex（TOML → codex.json）。
    // -------------------------------------------------------------------------
    let codex = convert::config_to_codex(&config);
    info!("Converted config to codex / 已转换配置为 codex");

    // -------------------------------------------------------------------------
    // Step 3: Read content/struct.json (required).
    // 步骤 3：读取 content/struct.json（必需）。
    // -------------------------------------------------------------------------
    let struct_path = project_path.join("content").join("struct.json");
    if !struct_path.exists() {
        return Err(BuildError::InvalidStructure(
            "content/struct.json not found — this file is required for building".to_string(),
        ));
    }

    let struct_content = fs::read_to_string(&struct_path)?;
    let structure: ucx_types::Structure = serde_json::from_str(&struct_content)
        .map_err(|e| BuildError::ConfigParse(format!("struct.json parse error: {e}")))?;

    info!(
        path = %struct_path.display(),
        "Read content structure / 已读取内容结构"
    );

    // -------------------------------------------------------------------------
    // Step 4: Collect all files from content/ and assets/ directories.
    // 步骤 4：收集 content/ 和 assets/ 目录下的所有文件。
    // -------------------------------------------------------------------------
    let collected_files = archive::collect_project_files(project_path)?;
    info!(
        count = collected_files.len(),
        "Collected project files / 已收集项目文件"
    );

    // -------------------------------------------------------------------------
    // Step 5: Determine output path.
    // 步骤 5：确定输出路径。
    // -------------------------------------------------------------------------
    let output_dir = resolve_output_dir(project_path, &config, options);
    let output_name = resolve_output_name(project_path, &config, options);
    let output_path = output_dir.join(format!("{output_name}.ucx"));

    // Create output directory if it does not exist.
    // 如果输出目录不存在则创建。
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)?;
    }

    info!(
        path = %output_path.display(),
        "Output path resolved / 已确定输出路径"
    );

    // -------------------------------------------------------------------------
    // Step 6: Create the UCX ZIP archive.
    // 步骤 6：创建 UCX ZIP 归档。
    // -------------------------------------------------------------------------
    let ucx_version = &config.project.version;
    archive::create_ucx_archive(&output_path, &codex, &structure, &collected_files, ucx_version)?;

    info!(
        path = %output_path.display(),
        "UCX file built successfully / UCX 文件构建成功"
    );

    Ok(output_path)
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Resolve the output directory path.
///
/// Priority order:
/// 1. `options.output_dir` (CLI override)
/// 2. `config.build.output_dir` (from unicodex.toml)
/// 3. Default: `{project_path}/dist`
///
/// 确定输出目录路径。
/// 优先级：
/// 1. `options.output_dir`（CLI 覆盖）
/// 2. `config.build.output_dir`（来自 unicodex.toml）
/// 3. 默认：`{project_path}/dist`
fn resolve_output_dir(
    project_path: &Path,
    config: &ucx_types::ProjectConfig,
    options: &BuildOptions,
) -> PathBuf {
    if let Some(ref dir) = options.output_dir {
        // Use the CLI-specified output directory.
        // 使用 CLI 指定的输出目录。
        dir.clone()
    } else if let Some(ref build) = config.build {
        if let Some(ref dir) = build.output_dir {
            // Use the config-specified output directory (relative to project root).
            // 使用配置指定的输出目录（相对于项目根目录）。
            project_path.join(dir)
        } else {
            project_path.join("dist")
        }
    } else {
        project_path.join("dist")
    }
}

/// Resolve the output file name (without `.ucx` extension).
///
/// Priority order:
/// 1. `options.output_name` (CLI override)
/// 2. `config.build.output_name` (from unicodex.toml)
/// 3. Default: project directory name
///
/// 确定输出文件名（不含 `.ucx` 扩展名）。
/// 优先级：
/// 1. `options.output_name`（CLI 覆盖）
/// 2. `config.build.output_name`（来自 unicodex.toml）
/// 3. 默认：项目目录名
fn resolve_output_name(
    project_path: &Path,
    config: &ucx_types::ProjectConfig,
    options: &BuildOptions,
) -> String {
    if let Some(ref name) = options.output_name {
        name.clone()
    } else if let Some(ref build) = config.build {
        if let Some(ref name) = build.output_name {
            name.clone()
        } else {
            dir_name_or_default(project_path)
        }
    } else {
        dir_name_or_default(project_path)
    }
}

/// Extract the directory name from a path, or return "output" as fallback.
///
/// 从路径中提取目录名，如果无法获取则返回 "output"。
fn dir_name_or_default(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("output")
        .to_string()
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::TempDir;

    /// Helper: create a minimal project directory for testing.
    ///
    /// Creates:
    /// ```text
    /// temp_dir/
    /// +-- unicodex.toml
    /// +-- content/
    /// |   +-- struct.json
    /// |   +-- chapter-001.md
    /// ```
    ///
    /// 辅助函数：创建一个最小的项目目录用于测试。
    fn create_test_project(dir: &Path) {
        // Create content directory.
        // 创建 content 目录。
        fs::create_dir_all(dir.join("content")).unwrap();

        // Write unicodex.toml.
        // 写入 unicodex.toml。
        let toml = r#"
[project]
version = "1.0"

[identifier]
ucx_id = "urn:ucx:550e8400-e29b-41d4-a716-446655440000"

[title]
main = "测试小说"

[[creators]]
name = "测试作者"
role = "author"

[book]
language = "zh-CN"
"#;
        fs::write(dir.join("unicodex.toml"), toml).unwrap();

        // Write struct.json.
        // 写入 struct.json。
        let struct_json = r#"{
    "version": "1.0",
    "structure": [
        {
            "title": "第一章 开端",
            "file": "chapter-001.md"
        }
    ]
}"#;
        fs::write(dir.join("content/struct.json"), struct_json).unwrap();

        // Write chapter-001.md.
        // 写入 chapter-001.md。
        fs::write(
            dir.join("content/chapter-001.md"),
            "# 第一章 开端\n\n这是第一章的内容。\n",
        )
        .unwrap();
    }

    /// Test: build() should produce a valid .ucx ZIP file.
    /// 测试：build() 应生成有效的 .ucx ZIP 文件。
    #[test]
    fn test_build_simple_project() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("test-novel");
        fs::create_dir_all(&project_dir).unwrap();
        create_test_project(&project_dir);

        // Run build with default options.
        // 使用默认选项运行构建。
        let options = BuildOptions::default();
        let result = build(&project_dir, &options);
        assert!(result.is_ok(), "build() failed: {:?}", result.err());

        let ucx_path = result.unwrap();

        // Verify the output file exists.
        // 验证输出文件存在。
        assert!(ucx_path.exists(), ".ucx file should exist");
        assert!(
            ucx_path.to_string_lossy().ends_with(".ucx"),
            "output should have .ucx extension"
        );

        // Open the ZIP and verify structure.
        // 打开 ZIP 并验证结构。
        let file = fs::File::open(&ucx_path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();

        // Verify mimetype is the first entry and uses STORED compression.
        // 验证 mimetype 是第一个条目且使用 STORED 压缩。
        {
            let entry = zip.by_index(0).unwrap();
            assert_eq!(entry.name(), "mimetype", "first entry should be mimetype");
            assert_eq!(
                entry.compression(),
                zip::CompressionMethod::Stored,
                "mimetype should use STORED compression"
            );
        }

        // Verify mimetype content.
        // 验证 mimetype 内容。
        {
            let mut entry = zip.by_name("mimetype").unwrap();
            let mut content = String::new();
            entry.read_to_string(&mut content).unwrap();
            assert_eq!(content, "application/vnd.unicodex+zip");
        }

        // Verify META-INF/MANIFEST.MF exists.
        // 验证 META-INF/MANIFEST.MF 存在。
        {
            let mut entry = zip.by_name("META-INF/MANIFEST.MF").unwrap();
            let mut content = String::new();
            entry.read_to_string(&mut content).unwrap();
            assert!(content.contains("Manifest-Version: 1.0"));
            assert!(content.contains("Hash-Algorithm: BLAKE3"));
        }

        // Verify metadata/codex.json exists and is valid.
        // 验证 metadata/codex.json 存在且有效。
        {
            let mut entry = zip.by_name("metadata/codex.json").unwrap();
            let mut content = String::new();
            entry.read_to_string(&mut content).unwrap();
            let codex: ucx_types::Codex =
                serde_json::from_str(&content).expect("codex.json should be valid");
            assert_eq!(codex.title.main, "测试小说");
            assert_eq!(codex.language, "zh-CN");
        }

        // Verify content/struct.json exists.
        // 验证 content/struct.json 存在。
        assert!(zip.by_name("content/struct.json").is_ok());

        // Verify content/chapter-001.md exists.
        // 验证 content/chapter-001.md 存在。
        {
            let mut entry = zip.by_name("content/chapter-001.md").unwrap();
            let mut content = String::new();
            entry.read_to_string(&mut content).unwrap();
            assert!(content.contains("第一章"));
        }
    }

    /// Test: build() should fail if unicodex.toml is missing.
    /// 测试：如果 unicodex.toml 缺失，build() 应失败。
    #[test]
    fn test_build_missing_config() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("no-config");
        fs::create_dir_all(&project_dir).unwrap();

        let result = build(&project_dir, &BuildOptions::default());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BuildError::ConfigNotFound(_)));
    }

    /// Test: build() should fail if content/struct.json is missing.
    /// 测试：如果 content/struct.json 缺失，build() 应失败。
    #[test]
    fn test_build_missing_struct_json() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("no-struct");
        fs::create_dir_all(project_dir.join("content")).unwrap();

        // Write only unicodex.toml, no struct.json.
        // 仅写入 unicodex.toml，不写 struct.json。
        let toml = r#"
[project]
version = "1.0"
[identifier]
ucx_id = "urn:ucx:550e8400-e29b-41d4-a716-446655440000"
[title]
main = "测试"
[[creators]]
name = "作者"
role = "author"
[book]
language = "zh-CN"
"#;
        fs::write(project_dir.join("unicodex.toml"), toml).unwrap();

        let result = build(&project_dir, &BuildOptions::default());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            BuildError::InvalidStructure(_)
        ));
    }

    /// Test: BuildOptions should allow custom output dir and name.
    /// 测试：BuildOptions 应允许自定义输出目录和文件名。
    #[test]
    fn test_build_custom_output() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("custom-output");
        fs::create_dir_all(&project_dir).unwrap();
        create_test_project(&project_dir);

        let custom_out = tmp.path().join("my-output");
        let options = BuildOptions {
            output_dir: Some(custom_out.clone()),
            output_name: Some("my-book".to_string()),
        };

        let result = build(&project_dir, &options);
        assert!(result.is_ok(), "build() failed: {:?}", result.err());

        let ucx_path = result.unwrap();
        assert_eq!(ucx_path, custom_out.join("my-book.ucx"));
        assert!(ucx_path.exists());
    }
}
