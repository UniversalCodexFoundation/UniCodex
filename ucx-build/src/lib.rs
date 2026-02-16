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
//!     ..Default::default()
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
/// - `dry_run` — If `true`, validate only; do not create the `.ucx` file.
///   如果为 `true`，仅执行校验，不创建 `.ucx` 文件。
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    /// Output directory path (default: "dist" relative to project root).
    /// 输出目录路径（默认：相对于项目根目录的 "dist"）。
    pub output_dir: Option<PathBuf>,

    /// Output file name without the `.ucx` extension.
    /// 输出文件名（不含 `.ucx` 扩展名）。
    pub output_name: Option<String>,

    /// Dry-run mode: validate project without creating the archive.
    /// 预演模式：仅校验项目，不创建归档文件。
    pub dry_run: bool,
}

// =============================================================================
// DryRunResult / 预演结果
// =============================================================================

/// Result of a dry-run build — project validation without archive creation.
///
/// Contains the information that *would* be used for a real build.
///
/// 预演构建的结果 — 仅校验项目，不创建归档。
/// 包含实际构建中*将会*使用的信息。
#[derive(Debug)]
pub struct DryRunResult {
    /// The output path where the `.ucx` file *would* be created.
    /// `.ucx` 文件*将会*被创建的输出路径。
    pub output_path: PathBuf,

    /// List of files that *would* be included in the archive.
    /// *将会*被包含在归档中的文件列表。
    pub files: Vec<DryRunFile>,

    /// Total size in bytes of all files to be packed.
    /// 所有待打包文件的总大小（字节）。
    pub total_size: u64,
}

/// A single file entry in a dry-run result.
///
/// 预演结果中的单个文件条目。
#[derive(Debug)]
pub struct DryRunFile {
    /// Path inside the ZIP archive (e.g., "content/chapter-001.md").
    /// ZIP 归档内的路径（如 "content/chapter-001.md"）。
    pub archive_path: String,

    /// Size in bytes on disk.
    /// 磁盘上的文件大小（字节）。
    pub size: u64,
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
    // Step 3.5: Validate struct.json constraints.
    // 步骤 3.5：校验 struct.json 约束条件。
    // -------------------------------------------------------------------------
    // P-005: Validate that `file` and `children` are mutually exclusive.
    // P-005：校验 `file` 和 `children` 互斥。
    validate_structure_nodes(&structure.structure)?;

    // P-006: Validate that all referenced files exist in content/ directory.
    // P-006：校验所有引用的文件存在于 content/ 目录下。
    validate_file_references(&structure.structure, &project_path.join("content"))?;

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

