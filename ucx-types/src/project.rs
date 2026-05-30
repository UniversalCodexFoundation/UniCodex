//! Project configuration types — corresponds to `unicodex.toml`.
//!
//! This module defines the data structures for the development-time project
//! configuration file (`unicodex.toml`), as specified in `02-metadata-spec.md §5.2`.
//! At build time, relevant sections are converted to `metadata/codex.json` and
//! other files inside the UCX archive.
//!
//! 项目配置类型 — 对应 `unicodex.toml`。
//! 本模块定义开发时的项目配置文件数据结构（规范见 `02-metadata-spec.md §5.2`）。
//! 构建时，相关段落被转换为 UCX 归档内的 `metadata/codex.json` 等文件。

use serde::{Deserialize, Serialize};

use crate::codex::{Creator, Dates, Description, Publisher, Rating, Rights};
use crate::ucx_id::UcxId;

// =============================================================================
// ProjectConfig — root config type / 根配置类型
// =============================================================================

/// The root project configuration, corresponding to `unicodex.toml`.
///
/// This is the development-time configuration. At build time, `ucx-build`
/// converts this into `codex.json`, `struct.json`, and other UCX files.
///
/// 根项目配置，对应 `unicodex.toml`。
/// 这是开发时的配置。构建时，`ucx-build` 将其转换为
/// `codex.json`、`struct.json` 等 UCX 文件。
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys/sections so a misspelled field (e.g. `verson` instead of
// `version`) or a stray `[unknown_section]` in `unicodex.toml` is reported as an
// error rather than silently ignored — upholding the "verifiable / explainable"
// principle. Safe here because no field uses `#[serde(flatten)]`.
// 拒绝未知键/段，使 `unicodex.toml` 中拼错的字段（如 `verson` 误写 `version`）或
// 多余的 `[unknown_section]` 被报错而非静默忽略——践行"可验证/可解释"原则。
// 此处安全，因为没有字段使用 `#[serde(flatten)]`。
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// Project-level settings.
    /// 项目级设置。
    pub project: ProjectSection,

    /// Work identifier settings.
    /// 作品标识符设置。
    pub identifier: IdentifierSection,

    /// Title information.
    /// 标题信息。
    pub title: TitleSection,

    /// Series information (optional).
    /// 系列信息（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<SeriesSection>,

    /// Creator list.
    /// 创作者列表。
    pub creators: Vec<Creator>,

    /// Publisher information (optional).
    /// 出版方信息（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<Publisher>,

    /// Book-level metadata.
    /// 书籍级元数据。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book: Option<BookSection>,

    /// Work description (optional).
    /// 作品简介（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,

    /// Copyright information (optional).
    /// 版权信息（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rights: Option<Rights>,

    /// Cover image configuration (optional).
    /// 封面图配置（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<CoverSection>,

    /// Content rating (optional).
    /// 内容分级（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<Rating>,

    /// Important dates (optional).
    /// 重要日期（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dates: Option<Dates>,

    /// Build configuration (optional).
    /// 构建配置（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildSection>,

    /// Signing configuration (optional, not included in UCX output).
    /// 签名配置（可选，不包含在 UCX 输出中）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing: Option<SigningSection>,

    /// Version management configuration (optional).
    /// Maps to the `[version]` section in `unicodex.toml`.
    /// 版本管理配置（可选）。
    /// 对应 `unicodex.toml` 中的 `[version]` 段。
    #[serde(rename = "version", skip_serializing_if = "Option::is_none")]
    pub version_config: Option<VersionSection>,
}

// =============================================================================
// Sub-sections / 子段
// =============================================================================

/// The `[project]` section of `unicodex.toml`.
///
/// `unicodex.toml` 的 `[project]` 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys so typos in `[project]` surface as errors. / 拒绝未知键，使 `[project]` 中的拼写错误报错。
#[serde(deny_unknown_fields)]
pub struct ProjectSection {
    /// UCX specification version (e.g., "1.0").
    /// UCX 规范版本（如 "1.0"）。
    pub version: String,
}

