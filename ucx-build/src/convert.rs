//! ProjectConfig → Codex conversion logic.
//!
//! Converts the development-time `unicodex.toml` (ProjectConfig) into the
//! runtime `metadata/codex.json` (Codex) format for inclusion in UCX archives.
//!
//! ProjectConfig → Codex 转换逻辑。
//! 将开发时的 `unicodex.toml`（ProjectConfig）转换为
//! 运行时的 `metadata/codex.json`（Codex）格式，用于打包到 UCX 归档中。

use ucx_types::codex::{Codex, Dates, Description, FileVersion, Identifier, Rights, Title};
use ucx_types::project::ProjectConfig;

// =============================================================================
// Public conversion function / 公开转换函数
// =============================================================================

/// Convert a `ProjectConfig` (from `unicodex.toml`) to a `Codex` (for `codex.json`).
///
/// This maps the TOML configuration sections to the JSON metadata format
/// as specified in `02-metadata-spec.md`.
///
/// 将 `ProjectConfig`（来自 `unicodex.toml`）转换为 `Codex`（用于 `codex.json`）。
/// 按照 `02-metadata-spec.md` 的规范，将 TOML 配置段映射到 JSON 元数据格式。
///
/// # Mapping rules / 映射规则
///
/// | ProjectConfig field      | Codex field       |
/// |--------------------------|-------------------|
/// | `identifier.ucx_id`     | `identifier.ucx_id` |
/// | `identifier.isbn`       | `identifier.isbn`   |
/// | `title.main`            | `title.main`        |
/// | `title.subtitle`        | `title.subtitle`    |
/// | `creators`              | `creators`          |
/// | `publisher`             | `publisher`         |
/// | `book.language`         | `language`          |
/// | `book.genre`            | `genre`             |
/// | `book.tags`             | `tags`              |
/// | `book.status`           | `status`            |
/// | `description`           | `description`       |
/// | `rights`                | `rights`            |
/// | `cover.path`            | `cover`             |
/// | `rating`                | `rating`            |
pub fn config_to_codex(config: &ProjectConfig) -> Codex {
    // --- Extract language from [book] section (required for Codex). ---
    // --- 从 [book] 段提取语言（Codex 必需字段）。 ---
    // Default to "und" (undetermined) if book section is missing.
    // 如果 book 段缺失，默认使用 "und"（未确定）。
    let language = config
        .book
        .as_ref()
        .map(|b| b.language.clone())
        .unwrap_or_else(|| "und".to_string());

    // --- Extract optional fields from [book] section. ---
    // --- 从 [book] 段提取可选字段。 ---
    let genre = config.book.as_ref().and_then(|b| b.genre.clone());
    let tags = config.book.as_ref().and_then(|b| b.tags.clone());
    let status = config.book.as_ref().and_then(|b| b.status.clone());

    // --- Map cover.path to cover string. ---
    // --- 将 cover.path 映射为 cover 字符串。 ---
    let cover = config.cover.as_ref().map(|c| c.path.clone());

    // --- Map description: reuse the same Description type. ---
    // --- 映射 description：复用相同的 Description 类型。 ---
    let description: Option<Description> = config.description.clone();

    // --- Map rights: reuse the same Rights type. ---
    // --- 映射 rights：复用相同的 Rights 类型。 ---
    let rights: Option<Rights> = config.rights.clone();

    // --- Build the Codex struct. ---
    // --- 构建 Codex 结构体。 ---
    Codex {
        // JSON Schema reference (optional, can be set by tools).
        // JSON Schema 引用（可选，可由工具设置）。
        schema: Some("https://unicodex.org/schemas/codex-v1.json".to_string()),

        // Metadata schema version — matches project.version.
        // 元数据 schema 版本 — 匹配 project.version。
        version: config.project.version.clone(),

        // Identifier block — maps ucx_id and isbn.
        // 标识符块 — 映射 ucx_id 和 isbn。
        identifier: Identifier {
            ucx_id: config.identifier.ucx_id.clone(),
            isbn: config.identifier.isbn.clone(),
            issn: None,
            doi: None,
            custom: None,
        },

        // Title block — maps main and subtitle.
        // 标题块 — 映射主标题和副标题。
        title: Title {
            main: config.title.main.clone(),
            subtitle: config.title.subtitle.clone(),
            original: None,
            short: None,
        },

        // Series information (mapped from [series] section if present).
        // 系列信息（如果存在 [series] 段则映射）。
        series: config.series.as_ref().map(|s| ucx_types::codex::Series {
            name: s.name.clone(),
            index: s.index,
            total: None,
        }),

        // Creators list — directly reused (same type in both configs).
        // 创作者列表 — 直接复用（两个配置中类型相同）。
        creators: config.creators.clone(),

        // Publisher info — directly reused.
        // 出版方信息 — 直接复用。
        publisher: config.publisher.clone(),

        // Language from [book] section.
        // 来自 [book] 段的语言。
        language,

        // Genre tags from [book] section.
        // 来自 [book] 段的体裁标签。
        genre,

        // Free-form tags from [book] section.
        // 来自 [book] 段的自由标签。
        tags,

        // Work status from [book] section.
        // 来自 [book] 段的作品状态。
        status,

        // Word count is not tracked in ProjectConfig — always None at build time.
        // ProjectConfig 中不跟踪字数 — 构建时始终为 None。
        word_count: None,

        // Description (optional).
        // 作品简介（可选）。
        description,

        // Rights/license (optional).
        // 版权/许可证（可选）。
        rights,

        // Dates — map from config.dates, auto-fill modified with today's date.
        // 日期 — 从 config.dates 映射，自动填充 modified 为当天日期。
        dates: {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            match &config.dates {
                Some(d) => Some(Dates {
                    created: d.created.clone(),
                    published: d.published.clone(),
                    modified: Some(today),
                }),
                None => Some(Dates {
                    created: None,
                    published: None,
                    modified: Some(today),
                }),
            }
        },

        // Cover image path.
        // 封面图路径。
        cover,

        // Content rating (optional).
        // 内容分级（可选）。
        rating: config.rating.clone(),

        // File version — not populated from TOML conversion;
        // managed by ucx-version module and injected separately.
        // 文件版本 — 不从 TOML 转换填充；
        // 由 ucx-version 模块管理，单独注入。
        file_version: None,
    }
}

