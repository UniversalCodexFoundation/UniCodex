//! UCX Project Initialization Module.
//!
//! This module provides functionality to create new UCX project directories,
//! generate `unicodex.toml` configuration files, scaffold the standard
//! directory structure, and assign UCX IDs.
//!
//! UCX 项目初始化模块。
//! 提供创建新 UCX 项目目录、生成 `unicodex.toml` 配置、
//! 搭建标准目录结构和分配 UCX ID 的功能。
//!
//! # Usage / 使用方法
//!
//! ```no_run
//! use std::path::Path;
//! use ucx_init::{InitOptions, init};
//!
//! let options = InitOptions {
//!     name: "我的小说".to_string(),
//!     author: "作者名".to_string(),
//!     language: "zh-CN".to_string(),
//!     ..Default::default()
//! };
//! init(Path::new("./my-novel"), &options).unwrap();
//! ```

use std::fs;
use std::path::Path;

use thiserror::Error;
use tracing::info;

// Import shared types from ucx-types.
// 从 ucx-types 导入共享类型。
use ucx_types::project::{
    BookSection, BuildSection, IdentifierSection, ProjectConfig, ProjectSection, TitleSection,
};
use ucx_types::structure::{Structure, StructureNode};
use ucx_types::ucx_id::UcxId;
use ucx_types::codex::Creator;

// =============================================================================
// Constants / 常量
// =============================================================================

/// The name of the project configuration file.
/// 项目配置文件的名称。
const CONFIG_FILE_NAME: &str = "unicodex.toml";

/// The name of the content structure file.
/// 内容结构文件的名称。
const STRUCT_FILE_NAME: &str = "struct.json";

/// Standard subdirectories created during default project initialization.
/// 默认项目初始化时创建的标准子目录。
///
/// Only `content/` is created by default. Use `--full` flag to create
/// the complete directory structure.
/// 默认只创建 `content/`。使用 `--full` 标志创建完整目录结构。
const DEFAULT_DIRS: &[&str] = &["content"];

/// Full set of subdirectories created with `--full` flag.
/// 使用 `--full` 标志时创建的完整子目录集合。
///
/// - `content/`  — Markdown chapter files / Markdown 章节文件
/// - `assets/`   — Images, fonts, and other assets / 图片、字体等资源
/// - `extras/`   — Supplementary materials / 附录资料
///
/// Note: `dist/` is always created by `ucx build` on demand.
/// 注意：`dist/` 始终由 `ucx build` 按需创建。
const FULL_DIRS: &[&str] = &["content", "assets", "extras"];

/// The default first chapter filename.
/// 默认的第一章文件名。
const FIRST_CHAPTER_FILE: &str = "chapter-001.md";

/// The JSON Schema URL for struct.json.
/// struct.json 的 JSON Schema URL。
const STRUCT_SCHEMA_URL: &str = "https://unicodex.org/schemas/struct-v1.json";

// =============================================================================
// Error types / 错误类型
// =============================================================================

/// Errors that can occur during project initialization.
///
/// 项目初始化过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum InitError {
    /// The target directory already contains a UCX project (unicodex.toml exists).
    /// 目标目录已包含 UCX 项目（unicodex.toml 已存在）。
    #[error("directory already contains a UCX project: {0}")]
    AlreadyExists(String),

    /// The provided path contains path traversal components (`..`).
    /// 提供的路径包含路径遍历组件（`..`）。
    #[error("path contains '..' traversal component, which is not allowed: {0}")]
    PathTraversal(String),

    /// A required input field is empty or invalid.
    /// 必需的输入字段为空或无效。
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Failed to perform an I/O operation (create directory, write file, etc.).
    /// I/O 操作失败（创建目录、写文件等）。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to serialize configuration to TOML format.
    /// 将配置序列化为 TOML 格式失败。
    #[error("failed to serialize TOML: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// Failed to serialize structure to JSON format.
    /// 将结构序列化为 JSON 格式失败。
    #[error("failed to serialize JSON: {0}")]
    JsonSerialize(#[from] serde_json::Error),
}

// =============================================================================
// InitOptions / 初始化选项
// =============================================================================