impl ProjectSection {
    /// Validate that `version` matches an acceptable spec version format.
    ///
    /// Acceptable shapes, matched case-insensitively for the pre-release tag:
    /// - `MAJOR.MINOR`                     (e.g., `1.0`)
    /// - `MAJOR.MINOR.PATCH`               (e.g., `1.0.0`)
    /// - `MAJOR.MINOR.PATCH-pre`           (e.g., `1.0.0-alpha.1`, `0.4.0-beta`)
    ///
    /// Explicit rejections (matching the ROB-4 checklist):
    /// - single-component or prefix like `v1`, `v1.0` (leading `v`);
    /// - empty string;
    /// - negative-looking components (`-1.0.0`);
    /// - four-component `MAJOR.MINOR.PATCH.EXTRA` (`1.0.0.0`);
    /// - trailing-dash pre-release (`1.0.0-`) or empty-segment forms.
    ///
    /// 校验 `version` 字段是否为可接受的规范版本字符串。
    pub fn validate_version(&self) -> Result<(), VersionFormatError> {
        validate_project_version(&self.version)
    }
}

/// Errors produced by [`ProjectSection::validate_version`].
///
/// [`ProjectSection::validate_version`] 产生的错误。
#[derive(Debug, thiserror::Error)]
pub enum VersionFormatError {
    /// The version string does not match the accepted format.
    /// 版本字符串不符合允许的格式。
    #[error("invalid project version '{0}': {1}")]
    Invalid(String, &'static str),
}

/// Validate a version string according to ROB-4 rules.
///
/// This helper lives at module level so callers (e.g., `ucx-build check`)
/// can reuse it without constructing a full `ProjectSection`.
///
/// 按 ROB-4 规则校验版本字符串；公开供 `ucx-build check` 等复用。
pub fn validate_project_version(v: &str) -> Result<(), VersionFormatError> {
    // Quick rejects.
    // 快速拒绝。
    if v.is_empty() {
        return Err(VersionFormatError::Invalid(
            v.to_string(),
            "version must not be empty",
        ));
    }
    if v.starts_with('v') || v.starts_with('V') {
        return Err(VersionFormatError::Invalid(
            v.to_string(),
            "version must not have a leading 'v'",
        ));
    }
    if v.starts_with('-') || v.starts_with('.') {
        return Err(VersionFormatError::Invalid(
            v.to_string(),
            "version must not start with '-' or '.'",
        ));
    }

    // Split off the optional pre-release (after the first '-').
    // 拆出可选的 pre-release（首个 '-' 之后）。
    let (numeric_part, pre_part) = match v.split_once('-') {
        Some((n, p)) => (n, Some(p)),
        None => (v, None),
    };

    // Numeric part must have 2 or 3 dot-separated components, each a non-negative
    // integer with no internal sign, and non-empty.
    // 数字部分必须是 2 或 3 个点分段，每段为非负整数，内部无符号且非空。
    let segments: Vec<&str> = numeric_part.split('.').collect();
    if segments.len() < 2 || segments.len() > 3 {
        return Err(VersionFormatError::Invalid(
            v.to_string(),
            "expected 2 or 3 dot-separated numeric components (e.g. '1.0' or '1.0.0')",
        ));
    }
    for seg in &segments {
        if seg.is_empty() {
            return Err(VersionFormatError::Invalid(
                v.to_string(),
                "numeric component must not be empty",
            ));
        }
        // Ensure only ASCII digits — this also rejects '-1' or '+1'.
        // 仅允许 ASCII 数字 — 同时拒绝 '-1' 或 '+1'。
        if !seg.bytes().all(|b| b.is_ascii_digit()) {
            return Err(VersionFormatError::Invalid(
                v.to_string(),
                "numeric component must contain only ASCII digits",
            ));
        }
    }

    // Pre-release part (if present): must be non-empty; each dot-separated
    // identifier must be non-empty and composed of `[A-Za-z0-9-]`.
    // 如有 pre-release 部分：整体非空；每个点分标识非空且仅含 `[A-Za-z0-9-]`。
    if let Some(pre) = pre_part {
        if pre.is_empty() {
            return Err(VersionFormatError::Invalid(
                v.to_string(),
                "pre-release suffix must not be empty",
            ));
        }
        for id in pre.split('.') {
            if id.is_empty() {
                return Err(VersionFormatError::Invalid(
                    v.to_string(),
                    "pre-release identifier must not be empty",
                ));
            }
            if !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                return Err(VersionFormatError::Invalid(
                    v.to_string(),
                    "pre-release identifier must match [A-Za-z0-9-]+",
                ));
            }
        }
    }

    Ok(())
}

/// The `[identifier]` section of `unicodex.toml`.
///
/// `unicodex.toml` 的 `[identifier]` 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys in `[identifier]`. / 拒绝 `[identifier]` 中的未知键。
#[serde(deny_unknown_fields)]
pub struct IdentifierSection {
    /// UCX unique identifier.
    /// UCX 唯一标识。
    pub ucx_id: UcxId,

    /// ISBN (optional).
    /// ISBN（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,
}

