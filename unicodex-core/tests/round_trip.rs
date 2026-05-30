//! Round-trip integration tests for the UCX pipeline.
//!
//! These tests verify the complete lifecycle:
//! `ucx init` → `ucx build` → `ucx parse/info` → `ucx verify`
//!
//! UCX 管线的往返集成测试。
//! 验证完整生命周期：`ucx init` → `ucx build` → `ucx parse/info` → `ucx verify`

use tempfile::TempDir;

// =============================================================================
// Test: Full round-trip (init → build → parse → verify)
// =============================================================================

/// Test the complete lifecycle: init a project, build it, parse the output,
/// and verify its integrity.
///
/// 测试完整生命周期：初始化项目 → 构建 → 解析输出 → 验证完整性。
#[test]
fn test_full_round_trip() {
    // --- Step 1: Initialize a new UCX project. ---
    // --- 步骤 1：初始化新的 UCX 项目。 ---
    let tmp = TempDir::new().expect("failed to create temp dir");
    let project_dir = tmp.path().join("round-trip-novel");

    let init_options = ucx_init::InitOptions {
        name: "往返测试小说".to_string(),
        author: "测试作者".to_string(),
        language: "zh-CN".to_string(),
        ..Default::default()
    };

    ucx_init::init(&project_dir, &init_options).expect("init should succeed");

    // Verify project files were created.
    // 验证项目文件已创建。
    assert!(project_dir.join("unicodex.toml").exists());
    assert!(project_dir.join("content/struct.json").exists());
    assert!(project_dir.join("content/chapter-001.md").exists());

    // --- Step 2: Build the UCX file. ---
    // --- 步骤 2：构建 UCX 文件。 ---
    let build_options = ucx_build::BuildOptions::default();
    let ucx_path = ucx_build::build(&project_dir, &build_options).expect("build should succeed");

    // Verify the .ucx file was created and has content.
    // 验证 .ucx 文件已创建且有内容。
    assert!(ucx_path.exists(), ".ucx file should exist");
    let file_size = std::fs::metadata(&ucx_path).unwrap().len();
    assert!(file_size > 0, ".ucx file should not be empty");

    // --- Step 3: Parse the UCX file and verify metadata. ---
    // --- 步骤 3：解析 UCX 文件并验证元数据。 ---
    let mut archive = ucx_parse::open(&ucx_path).expect("parse should succeed");

    // Verify codex metadata matches what we initialized.
    // 验证 codex 元数据与初始化内容匹配。
    let codex = archive.codex();
    assert_eq!(codex.title.main, "往返测试小说");
    assert_eq!(codex.language, "zh-CN");
    assert_eq!(codex.creators.len(), 1);
    assert_eq!(codex.creators[0].name, "测试作者");
    assert_eq!(codex.creators[0].role, "author");

    // Verify the UCX ID has the correct format.
    // 验证 UCX ID 格式正确。
    assert!(
        codex.identifier.ucx_id.as_str().starts_with("urn:ucx:"),
        "UCX ID should start with 'urn:ucx:'"
    );

    // Verify content structure.
    // 验证内容结构。
    let structure = archive.structure();
    assert_eq!(structure.structure.len(), 1);
    assert_eq!(structure.structure[0].title, "第一章");
    assert_eq!(
        structure.structure[0].file.as_deref(),
        Some("chapter-001.md")
    );

    // Verify manifest has the expected entries.
    // 验证清单包含预期条目。
    let manifest = archive.manifest();
    assert!(
        manifest.entries.len() >= 3,
        "manifest should have at least 3 entries (codex.json, struct.json, chapter-001.md)"
    );
    let entry_names: Vec<&str> = manifest.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(entry_names.contains(&"metadata/codex.json"));
    assert!(entry_names.contains(&"content/struct.json"));
    assert!(entry_names.contains(&"content/chapter-001.md"));

    // --- Step 4: Read chapter content. ---
    // --- 步骤 4：读取章节内容。 ---
    let chapter = archive
        .read_chapter("chapter-001.md")
        .expect("should read chapter");
    assert!(chapter.contains("第一章"), "chapter should contain heading");
    assert!(
        chapter.contains("开始你的创作"),
        "chapter should contain placeholder text"
    );

    // --- Step 5: Verify file integrity (BLAKE3 hashes). ---
    // --- 步骤 5：验证文件完整性（BLAKE3 哈希）。 ---
    let results = archive
        .verify_hashes()
        .expect("hash verification should succeed");

    assert!(
        !results.is_empty(),
        "should have at least one verification result"
    );
    for result in &results {
        assert!(
            result.valid,
            "hash mismatch for '{}': expected={}, actual={}",
            result.name, result.expected, result.actual
        );
    }
}

// =============================================================================
// Test: Build with custom options
// =============================================================================