    // -------------------------------------------------------------------------
    // Dry-run mode: return result without creating archive.
    // 预演模式：返回结果但不创建归档文件。
    // -------------------------------------------------------------------------
    if options.dry_run {
        info!(
            path = %output_path.display(),
            "Dry-run: would output to / 预演：将输出到"
        );
        return Ok(output_path);
    }

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
// Public helpers / 公开辅助函数
// =============================================================================

/// Resolve the expected output file path for a build, without actually building.
///
/// Reads `unicodex.toml` to determine the output directory and file name,
/// then returns the full path. Used by the CLI to check for existing files.
///
/// 解析预期的构建输出文件路径，但不实际执行构建。
/// 读取 `unicodex.toml` 以确定输出目录和文件名，然后返回完整路径。
/// 由 CLI 用于检查已存在的文件。
pub fn resolve_output_path(project_path: &Path, options: &BuildOptions) -> Result<PathBuf, BuildError> {
    let config_path = project_path.join("unicodex.toml");
    let config_str = fs::read_to_string(&config_path).map_err(|_| {
        BuildError::ConfigNotFound(config_path.display().to_string())
    })?;
    let config: ucx_types::ProjectConfig = toml::from_str(&config_str).map_err(|e| {
        BuildError::ConfigParse(e.to_string())
    })?;
    let output_dir = resolve_output_dir(project_path, &config, options);
    let output_name = resolve_output_name(project_path, &config, options);
    Ok(output_dir.join(format!("{output_name}.ucx")))
}

/// Perform a dry-run build: validate the project and report what *would* be packed.
///
/// Runs Steps 1–4 of the normal build process (read config, validate structure,
/// collect files), then returns a [`DryRunResult`] without creating the archive.
///
/// 执行预演构建：校验项目并报告*将会*打包的内容。
/// 运行正常构建的步骤 1–4（读取配置、校验结构、收集文件），
/// 然后返回 [`DryRunResult`] 而不创建归档。
pub fn dry_run(project_path: &Path, options: &BuildOptions) -> Result<DryRunResult, BuildError> {
    // Step 1: Read and parse unicodex.toml.
    // 步骤 1：读取并解析 unicodex.toml。
    let config_path = project_path.join("unicodex.toml");
    if !config_path.exists() {
        return Err(BuildError::ConfigNotFound(
            config_path.display().to_string(),
        ));
    }
    let toml_content = fs::read_to_string(&config_path)?;
    let config: ucx_types::ProjectConfig =
        toml::from_str(&toml_content).map_err(|e| BuildError::ConfigParse(e.to_string()))?;

    // Step 2: Convert config (to validate metadata).
    // 步骤 2：转换配置（以校验元数据）。
    let _codex = convert::config_to_codex(&config);

    // Step 3: Read and validate struct.json.
    // 步骤 3：读取并校验 struct.json。
    let struct_path = project_path.join("content").join("struct.json");
    if !struct_path.exists() {
        return Err(BuildError::InvalidStructure(
            "content/struct.json not found — this file is required for building".to_string(),
        ));
    }
    let struct_content = fs::read_to_string(&struct_path)?;
    let structure: ucx_types::Structure = serde_json::from_str(&struct_content)
        .map_err(|e| BuildError::ConfigParse(format!("struct.json parse error: {e}")))?;

    validate_structure_nodes(&structure.structure)?;
    validate_file_references(&structure.structure, &project_path.join("content"))?;

    // Step 4: Collect files and compute sizes.
    // 步骤 4：收集文件并计算大小。
    let collected_files = archive::collect_project_files(project_path)?;
    let mut files = Vec::with_capacity(collected_files.len());
    let mut total_size: u64 = 0;

    for cf in &collected_files {
        let size = fs::metadata(&cf.disk_path)
            .map(|m| m.len())
            .unwrap_or(0);
        total_size += size;
        files.push(DryRunFile {
            archive_path: cf.archive_path.clone(),
            size,
        });
    }

    // Determine output path.
    // 确定输出路径。
    let output_dir = resolve_output_dir(project_path, &config, options);
    let output_name = resolve_output_name(project_path, &config, options);
    let output_path = output_dir.join(format!("{output_name}.ucx"));

    Ok(DryRunResult {
        output_path,
        files,
        total_size,
    })
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

/// Recursively validate structure nodes — enforce `file`/`children` mutual exclusivity.
///
/// Per spec `01-file-structure.md §4.1`:
/// - A node with `file` is a leaf and MUST NOT have `children`.
/// - A node with `children` is a container and MUST NOT have `file`.
/// - A node must have exactly one of `file` or `children`.
///
/// 递归校验结构节点 — 强制 `file`/`children` 互斥约束。
/// 根据规范 `01-file-structure.md §4.1`：
/// - 有 `file` 的节点是叶子节点，不能有 `children`。
/// - 有 `children` 的节点是容器节点，不能有 `file`。
/// - 节点必须恰好有 `file` 或 `children` 之一。
fn validate_structure_nodes(nodes: &[ucx_types::StructureNode]) -> Result<(), BuildError> {
    for node in nodes {
        // Both `file` and `children` present — violates mutual exclusivity.
        // `file` 和 `children` 同时存在 — 违反互斥约束。
        if node.file.is_some() && node.children.is_some() {
            return Err(BuildError::InvalidStructure(format!(
                "node '{}' has both 'file' and 'children' — they are mutually exclusive per spec §4.1",
                node.title
            )));
        }

        // Neither `file` nor `children` — invalid node.
        // `file` 和 `children` 都不存在 — 无效节点。
        if node.file.is_none() && node.children.is_none() {
            return Err(BuildError::InvalidStructure(format!(
                "node '{}' has neither 'file' nor 'children' — one is required per spec §4.1",
                node.title
            )));
        }

        // Recurse into children if present.
        // 如果有子节点则递归校验。
        if let Some(ref children) = node.children {
            validate_structure_nodes(children)?;
        }
    }
    Ok(())
}

/// Validate that all file references in struct.json exist in the content directory.
///
/// Warns for missing files but does not fail the build by default.
/// Instead, it collects missing files and emits warnings.
///
/// 校验 struct.json 中所有文件引用是否存在于 content 目录中。
/// 对缺失的文件发出警告并报错终止构建。
fn validate_file_references(
    nodes: &[ucx_types::StructureNode],
    content_dir: &Path,
) -> Result<(), BuildError> {
    let mut missing = Vec::new();
    collect_missing_files(nodes, content_dir, &mut missing);

    if !missing.is_empty() {
        return Err(BuildError::InvalidStructure(format!(
            "struct.json references {} missing file(s): {}",
            missing.len(),
            missing.join(", ")
        )));
    }

    Ok(())
}

/// Recursively collect file references that do not exist on disk.
///
/// 递归收集磁盘上不存在的文件引用。
fn collect_missing_files(
    nodes: &[ucx_types::StructureNode],
    content_dir: &Path,
    missing: &mut Vec<String>,
) {
    for node in nodes {
        if let Some(ref file) = node.file {
            let file_path = content_dir.join(file);
            if !file_path.exists() {
                missing.push(file.clone());
            }
        }
        if let Some(ref children) = node.children {
            collect_missing_files(children, content_dir, missing);
        }
    }
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
            ..Default::default()
        };

        let result = build(&project_dir, &options);
        assert!(result.is_ok(), "build() failed: {:?}", result.err());

        let ucx_path = result.unwrap();
        assert_eq!(ucx_path, custom_out.join("my-book.ucx"));
        assert!(ucx_path.exists());
    }

