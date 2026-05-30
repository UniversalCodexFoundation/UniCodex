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

pub mod bump;
pub mod git;
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
// `PartialOrd`/`Ord` compare fields in declaration order (volume, then chapter,
// then patch), which is exactly the intended semantic-version ordering — enabling
// monotonicity checks (e.g. detecting a downgrade).
// `PartialOrd`/`Ord` 按声明顺序（volume、chapter、patch）比较字段，恰为预期的
// 语义版本顺序——便于做单调性检查（如检测降级）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
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
        Self {
            volume,
            chapter,
            patch,
        }
    }

    /// The zero version (0.0.0), used as the initial version before any release.
    ///
    /// 零版本（0.0.0），用作首次发布前的初始版本。
    pub fn zero() -> Self {
        Self {
            volume: 0,
            chapter: 0,
            patch: 0,
        }
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

        Ok(Self {
            volume,
            chapter,
            patch,
        })
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
///   项目根目录路径。
///
/// # Returns / 返回
///
/// Returns a tuple of (current_version, changes), or a `VersionError` on failure.
/// 返回 (当前版本, 变更集) 元组，或在失败时返回 `VersionError`。
pub fn detect_changes(
    project_path: &std::path::Path,
) -> Result<(UcxVersion, ChangeSet), VersionError> {
    // Try git2-based detection first.
    // 首先尝试基于 git2 的检测。
    match git::detect_changes_git(project_path) {
        Ok(result) => {
            tracing::debug!("Change detection via git2 succeeded");
            Ok(result)
        }
        Err(VersionError::GitError(e)) => {
            // Git not available — fall back to snapshot-based detection.
            // Git 不可用 — 回退到基于快照的检测。
            tracing::info!("Git not available ({e}), falling back to snapshot detection");
            detect_changes_snapshot(project_path)
        }
        Err(e) => Err(e),
    }
}

/// Fallback: detect changes using `.ucx-snapshot.json` when Git is unavailable.
///
/// 回退方案：当 Git 不可用时，使用 `.ucx-snapshot.json` 检测变更。
fn detect_changes_snapshot(
    project_path: &std::path::Path,
) -> Result<(UcxVersion, ChangeSet), VersionError> {
    let old_snapshot = snapshot::Snapshot::load(project_path)?;
    let current = snapshot::Snapshot::compute_current(project_path, "0.0.0")?;

    match old_snapshot {
        Some(old) => {
            // Parse the version from the old snapshot.
            // 从旧快照解析版本。
            let version = UcxVersion::parse(&old.version).unwrap_or_else(|_| UcxVersion::zero());
            let changes = old.diff(&current);
            Ok((version, changes))
        }
        None => {
            // No previous snapshot — all current content files are "added".
            // 无先前快照 — 所有当前内容文件视为"新增"。
            let changes = ChangeSet {
                added: current.files.keys().cloned().collect(),
                modified: vec![],
                deleted: vec![],
            };
            Ok((UcxVersion::zero(), changes))
        }
    }
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
///   当前版本。
/// * `changes`     - The detected changes.
///   检测到的变更。
/// * `project_path` - Path to the project root (for reading struct.json).
///   项目根目录路径（用于读取 struct.json）。
///
/// # Returns / 返回
///
/// Returns the new `UcxVersion`, or a `VersionError` on failure.
/// 返回新的 `UcxVersion`，或在失败时返回 `VersionError`。
pub fn auto_version(
    current: &UcxVersion,
    changes: &ChangeSet,
    project_path: &std::path::Path,
) -> Result<UcxVersion, VersionError> {
    // Classify the changes by analyzing struct.json.
    // 通过分析 struct.json 对变更进行分类。
    let change_kind = bump::classify_changes(changes, project_path)?;
    tracing::info!("Change kind: {change_kind:?}");

    // Read the current structure to determine chapter numbers.
    // 读取当前结构以确定章节编号。
    let struct_path = project_path.join("content").join("struct.json");
    let structure = if struct_path.exists() {
        // M-3: enforce the SAME struct.json DoS bounds as ucx-build (shared
        // constants/guard in ucx-types). `ucx version auto/chapter` reads
        // struct.json on an untrusted project dir, so it needs the size + node/
        // depth caps too — otherwise the parse-amplification DoS just moves here.
        // M-3：强制与 ucx-build 相同的 struct.json DoS 上界（共享常量/守卫在
        // ucx-types）。`ucx version auto/chapter` 在不可信项目目录读取 struct.json，
        // 同样需要大小 + 节点/深度上限——否则解析放大 DoS 只是转移到此处。
        let meta = std::fs::metadata(&struct_path)
            .map_err(|e| VersionError::Structure(format!("failed to stat struct.json: {e}")))?;
        if meta.len() > ucx_types::structure::MAX_STRUCT_JSON_BYTES {
            return Err(VersionError::Structure(format!(
                "content/struct.json is too large ({} bytes; limit {})",
                meta.len(),
                ucx_types::structure::MAX_STRUCT_JSON_BYTES
            )));
        }
        let content = std::fs::read_to_string(&struct_path)
            .map_err(|e| VersionError::Structure(format!("failed to read struct.json: {e}")))?;
        let structure: ucx_types::Structure = serde_json::from_str(&content)
            .map_err(|e| VersionError::Structure(format!("failed to parse struct.json: {e}")))?;
        ucx_types::structure::enforce_structure_limits(&structure.structure)
            .map_err(|e| VersionError::Structure(e.to_string()))?;
        structure
    } else {
        // No struct.json — use an empty structure.
        // 无 struct.json — 使用空结构。
        ucx_types::Structure {
            schema: None,
            version: "1.0".to_string(),
            structure: vec![],
        }
    };

    let next = bump::compute_next_version(current, &change_kind, &structure);
    tracing::info!("Version bump: {current} → {next}");
    Ok(next)
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

    // -------------------------------------------------------------------------
    // detect_changes integration tests / detect_changes 集成测试
    // -------------------------------------------------------------------------

    #[test]
    fn detect_changes_no_git_no_snapshot() {
        // In a directory without Git or snapshot, all content files are "added".
        // 在没有 Git 或快照的目录中，所有内容文件视为"新增"。
        let dir = tempfile::tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir_all(&content_dir).unwrap();
        std::fs::write(content_dir.join("ch-001.md"), "# 第一章").unwrap();

        let (version, changes) = detect_changes(dir.path()).unwrap();
        assert_eq!(version, UcxVersion::zero());
        assert_eq!(changes.added.len(), 1);
        assert!(changes.added[0].contains("ch-001.md"));
    }

    #[test]
    fn detect_changes_no_git_with_snapshot() {
        // With a snapshot but no Git, changes are detected by comparing hashes.
        // 有快照但无 Git 时，通过比较哈希检测变更。
        let dir = tempfile::tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir_all(&content_dir).unwrap();
        std::fs::write(content_dir.join("ch-001.md"), "# 第一章").unwrap();

        // Create a snapshot of the current state.
        // 创建当前状态的快照。
        let snap = snapshot::Snapshot::compute_current(dir.path(), "1.0.0").unwrap();
        snap.save(dir.path()).unwrap();

        // Modify the file.
        // 修改文件。
        std::fs::write(content_dir.join("ch-001.md"), "# 第一章（修改后）").unwrap();

        let (version, changes) = detect_changes(dir.path()).unwrap();
        assert_eq!(version, UcxVersion::new(1, 0, 0));
        assert!(changes.modified.contains(&"content/ch-001.md".to_string()));
    }
}