/// The `[title]` section of `unicodex.toml`.
///
/// `unicodex.toml` 的 `[title]` 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys in `[title]`. / 拒绝 `[title]` 中的未知键。
#[serde(deny_unknown_fields)]
pub struct TitleSection {
    /// Main title.
    /// 主标题。
    pub main: String,

    /// Subtitle (optional).
    /// 副标题（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
}

/// The `[series]` section of `unicodex.toml`.
///
/// `unicodex.toml` 的 `[series]` 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys in `[series]`. / 拒绝 `[series]` 中的未知键。
#[serde(deny_unknown_fields)]
pub struct SeriesSection {
    /// Series name.
    /// 系列名称。
    pub name: String,

    /// Position in the series (1-based).
    /// 系列中的序号（从 1 开始）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

/// The `[book]` section of `unicodex.toml` — book-level metadata.
///
/// `unicodex.toml` 的 `[book]` 段 — 书籍级元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys in `[book]`. / 拒绝 `[book]` 中的未知键。
#[serde(deny_unknown_fields)]
pub struct BookSection {
    /// Primary language (BCP 47 tag).
    /// 主要语言（BCP 47 标签）。
    pub language: String,

    /// Genre tags (optional).
    /// 体裁标签（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<Vec<String>>,

    /// Free-form tags (optional).
    /// 自由标签（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Work status (optional).
    /// 作品状态（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// The `[cover]` section of `unicodex.toml`.
///
/// `unicodex.toml` 的 `[cover]` 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys in `[cover]`. / 拒绝 `[cover]` 中的未知键。
#[serde(deny_unknown_fields)]
pub struct CoverSection {
    /// Path to the cover image (relative to project root).
    /// 封面图路径（相对于项目根目录）。
    pub path: String,
}

/// The `[build]` section of `unicodex.toml`.
///
/// `unicodex.toml` 的 `[build]` 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys in `[build]`. / 拒绝 `[build]` 中的未知键。
#[serde(deny_unknown_fields)]
pub struct BuildSection {
    /// Output directory for built UCX files (default: "dist").
    /// 构建输出目录（默认："dist"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,

    /// Output file name without extension (default: project directory name).
    /// 输出文件名，不含扩展名（默认：项目目录名）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_name: Option<String>,

    /// Compression method (default: "deflate").
    /// 压缩方式（默认："deflate"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression: Option<String>,
}

/// The `[signing]` section of `unicodex.toml`.
/// This is used for the signing process and is NOT written into the UCX file.
///
/// `unicodex.toml` 的 `[signing]` 段。
/// 用于签名过程，不写入 UCX 文件。
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys in `[signing]`. / 拒绝 `[signing]` 中的未知键。
#[serde(deny_unknown_fields)]
pub struct SigningSection {
    /// Path to the private key file.
    /// 私钥文件路径。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,

    /// Path to the certificate file.
    /// 证书文件路径。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_path: Option<String>,

    /// Signer identifier (e.g., "AUTHOR").
    /// 签名者标识（如 "AUTHOR"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer_id: Option<String>,
}

/// The `[version]` section of `unicodex.toml` — version management configuration.
///
/// Controls how UCX file versioning behaves during development and build.
///
/// `unicodex.toml` 的 `[version]` 段 — 版本管理配置。
/// 控制开发和构建过程中 UCX 文件版本的行为。
///
/// # Example TOML / TOML 示例
///
/// ```toml
/// [version]
/// strategy = "auto"
/// auto_on_build = true
/// semantic = "volume.chapter.patch"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
// Reject unknown keys in `[version]`. / 拒绝 `[version]` 中的未知键。
#[serde(deny_unknown_fields)]
pub struct VersionSection {
    /// Version strategy: "auto" (recommended) or "manual".
    /// 版本策略："auto"（推荐）或 "manual"。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,

    /// Whether to automatically run `ucx version auto` during `ucx build`.
    /// 是否在 `ucx build` 时自动执行 `ucx version auto`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_on_build: Option<bool>,

    /// Version semantic scheme: "volume.chapter.patch" (default) or "semver".
    /// 版本语义方案："volume.chapter.patch"（默认）或 "semver"。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic: Option<String>,
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal ProjectConfig for testing.
    /// 辅助函数：创建一个最小的 ProjectConfig 用于测试。
    fn minimal_config() -> ProjectConfig {
        ProjectConfig {
            project: ProjectSection {
                version: "1.0".to_string(),
            },
            identifier: IdentifierSection {
                ucx_id: UcxId::parse("urn:ucx:550e8400-e29b-41d4-a716-446655440000").unwrap(),
                isbn: None,
            },
            title: TitleSection {
                main: "测试小说".to_string(),
                subtitle: None,
            },
            series: None,
            creators: vec![Creator {
                name: "测试作者".to_string(),
                role: "author".to_string(),
                signature_ref: None,
            }],
            publisher: None,
            book: Some(BookSection {
                language: "zh-CN".to_string(),
                genre: None,
                tags: None,
                status: None,
            }),
            description: None,
            rights: None,
            cover: None,
            rating: None,
            dates: None,
            build: None,
            signing: None,
            version_config: None,
        }
    }

