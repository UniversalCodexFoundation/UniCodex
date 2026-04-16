//! Git-based change detection using git2 (libgit2 Rust bindings).
//!
//! Detects changes by comparing the current HEAD with the latest `ucx-v*` tag.
//! This is the primary detection method when a Git repository is available.
//!
//! 基于 git2（libgit2 Rust 绑定）的变更检测。
//! 通过比较当前 HEAD 与最近的 `ucx-v*` 标签来检测变更。
//! 当 Git 仓库可用时，这是主要的检测方法。

use std::path::Path;

use git2::{DiffOptions, Repository};
use tracing;

use crate::{ChangeSet, UcxVersion, VersionError};

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Detect changes since the last `ucx-v*` tag using git2.
///
/// Returns the current version (from the latest tag, or 0.0.0 if no tag exists)
/// and the set of changes in `content/` since that tag.
///
/// 使用 git2 检测自最近的 `ucx-v*` 标签以来的变更。
/// 返回当前版本（来自最近标签，如无标签则为 0.0.0）
/// 及该标签以来 `content/` 目录的变更集。
///
/// # Arguments / 参数
///
/// * `project_path` - Path to the project root (must contain a `.git` directory).
///   项目根目录路径（必须包含 `.git` 目录）。
pub fn detect_changes_git(
    project_path: &Path,
) -> Result<(UcxVersion, ChangeSet), VersionError> {
    // Open the Git repository via git2.
    // 通过 git2 打开 Git 仓库。
    let repo = Repository::open(project_path)?;
    tracing::debug!("Opened Git repository at {}", project_path.display());

    // Find the latest ucx-v* tag.
    // 查找最近的 ucx-v* 标签。
    let tag_result = find_latest_ucx_tag(&repo);

    match tag_result {
        Ok((tag_name, tag_oid)) => {
            // Parse the version from the tag name.
            // 从标签名解析版本号。
            let current_version = parse_tag_version(&tag_name)?;
            tracing::info!("Found UCX tag: {tag_name} (version {current_version})");

            // Get the tree from the tag's commit.
            // 获取标签 commit 的 tree。
            let tag_commit = repo
                .find_commit(tag_oid)
                .or_else(|_| {
                    // The OID might be a tag object, not a commit directly.
                    // Peel the tag to a commit.
                    // OID 可能是 tag 对象而非直接的 commit，需要剥离到 commit。
                    let obj = repo.find_object(tag_oid, None)?;
                    obj.peel_to_commit()
                })?;
            let base_tree = tag_commit.tree()?;

            // Get the current HEAD tree.
            // 获取当前 HEAD 的 tree。
            let head_ref = repo.head()?;
            let head_commit = head_ref.peel_to_commit()?;
            let head_tree = head_commit.tree()?;

            // Compute diff between the two trees, filtered to content/ paths.
            // 计算两棵树之间的 diff，筛选 content/ 路径。
            let changes = diff_trees(&repo, Some(&base_tree), &head_tree)?;

            Ok((current_version, changes))
        }
        Err(VersionError::NoVersionTag) => {
            // No ucx-v* tag exists. Treat all content files as "added".
            // 不存在 ucx-v* 标签。将所有内容文件视为"新增"。
            tracing::info!("No UCX version tag found, treating all content as new");
            let head_ref = repo.head()?;
            let head_commit = head_ref.peel_to_commit()?;
            let head_tree = head_commit.tree()?;

            let changes = diff_trees(&repo, None, &head_tree)?;
            Ok((UcxVersion::zero(), changes))
        }
        Err(e) => Err(e),
    }
}

/// Create a lightweight Git tag at the current HEAD for the given version.
///
/// Tag format: `ucx-v{X.Y.Z}`.
///
/// 在当前 HEAD 为给定版本创建轻量 Git 标签。
/// 标签格式：`ucx-v{X.Y.Z}`。
pub fn create_version_tag(
    project_path: &Path,
    version: &UcxVersion,
) -> Result<(), VersionError> {
    let repo = Repository::open(project_path)?;
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    let tag_name = format!("ucx-v{version}");

    repo.tag_lightweight(&tag_name, head_commit.as_object(), false)?;
    tracing::info!("Created tag: {tag_name}");
    Ok(())
}

// =============================================================================
// Internal functions / 内部函数
// =============================================================================

