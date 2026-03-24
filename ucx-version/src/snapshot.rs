//! Snapshot-based change detection (fallback when Git is unavailable).
//!
//! Provides file hash comparison for projects without Git.
//! The snapshot file (`.ucx-snapshot.json`) records BLAKE3 hashes of all
//! content files at the time of the last build. By comparing the current
//! file hashes against the snapshot, we can detect added, modified, and
//! deleted files.
//!
//! 基于快照的变更检测（Git 不可用时的回退方案）。
//! 为无 Git 的项目提供文件哈希比较。
//! 快照文件（`.ucx-snapshot.json`）记录上次构建时所有内容文件的 BLAKE3 哈希。
//! 通过比较当前文件哈希与快照，可以检测新增、修改和删除的文件。

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{ChangeSet, VersionError};

// =============================================================================
// Snapshot file name constant / 快照文件名常量
// =============================================================================

/// The file name for the UCX snapshot file.
/// UCX 快照文件的文件名。
pub const SNAPSHOT_FILE: &str = ".ucx-snapshot.json";

// =============================================================================
// Snapshot struct / 快照结构体
// =============================================================================

/// A snapshot of content file hashes at a point in time.
///
/// Used as a fallback for change detection when no Git repository is available.
/// Stored as `.ucx-snapshot.json` in the project root.
///
/// 某一时刻的内容文件哈希快照。
/// 当 Git 仓库不可用时用作变更检测的回退方案。
/// 存储为项目根目录下的 `.ucx-snapshot.json`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// The UCX file version at the time of the snapshot.
    /// 快照时刻的 UCX 文件版本。
    pub version: String,

    /// ISO 8601 timestamp of when the snapshot was created.
    /// 快照创建时的 ISO 8601 时间戳。
    pub timestamp: String,

    /// Map of relative file paths (from project root) to their BLAKE3 hashes.
    /// 相对文件路径（从项目根目录）到其 BLAKE3 哈希的映射。
    pub files: HashMap<String, String>,
}

impl Snapshot {
    /// Create an empty snapshot with the given version.
    ///
    /// 创建一个指定版本的空快照。
    pub fn empty(version: &str) -> Self {
        Self {
            version: version.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            files: HashMap::new(),
        }
    }

