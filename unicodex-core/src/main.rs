//! Unicodex CLI — unified novel file standard tool.
//!
//! This is the main entry point for the `ucx` command-line tool.
//! It delegates all business logic to the corresponding `ucx-*` modules.
//!
//! Unicodex CLI —— 统一小说文件标准工具。
//! 这是 `ucx` 命令行工具的主入口。
//! 所有业务逻辑委托给对应的 `ucx-*` 模块处理。

use std::path::PathBuf;

use clap::{Parser, Subcommand};

// =============================================================================
// CLI argument definitions / CLI 参数定义
// =============================================================================

/// Unicodex CLI — unified novel file standard tool.
///
/// UCX 命令行工具，用于创建、构建、验证和管理 UCX 格式的小说文件。
#[derive(Parser)]
#[command(name = "ucx")]
#[command(version, about, long_about = None)]
struct Cli {
    /// The subcommand to execute.
    /// 要执行的子命令。
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands for the `ucx` tool.
///
/// `ucx` 工具的可用子命令。
#[derive(Subcommand)]
enum Commands {
    /// Initialize a new UCX project.
    /// 初始化一个新的 UCX 项目。
    Init {
        /// Project directory path (default: current directory).
        /// 项目目录路径（默认：当前目录）。
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Work title.
        /// 作品标题。
        #[arg(short, long, default_value = "无标题")]
        name: String,

        /// Primary author name.
        /// 主要作者名称。
        #[arg(short, long, default_value = "未知")]
        author: String,

        /// Primary language (BCP 47 tag, e.g., "zh-CN", "en").
        /// 主要语言（BCP 47 标签，如 "zh-CN"、"en"）。
        #[arg(short, long, default_value = "zh-CN")]
        language: String,

        /// Allow name/author fields longer than 500 characters.
        /// 允许 name/author 字段超过 500 字符。
        #[arg(long, default_value_t = false)]
        allow_long_fields: bool,

        /// Create full directory structure (content/, assets/, extras/).
        /// 创建完整目录结构（content/、assets/、extras/）。
        #[arg(long, default_value_t = false)]
        full: bool,

        /// Skip Git repository initialization.
        /// 跳过 Git 仓库初始化。
        #[arg(long, default_value_t = false)]
        no_git: bool,

        /// Skip confirmation prompt for non-empty directories.
        /// 跳过非空目录确认提示。
        #[arg(short = 'y', long, default_value_t = false)]
        yes: bool,

        /// Interactive mode: prompt for project metadata.
        /// 交互模式：逐项询问项目元数据。
        #[arg(short = 'i', long, default_value_t = false)]
        interactive: bool,

        /// Initialize from existing .md files in the directory.
        /// 从目录中现有的 .md 文件初始化。
        #[arg(long, default_value_t = false)]
        from_existing: bool,
    },