/// Find the latest `ucx-v*` tag in the repository.
///
/// Searches all tags matching the `ucx-v*` pattern, parses their version
/// numbers, and returns the one with the highest version.
///
/// 查找仓库中最近的 `ucx-v*` 标签。
/// 搜索所有匹配 `ucx-v*` 模式的标签，解析版本号，返回版本最高的一个。
///
/// # Returns / 返回
///
/// Tuple of (tag_name, tag_oid). Returns `VersionError::NoVersionTag` if
/// no matching tag is found.
/// 返回 (标签名, 标签 OID) 元组。未找到匹配标签时返回 `VersionError::NoVersionTag`。
fn find_latest_ucx_tag(repo: &Repository) -> Result<(String, git2::Oid), VersionError> {
    let mut tags: Vec<(String, git2::Oid, UcxVersion)> = Vec::new();

    // Iterate over all references to find ucx-v* tags.
    // 遍历所有引用以查找 ucx-v* 标签。
    repo.tag_foreach(|oid, name_bytes| {
        // tag_foreach gives raw bytes like "refs/tags/ucx-v1.0.0".
        // tag_foreach 提供原始字节，如 "refs/tags/ucx-v1.0.0"。
        if let Ok(name) = std::str::from_utf8(name_bytes) {
            // Strip the "refs/tags/" prefix to get the tag name.
            // 去除 "refs/tags/" 前缀以获取标签名。
            let short_name = name.strip_prefix("refs/tags/").unwrap_or(name);
            if short_name.starts_with("ucx-v")
                && let Ok(version) = parse_tag_version(short_name)
            {
                tags.push((short_name.to_string(), oid, version));
            }
        }
        true // continue iteration / 继续遍历
    })?;

    if tags.is_empty() {
        return Err(VersionError::NoVersionTag);
    }

    // Sort by version (volume, chapter, patch) descending to find the latest.
    // 按版本（卷、章、修订）降序排列以找到最新的。
    tags.sort_by(|a, b| {
        b.2.volume
            .cmp(&a.2.volume)
            .then(b.2.chapter.cmp(&a.2.chapter))
            .then(b.2.patch.cmp(&a.2.patch))
    });

    let (name, oid, _version) = tags.into_iter().next().unwrap();
    Ok((name, oid))
}

/// Parse a version from a tag name like "ucx-v1.5.0".
///
/// 从类似 "ucx-v1.5.0" 的标签名解析版本。
pub(crate) fn parse_tag_version(tag_name: &str) -> Result<UcxVersion, VersionError> {
    let version_str = tag_name
        .strip_prefix("ucx-v")
        .ok_or_else(|| {
            VersionError::InvalidVersion(format!(
                "tag '{tag_name}' does not start with 'ucx-v'"
            ))
        })?;
    UcxVersion::parse(version_str)
}

