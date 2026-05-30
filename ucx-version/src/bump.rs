//! Version bump logic and structure analysis.
//!
//! Analyzes `struct.json` and the `ChangeSet` to determine the type of
//! version change and compute the next version number.
//!
//! 版本升级逻辑和结构分析。
//! 分析 `struct.json` 和 `ChangeSet` 以确定版本变更类型并计算下一个版本号。

use std::path::Path;

use ucx_types::{Structure, StructureNode};

use crate::{ChangeKind, ChangeSet, UcxVersion, VersionError};

// =============================================================================
// Public API / 公开接口
// =============================================================================

/// Classify the kind of change from a `ChangeSet` and the current `struct.json`.
///
/// Reads `content/struct.json` to compare against the previous structure
/// (inferred from the change set) and determines:
/// - `NewVolume` if a new top-level container node was added.
/// - `NewChapter` if new leaf nodes (file references) were added.
/// - `ModificationOnly` if only existing files were modified.
/// - `NoChange` if the change set is empty.
///
/// 根据 `ChangeSet` 和当前 `struct.json` 分类变更类型。
/// 读取 `content/struct.json` 与之前的结构进行比较，确定：
/// - `NewVolume`：新增了顶层容器节点。
/// - `NewChapter`：新增了叶子节点（文件引用）。
/// - `ModificationOnly`：仅修改了现有文件。
/// - `NoChange`：变更集为空。
pub fn classify_changes(
    changes: &ChangeSet,
    project_path: &Path,
) -> Result<ChangeKind, VersionError> {
    // No changes → NoChange.
    // 无变更 → NoChange。
    if changes.is_empty() {
        return Ok(ChangeKind::NoChange);
    }

    // If there are no added files, it's modification only.
    // 如果没有新增文件，则仅为修改。
    if changes.added.is_empty() {
        return Ok(ChangeKind::ModificationOnly);
    }

    // Read the current struct.json to analyze the structure.
    // 读取当前 struct.json 分析结构。
    let struct_path = project_path.join("content").join("struct.json");
    if !struct_path.exists() {
        // No struct.json — can't determine structure changes, treat as modification.
        // 无 struct.json — 无法确定结构变更，视为修改。
        tracing::warn!("content/struct.json not found, treating changes as modifications");
        return Ok(ChangeKind::ModificationOnly);
    }

    // M-3: same struct.json DoS bounds as ucx-build (shared guard in ucx-types).
    // M-3：与 ucx-build 相同的 struct.json DoS 上界（ucx-types 共享守卫）。
    let struct_meta = std::fs::metadata(&struct_path)
        .map_err(|e| VersionError::Structure(format!("failed to stat struct.json: {e}")))?;
    if struct_meta.len() > ucx_types::structure::MAX_STRUCT_JSON_BYTES {
        return Err(VersionError::Structure(format!(
            "content/struct.json is too large ({} bytes; limit {})",
            struct_meta.len(),
            ucx_types::structure::MAX_STRUCT_JSON_BYTES
        )));
    }
    let struct_content = std::fs::read_to_string(&struct_path)
        .map_err(|e| VersionError::Structure(format!("failed to read struct.json: {e}")))?;
    let structure: Structure = serde_json::from_str(&struct_content)
        .map_err(|e| VersionError::Structure(format!("failed to parse struct.json: {e}")))?;
    ucx_types::structure::enforce_structure_limits(&structure.structure)
        .map_err(|e| VersionError::Structure(e.to_string()))?;

    // Check if any added files correspond to new content in the structure.
    // 检查是否有新增文件对应结构中的新内容。
    let all_leaf_files = collect_all_leaf_files(&structure.structure);

    // Determine if any added files are leaf nodes in the structure.
    // 确定是否有新增文件是结构中的叶子节点。
    let added_content_files: Vec<&String> = changes
        .added
        .iter()
        .filter(|f| {
            // Check if the added file matches a leaf node.
            // 检查新增文件是否匹配叶子节点。
            let file_in_content = f.strip_prefix("content/").unwrap_or(f.as_str());
            all_leaf_files.iter().any(|leaf| leaf == file_in_content)
        })
        .collect();

    if added_content_files.is_empty() {
        // Added files don't match any structure leaf — treat as modification.
        // 新增文件不匹配任何结构叶子 — 视为修改。
        return Ok(ChangeKind::ModificationOnly);
    }

    // Check if struct.json itself was added/modified (indicating structural change).
    // We look at the structure to see if new top-level volumes exist.
    // 检查 struct.json 是否被新增/修改（表示结构变更）。
    // 查看结构中是否存在新的顶层卷。
    let has_struct_change = changes.added.iter().any(|f| f.ends_with("struct.json"))
        || changes.modified.iter().any(|f| f.ends_with("struct.json"));

    if has_struct_change && structure.structure.len() > 1 {
        // Multiple top-level nodes might indicate new volumes.
        // Heuristic: if there are container nodes at the top level and
        // added files belong to a different volume than existing ones,
        // it's likely a new volume. For simplicity, we check if the
        // last top-level node has children containing the added files.
        // 多个顶层节点可能表示新卷。
        // 启发式：如果顶层有容器节点且新增文件属于不同于现有的卷，
        // 则可能是新卷。简化处理：检查最后一个顶层节点的子节点是否包含新增文件。
        let last_volume = structure.structure.last().unwrap();
        if last_volume.children.is_some() {
            let last_volume_files =
                collect_all_leaf_files(last_volume.children.as_deref().unwrap_or(&[]));
            let added_in_last_volume = added_content_files.iter().any(|f| {
                let file_in_content = f.strip_prefix("content/").unwrap_or(f.as_str());
                last_volume_files.iter().any(|leaf| leaf == file_in_content)
            });
            if added_in_last_volume && structure.structure.len() > 1 {
                return Ok(ChangeKind::NewVolume);
            }
        }
    }

    // Default: new chapter (added leaf files).
    // 默认：新章节（新增叶子文件）。
    Ok(ChangeKind::NewChapter)
}

