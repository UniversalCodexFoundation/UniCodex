//! Codex metadata types — corresponds to `metadata/codex.json`.
//!
//! This module defines the data structures for the UCX work metadata,
//! as specified in `02-metadata-spec.md`. The `Codex` struct is the root
//! type that maps directly to the `codex.json` file inside a UCX archive.
//!
//! 作品核心元数据类型 — 对应 `metadata/codex.json`。
//! 本模块定义 UCX 作品元数据的数据结构（规范见 `02-metadata-spec.md`）。
//! `Codex` 结构体是根类型，直接映射到 UCX 归档中的 `codex.json` 文件。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ucx_id::UcxId;

// =============================================================================
// Codex — root metadata type / 根元数据类型
// =============================================================================

/// The root metadata structure for a UCX work, corresponding to `codex.json`.
///
/// Required fields: `identifier.ucx_id`, `title.main`, `creators` (≥1), `language`.
///
/// UCX 作品的根元数据结构，对应 `codex.json`。
/// 必需字段：`identifier.ucx_id`、`title.main`、`creators`（至少一个）、`language`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Codex {
    /// JSON Schema URL (optional).
    /// JSON Schema URL（可选）。
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// UCX metadata schema version (e.g., "1.0").
    /// UCX 元数据 schema 版本（如 "1.0"）。
    pub version: String,

    /// Work identifiers (UCX ID, ISBN, etc.).
    /// 作品标识符（UCX ID、ISBN 等）。
    pub identifier: Identifier,

    /// Work title information.
    /// 作品标题信息。
    pub title: Title,

    /// Series information (optional).
    /// 系列信息（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<Series>,

    /// List of creators (authors, illustrators, etc.).
    /// 创作者列表（作者、插画师等）。
    pub creators: Vec<Creator>,

    /// Publisher information (optional).
    /// 出版方信息（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<Publisher>,

    /// Primary language of the work (BCP 47 tag, e.g., "zh-CN").
    /// 作品的主要语言（BCP 47 标签，如 "zh-CN"）。
    pub language: String,

    /// Genre/category tags (optional).
    /// 体裁/分类标签（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<Vec<String>>,

    /// Free-form tags/keywords (optional).
    /// 自由标签/关键词（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Work status: ongoing, completed, hiatus, abandoned, draft (optional).
    /// 作品状态：ongoing, completed, hiatus, abandoned, draft（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Total word count (optional).
    /// 总字数（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_count: Option<u64>,

    /// Work description/synopsis (optional).
    /// 作品简介（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,

    /// Copyright and licensing information (optional).
    /// 版权与许可证信息（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rights: Option<Rights>,

    /// Important dates (created, published, modified) (optional).
    /// 重要日期（创建、发布、修改）（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dates: Option<Dates>,

    /// Cover image path relative to the ZIP root (optional).
    /// 封面图路径，相对于 ZIP 根目录（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,

    /// Content rating information (optional).
    /// 内容分级信息（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<Rating>,

    /// UCX file version information (optional, managed by ucx-version).
    /// UCX 文件版本信息（可选，由 ucx-version 管理）。
    ///
    /// This tracks the work's revision history (volume.chapter.patch),
    /// not the metadata schema version (which is in the `version` field).
    /// 此字段追踪作品的修订历史（卷.章.修订），
    /// 而非元数据 schema 版本（记录在 `version` 字段中）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_version: Option<FileVersion>,
}

// =============================================================================
// Identifier / 标识符
// =============================================================================

/// Work identifiers — at minimum contains the UCX ID.
///
/// 作品标识符 — 至少包含 UCX ID。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identifier {
    /// The UCX unique identifier (format: `urn:ucx:{UUID v4}`).
    /// UCX 唯一标识（格式：`urn:ucx:{UUID v4}`）。
    pub ucx_id: UcxId,

    /// ISBN (optional, for formally published works).
    /// ISBN（可选，用于正式出版物）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,

    /// ISSN (optional, for serial publications).
    /// ISSN（可选，用于连载刊物）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issn: Option<String>,

    /// DOI (optional, for academic/digital publications).
    /// DOI（可选，用于学术/数字出版）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,

    /// Custom identifiers (e.g., platform-specific IDs).
    /// 自定义标识符（如平台内部 ID）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, String>>,
}

// =============================================================================
// Title / 标题
// =============================================================================

