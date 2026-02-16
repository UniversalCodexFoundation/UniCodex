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

use crate::codex::{Creator, Description, Publisher, Rating, Rights};
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

    /// Build configuration (optional).
    /// 构建配置（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildSection>,

    /// Signing configuration (optional, not included in UCX output).
    /// 签名配置（可选，不包含在 UCX 输出中）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing: Option<SigningSection>,
}

// =============================================================================
// Sub-sections / 子段
// =============================================================================

/// The `[project]` section of `unicodex.toml`.
///
/// `unicodex.toml` 的 `[project]` 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSection {
    /// UCX specification version (e.g., "1.0").
    /// UCX 规范版本（如 "1.0"）。
    pub version: String,
}

/// The `[identifier]` section of `unicodex.toml`.
///
/// `unicodex.toml` 的 `[identifier]` 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct CoverSection {
    /// Path to the cover image (relative to project root).
    /// 封面图路径（相对于项目根目录）。
    pub path: String,
}

/// The `[build]` section of `unicodex.toml`.
///
/// `unicodex.toml` 的 `[build]` 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            build: None,
            signing: None,
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
}