/// Count the total number of leaf nodes (chapters) in a set of structure nodes.
///
/// 计算一组结构节点中叶子节点（章节）的总数。
pub fn count_leaf_nodes(nodes: &[StructureNode]) -> u32 {
    let mut count = 0u32;
    for node in nodes {
        if node.file.is_some() {
            count += 1;
        } else if let Some(children) = &node.children {
            count += count_leaf_nodes(children);
        }
    }
    count
}

/// Find the latest chapter number in the structure.
///
/// The chapter number is the total count of leaf nodes in the last
/// top-level volume (or the entire structure if no volume containers exist).
///
/// 查找结构中的最新章节编号。
/// 章节编号是最后一个顶层卷中叶子节点的总数
/// （如果不存在卷容器，则为整个结构的叶子节点总数）。
pub fn find_latest_chapter_number(structure: &Structure) -> u32 {
    if structure.structure.is_empty() {
        return 0;
    }

    // Check if the structure has volume containers (top-level nodes with children).
    // 检查结构是否有卷容器（带 children 的顶层节点）。
    let has_volumes = structure.structure.iter().any(|n| n.children.is_some());

    if has_volumes {
        // Count leaf nodes in the last volume.
        // 计算最后一卷的叶子节点数。
        let last_volume = structure.structure.last().unwrap();
        if let Some(children) = &last_volume.children {
            count_leaf_nodes(children)
        } else {
            // The last top-level node is a leaf itself → chapter 1.
            // 最后一个顶层节点本身就是叶子 → 第 1 章。
            1
        }
    } else {
        // No volume containers — count all top-level leaf nodes.
        // 无卷容器 — 计算所有顶层叶子节点。
        count_leaf_nodes(&structure.structure)
    }
}