/// Convert a `ProjectConfig` to a `Codex`, injecting the given `file_version`.
///
/// This is the primary conversion function used during build when version
/// information is available from the `ucx-version` module.
///
/// 将 `ProjectConfig` 转换为 `Codex`，同时注入指定的 `file_version`。
/// 这是构建过程中 ucx-version 模块提供版本信息时使用的主要转换函数。
///
/// # Arguments / 参数
///
/// * `config` - The project configuration from `unicodex.toml`.
///              来自 `unicodex.toml` 的项目配置。
/// * `file_version` - The file version to inject into `codex.json`.
///                     要注入到 `codex.json` 中的文件版本。
pub fn config_to_codex_with_version(
    config: &ProjectConfig,
    file_version: Option<FileVersion>,
) -> Codex {
    let mut codex = config_to_codex(config);
    codex.file_version = file_version;
    codex
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ucx_types::codex::Creator;
    use ucx_types::project::*;
    use ucx_types::UcxId;

    /// Helper: create a minimal ProjectConfig for testing.
    /// 辅助函数：创建一个最小的 ProjectConfig 用于测试。
    fn test_config() -> ProjectConfig {
        ProjectConfig {
            project: ProjectSection {
                version: "1.0".to_string(),
            },
            identifier: IdentifierSection {
                ucx_id: UcxId::parse("urn:ucx:550e8400-e29b-41d4-a716-446655440000").unwrap(),
                isbn: Some("978-3-16-148410-0".to_string()),
            },
            title: TitleSection {
                main: "测试小说".to_string(),
                subtitle: Some("副标题".to_string()),
            },
            series: None,
            creators: vec![
                Creator {
                    name: "张三".to_string(),
                    role: "author".to_string(),
                    signature_ref: None,
                },
                Creator {
                    name: "李四".to_string(),
                    role: "illustrator".to_string(),
                    signature_ref: None,
                },
            ],
            publisher: None,
            book: Some(BookSection {
                language: "zh-CN".to_string(),
                genre: Some(vec!["fantasy".to_string()]),
                tags: Some(vec!["穿越".to_string(), "重生".to_string()]),
                status: Some("ongoing".to_string()),
            }),
            description: Some(Description {
                short: Some("一句话简介".to_string()),
                long: None,
            }),
            rights: Some(Rights {
                statement: Some("Copyright 2025".to_string()),
                license: Some("CC-BY-NC-4.0".to_string()),
            }),
            cover: Some(CoverSection {
                path: "assets/cover.jpg".to_string(),
            }),
            rating: Some(ucx_types::codex::Rating {
                system: "age".to_string(),
                value: "16+".to_string(),
            }),
            dates: None,
            build: None,
            signing: None,
            version_config: None,
        }
    }

    /// Test: config_to_codex should correctly map all fields.
    /// 测试：config_to_codex 应正确映射所有字段。
    #[test]
    fn test_config_to_codex() {
        let config = test_config();
        let codex = config_to_codex(&config);

        // Verify identifier mapping.
        // 验证标识符映射。
        assert_eq!(
            codex.identifier.ucx_id.as_str(),
            "urn:ucx:550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(codex.identifier.isbn.as_deref(), Some("978-3-16-148410-0"));

        // Verify title mapping.
        // 验证标题映射。
        assert_eq!(codex.title.main, "测试小说");
        assert_eq!(codex.title.subtitle.as_deref(), Some("副标题"));

        // Verify creators are preserved.
        // 验证创作者被保留。
        assert_eq!(codex.creators.len(), 2);
        assert_eq!(codex.creators[0].name, "张三");
        assert_eq!(codex.creators[1].role, "illustrator");

        // Verify book section fields.
        // 验证 book 段字段。
        assert_eq!(codex.language, "zh-CN");
        assert_eq!(codex.genre.as_ref().unwrap(), &vec!["fantasy".to_string()]);
        assert_eq!(codex.tags.as_ref().unwrap().len(), 2);
        assert_eq!(codex.status.as_deref(), Some("ongoing"));

        // Verify description mapping.
        // 验证简介映射。
        assert_eq!(
            codex.description.as_ref().unwrap().short.as_deref(),
            Some("一句话简介")
        );

        // Verify rights mapping.
        // 验证版权映射。
        assert_eq!(
            codex.rights.as_ref().unwrap().license.as_deref(),
            Some("CC-BY-NC-4.0")
        );

        // Verify cover path mapping.
        // 验证封面路径映射。
        assert_eq!(codex.cover.as_deref(), Some("assets/cover.jpg"));

        // Verify rating mapping.
        // 验证分级映射。
        assert_eq!(codex.rating.as_ref().unwrap().value, "16+");

        // Verify version mapping.
        // 验证版本映射。
        assert_eq!(codex.version, "1.0");
    }

    /// Test: config_to_codex should use "und" language when book section is missing.
    /// 测试：当 book 段缺失时，config_to_codex 应使用 "und" 语言。
    #[test]
    fn test_config_to_codex_no_book_section() {
        let mut config = test_config();
        // Remove the book section.
        // 移除 book 段。
        config.book = None;

        let codex = config_to_codex(&config);
        assert_eq!(codex.language, "und");
        assert!(codex.genre.is_none());
        assert!(codex.tags.is_none());
        assert!(codex.status.is_none());
    }

    /// Test: config_to_codex output should serialize to valid JSON.
    /// 测试：config_to_codex 的输出应可序列化为有效的 JSON。
    #[test]
    fn test_config_to_codex_serializes_to_json() {
        let config = test_config();
        let codex = config_to_codex(&config);
        let json = serde_json::to_string_pretty(&codex).expect("codex should serialize to JSON");

        // Verify key fields appear in JSON output.
        // 验证 JSON 输出中包含关键字段。
        assert!(json.contains("\"ucx_id\""));
        assert!(json.contains("测试小说"));
        assert!(json.contains("zh-CN"));

        // Verify it round-trips through deserialization.
        // 验证可以通过反序列化往返。
        let _: ucx_types::Codex =
            serde_json::from_str(&json).expect("codex JSON should deserialize");
    }

    #[test]
    fn test_config_to_codex_default_no_file_version() {
        // config_to_codex should produce a Codex with file_version = None.
        // config_to_codex 应生成 file_version = None 的 Codex。
        let config = test_config();
        let codex = config_to_codex(&config);
        assert!(codex.file_version.is_none());
    }

    #[test]
    fn test_config_to_codex_with_version_injects_file_version() {
        // config_to_codex_with_version should inject the given FileVersion.
        // config_to_codex_with_version 应注入给定的 FileVersion。
        let config = test_config();
        let fv = FileVersion {
            version: Some("1.5.0".to_string()),
            revision: Some(3),
            released_at: Some("2026-03-25T00:00:00Z".to_string()),
            changelog: Some("新增第五章".to_string()),
        };
        let codex = config_to_codex_with_version(&config, Some(fv.clone()));
        let result = codex.file_version.as_ref().unwrap();
        assert_eq!(result.version.as_deref(), Some("1.5.0"));
        assert_eq!(result.revision, Some(3));

        // Verify it serializes to JSON with file_version present.
        // 验证序列化后 JSON 包含 file_version。
        let json = serde_json::to_string_pretty(&codex).unwrap();
        assert!(json.contains("\"file_version\""));
        assert!(json.contains("1.5.0"));
    }
}