/// Test building with custom output directory and file name.
///
/// 测试使用自定义输出目录和文件名进行构建。
#[test]
fn test_build_with_custom_options() {
    let tmp = TempDir::new().expect("failed to create temp dir");
    let project_dir = tmp.path().join("custom-opts-project");

    // Initialize project.
    // 初始化项目。
    ucx_init::init(
        &project_dir,
        &ucx_init::InitOptions {
            name: "自定义选项测试".to_string(),
            author: "作者".to_string(),
            language: "en".to_string(),
            ..Default::default()
        },
    )
    .expect("init failed");

    // Build with custom output.
    // 使用自定义输出构建。
    let custom_output = tmp.path().join("custom-build-output");
    let options = ucx_build::BuildOptions {
        output_dir: Some(custom_output.clone()),
        output_name: Some("my-custom-book".to_string()),
        ..Default::default()
    };

    let ucx_path = ucx_build::build(&project_dir, &options).expect("build failed");

    // Verify output path matches expectations.
    // 验证输出路径符合预期。
    assert_eq!(ucx_path, custom_output.join("my-custom-book.ucx"));
    assert!(ucx_path.exists());

    // Verify the built file is valid.
    // 验证构建的文件有效。
    let archive = ucx_parse::open(&ucx_path).expect("parse failed");
    assert_eq!(archive.codex().title.main, "自定义选项测试");
    assert_eq!(archive.codex().language, "en");
}

// =============================================================================
// Test: Multi-chapter project
// =============================================================================

/// Test building a project with multiple chapters and a nested structure.
///
/// 测试构建包含多个章节和嵌套结构的项目。
#[test]
fn test_multi_chapter_build() {
    let tmp = TempDir::new().expect("failed to create temp dir");
    let project_dir = tmp.path().join("multi-chapter");

    // Initialize project.
    // 初始化项目。
    ucx_init::init(
        &project_dir,
        &ucx_init::InitOptions {
            name: "多章节测试".to_string(),
            author: "作者".to_string(),
            language: "zh-CN".to_string(),
            ..Default::default()
        },
    )
    .expect("init failed");

    // Add more chapters.
    // 添加更多章节。
    std::fs::write(
        project_dir.join("content/chapter-002.md"),
        "# 第二章\n\n第二章的内容。\n",
    )
    .unwrap();
    std::fs::write(
        project_dir.join("content/chapter-003.md"),
        "# 第三章\n\n第三章的内容。\n",
    )
    .unwrap();

    // Update struct.json to include all chapters.
    // 更新 struct.json 以包含所有章节。
    let structure = ucx_types::Structure {
        schema: Some("https://unicodex.org/schemas/struct-v1.json".to_string()),
        version: "1.0".to_string(),
        structure: vec![
            ucx_types::StructureNode {
                title: "第一章".to_string(),
                file: Some("chapter-001.md".to_string()),
                children: None,
                node_type: None,
                id: None,
                name: None,
                style: None,
                encryption: None,
            },
            ucx_types::StructureNode {
                title: "第二章".to_string(),
                file: Some("chapter-002.md".to_string()),
                children: None,
                node_type: None,
                id: None,
                name: None,
                style: None,
                encryption: None,
            },
            ucx_types::StructureNode {
                title: "第三章".to_string(),
                file: Some("chapter-003.md".to_string()),
                children: None,
                node_type: None,
                id: None,
                name: None,
                style: None,
                encryption: None,
            },
        ],
    };
    let struct_json = serde_json::to_string_pretty(&structure).unwrap();
    std::fs::write(project_dir.join("content/struct.json"), &struct_json).unwrap();

    // Build.
    // 构建。
    let ucx_path =
        ucx_build::build(&project_dir, &ucx_build::BuildOptions::default()).expect("build failed");

    // Parse and verify.
    // 解析并验证。
    let mut archive = ucx_parse::open(&ucx_path).expect("parse failed");

    // Verify structure has 3 chapters.
    // 验证结构包含 3 章。
    assert_eq!(archive.structure().structure.len(), 3);

    // Verify manifest has entries for all chapters.
    // 验证清单包含所有章节的条目。
    let entry_names: Vec<&str> = archive
        .manifest()
        .entries
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert!(entry_names.contains(&"content/chapter-001.md"));
    assert!(entry_names.contains(&"content/chapter-002.md"));
    assert!(entry_names.contains(&"content/chapter-003.md"));

    // Read each chapter.
    // 读取每个章节。
    let ch1 = archive.read_chapter("chapter-001.md").unwrap();
    assert!(ch1.contains("第一章"));
    let ch2 = archive.read_chapter("chapter-002.md").unwrap();
    assert!(ch2.contains("第二章"));
    let ch3 = archive.read_chapter("chapter-003.md").unwrap();
    assert!(ch3.contains("第三章"));

    // Verify all hashes pass.
    // 验证所有哈希通过。
    let results = archive.verify_hashes().unwrap();
    for result in &results {
        assert!(result.valid, "hash mismatch for '{}'", result.name);
    }
}

// =============================================================================
// Test: Init idempotency check (double init should fail)
// =============================================================================

/// Test that initializing twice in the same directory fails correctly.
///
/// 测试在同一目录二次初始化应正确失败。
#[test]
fn test_double_init_fails() {
    let tmp = TempDir::new().expect("failed to create temp dir");
    let project_dir = tmp.path().join("double-init");

    let options = ucx_init::InitOptions {
        name: "第一次".to_string(),
        author: "作者".to_string(),
        language: "zh-CN".to_string(),
        ..Default::default()
    };

    // First init should succeed.
    // 第一次初始化应成功。
    ucx_init::init(&project_dir, &options).expect("first init failed");

    // Second init should fail with AlreadyExists.
    // 第二次初始化应以 AlreadyExists 失败。
    let result = ucx_init::init(&project_dir, &options);
    assert!(result.is_err(), "second init should fail");
}