    /// Build (pack) a UCX file from project directory.
    /// 从项目目录构建（打包）UCX 文件。
    Build {
        /// Project directory path (default: current directory).
        /// 项目目录路径（默认：当前目录）。
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Override output directory.
        /// 覆盖输出目录。
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Override output file name (without .ucx extension).
        /// 覆盖输出文件名（不含 .ucx 扩展名）。
        #[arg(short = 'O', long)]
        output_name: Option<String>,

        /// Force overwrite without confirmation.
        /// 强制覆盖，不提示确认。
        #[arg(short = 'f', long, default_value_t = false)]
        force: bool,

        /// Dry-run mode: validate and preview without creating the archive.
        /// 预演模式：仅校验和预览，不创建归档文件。
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Display information about a UCX file.
    /// 显示 UCX 文件的信息。
    Info {
        /// Path to the .ucx file.
        /// .ucx 文件路径。
        file: PathBuf,

        /// Output in JSON format (machine-readable).
        /// 以 JSON 格式输出（机器可读）。
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Verify the integrity and signatures of a UCX file.
    /// 验证 UCX 文件的完整性和签名。
    Verify {
        /// Path to the .ucx file.
        /// .ucx 文件路径。
        file: PathBuf,

        /// Verbose mode: show hash details for all files, even if valid.
        /// 详细模式：显示所有文件的哈希详情，即使验证通过。
        #[arg(short, long, default_value_t = false)]
        verbose: bool,
    },

    /// Check (validate) a UCX project without building.
    /// 校验 UCX 项目（不构建）。
    Check {
        /// Project directory path (default: current directory).
        /// 项目目录路径（默认：当前目录）。
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Unpack a UCX file to a directory.
    /// 将 UCX 文件解包到目录。
    Unpack {
        /// Arguments placeholder (command not yet implemented).
        /// 参数占位（命令尚未实现）。
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        _args: Vec<String>,
    },

    /// Sign a UCX file.
    /// 对 UCX 文件签名。
    Sign {
        /// Arguments placeholder (command not yet implemented).
        /// 参数占位（命令尚未实现）。
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        _args: Vec<String>,
    },

    /// Manage version numbers.
    /// 管理版本号。
    Version {
        /// Arguments placeholder (command not yet implemented).
        /// 参数占位（命令尚未实现）。
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        _args: Vec<String>,
    },

    /// Encrypt chapters or resources.
    /// 加密章节或资源。
    Encrypt {
        /// Arguments placeholder (command not yet implemented).
        /// 参数占位（命令尚未实现）。
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        _args: Vec<String>,
    },

    /// Decrypt chapters or resources.
    /// 解密章节或资源。
    Decrypt {
        /// Arguments placeholder (command not yet implemented).
        /// 参数占位（命令尚未实现）。
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        _args: Vec<String>,
    },
}

// =============================================================================
// Main entry point / 主入口
// =============================================================================

fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber with default level WARN.
    // Output is simplified: no timestamp, no module target — only level + message.
    // Users can override with RUST_LOG env var (e.g., RUST_LOG=info for verbose output).
    // 初始化 tracing 订阅器，默认级别为 WARN。
    // 输出已简化：无时间戳、无模块目标 — 仅级别 + 消息。
    // 用户可通过 RUST_LOG 环境变量覆盖（如 RUST_LOG=info 启用详细输出）。
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .without_time()
        .with_target(false)
        .init();

    // Parse CLI arguments.
    // 解析命令行参数。
    let cli = Cli::parse();

    // Dispatch to the corresponding module based on the subcommand.
    // 根据子命令分发到对应的模块。
    match cli.command {
        // =====================================================================
        // ucx init — Initialize a new UCX project.
        // ucx init — 初始化新的 UCX 项目。
        // =====================================================================
        Commands::Init {
            path,
            name,
            author,
            language,
            allow_long_fields,
            full,
            no_git,
            yes,
            interactive,
            from_existing,
        } => {
            // Determine final name/author/language values.
            // In interactive mode, prompt for each value with CLI args as defaults.
            // 确定最终的 name/author/language 值。
            // 交互模式下，逐项询问，CLI 参数作为默认值显示。
            let (final_name, final_author, final_language) = if interactive {
                let n = prompt_with_default("作品标题", &name);
                let a = prompt_with_default("作者名", &author);
                let l = prompt_with_default("语言标签 (BCP 47)", &language);
                (n, a, l)
            } else {
                (name.clone(), author.clone(), language.clone())
            };
            // Check if initializing in a non-empty directory without unicodex.toml.
            // 检查是否在非空目录中初始化（且不存在 unicodex.toml）。
            if path == std::path::Path::new(".") && !yes {
                let has_unicodex_toml = path.join("unicodex.toml").exists();
                if !has_unicodex_toml {
                    // Check if directory is non-empty.
                    // 检查目录是否非空。
                    let is_non_empty = path.read_dir()
                        .map(|mut d| d.next().is_some())
                        .unwrap_or(false);
                    if is_non_empty {
                        eprintln!("Warning: current directory is not empty.");
                        eprint!("Continue initializing UCX project here? [y/N] ");
                        let mut input = String::new();
                        if std::io::stdin().read_line(&mut input).is_ok() {
                            let answer = input.trim().to_lowercase();
                            if answer != "y" && answer != "yes" {
                                println!("Aborted.");
                                return Ok(());
                            }
                        } else {
                            // Non-interactive environment: proceed by default.
                            // 非交互环境：默认继续。
                        }
                    }
                }
            }

            let options = ucx_init::InitOptions {
                name: final_name.clone(),
                author: final_author,
                language: final_language,
                allow_long_fields,
                full,
                no_git,
            };

            if from_existing {
                ucx_init::init_from_existing(&path, &options)?;
                println!("UCX project initialized from existing files: \"{}\" at {}", final_name, path.display());
            } else {
                ucx_init::init(&path, &options)?;
                println!("UCX project initialized: \"{}\" at {}", final_name, path.display());
            }
        }

        // =====================================================================
        // ucx build — Build a UCX file from project directory.
        // ucx build — 从项目目录构建 UCX 文件。
        // =====================================================================
        Commands::Build {
            path,
            output_dir,
            output_name,
            force,
            dry_run,
        } => {
            // Dry-run mode: validate project and print what would be packed.
            // 预演模式：校验项目并打印将会打包的内容。
            if dry_run {
                let options = ucx_build::BuildOptions {
                    output_dir,
                    output_name,
                    dry_run: true,
                };
                let result = ucx_build::dry_run(&path, &options)?;
                println!("=== Dry Run ===");
                println!("Output: {}", result.output_path.display());
                println!();
                println!("Files ({}):", result.files.len());
                for file in &result.files {
                    println!("  {} ({} bytes)", file.archive_path, file.size);
                }
                println!();
                println!("Total size: {} bytes", result.total_size);
                println!();
                println!("Validation passed. Ready to build.");
                return Ok(());
            }

            // Pre-check: if the output file already exists and --force not specified,
            // prompt the user for confirmation.
            // 预检查：如果输出文件已存在且未指定 --force，提示用户确认。
            let expected_output = ucx_build::resolve_output_path(
                &path,
                &ucx_build::BuildOptions {
                    output_dir: output_dir.clone(),
                    output_name: output_name.clone(),
                    ..Default::default()
                },
            );
            if let Ok(ref out_path) = expected_output {
                if out_path.exists() && !force {
                    eprintln!("Warning: output file already exists: {}", out_path.display());
                    eprint!("Overwrite? [y/N] ");
                    let mut input = String::new();
                    if std::io::stdin().read_line(&mut input).is_ok() {
                        let answer = input.trim().to_lowercase();
                        if answer != "y" && answer != "yes" {
                            println!("Aborted.");
                            return Ok(());
                        }
                    }
                }
            }

            let options = ucx_build::BuildOptions {
                output_dir,
                output_name,
                ..Default::default()
            };

            let ucx_path = ucx_build::build(&path, &options)?;
            println!("UCX file built: {}", ucx_path.display());
        }

        // =====================================================================
        // ucx info — Display UCX file metadata.
        // ucx info — 显示 UCX 文件元数据。
        // =====================================================================
        Commands::Info { file, json } => {
            let archive = ucx_parse::open(&file)?;
            let codex = archive.codex();

            // JSON output mode: print codex.json pretty-printed and exit.
            // JSON 输出模式：输出格式化的 codex.json 并退出。
            if json {
                let json_str = serde_json::to_string_pretty(codex)
                    .map_err(|e| anyhow::anyhow!("JSON serialization error: {e}"))?;
                println!("{json_str}");
                return Ok(());
            }

            // Print work information.
            // 打印作品信息。
            println!("=== UCX File Info ===");
            println!("File: {}", file.display());

            // File size / 文件大小
            let size = archive.file_size();
            println!("Size: {}", format_file_size(size));
            println!();

            // Title / 标题
            println!("Title: {}", codex.title.main);
            if let Some(ref subtitle) = codex.title.subtitle {
                println!("Subtitle: {subtitle}");
            }

            // UCX ID / 标识
            println!("UCX ID: {}", codex.identifier.ucx_id);
            if let Some(ref isbn) = codex.identifier.isbn {
                println!("ISBN: {isbn}");
            }

            // Version / 版本
            println!("Version: {}", codex.version);

            // Created-By / 创建工具
            let manifest = archive.manifest();
            if let Some(ref created_by) = manifest.created_by {
                println!("Created-By: {created_by}");
            }

            // Creators / 创作者
            println!();
            println!("Creators:");
            for creator in &codex.creators {
                println!("  - {} ({})", creator.name, creator.role);
            }

            // Language / 语言
            println!();
            println!("Language: {}", codex.language);

            // Status / 状态
            if let Some(ref status) = codex.status {
                println!("Status: {status}");
            }

            // Word count / 字数
            if let Some(word_count) = codex.word_count {
                println!("Word count: {word_count}");
            }

            // Genre / 体裁
            if let Some(ref genre) = codex.genre {
                println!("Genre: {}", genre.join(", "));
            }

            // Tags / 标签
            if let Some(ref tags) = codex.tags {
                println!("Tags: {}", tags.join(", "));
            }

            // Dates / 日期
            if let Some(ref dates) = codex.dates {
                println!();
                println!("Dates:");
                if let Some(ref created) = dates.created {
                    println!("  Created:  {created}");
                }
                if let Some(ref published) = dates.published {
                    println!("  Published: {published}");
                }
                if let Some(ref modified) = dates.modified {
                    println!("  Modified: {modified}");
                }
            }

            // Description / 简介
            if let Some(ref desc) = codex.description {
                if let Some(ref short) = desc.short {
                    println!();
                    println!("Description: {short}");
                }
            }

            // Structure / 结构
            let structure = archive.structure();
            let chapter_count = archive.chapter_count();
            println!();
            println!("Structure ({} top-level nodes, {} chapters):",
                structure.structure.len(),
                chapter_count
            );
            print_structure_tree(&structure.structure, 1);

            // Manifest / 清单
            println!();
            println!("Manifest ({} entries, {}):",
                manifest.entries.len(),
                manifest.hash_algorithm
            );
            for entry in &manifest.entries {
                println!("  {} ({} bytes)", entry.name, entry.size);
            }
        }

        // =====================================================================
        // ucx verify — Verify UCX file integrity.
        // ucx verify — 验证 UCX 文件完整性。
        // =====================================================================
        Commands::Verify { file, verbose } => {
            let start = std::time::Instant::now();

            let mut archive = ucx_parse::open(&file)?;
            let results = archive.verify_hashes()?;

            let elapsed = start.elapsed();

            let total = results.len();
            let valid = results.iter().filter(|r| r.valid).count();
            let invalid = total - valid;

            println!("=== UCX Integrity Verification ===");
            println!("File: {}", file.display());
            println!();

            for result in &results {
                let status = if result.valid { "OK" } else { "FAIL" };
                println!("  [{status}] {}", result.name);
                if !result.valid || verbose {
                    println!("       expected: {}", result.expected);
                    println!("       actual:   {}", result.actual);
                }
            }

            println!();
            println!("Result: {valid}/{total} files valid");

            // Show timing information.
            // 显示耗时信息。
            let ms = elapsed.as_millis();
            if ms < 1000 {
                println!("Verified in {ms}ms");
            } else {
                println!("Verified in {:.2}s", elapsed.as_secs_f64());
            }

            // Check for digital signatures.
            // 检查数字签名。
            let has_signatures = archive.list_files().iter().any(|f| f.starts_with("META-INF/signatures/"));
            if !has_signatures {
                println!();
                println!("Note: no digital signatures found — integrity verified against MANIFEST.MF hashes only");
            }

            if invalid > 0 {
                anyhow::bail!("{invalid} file(s) failed integrity check");
            }
        }

        // =====================================================================
        // ucx check — Validate project without building.
        // ucx check — 校验项目（不构建）。
        // =====================================================================
        Commands::Check { path } => {
            let result = ucx_build::check(&path)?;

            println!("=== UCX Project Check ===");
            println!("Path: {}", path.display());
            println!();

            let mut passed = 0;
            let mut failed = 0;

            for item in &result.items {
                let icon = if item.passed { "OK" } else { "FAIL" };
                println!("  [{icon}] {}: {}", item.name, item.message);
                if item.passed {
                    passed += 1;
                } else {
                    failed += 1;
                }
            }

            println!();
            println!("Result: {passed} passed, {failed} failed");

            if !result.all_passed() {
                anyhow::bail!("{failed} check(s) failed");
            }
        }

        // =====================================================================
        // Unimplemented subcommands (Phase 2+) / 未实现的子命令（第二阶段+）
        // =====================================================================
        Commands::Unpack { .. } => {
            println!("ucx unpack: not yet implemented (planned for Phase 2)");
        }
        Commands::Sign { .. } => {
            println!("ucx sign: not yet implemented (planned for Phase 3)");
        }
        Commands::Version { .. } => {
            println!("ucx version: not yet implemented (planned for Phase 2)");
        }
        Commands::Encrypt { .. } => {
            println!("ucx encrypt: not yet implemented (planned for Phase 4)");
        }
        Commands::Decrypt { .. } => {
            println!("ucx decrypt: not yet implemented (planned for Phase 4)");
        }
    }

    Ok(())
}

// =============================================================================
// Helper functions / 辅助函数
// =============================================================================

/// Prompt the user for input with a default value shown in brackets.
///
/// Returns the user's input, or the default if they press Enter.
///
/// 提示用户输入，括号中显示默认值。
/// 返回用户输入，或按 Enter 时返回默认值。
fn prompt_with_default(label: &str, default: &str) -> String {
    use std::io::Write;
    eprint!("{label} [{default}]: ");
    std::io::stderr().flush().ok();
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            default.to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        default.to_string()
    }
}

/// Recursively print the content structure tree with indentation.
///
/// 递归打印内容结构树（带缩进）。
///
/// # Arguments / 参数
///
/// * `nodes` - The structure nodes to print. / 要打印的结构节点。
/// * `depth` - Current indentation depth. / 当前缩进深度。
fn print_structure_tree(nodes: &[ucx_types::StructureNode], depth: usize) {
    let indent = "  ".repeat(depth);
    for node in nodes {
        if let Some(ref file) = node.file {
            // Leaf node: show title and file path.
            // 叶子节点：显示标题和文件路径。
            println!("{indent}- {} [{}]", node.title, file);
        } else {
            // Container node: show title and recurse into children.
            // 容器节点：显示标题并递归子节点。
            let type_hint = node
                .node_type
                .as_deref()
                .map(|t| format!(" ({t})"))
                .unwrap_or_default();
            println!("{indent}+ {}{type_hint}", node.title);
            if let Some(ref children) = node.children {
                print_structure_tree(children, depth + 1);
            }
        }
    }
}

/// Format a file size in bytes into a human-readable string.
///
/// 将字节数格式化为人类可读的字符串。
///
/// # Examples / 示例
///
/// - 512 → "512 B"
/// - 1536 → "1.5 KB"
/// - 1_048_576 → "1.0 MB"
fn format_file_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size = bytes as f64;
    if size < KB {
        format!("{bytes} B")
    } else if size < MB {
        format!("{:.1} KB", size / KB)
    } else if size < GB {
        format!("{:.1} MB", size / MB)
    } else {
        format!("{:.1} GB", size / GB)
    }
}
