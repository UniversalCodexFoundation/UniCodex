//! UCX Version Management Module (git2-based).
//!
//! This module manages UCX file versioning using the recommended
//! 卷.章.修订 (volume.chapter.patch) scheme. It detects changes
//! via git2 (libgit2 Rust bindings) and automatically generates
//! version numbers. Falls back to file snapshot diff when no Git
//! repository is available.
//!
//! UCX 版本管理模块（基于 git2）。
//! 使用推荐的 卷.章.修订 版本方案管理 UCX 文件版本。
//! 通过 git2（libgit2 Rust 绑定）检测变更并自动生成版本号。
//! 无 Git 仓库时回退为文件快照 diff。

pub mod snapshot;

use thiserror::Error;

// =============================================================================
// Error types / 错误类型
// =============================================================================

/// Errors that can occur during version management.
///
/// 版本管理过程中可能发生的错误。
#[derive(Debug, Error)]
pub enum VersionError {
    /// Failed to open Git repository.
    /// 打开 Git 仓库失败。
    #[error("git repository error: {0}")]
    GitError(#[from] git2::Error),

    /// No version tag found in the repository.
    /// 仓库中未找到版本标签。
    #[error("no UCX version tag found")]
    NoVersionTag,

    /// Version string parsing error.
    /// 版本字符串解析错误。
    #[error("invalid version format: {0}")]
    InvalidVersion(String),

    /// I/O error during version operations.
    /// 版本操作过程中的 I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Snapshot file error (used when Git is not available).
    /// 快照文件错误（Git 不可用时使用）。
    #[error("snapshot error: {0}")]
    Snapshot(String),

    /// struct.json parsing or analysis error.
    /// struct.json 解析或分析错误。
    #[error("structure error: {0}")]
    Structure(String),
}

// =============================================================================
// Types / 类型定义
// =============================================================================

/// Represents a UCX file version in the X.Y.Z format.
///
/// 表示 X.Y.Z 格式的 UCX 文件版本。
///
/// Recommended semantic: X = volume, Y = chapter, Z = patch.
/// 推荐语义：X = 卷号, Y = 章节数, Z = 修订号。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UcxVersion {
    /// Volume number (X). Increments when a new volume starts.
    /// 卷号（X）。开始新卷时递增。
    pub volume: u32,

    /// Chapter number (Y). Updates to the latest chapter number in the current volume.
    /// 章节数（Y）。更新为当前卷内的最新章节编号。
    pub chapter: u32,

    /// Patch number (Z). Increments on fixes; resets to 0 on new chapter.
    /// 修订号（Z）。修正时递增；发布新章节时归零。
    pub patch: u32,
}

impl UcxVersion {
    /// Create a new `UcxVersion` with the given components.
    ///
    /// 创建一个新的 `UcxVersion`。
    pub fn new(volume: u32, chapter: u32, patch: u32) -> Self {
        Self { volume, chapter, patch }
    }

    /// The zero version (0.0.0), used as the initial version before any release.
    ///
    /// 零版本（0.0.0），用作首次发布前的初始版本。
    pub fn zero() -> Self {
        Self { volume: 0, chapter: 0, patch: 0 }
    }

    /// Parse a version string in "X.Y.Z" format.
    ///
    /// 解析 "X.Y.Z" 格式的版本字符串。
    ///
    /// # Errors / 错误
    ///
    /// Returns `VersionError::InvalidVersion` if the string is not in valid
    /// X.Y.Z format where X, Y, Z are non-negative integers.
    /// 如果字符串不是有效的 X.Y.Z 格式（X、Y、Z 为非负整数），
    /// 则返回 `VersionError::InvalidVersion`。
    pub fn parse(s: &str) -> Result<Self, VersionError> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::InvalidVersion(format!(
                "expected X.Y.Z format, got '{s}'"
            )));
        }

        let volume = parts[0].parse::<u32>().map_err(|_| {
            VersionError::InvalidVersion(format!("invalid volume number: '{}'", parts[0]))
        })?;
        let chapter = parts[1].parse::<u32>().map_err(|_| {
            VersionError::InvalidVersion(format!("invalid chapter number: '{}'", parts[1]))
        })?;
        let patch = parts[2].parse::<u32>().map_err(|_| {
            VersionError::InvalidVersion(format!("invalid patch number: '{}'", parts[2]))
        })?;

        Ok(Self { volume, chapter, patch })
    }
}