/// Work title information.
///
/// 作品标题信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Title {
    /// The main title of the work (required).
    /// 作品主标题（必需）。
    pub main: String,

    /// Subtitle (optional).
    /// 副标题（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,

    /// Original title for translated works (optional).
    /// 翻译作品的原文标题（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,

    /// Short name / abbreviation (optional).
    /// 简称/缩写（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
}

// =============================================================================
// Series / 系列
// =============================================================================

/// Series information for works that belong to a series.
///
/// 系列信息，用于属于系列的作品。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Series {
    /// Name of the series.
    /// 系列名称。
    pub name: String,

    /// Position of this work in the series (1-based).
    /// 本作在系列中的序号（从 1 开始）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,

    /// Total number of works in the series (if known).
    /// 系列总数（已知时填写）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
}

// =============================================================================
// Creator / 创作者
// =============================================================================

/// A creator (author, illustrator, etc.) of the work.
///
/// 作品的创作者（作者、插画师等）。
///
/// Standard role values: `author`, `coauthor`, `translator`, `editor`,
/// `illustrator`, `cover_artist`, `proofreader`, `narrator`, `adapter`.
/// Custom roles are also accepted.
///
/// 标准角色值：`author`、`coauthor`、`translator`、`editor`、
/// `illustrator`、`cover_artist`、`proofreader`、`narrator`、`adapter`。
/// 也接受自定义角色。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    /// Creator's name or pen name.
    /// 创作者名称或笔名。
    pub name: String,

    /// Role identifier (e.g., "author", "illustrator").
    /// 角色标识（如 "author"、"illustrator"）。
    pub role: String,

    /// Reference to the signer ID in META-INF/signatures/ (optional).
    /// 关联 META-INF/signatures/ 中的签名者 ID（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_ref: Option<String>,
}

// =============================================================================
// Publisher / 出版方
// =============================================================================

/// Publisher information.
///
/// 出版方信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publisher {
    /// Publisher name.
    /// 出版方名称。
    pub name: String,

    /// Imprint / sub-brand (optional).
    /// 出版品牌/副牌（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imprint: Option<String>,

    /// Reference to the signer ID (optional).
    /// 关联的签名者 ID（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_ref: Option<String>,
}

// =============================================================================
// Description / 作品简介
// =============================================================================

/// Work description / synopsis.
///
/// 作品简介。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    /// One-line synopsis (≤ 100 characters) (optional).
    /// 一句话简介（≤ 100 字符）（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,

    /// Detailed synopsis (supports newline `\n`) (optional).
    /// 详细简介（支持换行符 `\n`）（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<String>,
}

// =============================================================================
// Rights / 版权信息
// =============================================================================

/// Copyright and licensing information.
///
/// 版权与许可证信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rights {
    /// Copyright statement text (optional).
    /// 版权声明文本（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<String>,

    /// License identifier in SPDX format or custom (optional).
    /// SPDX 格式或自定义的许可证标识（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

// =============================================================================
// Dates / 日期信息
// =============================================================================

/// Important dates for the work (ISO 8601 format: `YYYY-MM-DD`).
///
/// 作品的重要日期（ISO 8601 格式：`YYYY-MM-DD`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dates {
    /// Creation date (optional).
    /// 创作日期（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    /// Publication date (optional).
    /// 出版/发布日期（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,

    /// Last modification date (optional).
    /// 最后修改日期（可选）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

// =============================================================================
// Rating / 分级信息
// =============================================================================

/// Content rating information.
///
/// 内容分级信息。
///
/// Rating systems: `age` (values: "all", "12+", "16+", "18+"), `custom`.
/// 分级系统：`age`（值："all"、"12+"、"16+"、"18+"）、`custom`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    /// Rating system identifier (e.g., "age", "custom").
    /// 分级系统标识（如 "age"、"custom"）。
    pub system: String,

    /// Rating value (e.g., "16+").
    /// 分级值（如 "16+"）。
    pub value: String,
}

// =============================================================================
// FileVersion / 文件版本
// =============================================================================