/// Compute the next version based on the change kind and current structure.
///
/// 根据变更类型和当前结构计算下一个版本。
pub fn compute_next_version(
    current: &UcxVersion,
    change_kind: &ChangeKind,
    structure: &Structure,
) -> UcxVersion {
    match change_kind {
        ChangeKind::NewVolume => {
            // New volume: X+1, Y = chapter count in new volume, Z = 0.
            // 新卷：X+1, Y = 新卷中的章节数, Z = 0。
            let chapter_num = find_latest_chapter_number(structure).max(1);
            UcxVersion::new(current.volume + 1, chapter_num, 0)
        }
        ChangeKind::NewChapter => {
            // New chapter: keep X, Y = latest chapter number, Z = 0.
            // 新章节：保持 X, Y = 最新章节编号, Z = 0。
            let chapter_num = find_latest_chapter_number(structure);
            let volume = if current.volume == 0 {
                1
            } else {
                current.volume
            };
            UcxVersion::new(volume, chapter_num, 0)
        }
        ChangeKind::ModificationOnly => {
            // Modification only: keep X and Y, Z + 1.
            // 仅修改：保持 X 和 Y, Z + 1。
            let volume = if current.volume == 0 {
                1
            } else {
                current.volume
            };
            let chapter = if current.chapter == 0 {
                find_latest_chapter_number(structure).max(1)
            } else {
                current.chapter
            };
            UcxVersion::new(volume, chapter, current.patch + 1)
        }
        ChangeKind::NoChange => {
            // No change: return current version.
            // 无变更：返回当前版本。
            current.clone()
        }
    }
}

// =============================================================================
// Manual bump functions / 手动版本升级函数
// =============================================================================

/// Manually bump the patch version (Z + 1).
///
/// 手动升级修订版本（Z + 1）。
pub fn bump_patch(current: &UcxVersion) -> UcxVersion {
    UcxVersion::new(current.volume, current.chapter, current.patch + 1)
}

/// Manually bump the chapter version (Y = new_chapter, Z = 0).
///
/// 手动升级章节版本（Y = new_chapter, Z = 0）。
pub fn bump_chapter(current: &UcxVersion, new_chapter: u32) -> UcxVersion {
    UcxVersion::new(current.volume, new_chapter, 0)
}

/// Manually bump the volume version (X + 1, Y = first_chapter, Z = 0).
///
/// 手动升级卷版本（X + 1, Y = first_chapter, Z = 0）。
pub fn bump_volume(current: &UcxVersion, first_chapter: u32) -> UcxVersion {
    UcxVersion::new(current.volume + 1, first_chapter, 0)
}

// =============================================================================
// Internal helpers / 内部辅助函数
// =============================================================================