    /// Load a snapshot from the `.ucx-snapshot.json` file in the project root.
    ///
    /// Returns `None` if the file does not exist.
    ///
    /// 从项目根目录的 `.ucx-snapshot.json` 文件加载快照。
    /// 如果文件不存在则返回 `None`。
    pub fn load(project_path: &Path) -> Result<Option<Self>, VersionError> {
        let snapshot_path = project_path.join(SNAPSHOT_FILE);
        if !snapshot_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&snapshot_path)?;
        let snapshot: Snapshot = serde_json::from_str(&content).map_err(|e| {
            VersionError::Snapshot(format!("failed to parse {SNAPSHOT_FILE}: {e}"))
        })?;
        Ok(Some(snapshot))
    }

    /// Save this snapshot to `.ucx-snapshot.json` in the project root.
    ///
    /// 将此快照保存到项目根目录的 `.ucx-snapshot.json`。
    pub fn save(&self, project_path: &Path) -> Result<(), VersionError> {
        let snapshot_path = project_path.join(SNAPSHOT_FILE);
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            VersionError::Snapshot(format!("failed to serialize snapshot: {e}"))
        })?;
        std::fs::write(&snapshot_path, json)?;
        Ok(())
    }

    /// Compute a snapshot of the current state of content files.
    ///
    /// Walks the `content/` directory under `project_path`, hashing each file
    /// with BLAKE3, and stores the relative paths and hex hashes.
    ///
    /// 计算当前内容文件状态的快照。
    /// 遍历 `project_path` 下的 `content/` 目录，用 BLAKE3 哈希每个文件，
    /// 存储相对路径和十六进制哈希。
    pub fn compute_current(project_path: &Path, version: &str) -> Result<Self, VersionError> {
        let content_dir = project_path.join("content");
        let mut files = HashMap::new();

        if content_dir.exists() {
            for entry in walkdir::WalkDir::new(&content_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let abs_path = entry.path();
                // Compute the relative path from the project root.
                // 计算相对于项目根目录的相对路径。
                let rel_path = abs_path
                    .strip_prefix(project_path)
                    .unwrap_or(abs_path)
                    .to_string_lossy()
                    // Normalize path separators to forward slashes for cross-platform consistency.
                    // 将路径分隔符统一为正斜杠，保持跨平台一致性。
                    .replace('\\', "/");

                // Hash the file contents with BLAKE3.
                // 使用 BLAKE3 哈希文件内容。
                let data = std::fs::read(abs_path)?;
                let hash = blake3::hash(&data).to_hex().to_string();
                files.insert(rel_path, hash);
            }
        }

        Ok(Self {
            version: version.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            files,
        })
    }

    /// Compare this snapshot (baseline) with another (current) to produce a `ChangeSet`.
    ///
    /// - Files in `current` but not in `self` → added.
    /// - Files in both but with different hashes → modified.
    /// - Files in `self` but not in `current` → deleted.
    ///
    /// 将此快照（基线）与另一个快照（当前）进行比较，生成 `ChangeSet`。
    /// - 在 `current` 中但不在 `self` 中的文件 → 新增。
    /// - 两者都有但哈希不同的文件 → 修改。
    /// - 在 `self` 中但不在 `current` 中的文件 → 删除。
    pub fn diff(&self, current: &Snapshot) -> ChangeSet {
        let mut changes = ChangeSet::new();

        // Check for added and modified files.
        // 检查新增和修改的文件。
        for (path, current_hash) in &current.files {
            match self.files.get(path) {
                None => changes.added.push(path.clone()),
                Some(old_hash) if old_hash != current_hash => {
                    changes.modified.push(path.clone());
                }
                _ => {} // unchanged / 未变更
            }
        }

        // Check for deleted files.
        // 检查删除的文件。
        for path in self.files.keys() {
            if !current.files.contains_key(path) {
                changes.deleted.push(path.clone());
            }
        }

        // Sort for deterministic output.
        // 排序以获得确定性输出。
        changes.added.sort();
        changes.modified.sort();
        changes.deleted.sort();

        changes
    }
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_snapshot_empty() {
        // An empty snapshot should have no files.
        // 空快照不应包含任何文件。
        let snap = Snapshot::empty("0.0.0");
        assert_eq!(snap.version, "0.0.0");
        assert!(snap.files.is_empty());
    }

    #[test]
    fn test_snapshot_save_load_round_trip() {
        // Saving and loading a snapshot should produce an identical result.
        // 保存并加载快照应产生相同的结果。
        let dir = tempfile::tempdir().unwrap();
        let mut snap = Snapshot::empty("1.2.0");
        snap.files.insert("content/ch-001.md".to_string(), "abc123".to_string());
        snap.files.insert("content/ch-002.md".to_string(), "def456".to_string());

        snap.save(dir.path()).unwrap();
        let loaded = Snapshot::load(dir.path()).unwrap().unwrap();

        assert_eq!(loaded.version, "1.2.0");
        assert_eq!(loaded.files.len(), 2);
        assert_eq!(loaded.files.get("content/ch-001.md").unwrap(), "abc123");
    }

    #[test]
    fn test_snapshot_load_nonexistent() {
        // Loading from a directory without a snapshot file should return None.
        // 从没有快照文件的目录加载应返回 None。
        let dir = tempfile::tempdir().unwrap();
        let result = Snapshot::load(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_snapshot_compute_current() {
        // compute_current should hash all files in content/.
        // compute_current 应哈希 content/ 中的所有文件。
        let dir = tempfile::tempdir().unwrap();
        let content_dir = dir.path().join("content");
        fs::create_dir_all(&content_dir).unwrap();
        fs::write(content_dir.join("ch-001.md"), "# 第一章\n内容").unwrap();
        fs::write(content_dir.join("ch-002.md"), "# 第二章\n内容").unwrap();

        let snap = Snapshot::compute_current(dir.path(), "1.0.0").unwrap();
        assert_eq!(snap.version, "1.0.0");
        assert_eq!(snap.files.len(), 2);
        assert!(snap.files.contains_key("content/ch-001.md"));
        assert!(snap.files.contains_key("content/ch-002.md"));
        // Each hash should be a 64-character hex string (BLAKE3 = 32 bytes).
        // 每个哈希应为 64 字符的十六进制字符串（BLAKE3 = 32 字节）。
        for hash in snap.files.values() {
            assert_eq!(hash.len(), 64);
        }
    }

    #[test]
    fn test_snapshot_compute_empty_content() {
        // compute_current on a project with no content/ dir should return empty.
        // 在没有 content/ 目录的项目上 compute_current 应返回空。
        let dir = tempfile::tempdir().unwrap();
        let snap = Snapshot::compute_current(dir.path(), "0.0.0").unwrap();
        assert!(snap.files.is_empty());
    }

    #[test]
    fn test_snapshot_diff_added() {
        // New files in the current snapshot should be detected as added.
        // 当前快照中的新文件应被检测为新增。
        let old = Snapshot::empty("1.0.0");
        let mut current = Snapshot::empty("1.1.0");
        current.files.insert("content/ch-001.md".to_string(), "aaa".to_string());

        let changes = old.diff(&current);
        assert_eq!(changes.added, vec!["content/ch-001.md"]);
        assert!(changes.modified.is_empty());
        assert!(changes.deleted.is_empty());
    }

    #[test]
    fn test_snapshot_diff_modified() {
        // Files with different hashes should be detected as modified.
        // 哈希不同的文件应被检测为修改。
        let mut old = Snapshot::empty("1.0.0");
        old.files.insert("content/ch-001.md".to_string(), "aaa".to_string());

        let mut current = Snapshot::empty("1.0.1");
        current.files.insert("content/ch-001.md".to_string(), "bbb".to_string());

        let changes = old.diff(&current);
        assert!(changes.added.is_empty());
        assert_eq!(changes.modified, vec!["content/ch-001.md"]);
        assert!(changes.deleted.is_empty());
    }

    #[test]
    fn test_snapshot_diff_deleted() {
        // Files missing from the current snapshot should be detected as deleted.
        // 当前快照中缺少的文件应被检测为删除。
        let mut old = Snapshot::empty("1.0.0");
        old.files.insert("content/ch-001.md".to_string(), "aaa".to_string());

        let current = Snapshot::empty("1.0.0");

        let changes = old.diff(&current);
        assert!(changes.added.is_empty());
        assert!(changes.modified.is_empty());
        assert_eq!(changes.deleted, vec!["content/ch-001.md"]);
    }

    #[test]
    fn test_snapshot_diff_no_changes() {
        // Identical snapshots should produce an empty changeset.
        // 相同的快照应产生空的变更集。
        let mut old = Snapshot::empty("1.0.0");
        old.files.insert("content/ch-001.md".to_string(), "aaa".to_string());

        let mut current = Snapshot::empty("1.0.0");
        current.files.insert("content/ch-001.md".to_string(), "aaa".to_string());

        let changes = old.diff(&current);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_snapshot_diff_mixed() {
        // A mix of additions, modifications, and deletions.
        // 新增、修改和删除的混合测试。
        let mut old = Snapshot::empty("1.0.0");
        old.files.insert("content/ch-001.md".to_string(), "aaa".to_string());
        old.files.insert("content/ch-002.md".to_string(), "bbb".to_string());
        old.files.insert("content/ch-003.md".to_string(), "ccc".to_string());

        let mut current = Snapshot::empty("1.1.0");
        // ch-001: unchanged / 未变更
        current.files.insert("content/ch-001.md".to_string(), "aaa".to_string());
        // ch-002: modified / 修改
        current.files.insert("content/ch-002.md".to_string(), "bbb_modified".to_string());
        // ch-003: deleted (not in current) / 删除（不在当前中）
        // ch-004: added / 新增
        current.files.insert("content/ch-004.md".to_string(), "ddd".to_string());

        let changes = old.diff(&current);
        assert_eq!(changes.added, vec!["content/ch-004.md"]);
        assert_eq!(changes.modified, vec!["content/ch-002.md"]);
        assert_eq!(changes.deleted, vec!["content/ch-003.md"]);
    }
}