/// UCX file version information, stored in `codex.json` under `file_version`.
///
/// Tracks the work's revision history using the recommended X.Y.Z
/// (volume.chapter.patch) scheme. All fields are optional to allow
/// incremental adoption.
///
/// UCX 文件版本信息，存储在 `codex.json` 的 `file_version` 中。
/// 使用推荐的 X.Y.Z（卷.章.修订）方案追踪作品修订历史。
/// 所有字段可选，支持渐进式采用。
///
/// # Example JSON / JSON 示例
///
/// ```json
/// {
///   "version": "1.12.0",
///   "revision": 15,
///   "released_at": "2025-12-01T00:00:00Z",
///   "changelog": "修复第三章错别字，新增第十二章"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileVersion {
    /// Version string in X.Y.Z format (e.g., "1.12.0").
    /// X.Y.Z 格式的版本字符串（如 "1.12.0"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Revision counter, auto-incremented with each build.
    /// 修订计数器，每次构建自动递增。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,

    /// Publication timestamp in ISO 8601 format.
    /// ISO 8601 格式的发布时间戳。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,

    /// Changelog / update notes for this version.
    /// 此版本的更新说明。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal valid Codex for testing.
    /// 辅助函数：创建一个最小有效的 Codex 用于测试。
    fn minimal_codex() -> Codex {
        Codex {
            schema: Some("https://unicodex.org/schemas/codex-v1.json".to_string()),
            version: "1.0".to_string(),
            identifier: Identifier {
                ucx_id: UcxId::parse("urn:ucx:550e8400-e29b-41d4-a716-446655440000").unwrap(),
                isbn: None,
                issn: None,
                doi: None,
                custom: None,
            },
            title: Title {
                main: "我的小说".to_string(),
                subtitle: None,
                original: None,
                short: None,
            },
            series: None,
            creators: vec![Creator {
                name: "作者".to_string(),
                role: "author".to_string(),
                signature_ref: None,
            }],
            publisher: None,
            language: "zh-CN".to_string(),
            genre: None,
            tags: None,
            status: None,
            word_count: None,
            description: None,
            rights: None,
            dates: None,
            cover: None,
            rating: None,
            file_version: None,
        }
    }

    #[test]
    fn test_codex_serialize_minimal() {
        // Minimal codex should serialize to valid JSON matching the spec.
        // 最小 codex 应序列化为符合规范的有效 JSON。
        let codex = minimal_codex();
        let json = serde_json::to_string_pretty(&codex).unwrap();
        assert!(json.contains("\"ucx_id\""));
        assert!(json.contains("\"我的小说\""));
        assert!(json.contains("\"zh-CN\""));
    }

    #[test]
    fn test_codex_deserialize_minimal() {
        // The minimal JSON example from the spec should deserialize correctly.
        // 规范中的最小 JSON 示例应正确反序列化。
        let json = r#"{
            "$schema": "https://unicodex.org/schemas/codex-v1.json",
            "version": "1.0",
            "identifier": {
                "ucx_id": "urn:ucx:550e8400-e29b-41d4-a716-446655440000"
            },
            "title": {
                "main": "我的小说"
            },
            "creators": [
                {
                    "name": "作者",
                    "role": "author"
                }
            ],
            "language": "zh-CN"
        }"#;
        let codex: Codex = serde_json::from_str(json).unwrap();
        assert_eq!(codex.title.main, "我的小说");
        assert_eq!(codex.language, "zh-CN");
        assert_eq!(codex.creators.len(), 1);
        assert_eq!(codex.creators[0].name, "作者");
    }

    #[test]
    fn test_codex_serde_round_trip() {
        // Serialize then deserialize should produce an equivalent Codex.
        // 序列化后反序列化应产生等价的 Codex。
        let original = minimal_codex();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Codex = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title.main, original.title.main);
        assert_eq!(deserialized.language, original.language);
        assert_eq!(
            deserialized.identifier.ucx_id.as_str(),
            original.identifier.ucx_id.as_str()
        );
    }

    #[test]
    fn test_codex_full_example() {
        // A full codex with all fields should serialize and deserialize correctly.
        // 包含所有字段的完整 codex 应正确序列化和反序列化。
        let codex = Codex {
            schema: Some("https://unicodex.org/schemas/codex-v1.json".to_string()),
            version: "1.0".to_string(),
            identifier: Identifier {
                ucx_id: UcxId::parse("urn:ucx:550e8400-e29b-41d4-a716-446655440000").unwrap(),
                isbn: Some("978-3-16-148410-0".to_string()),
                issn: None,
                doi: None,
                custom: Some(HashMap::from([(
                    "platform_id".to_string(),
                    "qidian-123456".to_string(),
                )])),
            },
            title: Title {
                main: "作品主标题".to_string(),
                subtitle: Some("副标题".to_string()),
                original: None,
                short: Some("简称".to_string()),
            },
            series: Some(Series {
                name: "系列名称".to_string(),
                index: Some(1),
                total: Some(5),
            }),
            creators: vec![Creator {
                name: "张三".to_string(),
                role: "author".to_string(),
                signature_ref: Some("AUTHOR".to_string()),
            }],
            publisher: Some(Publisher {
                name: "某某出版社".to_string(),
                imprint: None,
                signature_ref: Some("PUBLISHER".to_string()),
            }),
            language: "zh-CN".to_string(),
            genre: Some(vec!["fantasy".to_string(), "romance".to_string()]),
            tags: Some(vec!["穿越".to_string(), "重生".to_string()]),
            status: Some("completed".to_string()),
            word_count: Some(1_500_000),
            description: Some(Description {
                short: Some("一句话简介".to_string()),
                long: Some("详细简介".to_string()),
            }),
            rights: Some(Rights {
                statement: Some("版权所有 © 2025 张三".to_string()),
                license: Some("CC-BY-NC-4.0".to_string()),
            }),
            dates: Some(Dates {
                created: Some("2025-01-01".to_string()),
                published: Some("2025-06-01".to_string()),
                modified: Some("2025-12-01".to_string()),
            }),
            cover: Some("assets/cover.jpg".to_string()),
            rating: Some(Rating {
                system: "age".to_string(),
                value: "16+".to_string(),
            }),
            file_version: Some(FileVersion {
                version: Some("1.12.0".to_string()),
                revision: Some(15),
                released_at: Some("2025-12-01T00:00:00Z".to_string()),
                changelog: Some("修复第三章错别字，新增第十二章".to_string()),
            }),
        };
        let json = serde_json::to_string_pretty(&codex).unwrap();
        let deserialized: Codex = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title.main, "作品主标题");
        assert_eq!(deserialized.word_count, Some(1_500_000));
        assert_eq!(deserialized.rating.unwrap().value, "16+");
        // Verify file_version round-trips correctly.
        // 验证 file_version 正确往返。
        let fv = deserialized.file_version.unwrap();
        assert_eq!(fv.version.as_deref(), Some("1.12.0"));
        assert_eq!(fv.revision, Some(15));
    }

    #[test]
    fn test_skip_serializing_none_fields() {
        // Optional fields set to None should not appear in the JSON output.
        // 设为 None 的可选字段不应出现在 JSON 输出中。
        let codex = minimal_codex();
        let json = serde_json::to_string(&codex).unwrap();
        assert!(!json.contains("\"series\""));
        assert!(!json.contains("\"publisher\""));
        assert!(!json.contains("\"genre\""));
        assert!(!json.contains("\"tags\""));
        assert!(!json.contains("\"status\""));
        assert!(!json.contains("\"word_count\""));
        assert!(!json.contains("\"description\""));
        assert!(!json.contains("\"rights\""));
        assert!(!json.contains("\"dates\""));
        assert!(!json.contains("\"cover\""));
        assert!(!json.contains("\"rating\""));
        assert!(!json.contains("\"file_version\""));
    }

    #[test]
    fn test_file_version_serde_round_trip() {
        // FileVersion should serialize and deserialize correctly.
        // FileVersion 应正确序列化和反序列化。
        let fv = FileVersion {
            version: Some("2.3.1".to_string()),
            revision: Some(42),
            released_at: Some("2026-03-25T10:00:00Z".to_string()),
            changelog: Some("全卷校对修正".to_string()),
        };
        let json = serde_json::to_string(&fv).unwrap();
        let deserialized: FileVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, fv);
    }

    #[test]
    fn test_file_version_all_none() {
        // FileVersion with all None fields should serialize to "{}".
        // 所有字段为 None 的 FileVersion 应序列化为 "{}"。
        let fv = FileVersion {
            version: None,
            revision: None,
            released_at: None,
            changelog: None,
        };
        let json = serde_json::to_string(&fv).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_codex_backward_compat_no_file_version() {
        // JSON without file_version should still deserialize correctly.
        // This ensures backward compatibility with existing UCX files.
        // 不包含 file_version 的 JSON 应仍能正确反序列化。
        // 确保与现有 UCX 文件的向后兼容。
        let json = r#"{
            "version": "1.0",
            "identifier": {
                "ucx_id": "urn:ucx:550e8400-e29b-41d4-a716-446655440000"
            },
            "title": {
                "main": "向后兼容测试"
            },
            "creators": [
                {
                    "name": "作者",
                    "role": "author"
                }
            ],
            "language": "zh-CN"
        }"#;
        let codex: Codex = serde_json::from_str(json).unwrap();
        assert_eq!(codex.title.main, "向后兼容测试");
        assert!(codex.file_version.is_none());
    }
}