/// Compute a diff between two trees (or from empty to a tree), filtered to `content/`.
///
/// If `base_tree` is `None`, all files in `head_tree` are treated as added.
///
/// 计算两棵树之间的 diff（或从空到一棵树），筛选 `content/`。
/// 如果 `base_tree` 为 `None`，则 `head_tree` 中的所有文件被视为新增。
fn diff_trees(
    repo: &Repository,
    base_tree: Option<&git2::Tree<'_>>,
    head_tree: &git2::Tree<'_>,
) -> Result<ChangeSet, VersionError> {
    let mut diff_opts = DiffOptions::new();
    // Filter to content/ directory and content/struct.json.
    // 筛选 content/ 目录和 content/struct.json。
    diff_opts.pathspec("content/*");

    let diff = repo.diff_tree_to_tree(base_tree, Some(head_tree), Some(&mut diff_opts))?;

    let mut changes = ChangeSet::new();

    diff.foreach(
        &mut |delta, _progress| {
            // Get the file path from the delta.
            // 从 delta 获取文件路径。
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();

            match delta.status() {
                git2::Delta::Added | git2::Delta::Untracked => {
                    changes.added.push(path);
                }
                git2::Delta::Modified => {
                    changes.modified.push(path);
                }
                git2::Delta::Deleted => {
                    changes.deleted.push(path);
                }
                git2::Delta::Renamed | git2::Delta::Copied => {
                    // Treat renames as a delete + add pair.
                    // 将重命名视为删除 + 新增对。
                    if let Some(old_path) = delta.old_file().path() {
                        changes.deleted.push(old_path.to_string_lossy().replace('\\', "/"));
                    }
                    changes.added.push(path);
                }
                _ => {} // Ignore other delta types. / 忽略其他 delta 类型。
            }
            true // continue iteration / 继续遍历
        },
        None,
        None,
        None,
    )?;

    // Sort for deterministic output.
    // 排序以获得确定性输出。
    changes.added.sort();
    changes.modified.sort();
    changes.deleted.sort();

    Ok(changes)
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper: create a temp Git repo with an initial commit.
    /// 辅助函数：创建一个带有初始 commit 的临时 Git 仓库。
    fn init_test_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Configure git user for commits.
        // 配置 git 用户用于 commit。
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@test.com").unwrap();
        }

        // Create content/ directory with a file.
        // 创建 content/ 目录并添加文件。
        let content_dir = dir.path().join("content");
        fs::create_dir_all(&content_dir).unwrap();
        fs::write(content_dir.join("ch-001.md"), "# Chapter 1").unwrap();

        // Stage and commit.
        // 暂存并提交。
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("content/ch-001.md")).unwrap();
            index.write().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn test_parse_tag_version_valid() {
        // A valid tag name should parse correctly.
        // 有效的标签名应正确解析。
        let v = parse_tag_version("ucx-v1.5.0").unwrap();
        assert_eq!(v, UcxVersion::new(1, 5, 0));

        let v2 = parse_tag_version("ucx-v0.0.0").unwrap();
        assert_eq!(v2, UcxVersion::zero());
    }

    #[test]
    fn test_parse_tag_version_invalid() {
        // Invalid tag names should return errors.
        // 无效的标签名应返回错误。
        assert!(parse_tag_version("v1.0.0").is_err());
        assert!(parse_tag_version("ucx-1.0.0").is_err());
        assert!(parse_tag_version("ucx-va.b.c").is_err());
    }

    #[test]
    fn test_detect_changes_no_tag() {
        // A repo with no ucx-v* tag should return version 0.0.0 and all files as added.
        // 无 ucx-v* 标签的仓库应返回版本 0.0.0，所有文件视为新增。
        let (dir, _repo) = init_test_repo();

        let (version, changes) = detect_changes_git(dir.path()).unwrap();
        assert_eq!(version, UcxVersion::zero());
        assert!(changes.added.contains(&"content/ch-001.md".to_string()));
        assert!(changes.modified.is_empty());
        assert!(changes.deleted.is_empty());
    }

    #[test]
    fn test_detect_changes_with_tag_no_changes() {
        // After tagging, with no further changes, the changeset should be empty.
        // 标记后无进一步变更时，变更集应为空。
        let (dir, repo) = init_test_repo();

        // Create a ucx-v tag at current HEAD.
        // 在当前 HEAD 创建 ucx-v 标签。
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.tag_lightweight("ucx-v1.0.0", head.as_object(), false)
            .unwrap();

        let (version, changes) = detect_changes_git(dir.path()).unwrap();
        assert_eq!(version, UcxVersion::new(1, 0, 0));
        assert!(changes.is_empty());
    }

    #[test]
    fn test_detect_changes_with_modifications() {
        // Modifications after a tag should be detected.
        // 标记后的修改应被检测到。
        let (dir, repo) = init_test_repo();

        // Tag the current state.
        // 标记当前状态。
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.tag_lightweight("ucx-v1.0.0", head.as_object(), false)
            .unwrap();

        // Modify an existing file and add a new one.
        // 修改现有文件并添加新文件。
        let content_dir = dir.path().join("content");
        fs::write(content_dir.join("ch-001.md"), "# Chapter 1 (updated)").unwrap();
        fs::write(content_dir.join("ch-002.md"), "# Chapter 2").unwrap();

        // Commit the changes.
        // 提交变更。
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("content/ch-001.md")).unwrap();
        index.add_path(Path::new("content/ch-002.md")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Add chapter 2 and update chapter 1",
            &tree,
            &[&parent],
        )
        .unwrap();

        let (version, changes) = detect_changes_git(dir.path()).unwrap();
        assert_eq!(version, UcxVersion::new(1, 0, 0));
        assert!(changes.added.contains(&"content/ch-002.md".to_string()));
        assert!(changes.modified.contains(&"content/ch-001.md".to_string()));
    }

    #[test]
    fn test_find_latest_tag_multiple() {
        // When multiple ucx-v* tags exist, the highest version should be returned.
        // 存在多个 ucx-v* 标签时，应返回版本最高的。
        let (dir, repo) = init_test_repo();

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.tag_lightweight("ucx-v1.0.0", head.as_object(), false)
            .unwrap();
        repo.tag_lightweight("ucx-v1.2.0", head.as_object(), false)
            .unwrap();
        repo.tag_lightweight("ucx-v1.1.0", head.as_object(), false)
            .unwrap();

        let (name, _oid) = find_latest_ucx_tag(&repo).unwrap();
        assert_eq!(name, "ucx-v1.2.0");
    }

    #[test]
    fn test_create_version_tag() {
        // create_version_tag should create a lightweight tag at HEAD.
        // create_version_tag 应在 HEAD 创建轻量标签。
        let (dir, repo) = init_test_repo();

        let version = UcxVersion::new(1, 5, 0);
        create_version_tag(dir.path(), &version).unwrap();

        // Verify the tag exists.
        // 验证标签存在。
        let (name, _oid) = find_latest_ucx_tag(&repo).unwrap();
        assert_eq!(name, "ucx-v1.5.0");
    }
}