impl std::fmt::Display for UcxVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.volume, self.chapter, self.patch)
    }
}

/// Classification of changes detected since the last version.
///
/// 自上次版本以来检测到的变更分类。
#[derive(Debug, Clone, Default)]
pub struct ChangeSet {
    /// Files that were added since the last version.
    /// 自上次版本以来新增的文件。
    pub added: Vec<String>,

    /// Files that were modified since the last version.
    /// 自上次版本以来修改的文件。
    pub modified: Vec<String>,

    /// Files that were deleted since the last version.
    /// 自上次版本以来删除的文件。
    pub deleted: Vec<String>,
}

impl ChangeSet {
    /// Create an empty `ChangeSet`.
    ///
    /// 创建一个空的 `ChangeSet`。
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if no changes were detected.
    ///
    /// 如果未检测到任何变更则返回 `true`。
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// Total number of changed files.
    ///
    /// 变更文件的总数。
    pub fn total(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }
}

/// The kind of version change inferred from a `ChangeSet` and `struct.json`.
///
/// 从 `ChangeSet` 和 `struct.json` 推断出的版本变更类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    /// A new volume was detected (top-level container added in struct.json).
    /// 检测到新卷（struct.json 中新增了顶层容器节点）。
    NewVolume,

    /// A new chapter was detected (new leaf node with file reference).
    /// 检测到新章节（新增了带文件引用的叶子节点）。
    NewChapter,

    /// Only existing content was modified (no structural changes).
    /// 仅修改了已有内容（无结构变更）。
    ModificationOnly,

    /// No changes detected.
    /// 未检测到变更。
    NoChange,
}

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Detect changes since the last UCX version tag.
///
/// Uses git2 to compare the current HEAD with the latest `ucx-v*` tag.
/// If no Git repository is found, falls back to `.ucx-snapshot.json`.
///
/// 检测自上次 UCX 版本标签以来的变更。
/// 使用 git2 对比当前 HEAD 与最近的 `ucx-v*` tag。
/// 若无 Git 仓库，回退为 `.ucx-snapshot.json` 快照比较。
///
/// # Arguments / 参数
///
/// * `project_path` - Path to the project root.
///                    项目根目录路径。
///
/// # Returns / 返回
///
/// Returns a tuple of (current_version, changes), or a `VersionError` on failure.
/// 返回 (当前版本, 变更集) 元组，或在失败时返回 `VersionError`。
pub fn detect_changes(
    _project_path: &std::path::Path,
) -> Result<(UcxVersion, ChangeSet), VersionError> {
    // TODO: Implement in Step 4 — git2-based detection with snapshot fallback.
    // TODO: 在 Step 4 中实现 — 基于 git2 的检测与快照回退。
    todo!("ucx-version: change detection not yet implemented (Step 4)")
}