    #[test]
    fn test_toml_serialize() {
        // ProjectConfig should serialize to valid TOML.
        // ProjectConfig 应序列化为有效的 TOML。
        let config = minimal_config();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[project]"));
        assert!(toml_str.contains("[identifier]"));
        assert!(toml_str.contains("[title]"));
        assert!(toml_str.contains("测试小说"));
    }

    #[test]
    fn test_toml_deserialize() {
        // A TOML string should deserialize to ProjectConfig.
        // TOML 字符串应反序列化为 ProjectConfig。
        let toml_str = r#"
[project]
version = "1.0"

[identifier]
ucx_id = "urn:ucx:550e8400-e29b-41d4-a716-446655440000"

[title]
main = "我的小说"

[[creators]]
name = "作者"
role = "author"

[book]
language = "zh-CN"
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.title.main, "我的小说");
        assert_eq!(config.creators.len(), 1);
        assert_eq!(config.book.unwrap().language, "zh-CN");
    }

    /// Regression (M-2): unknown / misspelled keys and stray sections in
    /// `unicodex.toml` must be REJECTED rather than silently ignored, so a typo
    /// like `verson` (instead of `version`) surfaces as an error.
    ///
    /// 回归（M-2）：`unicodex.toml` 中未知/拼错的键与多余段必须被**拒绝**而非
    /// 静默忽略，使 `verson`（误写 `version`）这样的拼写错误暴露为错误。
    #[test]
    fn test_toml_rejects_unknown_fields() {
        // (a) misspelled top-level key inside a known section.
        // (a) 已知段内拼错的键。
        let typo_in_project = r#"
[project]
version = "1.0"
verson = "1.0"

[identifier]
ucx_id = "urn:ucx:550e8400-e29b-41d4-a716-446655440000"

[title]
main = "我的小说"

[[creators]]
name = "作者"
role = "author"
"#;
        assert!(
            toml::from_str::<ProjectConfig>(typo_in_project).is_err(),
            "misspelled key 'verson' in [project] must be rejected"
        );

        // (b) an entirely unknown top-level section.
        // (b) 完全未知的顶层段。
        let unknown_section = r#"
[project]
version = "1.0"

[identifier]
ucx_id = "urn:ucx:550e8400-e29b-41d4-a716-446655440000"

[title]
main = "我的小说"

[[creators]]
name = "作者"
role = "author"

[unknown_section]
foo = "bar"
"#;
        assert!(
            toml::from_str::<ProjectConfig>(unknown_section).is_err(),
            "stray [unknown_section] must be rejected"
        );
    }

    #[test]
    fn test_toml_round_trip() {
        // Serialize then deserialize should be equivalent.
        // 序列化后反序列化应等价。
        let original = minimal_config();
        let toml_str = toml::to_string_pretty(&original).unwrap();
        let deserialized: ProjectConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.title.main, original.title.main);
        assert_eq!(deserialized.creators.len(), original.creators.len());
    }

