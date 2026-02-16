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
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl std::fmt::Display for UcxVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.volume, self.chapter, self.patch)
    }
}

/// Classification of changes detected since the last version.
///
/// 自上次版本以来检测到的变更分类。
#[derive(Debug, Clone)]
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
/// Returns the detected `ChangeSet`, or a `VersionError` on failure.
/// 返回检测到的 `ChangeSet`，或在失败时返回 `VersionError`。
pub fn detect_changes(_project_path: &std::path::Path) -> Result<ChangeSet, VersionError> {
    // TODO: Implement change detection via git2.
    // TODO: 使用 git2 实现变更检测。
    //
    // Steps / 步骤:
    // 1. Try to open Git repository via git2::Repository::open().
    //    尝试通过 git2::Repository::open() 打开 Git 仓库。
    // 2. Find the latest ucx-v* tag.
    //    查找最近的 ucx-v* 标签。
    // 3. Compute diff between tag tree and HEAD tree.
    //    计算标签树和 HEAD 树之间的 diff。
    // 4. Filter changes in content/ directory.
    //    筛选 content/ 目录下的变更。
    // 5. Classify into added/modified/deleted.
    //    分类为新增/修改/删除。
    //
    // Fallback / 回退:
    // If Repository::open() fails, read .ucx-snapshot.json and
    // compare file hashes with current files.
    // 若 Repository::open() 失败，读取 .ucx-snapshot.json
    // 并比较文件哈希与当前文件。
    todo!("ucx-version: change detection not yet implemented")
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
    // TODO: Implement automatic version generation.
    // TODO: 实现自动版本生成。
    todo!("ucx-version: auto version not yet implemented")
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_display() {
        // Test that UcxVersion formats correctly as "X.Y.Z".
        // 测试 UcxVersion 正确格式化为 "X.Y.Z"。
        let v = UcxVersion {
            volume: 1,
            chapter: 5,
            patch: 0,
        };
        assert_eq!(v.to_string(), "1.5.0");
    }

    #[test]
    fn version_equality() {
        // Test that identical versions are equal.
        // 测试相同版本是否相等。
        let v1 = UcxVersion {
            volume: 2,
            chapter: 3,
            patch: 1,
        };
        let v2 = UcxVersion {
            volume: 2,
            chapter: 3,
            patch: 1,
        };
        assert_eq!(v1, v2);
    }
}