/// Automatically generate the next version number based on detected changes.
///
/// Analyzes the change set and struct.json to determine the version bump type:
/// - New volume → X +1, Y = first chapter number, Z = 0
/// - New chapter → Y = new chapter number, Z = 0
/// - Only modifications → Z +1
///
/// 根据检测到的变更自动生成下一个版本号。
/// 分析变更集和 struct.json 确定版本变更类型。
///
/// # Arguments / 参数
///
/// * `current`     - The current version.
///                   当前版本。
/// * `changes`     - The detected changes.
///                   检测到的变更。
/// * `project_path` - Path to the project root (for reading struct.json).
///                    项目根目录路径（用于读取 struct.json）。
///
/// # Returns / 返回
///
/// Returns the new `UcxVersion`, or a `VersionError` on failure.
/// 返回新的 `UcxVersion`，或在失败时返回 `VersionError`。
pub fn auto_version(
    _current: &UcxVersion,
    _changes: &ChangeSet,
    _project_path: &std::path::Path,
) -> Result<UcxVersion, VersionError> {
    // TODO: Implement in Step 5 — auto version bump logic.
    // TODO: 在 Step 5 中实现 — 自动版本升级逻辑。
    todo!("ucx-version: auto version not yet implemented (Step 5)")
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // UcxVersion tests / UcxVersion 测试
    // -------------------------------------------------------------------------

    #[test]
    fn version_display() {
        // Test that UcxVersion formats correctly as "X.Y.Z".
        // 测试 UcxVersion 正确格式化为 "X.Y.Z"。
        let v = UcxVersion::new(1, 5, 0);
        assert_eq!(v.to_string(), "1.5.0");
    }

    #[test]
    fn version_equality() {
        // Test that identical versions are equal.
        // 测试相同版本是否相等。
        let v1 = UcxVersion::new(2, 3, 1);
        let v2 = UcxVersion::new(2, 3, 1);
        assert_eq!(v1, v2);
    }

    #[test]
    fn version_zero() {
        // Test the zero version (0.0.0).
        // 测试零版本（0.0.0）。
        let v = UcxVersion::zero();
        assert_eq!(v.to_string(), "0.0.0");
        assert_eq!(v.volume, 0);
        assert_eq!(v.chapter, 0);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn version_parse_valid() {
        // Valid "X.Y.Z" strings should parse correctly.
        // 有效的 "X.Y.Z" 字符串应正确解析。
        let v = UcxVersion::parse("1.12.0").unwrap();
        assert_eq!(v.volume, 1);
        assert_eq!(v.chapter, 12);
        assert_eq!(v.patch, 0);

        let v2 = UcxVersion::parse("0.0.0").unwrap();
        assert_eq!(v2, UcxVersion::zero());

        let v3 = UcxVersion::parse("100.200.300").unwrap();
        assert_eq!(v3.volume, 100);
        assert_eq!(v3.chapter, 200);
        assert_eq!(v3.patch, 300);
    }

    #[test]
    fn version_parse_invalid() {
        // Invalid version strings should return VersionError::InvalidVersion.
        // 无效的版本字符串应返回 VersionError::InvalidVersion。
        assert!(UcxVersion::parse("").is_err());
        assert!(UcxVersion::parse("1").is_err());
        assert!(UcxVersion::parse("1.2").is_err());
        assert!(UcxVersion::parse("1.2.3.4").is_err());
        assert!(UcxVersion::parse("a.b.c").is_err());
        assert!(UcxVersion::parse("-1.0.0").is_err());
        assert!(UcxVersion::parse("1.0.0-beta").is_err());
    }

    #[test]
    fn version_parse_display_round_trip() {
        // Parsing a displayed version should produce the original.
        // 解析显示的版本应产生原始值。
        let original = UcxVersion::new(3, 15, 7);
        let parsed = UcxVersion::parse(&original.to_string()).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn version_serde_round_trip() {
        // UcxVersion should serialize/deserialize via serde correctly.
        // UcxVersion 应通过 serde 正确序列化/反序列化。
        let v = UcxVersion::new(2, 10, 3);
        let json = serde_json::to_string(&v).unwrap();
        let deserialized: UcxVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, v);
    }

    // -------------------------------------------------------------------------
    // ChangeSet tests / ChangeSet 测试
    // -------------------------------------------------------------------------

    #[test]
    fn changeset_new_is_empty() {
        // A new ChangeSet should be empty.
        // 新创建的 ChangeSet 应为空。
        let cs = ChangeSet::new();
        assert!(cs.is_empty());
        assert_eq!(cs.total(), 0);
    }

    #[test]
    fn changeset_non_empty() {
        // A ChangeSet with entries should not be empty.
        // 有条目的 ChangeSet 不应为空。
        let cs = ChangeSet {
            added: vec!["content/ch-002.md".to_string()],
            modified: vec!["content/ch-001.md".to_string()],
            deleted: vec![],
        };
        assert!(!cs.is_empty());
        assert_eq!(cs.total(), 2);
    }
}