/// Options for initializing a new UCX project.
///
/// 初始化新 UCX 项目的选项。
///
/// # Fields / 字段
///
/// - `name` — The title of the work (e.g., "我的小说"). / 作品标题。
/// - `author` — The primary author's name. / 主要作者名称。
/// - `language` — BCP 47 language tag (default: "zh-CN"). / BCP 47 语言标签。
/// - `allow_long_fields` — Allow fields longer than 500 chars (default: false).
///   允许超过 500 字符的字段（默认：false）。
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// The title of the work.
    /// 作品标题。
    pub name: String,

    /// The primary author's name or pen name.
    /// 主要作者的名称或笔名。
    pub author: String,

    /// The primary language of the work (BCP 47 tag, e.g., "zh-CN").
    /// 作品的主要语言（BCP 47 标签，如 "zh-CN"）。
    pub language: String,

    /// Whether to allow fields longer than the default 500-character limit.
    /// 是否允许超过默认 500 字符上限的字段。
    pub allow_long_fields: bool,

    /// Whether to create the full directory structure (content/, assets/, extras/).
    /// 是否创建完整目录结构（content/、assets/、extras/）。
    /// Default false: only creates content/.
    /// 默认 false：仅创建 content/。
    pub full: bool,
}

impl Default for InitOptions {
    /// Create default options with localized placeholder values.
    ///
    /// 使用本地化占位值创建默认选项。
    fn default() -> Self {
        Self {
            name: "无标题".to_string(),
            author: "未知".to_string(),
            language: "zh-CN".to_string(),
            allow_long_fields: false,
            full: false,
        }
    }
}

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Initialize a new UCX project at the given path.
///
/// Creates the standard directory structure and generates initial configuration
/// files (`unicodex.toml`, `content/struct.json`, `content/chapter-001.md`).
///
/// 在指定路径初始化新的 UCX 项目。
/// 创建标准目录结构并生成初始配置文件
/// （`unicodex.toml`、`content/struct.json`、`content/chapter-001.md`）。
///
/// # Steps / 步骤
///
/// 1. Check if `unicodex.toml` already exists (reject if so).
///    检查 `unicodex.toml` 是否已存在（已存在则拒绝）。
/// 2. Create the project root directory if it does not exist.
///    如果项目根目录不存在则创建。
/// 3. Create standard subdirectories: `content/`, `assets/`, `extras/`, `dist/`.
///    创建标准子目录。
/// 4. Generate a new UCX ID (UUID v4, format `urn:ucx:{uuid}`).
///    生成新的 UCX ID。
/// 5. Write `unicodex.toml` using the `ProjectConfig` type.
///    使用 `ProjectConfig` 类型写入 `unicodex.toml`。
/// 6. Write `content/struct.json` with one example chapter node.
///    写入 `content/struct.json`，包含一个示例章节节点。
/// 7. Write `content/chapter-001.md` as a starter chapter file.
///    写入 `content/chapter-001.md` 作为起始章节。
///
/// # Arguments / 参数
///
/// * `path` - The directory where the project will be created.
///            项目将创建在此目录中。
/// * `options` - Project metadata options (title, author, language).
///              项目元数据选项（标题、作者、语言）。
///
/// # Returns / 返回
///
/// Returns `Ok(())` on success, or an `InitError` on failure.
/// 成功返回 `Ok(())`，失败返回 `InitError`。
///
/// # Errors / 错误
///
/// - [`InitError::AlreadyExists`] — if `unicodex.toml` already exists at `path`.
///   如果 `path` 下已存在 `unicodex.toml`。
/// - [`InitError::Io`] — if any file or directory operation fails.
///   如果任何文件或目录操作失败。
/// - [`InitError::TomlSerialize`] — if TOML serialization fails.
///   如果 TOML 序列化失败。
/// - [`InitError::JsonSerialize`] — if JSON serialization fails.
///   如果 JSON 序列化失败。
pub fn init(path: &Path, options: &InitOptions) -> Result<(), InitError> {
    // -------------------------------------------------------------------------
    // Step 0: Validate the path — reject path traversal attacks.
    // 步骤 0：验证路径 — 拒绝路径遍历攻击。
    // -------------------------------------------------------------------------
    // Check for `..` components in the path to prevent directory traversal.
    // 检查路径中的 `..` 组件以防止目录遍历。
    for component in path.components() {
        if component == std::path::Component::ParentDir {
            return Err(InitError::PathTraversal(path.display().to_string()));
        }
    }

    // -------------------------------------------------------------------------
    // Step 0.5: Validate input options — reject empty or control-char inputs.
    // 步骤 0.5：验证输入选项 — 拒绝空值或含控制字符的输入。
    // -------------------------------------------------------------------------
    validate_input(&options.name, "name", options.allow_long_fields)?;
    validate_input(&options.author, "author", options.allow_long_fields)?;
    validate_language_tag(&options.language)?;

    // -------------------------------------------------------------------------
    // Step 1: Check if the project already exists.
    // 步骤 1：检查项目是否已存在。
    // -------------------------------------------------------------------------
    let config_path = path.join(CONFIG_FILE_NAME);
    if config_path.exists() {
        // Return an error indicating the project already exists.
        // 返回错误，表示项目已存在。
        return Err(InitError::AlreadyExists(
            path.display().to_string(),
        ));
    }

    // -------------------------------------------------------------------------
    // Step 2: Create the project root directory (and parents) if needed.
    // 步骤 2：如果需要，创建项目根目录（及其父目录）。
    // -------------------------------------------------------------------------
    if !path.exists() {
        fs::create_dir_all(path)?;
        info!(
            path = %path.display(),
            "Created project root directory / 已创建项目根目录"
        );
    }

    // -------------------------------------------------------------------------
    // Step 3: Create subdirectories.
    // 步骤 3：创建子目录。
    // -------------------------------------------------------------------------
    // Default: only content/. With --full: content/, assets/, extras/.
    // 默认：仅 content/。使用 --full 时：content/、assets/、extras/。
    let dirs = if options.full { FULL_DIRS } else { DEFAULT_DIRS };
    for dir_name in dirs {
        let dir_path = path.join(dir_name);
        // create_dir_all is idempotent — safe to call even if dir exists.
        // create_dir_all 是幂等的 — 即使目录已存在也安全。
        fs::create_dir_all(&dir_path)?;
        info!(
            dir = %dir_path.display(),
            "Created subdirectory / 已创建子目录"
        );
    }

    // -------------------------------------------------------------------------
    // Step 4: Generate a new UCX ID (UUID v4).
    // 步骤 4：生成新的 UCX ID（UUID v4）。
    // -------------------------------------------------------------------------
    let ucx_id = UcxId::new();
    info!(
        ucx_id = %ucx_id,
        "Generated UCX ID / 已生成 UCX ID"
    );

    // -------------------------------------------------------------------------
    // Step 5: Generate and write `unicodex.toml`.
    // 步骤 5：生成并写入 `unicodex.toml`。
    // -------------------------------------------------------------------------
    let project_config = build_project_config(&ucx_id, options);
    let mut toml_content = toml::to_string_pretty(&project_config)?;

    // Append commented-out examples for optional TOML sections.
    // 追加注释掉的可选 TOML 段示例。
    toml_content.push_str(r#"
# --- 以下为可选配置段示例（取消注释即可启用） ---

# [series]
# name = "系列名称"
# index = 1
# total = 5

# [description]
# short = "一句话简介（≤100字符）"
# long = "详细简介，支持换行"

# [rights]
# statement = "版权所有 © 2026 作者名"
# license = "CC-BY-NC-4.0"

# [cover]
# path = "assets/cover.jpg"

# [rating]
# system = "age"
# value = "all"

# [dates]
# created = "2026-01-01"
# published = ""
# modified = ""
"#);

    fs::write(&config_path, &toml_content)?;
    info!(
        path = %config_path.display(),
        "Written unicodex.toml / 已写入 unicodex.toml"
    );

    // -------------------------------------------------------------------------
    // Step 6: Generate and write `content/struct.json`.
    // 步骤 6：生成并写入 `content/struct.json`。
    // -------------------------------------------------------------------------
    let structure = build_default_structure();
    let struct_json = serde_json::to_string_pretty(&structure)?;
    let struct_path = path.join("content").join(STRUCT_FILE_NAME);
    fs::write(&struct_path, &struct_json)?;
    info!(
        path = %struct_path.display(),
        "Written struct.json / 已写入 struct.json"
    );

    // -------------------------------------------------------------------------
    // Step 7: Generate and write `content/chapter-001.md`.
    // 步骤 7：生成并写入 `content/chapter-001.md`。
    // -------------------------------------------------------------------------
    let chapter_content = build_first_chapter();
    let chapter_path = path.join("content").join(FIRST_CHAPTER_FILE);
    fs::write(&chapter_path, &chapter_content)?;
    info!(
        path = %chapter_path.display(),
        "Written first chapter / 已写入第一章"
    );

    info!(
        path = %path.display(),
        name = %options.name,
        "UCX project initialized successfully / UCX 项目初始化成功"
    );

    Ok(())
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Build a `ProjectConfig` from the given UCX ID and user options.
///
/// Constructs a minimal but valid project configuration that matches the
/// template specified in `docs/02-metadata-spec.md §5.2`.
///
/// 根据给定的 UCX ID 和用户选项构建 `ProjectConfig`。
/// 构造一个最小但合法的项目配置，符合 `docs/02-metadata-spec.md §5.2` 模板。
///
/// # Arguments / 参数
///
/// * `ucx_id` - The generated UCX ID for this project. / 为此项目生成的 UCX ID。
/// * `options` - User-provided project metadata. / 用户提供的项目元数据。
fn build_project_config(ucx_id: &UcxId, options: &InitOptions) -> ProjectConfig {
    ProjectConfig {
        // [project] section — UCX spec version.
        // [project] 段 — UCX 规范版本。
        project: ProjectSection {
            version: "1.0".to_string(),
        },

        // [identifier] section — unique ID for this work.
        // [identifier] 段 — 此作品的唯一标识。
        identifier: IdentifierSection {
            ucx_id: ucx_id.clone(),
            isbn: None,
        },

        // [title] section — work title.
        // [title] 段 — 作品标题。
        title: TitleSection {
            main: options.name.clone(),
            subtitle: None,
        },

        // No series info for a new project.
        // 新项目无系列信息。
        series: None,

        // [[creators]] — single author entry.
        // [[creators]] — 单个作者条目。
        creators: vec![Creator {
            name: options.author.clone(),
            role: "author".to_string(),
            signature_ref: None,
        }],

        // No publisher for a new project.
        // 新项目无出版方信息。
        publisher: None,

        // [book] section — language metadata.
        // [book] 段 — 语言元数据。
        book: Some(BookSection {
            language: options.language.clone(),
            genre: None,
            tags: None,
            status: None,
        }),

        // Optional sections — all None for a fresh project.
        // 可选段落 — 新项目全部为 None。
        description: None,
        rights: None,
        cover: None,
        rating: None,

        // [build] section — output directory defaults to "dist".
        // [build] 段 — 输出目录默认为 "dist"。
        build: Some(BuildSection {
            output_dir: Some("dist".to_string()),
            output_name: None,
            compression: None,
        }),

        // No signing config for a new project.
        // 新项目无签名配置。
        signing: None,
    }
}

/// Build the default `Structure` for a new project.
///
/// Creates a minimal structure with a single chapter node pointing to
/// `chapter-001.md`, as specified in the initialization template.
///
/// 为新项目构建默认的 `Structure`。
/// 创建一个最小结构，包含一个指向 `chapter-001.md` 的章节节点。
fn build_default_structure() -> Structure {
    Structure {
        // Include the JSON Schema reference for validation.
        // 包含 JSON Schema 引用以便校验。
        schema: Some(STRUCT_SCHEMA_URL.to_string()),

        // Structure schema version.
        // 结构 schema 版本。
        version: "1.0".to_string(),

        // A single leaf node: the first chapter.
        // 单个叶子节点：第一章。
        structure: vec![StructureNode {
            // Display title for this chapter.
            // 此章节的显示标题。
            title: "第一章".to_string(),

            // Points to the Markdown file relative to `content/`.
            // 指向相对于 `content/` 的 Markdown 文件。
            file: Some(FIRST_CHAPTER_FILE.to_string()),

            // Leaf node — no children.
            // 叶子节点 — 无子节点。
            children: None,

            // Optional annotations — not needed for a starter template.
            // 可选注释 — 起始模板不需要。
            node_type: None,
            id: None,
            name: None,
            style: None,
            encryption: None,
        }],
    }
}

/// Build the content of the first example chapter file.
///
/// Returns a simple Markdown string that serves as a starting point
/// for the user's creative work.
///
/// 构建第一个示例章节文件的内容。
/// 返回一个简单的 Markdown 字符串，作为用户创作的起点。
fn build_first_chapter() -> String {
    // Markdown with a heading, placeholder paragraph, and helpful comments.
    // Markdown，包含标题、占位文字和辅助注释。
    r#"# 第一章

在这里开始你的创作。

<!-- UCX 提示：
  - 使用标准 Markdown 语法编写章节内容
  - 支持的格式：标题(#)、粗体(**)、斜体(*)、列表、引用(>)等
  - 在 struct.json 中管理章节结构和顺序
  - 运行 `ucx build` 将项目打包为 .ucx 文件
  - 更多信息请参阅 https://unicodex.org/docs
-->
"#
    .to_string()
}

/// Validate a user-supplied input string.
///
/// Rejects:
/// - Empty strings or whitespace-only strings.
/// - Strings containing ASCII control characters (U+0000–U+001F except tab).
/// - Strings longer than 500 characters (unless `allow_long` is true).
///
/// 验证用户输入的字符串。
/// 拒绝：
/// - 空字符串或仅含空白的字符串。
/// - 包含 ASCII 控制字符的字符串（U+0000–U+001F，制表符除外）。
/// - 超过 500 字符的字符串（除非 `allow_long` 为 true）。
///
/// # Arguments / 参数
///
/// * `value` - The input string to validate. / 要验证的输入字符串。
/// * `field_name` - The field name for error messages. / 用于错误消息的字段名。
/// * `allow_long` - Whether to skip the 500-char length limit. / 是否跳过 500 字符上限。
fn validate_input(value: &str, field_name: &str, allow_long: bool) -> Result<(), InitError> {
    // Maximum allowed length for input fields (default limit).
    // 输入字段的最大允许长度（默认限制）。
    const MAX_INPUT_LENGTH: usize = 500;

    // Reject empty or whitespace-only input.
    // 拒绝空或仅含空白的输入。
    if value.trim().is_empty() {
        return Err(InitError::InvalidInput(format!(
            "'{field_name}' must not be empty"
        )));
    }

    // Reject strings exceeding the maximum length (unless allowed).
    // 拒绝超过最大长度的字符串（除非允许）。
    if !allow_long && value.len() > MAX_INPUT_LENGTH {
        return Err(InitError::InvalidInput(format!(
            "'{field_name}' exceeds maximum length of {MAX_INPUT_LENGTH} characters ({} given). Use --allow-long-fields to override",
            value.len()
        )));
    }

    // Reject strings containing control characters (U+0000–U+001F),
    // except for tab (\t). Newline (\n) and carriage return (\r) are also
    // rejected because name/author fields should be single-line values.
    // 拒绝包含控制字符（U+0000–U+001F）的字符串，
    // 制表符 (\t) 除外。换行符 (\n) 和回车符 (\r) 也被拒绝，
    // 因为 name/author 字段应为单行值。
    if let Some(pos) = value.chars().position(|c| {
        c.is_control() && c != '\t'
    }) {
        return Err(InitError::InvalidInput(format!(
            "'{field_name}' contains a control character at position {pos}"
        )));
    }

    Ok(())
}

/// Validate a BCP 47 language tag.
///
/// Performs basic format validation:
/// - 2-3 letter primary language subtag
/// - Optional hyphen-separated subtags (1-8 alphanumeric characters each)
/// - Examples: "zh-CN", "en", "en-US", "ja", "zh-Hant-TW"
///
/// 验证 BCP 47 语言标签。
/// 执行基本格式校验：
/// - 2-3 字母主语言子标签
/// - 可选的连字符分隔子标签（每个 1-8 字母数字）
/// - 示例："zh-CN"、"en"、"en-US"、"ja"、"zh-Hant-TW"
fn validate_language_tag(tag: &str) -> Result<(), InitError> {
    // Split by hyphen to get subtags.
    // 按连字符分割获取子标签。
    let parts: Vec<&str> = tag.split('-').collect();

    // The primary language subtag must be 2-3 ASCII letters.
    // 主语言子标签必须是 2-3 个 ASCII 字母。
    if parts.is_empty() {
        return Err(InitError::InvalidInput(
            "language tag must not be empty".to_string(),
        ));
    }

    let primary = parts[0];
    if primary.len() < 2 || primary.len() > 3 || !primary.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(InitError::InvalidInput(format!(
            "invalid BCP 47 language tag '{tag}': primary subtag must be 2-3 letters (e.g., 'zh', 'en')"
        )));
    }

    // Each subsequent subtag must be 1-8 alphanumeric characters.
    // 每个后续子标签必须是 1-8 个字母数字字符。
    for (i, part) in parts.iter().enumerate().skip(1) {
        if part.is_empty() || part.len() > 8 || !part.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(InitError::InvalidInput(format!(
                "invalid BCP 47 language tag '{tag}': subtag {} ('{}') must be 1-8 alphanumeric characters",
                i + 1, part
            )));
        }
    }

    Ok(())
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Test: A normal initialization should create all expected files and directories.
    ///
    /// 测试：正常初始化应创建所有预期的文件和目录。
    #[test]
    fn test_init_creates_project() {
        // Create a temporary directory for this test.
        // 为此测试创建一个临时目录。
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("my-novel");

        // Prepare initialization options.
        // 准备初始化选项。
        let options = InitOptions {
            name: "测试小说".to_string(),
            author: "测试作者".to_string(),
            language: "zh-CN".to_string(),
            ..Default::default()
        };

        // Run initialization — should succeed.
        // 运行初始化 — 应成功。
        let result = init(&project_dir, &options);
        assert!(result.is_ok(), "init() failed: {:?}", result.err());

        // Verify the project root directory was created.
        // 验证项目根目录已创建。
        assert!(project_dir.exists(), "project root should exist");

        // Verify the content/ subdirectory was created (default mode).
        // 验证 content/ 子目录已创建（默认模式）。
        assert!(project_dir.join("content").is_dir(), "content/ should be a directory");
        // In default mode, assets/ and extras/ should NOT be created.
        // 默认模式下，assets/ 和 extras/ 不应被创建。
        assert!(!project_dir.join("assets").exists(), "assets/ should not exist in default mode");
        assert!(!project_dir.join("extras").exists(), "extras/ should not exist in default mode");
        assert!(!project_dir.join("dist").exists(), "dist/ should not exist (created by build)");

        // Verify unicodex.toml was created.
        // 验证 unicodex.toml 已创建。
        let config_path = project_dir.join(CONFIG_FILE_NAME);
        assert!(config_path.is_file(), "unicodex.toml should exist");

        // Verify content/struct.json was created.
        // 验证 content/struct.json 已创建。
        let struct_path = project_dir.join("content").join(STRUCT_FILE_NAME);
        assert!(struct_path.is_file(), "content/struct.json should exist");

        // Verify content/chapter-001.md was created.
        // 验证 content/chapter-001.md 已创建。
        let chapter_path = project_dir.join("content").join(FIRST_CHAPTER_FILE);
        assert!(chapter_path.is_file(), "content/chapter-001.md should exist");

        // Verify the chapter content is correct.
        // 验证章节内容正确。
        let chapter_content = fs::read_to_string(&chapter_path).unwrap();
        assert!(
            chapter_content.contains("# 第一章"),
            "chapter should contain heading"
        );
        assert!(
            chapter_content.contains("开始你的创作"),
            "chapter should contain placeholder text"
        );
        assert!(
            chapter_content.contains("UCX 提示"),
            "chapter should contain guidance comments"
        );
    }

    /// Test: Initializing in a directory that already has `unicodex.toml` should fail.
    ///
    /// 测试：在已存在 `unicodex.toml` 的目录中初始化应失败。
    #[test]
    fn test_init_already_exists() {
        // Create a temporary directory and a pre-existing unicodex.toml.
        // 创建临时目录并预先创建 unicodex.toml。
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("existing-project");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join(CONFIG_FILE_NAME), "# existing").unwrap();

        // Attempt initialization — should return AlreadyExists error.
        // 尝试初始化 — 应返回 AlreadyExists 错误。
        let options = InitOptions {
            name: "重复项目".to_string(),
            author: "作者".to_string(),
            language: "zh-CN".to_string(),
            ..Default::default()
        };
        let result = init(&project_dir, &options);

        // Verify the error is AlreadyExists.
        // 验证错误为 AlreadyExists。
        assert!(result.is_err(), "should fail for existing project");
        let err = result.unwrap_err();
        assert!(
            matches!(err, InitError::AlreadyExists(_)),
            "expected AlreadyExists, got: {err:?}"
        );
    }

    /// Test: The generated `unicodex.toml` should be valid TOML that can be
    /// deserialized back into a `ProjectConfig`.
    ///
    /// 测试：生成的 `unicodex.toml` 应是有效的 TOML，
    /// 能够反序列化回 `ProjectConfig`。
    #[test]
    fn test_init_generates_valid_toml() {
        // Initialize a project.
        // 初始化一个项目。
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("toml-test");
        let options = InitOptions {
            name: "TOML测试小说".to_string(),
            author: "TOML作者".to_string(),
            language: "en".to_string(),
            ..Default::default()
        };
        init(&project_dir, &options).expect("init failed");

        // Read the generated unicodex.toml.
        // 读取生成的 unicodex.toml。
        let config_path = project_dir.join(CONFIG_FILE_NAME);
        let toml_content = fs::read_to_string(&config_path).unwrap();

        // Deserialize it — should succeed.
        // 反序列化 — 应成功。
        let config: ProjectConfig = toml::from_str(&toml_content)
            .expect("generated unicodex.toml should be valid TOML");

        // Verify key fields match the options we provided.
        // 验证关键字段与我们提供的选项匹配。
        assert_eq!(config.project.version, "1.0");
        assert_eq!(config.title.main, "TOML测试小说");
        assert_eq!(config.creators.len(), 1);
        assert_eq!(config.creators[0].name, "TOML作者");
        assert_eq!(config.creators[0].role, "author");

        // Verify the language is set in the book section.
        // 验证语言已设置在 book 段中。
        let book = config.book.expect("book section should exist");
        assert_eq!(book.language, "en");

        // Verify the UCX ID has the correct format.
        // 验证 UCX ID 格式正确。
        assert!(
            config.identifier.ucx_id.as_str().starts_with("urn:ucx:"),
            "UCX ID should start with 'urn:ucx:'"
        );

        // Verify the build section has output_dir = "dist".
        // 验证 build 段的 output_dir = "dist"。
        let build = config.build.expect("build section should exist");
        assert_eq!(build.output_dir.as_deref(), Some("dist"));
    }

    /// Test: The generated `content/struct.json` should be valid JSON that can
    /// be deserialized back into a `Structure`.
    ///
    /// 测试：生成的 `content/struct.json` 应是有效的 JSON，
    /// 能够反序列化回 `Structure`。
    #[test]
    fn test_init_generates_valid_struct_json() {
        // Initialize a project.
        // 初始化一个项目。
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("struct-test");
        let options = InitOptions {
            name: "结构测试".to_string(),
            author: "作者".to_string(),
            language: "zh-CN".to_string(),
            ..Default::default()
        };
        init(&project_dir, &options).expect("init failed");

        // Read the generated struct.json.
        // 读取生成的 struct.json。
        let struct_path = project_dir.join("content").join(STRUCT_FILE_NAME);
        let json_content = fs::read_to_string(&struct_path).unwrap();

        // Deserialize it — should succeed.
        // 反序列化 — 应成功。
        let structure: Structure = serde_json::from_str(&json_content)
            .expect("generated struct.json should be valid JSON");

        // Verify the schema URL is present.
        // 验证 schema URL 存在。
        assert_eq!(
            structure.schema.as_deref(),
            Some(STRUCT_SCHEMA_URL),
            "schema URL should match"
        );

        // Verify the version.
        // 验证版本。
        assert_eq!(structure.version, "1.0");

        // Verify there is exactly one structure node.
        // 验证恰好有一个结构节点。
        assert_eq!(structure.structure.len(), 1, "should have one chapter node");

        // Verify the node points to chapter-001.md.
        // 验证节点指向 chapter-001.md。
        let node = &structure.structure[0];
        assert_eq!(node.title, "第一章");
        assert_eq!(node.file.as_deref(), Some(FIRST_CHAPTER_FILE));
        assert!(node.is_leaf(), "the first chapter node should be a leaf");
    }

    /// Test: Initializing with a path containing `..` should fail with PathTraversal.
    ///
    /// 测试：使用包含 `..` 的路径初始化应返回 PathTraversal 错误。
    #[test]
    fn test_init_rejects_path_traversal() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        // Construct a path with `..` components (path traversal attempt).
        // 构造包含 `..` 组件的路径（路径遍历尝试）。
        let malicious_path = tmp.path().join("safe").join("..").join("..").join("evil");
        let options = InitOptions::default();

        let result = init(&malicious_path, &options);
        assert!(result.is_err(), "should reject path with '..'");
        let err = result.unwrap_err();
        assert!(
            matches!(err, InitError::PathTraversal(_)),
            "expected PathTraversal, got: {err:?}"
        );
    }

    /// Test (UX-001): Initializing with empty name or author should fail.
    ///
    /// 测试（UX-001）：使用空的 name 或 author 初始化应失败。
    #[test]
    fn test_init_rejects_empty_name() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("empty-name");

        let options = InitOptions {
            name: "".to_string(),
            author: "作者".to_string(),
            language: "zh-CN".to_string(),
            ..Default::default()
        };
        let result = init(&project_dir, &options);
        assert!(result.is_err(), "should reject empty name");
        assert!(
            matches!(result.unwrap_err(), InitError::InvalidInput(_)),
            "expected InvalidInput for empty name"
        );
    }

    /// Test (UX-001): Initializing with empty author should fail.
    ///
    /// 测试（UX-001）：使用空的 author 初始化应失败。
    #[test]
    fn test_init_rejects_empty_author() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("empty-author");

        let options = InitOptions {
            name: "书名".to_string(),
            author: "   ".to_string(),
            language: "zh-CN".to_string(),
            ..Default::default()
        };
        let result = init(&project_dir, &options);
        assert!(result.is_err(), "should reject whitespace-only author");
        assert!(
            matches!(result.unwrap_err(), InitError::InvalidInput(_)),
            "expected InvalidInput for whitespace-only author"
        );
    }

    /// Test (NEW-001): Initializing with newline in name should fail.
    ///
    /// 测试（NEW-001）：name 中包含换行符应失败。
    #[test]
    fn test_init_rejects_newline_in_name() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("newline-name");

        // Test LF (\n) in name.
        // 测试 name 中的 LF (\n)。
        let options = InitOptions {
            name: "title\nnewline".to_string(),
            author: "作者".to_string(),
            language: "zh-CN".to_string(),
            ..Default::default()
        };
        let result = init(&project_dir, &options);
        assert!(result.is_err(), "should reject LF in name");
        assert!(
            matches!(result.unwrap_err(), InitError::InvalidInput(_)),
            "expected InvalidInput for LF in name"
        );

        // Test CR (\r) in author.
        // 测试 author 中的 CR (\r)。
        let options_cr = InitOptions {
            name: "书名".to_string(),
            author: "author\rnewline".to_string(),
            language: "zh-CN".to_string(),
            ..Default::default()
        };
        let result_cr = init(&project_dir, &options_cr);
        assert!(result_cr.is_err(), "should reject CR in author");
        assert!(
            matches!(result_cr.unwrap_err(), InitError::InvalidInput(_)),
            "expected InvalidInput for CR in author"
        );
    }

    /// Test (SEC-003): Initializing with control characters in name should fail.
    ///
    /// 测试（SEC-003）：name 中包含控制字符应失败。
    #[test]
    fn test_init_rejects_control_chars() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("ctrl-chars");

        let options = InitOptions {
            name: "bad\x00name".to_string(),
            author: "作者".to_string(),
            language: "zh-CN".to_string(),
            ..Default::default()
        };
        let result = init(&project_dir, &options);
        assert!(result.is_err(), "should reject control character in name");
        assert!(
            matches!(result.unwrap_err(), InitError::InvalidInput(_)),
            "expected InvalidInput for control chars"
        );
    }

    /// Test (SEC-002): Initializing with an overlong name should fail.
    ///
    /// 测试（SEC-002）：name 超过 500 字符应失败。
    #[test]
    fn test_init_rejects_overlong_name() {
        let tmp = TempDir::new().expect("failed to create temp dir");
        let project_dir = tmp.path().join("overlong-name");

        // Create a name exceeding 500 characters.
        // 创建一个超过 500 字符的名称。
        let long_name = "a".repeat(501);
        let options = InitOptions {
            name: long_name,
            author: "作者".to_string(),
            language: "zh-CN".to_string(),
            allow_long_fields: false,
            ..Default::default()
        };
        let result = init(&project_dir, &options);
        assert!(result.is_err(), "should reject overlong name");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("exceeds maximum length"),
            "error should mention max length, got: {err_msg}"
        );

        // With allow_long_fields = true, should succeed.
        // 启用 allow_long_fields 后应成功。
        let long_name_ok = "b".repeat(501);
        let options_ok = InitOptions {
            name: long_name_ok,
            author: "作者".to_string(),
            language: "zh-CN".to_string(),
            allow_long_fields: true,
            ..Default::default()
        };
        let result_ok = init(&project_dir, &options_ok);
        assert!(result_ok.is_ok(), "should allow overlong name with allow_long_fields");
    }

    /// Test (UX-002): Invalid BCP 47 language tags should be rejected.
    ///
    /// 测试（UX-002）：无效的 BCP 47 语言标签应被拒绝。
    #[test]
    fn test_init_rejects_invalid_language_tag() {
        let tmp = TempDir::new().expect("failed to create temp dir");

        // Valid tags should succeed.
        // 有效标签应成功。
        for valid_tag in &["zh-CN", "en", "en-US", "ja", "zh-Hant-TW", "de"] {
            let project_dir = tmp.path().join(format!("valid-lang-{valid_tag}"));
            let options = InitOptions {
                name: "测试".to_string(),
                author: "作者".to_string(),
                language: valid_tag.to_string(),
                ..Default::default()
            };
            let result = init(&project_dir, &options);
            assert!(result.is_ok(), "should accept valid language tag: {valid_tag}");
        }

        // Invalid tags should fail.
        // 无效标签应失败。
        for invalid_tag in &["", "x", "toolong", "zh_CN", "en-", "123"] {
            let project_dir = tmp.path().join(format!("invalid-lang-{invalid_tag}"));
            let options = InitOptions {
                name: "测试".to_string(),
                author: "作者".to_string(),
                language: invalid_tag.to_string(),
                ..Default::default()
            };
            let result = init(&project_dir, &options);
            assert!(result.is_err(), "should reject invalid language tag: '{invalid_tag}'");
        }
    }
}