/// Collect all leaf file paths from a tree of structure nodes.
///
/// Returns paths relative to `content/` (as stored in `struct.json`).
///
/// 从结构节点树中收集所有叶子文件路径。
/// 返回相对于 `content/` 的路径（如 `struct.json` 中所存储的）。
fn collect_all_leaf_files(nodes: &[StructureNode]) -> Vec<String> {
    let mut files = Vec::new();
    for node in nodes {
        if let Some(file) = &node.file {
            files.push(file.clone());
        }
        if let Some(children) = &node.children {
            files.extend(collect_all_leaf_files(children));
        }
    }
    files
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ucx_types::StructureNode;

    /// Helper: create a leaf StructureNode with only required fields.
    /// 辅助函数：创建仅包含必需字段的叶子 StructureNode。
    fn leaf_node(title: &str, file: &str) -> StructureNode {
        StructureNode {
            title: title.to_string(),
            file: Some(file.to_string()),
            children: None,
            node_type: None,
            id: None,
            name: None,
            style: None,
            encryption: None,
        }
    }

    /// Helper: create a container StructureNode with only required fields.
    /// 辅助函数：创建仅包含必需字段的容器 StructureNode。
    fn container_node(title: &str, children: Vec<StructureNode>) -> StructureNode {
        StructureNode {
            title: title.to_string(),
            file: None,
            children: Some(children),
            node_type: None,
            id: None,
            name: None,
            style: None,
            encryption: None,
        }
    }

    /// Helper: create a minimal structure with chapters in a single volume.
    /// 辅助函数：创建包含单卷多章节的最小结构。
    fn single_volume_structure(chapter_count: u32) -> Structure {
        let mut chapters = Vec::new();
        for i in 1..=chapter_count {
            chapters.push(leaf_node(&format!("第{i}章"), &format!("ch-{i:03}.md")));
        }
        Structure {
            schema: None,
            version: "1.0".to_string(),
            structure: vec![container_node("第一卷", chapters)],
        }
    }

    /// Helper: create a structure with two volumes.
    /// 辅助函数：创建包含两卷的结构。
    fn two_volume_structure() -> Structure {
        Structure {
            schema: None,
            version: "1.0".to_string(),
            structure: vec![
                container_node(
                    "第一卷",
                    vec![
                        leaf_node("第一章", "vol1/ch-001.md"),
                        leaf_node("第二章", "vol1/ch-002.md"),
                    ],
                ),
                container_node("第二卷", vec![leaf_node("第一章", "vol2/ch-001.md")]),
            ],
        }
    }

    #[test]
    fn test_count_leaf_nodes() {
        // A single volume with 5 chapters should have 5 leaf nodes.
        // 含 5 章的单卷应有 5 个叶子节点。
        let s = single_volume_structure(5);
        assert_eq!(count_leaf_nodes(&s.structure), 5);
    }

    #[test]
    fn test_count_leaf_nodes_two_volumes() {
        // Two volumes: 2 + 1 = 3 leaf nodes total.
        // 两卷：2 + 1 = 3 个叶子节点。
        let s = two_volume_structure();
        assert_eq!(count_leaf_nodes(&s.structure), 3);
    }

    #[test]
    fn test_find_latest_chapter_single_volume() {
        // In a single volume with 5 chapters, latest chapter = 5.
        // 单卷 5 章，最新章节 = 5。
        let s = single_volume_structure(5);
        assert_eq!(find_latest_chapter_number(&s), 5);
    }

    #[test]
    fn test_find_latest_chapter_two_volumes() {
        // In two volumes (2 + 1), latest chapter in last volume = 1.
        // 两卷（2 + 1），最后一卷的最新章节 = 1。
        let s = two_volume_structure();
        assert_eq!(find_latest_chapter_number(&s), 1);
    }

    #[test]
    fn test_compute_next_version_new_chapter() {
        // Adding a new chapter should update Y and reset Z.
        // 新增章节应更新 Y 并重置 Z。
        let current = UcxVersion::new(1, 3, 2);
        let s = single_volume_structure(5);
        let next = compute_next_version(&current, &ChangeKind::NewChapter, &s);
        assert_eq!(next, UcxVersion::new(1, 5, 0));
    }

    #[test]
    fn test_compute_next_version_new_volume() {
        // Starting a new volume should increment X.
        // 开始新卷应递增 X。
        let current = UcxVersion::new(1, 10, 3);
        let s = two_volume_structure();
        let next = compute_next_version(&current, &ChangeKind::NewVolume, &s);
        assert_eq!(next, UcxVersion::new(2, 1, 0));
    }

    #[test]
    fn test_compute_next_version_patch() {
        // A patch should only increment Z.
        // 修订应仅递增 Z。
        let current = UcxVersion::new(1, 5, 0);
        let s = single_volume_structure(5);
        let next = compute_next_version(&current, &ChangeKind::ModificationOnly, &s);
        assert_eq!(next, UcxVersion::new(1, 5, 1));
    }

    #[test]
    fn test_compute_next_version_no_change() {
        // No change should return the same version.
        // 无变更应返回相同版本。
        let current = UcxVersion::new(1, 5, 0);
        let s = single_volume_structure(5);
        let next = compute_next_version(&current, &ChangeKind::NoChange, &s);
        assert_eq!(next, current);
    }

    #[test]
    fn test_bump_patch() {
        let v = bump_patch(&UcxVersion::new(1, 5, 0));
        assert_eq!(v, UcxVersion::new(1, 5, 1));
    }

    #[test]
    fn test_bump_chapter() {
        let v = bump_chapter(&UcxVersion::new(1, 5, 2), 6);
        assert_eq!(v, UcxVersion::new(1, 6, 0));
    }

    #[test]
    fn test_bump_volume() {
        let v = bump_volume(&UcxVersion::new(1, 10, 3), 1);
        assert_eq!(v, UcxVersion::new(2, 1, 0));
    }

    #[test]
    fn test_collect_all_leaf_files() {
        let s = two_volume_structure();
        let files = collect_all_leaf_files(&s.structure);
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"vol1/ch-001.md".to_string()));
        assert!(files.contains(&"vol2/ch-001.md".to_string()));
    }
}