    /// Test (P-005): build() should fail if struct.json has a node with both `file` and `children`.
    /// 测试（P-005）：如果 struct.json 的节点同时有 `file` 和 `children`，build() 应失败。
    #[test]
    fn test_build_rejects_file_and_children_together() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("both-file-children");
        fs::create_dir_all(project_dir.join("content")).unwrap();

        // Write unicodex.toml.
        // 写入 unicodex.toml。
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

        // Write struct.json with a node that has BOTH `file` and `children` — invalid.
        // 写入同时包含 `file` 和 `children` 的 struct.json — 无效。
        let struct_json = r#"{
    "version": "1.0",
    "structure": [{
        "title": "invalid_node",
        "file": "chapter-001.md",
        "children": [{"title": "sub", "file": "chapter-001.md"}]
    }]
}"#;
        fs::write(project_dir.join("content/struct.json"), struct_json).unwrap();
        fs::write(
            project_dir.join("content/chapter-001.md"),
            "# Test\n",
        )
        .unwrap();

        let result = build(&project_dir, &BuildOptions::default());
        assert!(result.is_err(), "should reject struct with both file and children");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("mutually exclusive"),
            "error should mention mutual exclusivity, got: {err_msg}"
        );
    }

    /// Test (P-006): build() should fail if struct.json references a nonexistent file.
    /// 测试（P-006）：如果 struct.json 引用了不存在的文件，build() 应失败。
    #[test]
    fn test_build_rejects_missing_file_reference() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("missing-file-ref");
        fs::create_dir_all(project_dir.join("content")).unwrap();

        // Write unicodex.toml.
        // 写入 unicodex.toml。
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

        // Write struct.json referencing a file that does NOT exist.
        // 写入引用不存在文件的 struct.json。
        let struct_json = r#"{
    "version": "1.0",
    "structure": [
        {"title": "第一章", "file": "chapter-001.md"},
        {"title": "第二章", "file": "nonexistent.md"}
    ]
}"#;
        fs::write(project_dir.join("content/struct.json"), struct_json).unwrap();
        fs::write(
            project_dir.join("content/chapter-001.md"),
            "# 第一章\n",
        )
        .unwrap();
        // NOTE: content/nonexistent.md is intentionally NOT created.
        // 注意：content/nonexistent.md 故意不创建。

        let result = build(&project_dir, &BuildOptions::default());
        assert!(result.is_err(), "should reject missing file reference");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("nonexistent.md"),
            "error should mention the missing file, got: {err_msg}"
        );
    }
}