    #[test]
    fn test_version_section_toml_round_trip() {
        // VersionSection should round-trip through TOML correctly.
        // VersionSection 应通过 TOML 正确往返。
        let vs = VersionSection {
            strategy: Some("auto".to_string()),
            auto_on_build: Some(true),
            semantic: Some("volume.chapter.patch".to_string()),
        };
        let toml_str = toml::to_string_pretty(&vs).unwrap();
        let deserialized: VersionSection = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.strategy.as_deref(), Some("auto"));
        assert_eq!(deserialized.auto_on_build, Some(true));
        assert_eq!(deserialized.semantic.as_deref(), Some("volume.chapter.patch"));
    }

    #[test]
    fn test_project_config_with_version_section() {
        // ProjectConfig with [version] section should parse correctly.
        // 包含 [version] 段的 ProjectConfig 应正确解析。
        let toml_str = r#"
[project]
version = "1.0"

[identifier]
ucx_id = "urn:ucx:550e8400-e29b-41d4-a716-446655440000"

[title]
main = "版本测试"

[[creators]]
name = "作者"
role = "author"

[book]
language = "zh-CN"

[version]
strategy = "auto"
auto_on_build = true
semantic = "volume.chapter.patch"
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.title.main, "版本测试");
        let vc = config.version_config.unwrap();
        assert_eq!(vc.strategy.as_deref(), Some("auto"));
        assert_eq!(vc.auto_on_build, Some(true));
    }

    // =========================================================================
    // ROB-4: 版本字符串格式校验测试
    // =========================================================================

    #[test]
    fn test_validate_version_accepts_two_component() {
        // "1.0" is the minimum acceptable shape.
        // "1.0" 为最小可接受形式。
        assert!(validate_project_version("1.0").is_ok());
        assert!(validate_project_version("10.20").is_ok());
    }

    #[test]
    fn test_validate_version_accepts_three_component() {
        // "1.0.0" is the classic semver numeric core.
        // "1.0.0" 为标准 semver 数字核心。
        assert!(validate_project_version("1.0.0").is_ok());
        assert!(validate_project_version("0.4.0").is_ok());
    }

    #[test]
    fn test_validate_version_accepts_pre_release() {
        // Pre-release suffix is permitted.
        // 允许 pre-release 后缀。
        assert!(validate_project_version("1.0.0-alpha").is_ok());
        assert!(validate_project_version("1.0.0-alpha.1").is_ok());
        assert!(validate_project_version("1.0.0-rc.2").is_ok());
        assert!(validate_project_version("0.4.0-beta").is_ok());
    }

    #[test]
    fn test_validate_version_rejects_v_prefix() {
        // Leading 'v' must be rejected.
        // 前缀 'v' 必须拒绝。
        assert!(validate_project_version("v1").is_err());
        assert!(validate_project_version("v1.0").is_err());
        assert!(validate_project_version("V2.0").is_err());
    }

    #[test]
    fn test_validate_version_rejects_empty() {
        // Empty string must be rejected.
        // 空串必须拒绝。
        assert!(validate_project_version("").is_err());
    }

    #[test]
    fn test_validate_version_rejects_negative() {
        // "-1.0.0" must be rejected (starts with '-').
        // "-1.0.0" 必须拒绝（以 '-' 开头）。
        assert!(validate_project_version("-1.0.0").is_err());
    }

    #[test]
    fn test_validate_version_rejects_four_component() {
        // "1.0.0.0" exceeds the allowed MAJOR.MINOR.PATCH shape.
        // "1.0.0.0" 超出允许的 MAJOR.MINOR.PATCH 形式。
        assert!(validate_project_version("1.0.0.0").is_err());
    }

    #[test]
    fn test_validate_version_rejects_trailing_dash() {
        // "1.0.0-" has an empty pre-release suffix.
        // "1.0.0-" 的 pre-release 后缀为空。
        assert!(validate_project_version("1.0.0-").is_err());
    }

    #[test]
    fn test_validate_version_rejects_single_component() {
        // Single-component "1" is not enough.
        // 仅单段 "1" 不足。
        assert!(validate_project_version("1").is_err());
    }

    #[test]
    fn test_validate_version_rejects_non_numeric() {
        // Numeric components must be ASCII digits.
        // 数字段必须为 ASCII 数字。
        assert!(validate_project_version("1.a").is_err());
        assert!(validate_project_version("1.0.b").is_err());
    }

    #[test]
    fn test_validate_version_rejects_empty_numeric_segment() {
        // Double-dot or trailing-dot must be rejected.
        // 双点或尾部句点必须拒绝。
        assert!(validate_project_version("1..0").is_err());
        assert!(validate_project_version("1.0.").is_err());
    }

    #[test]
    fn test_validate_version_rejects_empty_pre_release_segment() {
        // Trailing or empty pre-release identifier must be rejected.
        // pre-release 尾部或空标识必须拒绝。
        assert!(validate_project_version("1.0.0-alpha.").is_err());
        assert!(validate_project_version("1.0.0-.alpha").is_err());
    }

    #[test]
    fn test_project_config_backward_compat_no_version() {
        // ProjectConfig without [version] section should still parse.
        // 不包含 [version] 段的 ProjectConfig 应仍能解析。
        let toml_str = r#"
[project]
version = "1.0"

[identifier]
ucx_id = "urn:ucx:550e8400-e29b-41d4-a716-446655440000"

[title]
main = "兼容测试"

[[creators]]
name = "作者"
role = "author"

[book]
language = "zh-CN"
"#;
        let config: ProjectConfig = toml::from_str(toml_str).unwrap();
        assert!(config.version_config.is_none());
    }
}
