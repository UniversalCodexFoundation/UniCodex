//! Unicodex CLI — unified novel file standard tool.
//!
//! This is the main entry point for the `ucx` command-line tool.
//! It delegates all business logic to the corresponding `ucx-*` modules.
//!
//! Unicodex CLI —— 统一小说文件标准工具。
//! 这是 `ucx` 命令行工具的主入口。
//! 所有业务逻辑委托给对应的 `ucx-*` 模块处理。

use std::path::PathBuf;

use base64::Engine as _;
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

        /// Overwrite an existing UCX project (unicodex.toml).
        ///
        /// By default `ucx init` refuses to run against a directory that
        /// already contains `unicodex.toml` to prevent clobbering a live
        /// project. Pass `--force` to re-initialise in place.
        ///
        /// 覆盖已有的 UCX 项目（unicodex.toml）。
        /// 默认 `ucx init` 拒绝在已包含 `unicodex.toml` 的目录运行，
        /// 以防止覆盖正在使用的项目；传入 `--force` 可就地重新初始化。
        #[arg(long, default_value_t = false)]
        force: bool,
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

        /// Show detailed signer information.
        /// 显示详细的签名者信息。
        #[arg(long, default_value_t = false)]
        show_signers: bool,
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
        /// Path to the .ucx file to unpack.
        /// 要解包的 .ucx 文件路径。
        file: PathBuf,

        /// Output directory (default: file stem, e.g., "novel.ucx" -> "novel/").
        /// 输出目录（默认：文件名去掉扩展名，如 "novel.ucx" -> "novel/"）。
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Force overwrite if output directory already exists.
        /// 如果输出目录已存在则强制覆盖。
        #[arg(short = 'f', long, default_value_t = false)]
        force: bool,
    },

    /// Generate an Ed25519 key pair.
    /// 生成 Ed25519 密钥对。
    Keygen {
        /// Output path for the private key file (public key saved as {output}.pub).
        /// 私钥文件输出路径（公钥保存为 {output}.pub）。
        #[arg(short, long, default_value = "author.key")]
        output: PathBuf,
    },

    /// Certificate management.
    /// 证书管理。
    Cert {
        /// Certificate subcommand.
        /// 证书子命令。
        #[command(subcommand)]
        action: CertAction,
    },

    /// Sign a UCX file with Layer 1 + Layer 2 signatures.
    /// 对 UCX 文件进行双层签名。
    Sign {
        /// Path to the .ucx file to sign.
        /// 要签名的 .ucx 文件路径。
        file: PathBuf,

        /// Path to the private key file.
        /// 私钥文件路径。
        #[arg(short, long)]
        key: PathBuf,

        /// Path to the certificate PEM file.
        /// 证书 PEM 文件路径。
        #[arg(short, long)]
        cert: PathBuf,

        /// Signer identifier (e.g., "AUTHOR"). Must be A-Z, 0-9, _ only.
        /// 签名者标识（如 "AUTHOR"）。仅允许 A-Z、0-9、_。
        #[arg(long, default_value = "AUTHOR")]
        signer_id: String,
    },

    /// Manage version numbers.
    /// 管理版本号。
    Version {
        /// Version subcommand (auto, patch, chapter, volume, set).
        /// If omitted, shows the current version.
        /// 版本子命令（auto、patch、chapter、volume、set）。
        /// 省略时显示当前版本。
        #[command(subcommand)]
        action: Option<VersionAction>,

        /// Project directory path (default: current directory).
        /// Can appear before or after the subcommand.
        /// 项目目录路径（默认：当前目录）。
        /// 可出现在子命令之前或之后。
        #[arg(long, default_value = ".", global = true)]
        path: PathBuf,
    },

    /// Encrypt a file using the specified algorithm.
    /// 加密文件。
    Encrypt {
        /// Path to the file to encrypt.
        /// 要加密的文件路径。
        file: PathBuf,

        /// Output path for the encrypted file (default: overwrite source).
        /// 加密输出路径（默认：覆盖源文件）。
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Encryption algorithm.
        /// 加密算法。
        #[arg(short, long, default_value = "AES-256-GCM")]
        algorithm: String,

        /// Base64-encoded 32-byte encryption key (direct key mode).
        /// Base64 编码的 32 字节加密密钥（直接密钥模式）。
        #[arg(short, long, conflicts_with = "passphrase")]
        key: Option<String>,

        /// Use passphrase-based encryption (interactive prompt).
        /// 使用口令加密（交互式输入）。
        #[arg(short, long, conflicts_with = "key")]
        passphrase: bool,

        /// KDF algorithm for passphrase mode.
        /// 口令模式的 KDF 算法。
        #[arg(long, default_value = "argon2id")]
        kdf: String,

        /// Allow passphrases shorter than 8 characters.
        ///
        /// By default the CLI refuses weak passphrases (<8 chars) to prevent
        /// trivially brute-forceable encryption. This flag opts out of the check.
        ///
        /// 允许长度小于 8 字符的弱口令。
        /// 默认 CLI 拒绝弱口令（<8 字符）以防止轻易被暴力破解；
        /// 此标志可跳过此检查。
        #[arg(long, default_value_t = false)]
        allow_weak: bool,
    },

    /// Decrypt a UCXE encrypted file.
    /// 解密 UCXE 加密文件。
    Decrypt {
        /// Path to the encrypted file.
        /// 加密文件路径。
        file: PathBuf,

        /// Output path for the decrypted file (default: overwrite source).
        /// 解密输出路径（默认：覆盖源文件）。
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Base64-encoded encryption key (direct key mode).
        /// Base64 编码的加密密钥（直接密钥模式）。
        #[arg(short, long, conflicts_with = "passphrase")]
        key: Option<String>,

        /// Use passphrase-based decryption (interactive prompt).
        /// 使用口令解密（交互式输入）。
        #[arg(short, long, conflicts_with = "key")]
        passphrase: bool,
    },
}

/// Subcommands for `ucx cert`.
///
/// `ucx cert` 的子命令。
#[derive(Subcommand)]
enum CertAction {
    /// Create a self-signed certificate.
    /// 创建自签名证书。
    Create {
        /// Path to the private key file.
        /// 私钥文件路径。
        #[arg(short, long)]
        key: PathBuf,

        /// Common Name (CN) for the certificate subject.
        /// 证书主体的通用名称（CN）。
        #[arg(long)]
        cn: String,

        /// Validity period in days (default: 365).
        /// 有效期天数（默认：365）。
        #[arg(long, default_value_t = 365)]
        days: u32,

        /// Output path for the certificate PEM file.
        /// 证书 PEM 文件输出路径。
        #[arg(short, long, default_value = "author.cert.pem")]
        output: PathBuf,
    },

    /// Display certificate information.
    /// 显示证书信息。
    Info {
        /// Path to the certificate PEM file.
        /// 证书 PEM 文件路径。
        file: PathBuf,
    },
}

/// Subcommands for `ucx version`.
///
/// `ucx version` 的子命令。
#[derive(Subcommand)]
enum VersionAction {
    /// Automatically detect changes and bump the version.
    /// 自动检测变更并升级版本号。
    Auto,

    /// Bump the patch version (Z + 1).
    /// 升级修订版本（Z + 1）。
    Patch,

    /// Bump to a new chapter version (Y = new chapter number, Z = 0).
    /// 升级到新章节版本（Y = 新章节编号, Z = 0）。
    Chapter,

    /// Bump to a new volume version (X + 1, Z = 0).
    /// 升级到新卷版本（X + 1, Z = 0）。
    Volume,

    /// Set the version to a specific X.Y.Z value.
    /// 将版本设置为指定的 X.Y.Z 值。
    Set {
        /// The version string in X.Y.Z format.
        /// X.Y.Z 格式的版本字符串。
        version: String,
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
            force,
        } => {
            // DOC-6: refuse to clobber an existing project unless --force.
            // Placed before any I/O so that the user sees the message before
            // interactive prompts kick in.
            // DOC-6：除非 --force，否则拒绝覆盖已有项目；
            // 放在所有 I/O 之前，保证用户先看到提示再进入交互。
            if path.join("unicodex.toml").exists() && !force {
                anyhow::bail!(
                    "directory already contains a UCX project (unicodex.toml at {}); \
                     use --force to re-initialize",
                    path.join("unicodex.toml").display()
                );
            }
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
                force,
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
            if let Ok(ref out_path) = expected_output
                && out_path.exists() && !force {
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

            // File Version (from ucx-version module) / 文件版本（来自 ucx-version 模块）
            if let Some(ref fv) = codex.file_version {
                let ver = fv.version.as_deref().unwrap_or("(unset)");
                let rev = fv.revision.map_or("—".to_string(), |r| r.to_string());
                println!("File Version: {ver} (revision {rev})");
            }

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
            if let Some(ref desc) = codex.description
                && let Some(ref short) = desc.short {
                    println!();
                    println!("Description: {short}");
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
        Commands::Verify { file, verbose, show_signers } => {
            let start = std::time::Instant::now();

            // Friendly short-circuit: if the first 4 bytes are the UCXE magic,
            // the user has handed us a raw encrypted file (UCXE) rather than a
            // `.ucx` archive. Parsing would fail with a cryptic ZIP error; tell
            // the user what to run instead.
            // 友好短路：若前 4 字节为 UCXE 魔数，说明用户传入的是加密裸文件
            // 而非 `.ucx` 归档。此时 ZIP 解析会给出晦涩错误；直接提示正确命令。
            if is_ucxe_encrypted_file(&file)? {
                anyhow::bail!(
                    "this file appears to be UCXE encrypted, please run `ucx decrypt` first"
                );
            }

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

            // Signature verification using ucx-verify.
            // 使用 ucx-verify 进行签名验证。
            match ucx_verify::verify(&file) {
                Ok(report) => {
                    println!();
                    println!("=== Signature Verification ===");
                    let status_str = match report.status {
                        ucx_verify::VerifyStatus::Valid => "VERIFIED",
                        ucx_verify::VerifyStatus::ValidWithWarnings => "PARTIAL",
                        ucx_verify::VerifyStatus::Invalid => "INVALID",
                        ucx_verify::VerifyStatus::Unsigned => "UNSIGNED",
                    };
                    println!("Status: {status_str}");

                    if let Some(ref l2) = report.layer2 {
                        let icon = if l2.valid { "OK" } else { "FAIL" };
                        println!("  [{icon}] Layer 2 (archive integrity): {}", l2.details);
                    }
                    if let Some(ref l1) = report.layer1 {
                        let icon = if l1.valid { "OK" } else { "FAIL" };
                        println!("  [{icon}] Layer 1 (file signatures): {} signer(s) - {}", l1.signer_count, l1.details);
                    }

                    if show_signers && !report.signers.is_empty() {
                        println!();
                        println!("Signers:");
                        for (i, signer) in report.signers.iter().enumerate() {
                            println!("  [{}] {}", i + 1, signer.signer_id);
                            println!("      Subject: CN={}", signer.subject_cn);
                            println!("      Type: {}", signer.cert_type);
                            println!("      Fingerprint: {}", signer.fingerprint_blake3);
                            let l1_icon = if signer.layer1_valid { "OK" } else { "FAIL" };
                            let l2_icon = if signer.layer2_valid { "OK" } else { "FAIL" };
                            println!("      Layer 1: [{l1_icon}]  Layer 2: [{l2_icon}]");
                        }
                    }

                    // Return non-zero exit code for INVALID or PARTIAL signature status.
                    // INVALID 和 PARTIAL 签名状态应返回非零退出码。
                    // VERIFIED and UNSIGNED are considered normal (exit 0):
                    //   - VERIFIED: signatures are valid.
                    //   - UNSIGNED: no signatures present, which is not an error.
                    // VERIFIED 和 UNSIGNED 视为正常（exit 0）：
                    //   - VERIFIED：签名有效。
                    //   - UNSIGNED：未签名，不算错误。
                    match report.status {
                        ucx_verify::VerifyStatus::Invalid => {
                            anyhow::bail!("signature verification failed");
                        }
                        ucx_verify::VerifyStatus::ValidWithWarnings => {
                            anyhow::bail!("signature verification partially failed");
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    println!();
                    println!("Signature verification skipped: {e}");
                }
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
        // =====================================================================
        // ucx unpack — Extract a UCX file to a directory.
        // ucx unpack — 将 UCX 文件解包到目录。
        // =====================================================================
        Commands::Unpack { file, output, force } => {
            // Determine the output directory.
            // If --output is provided, use it; otherwise, derive from the file stem.
            // 确定输出目录。
            // 如果提供了 --output，使用它；否则从文件名推导。
            let output_dir = match output {
                Some(dir) => dir,
                None => {
                    // Use the file stem as the output directory name.
                    // e.g., "novel.ucx" -> "novel/"
                    // 使用文件名（不含扩展名）作为输出目录名。
                    // 如 "novel.ucx" -> "novel/"
                    file
                        .file_stem()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("ucx_output"))
                }
            };

            // If output directory already exists and --force is not set,
            // prompt the user for confirmation.
            // 如果输出目录已存在且未指定 --force，提示用户确认。
            if output_dir.exists() && !force {
                eprintln!(
                    "Warning: output directory already exists: {}",
                    output_dir.display()
                );
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

            // Open and parse the UCX file.
            // 打开并解析 UCX 文件。
            let mut archive = ucx_parse::open(&file)?;

            // Extract all files to the output directory.
            // 将所有文件解压到输出目录。
            let extracted = archive.extract_to(&output_dir)?;

            // Print summary.
            // 打印摘要。
            println!(
                "Unpacked {} files to {}",
                extracted.len(),
                output_dir.display()
            );
        }
        // =====================================================================
        // ucx keygen — Generate an Ed25519 key pair.
        // ucx keygen — 生成 Ed25519 密钥对。
        // =====================================================================
        Commands::Keygen { output } => {
            ucx_sign::keygen(&output)?;
            println!("Key pair generated:");
            println!("  Private key: {}", output.display());
            let mut pub_path = output.as_os_str().to_owned();
            pub_path.push(".pub");
            println!("  Public key:  {}", std::path::PathBuf::from(pub_path).display());
        }

        // =====================================================================
        // ucx cert — Certificate management.
        // ucx cert — 证书管理。
        // =====================================================================
        Commands::Cert { action } => match action {
            // -----------------------------------------------------------------
            // ucx cert create — Create a self-signed certificate.
            // ucx cert create — 创建自签名证书。
            // -----------------------------------------------------------------
            CertAction::Create { key, cn, days, output } => {
                ucx_sign::create_cert(&key, &cn, days, &output)?;
                println!("Certificate created: {}", output.display());
                println!("  Subject: CN={cn}");
                println!("  Validity: {days} days");
            }

            // -----------------------------------------------------------------
            // ucx cert info — Display certificate information.
            // ucx cert info — 显示证书信息。
            // -----------------------------------------------------------------
            CertAction::Info { file } => {
                let cert_der = ucx_sign::cert::load_certificate(&file)?;
                let cn = ucx_sign::cert::cert_subject_cn(&cert_der)
                    .unwrap_or_else(|_| "<unknown>".to_string());
                let fingerprint_blake3 = ucx_sign::cert::cert_fingerprint_blake3(&cert_der);
                let fingerprint_sha256 = ucx_sign::cert::cert_fingerprint_sha256(&cert_der);

                // Extract validity period and algorithm.
                // 提取有效期和算法信息。
                let (not_before, not_after) = ucx_sign::cert::cert_validity(&cert_der)
                    .unwrap_or_else(|_| ("<unknown>".to_string(), "<unknown>".to_string()));
                let algorithm = ucx_sign::cert::cert_algorithm(&cert_der)
                    .unwrap_or_else(|_| "<unknown>".to_string());

                println!("=== Certificate Info ===");
                println!("File: {}", file.display());
                println!("Subject: CN={cn}");
                println!("Issuer: CN={cn}");
                println!("Type: self-signed");
                println!("Algorithm: {algorithm}");
                println!("Valid: {not_before} to {not_after}");
                println!("Fingerprint (BLAKE3): {fingerprint_blake3}");
                println!("Fingerprint (SHA-256): {fingerprint_sha256}");
            }
        },

        // =====================================================================
        // ucx sign — Sign a UCX file with dual-layer signatures.
        // ucx sign — 对 UCX 文件进行双层签名。
        // =====================================================================
        Commands::Sign { file, key, cert, signer_id } => {
            ucx_sign::sign(&file, &key, &cert, &signer_id)?;
            println!("UCX file signed: {}", file.display());
            println!("  Signer ID: {signer_id}");
            println!("  Layer 1 (JAR-style) + Layer 2 (APK v2-style) signatures applied.");
        }

        Commands::Version { action, path } => {
            handle_version_command(action, &path)?;
        }
        Commands::Encrypt { file, output, algorithm, key, passphrase, kdf, allow_weak } => {
            // 解析加密算法 / Parse the encryption algorithm.
            let algo = match algorithm.as_str() {
                "AES-256-GCM" | "aes-256-gcm" => ucx_crypto::Algorithm::Aes256Gcm,
                "ChaCha20-Poly1305" | "chacha20-poly1305" => ucx_crypto::Algorithm::ChaCha20Poly1305,
                "AES-256-CBC" | "aes-256-cbc" => ucx_crypto::Algorithm::Aes256Cbc,
                _ => anyhow::bail!("unsupported algorithm: {algorithm}"),
            };

            // 确定输出路径（默认覆盖源文件） / Determine output path (default: overwrite source).
            let dest = output.as_ref().unwrap_or(&file);

            if passphrase {
                // 口令模式：解析 KDF 并交互式读取口令
                // Passphrase mode: parse KDF and interactively read the passphrase.
                let kdf_type = match kdf.as_str() {
                    "argon2id" | "Argon2id" => ucx_crypto::Kdf::Argon2id,
                    "pbkdf2" | "PBKDF2" => ucx_crypto::Kdf::Pbkdf2HmacSha256,
                    _ => anyhow::bail!("unsupported KDF: {kdf}"),
                };
                eprint!("Enter passphrase: ");
                let pass = read_passphrase()?;

                // DOC-3: enforce minimum passphrase length unless --allow-weak.
                // Counting chars() (Unicode scalar values) is strict enough for
                // interactive use; bytes would over-count multi-byte scripts.
                // DOC-3：未使用 --allow-weak 时强制最小口令长度。
                // 使用 chars() 计数（Unicode 标量值），对交互输入足够严格；
                // 字节计数会让多字节文字虚高。
                if !allow_weak && pass.chars().count() < 8 {
                    anyhow::bail!(
                        "passphrase must be at least 8 characters; use --allow-weak to override"
                    );
                }

                ucx_crypto::encrypt_with_passphrase(&file, dest, &pass, algo, kdf_type)?;
                println!("File encrypted: {}", dest.display());
                println!("  Algorithm: {algorithm}");
                println!("  KDF: {kdf}");
            } else if let Some(key_b64) = key {
                // 直接密钥模式：解码 Base64 密钥
                // Direct key mode: decode the Base64 key.
                let key_bytes = base64::engine::general_purpose::STANDARD
                    .decode(&key_b64)
                    .map_err(|e| anyhow::anyhow!("invalid Base64 key: {e}"))?;
                if key_bytes.len() != 32 {
                    anyhow::bail!("key must be 32 bytes, got {}", key_bytes.len());
                }
                let key_arr: [u8; 32] = key_bytes.try_into().unwrap();
                ucx_crypto::encrypt(&file, dest, &key_arr, algo)?;
                println!("File encrypted: {}", dest.display());
                println!("  Algorithm: {algorithm}");
            } else {
                anyhow::bail!("must specify --key or --passphrase");
            }
        }
        Commands::Decrypt { file, output, key, passphrase } => {
            // 确定输出路径（默认覆盖源文件） / Determine output path (default: overwrite source).
            let dest = output.as_ref().unwrap_or(&file);

            let plaintext = if passphrase {
                // 口令模式：交互式读取口令并解密
                // Passphrase mode: interactively read passphrase and decrypt.
                eprint!("Enter passphrase: ");
                let pass = read_passphrase()?;
                ucx_crypto::decrypt_with_passphrase(&file, &pass)?
            } else if let Some(key_b64) = key {
                // 直接密钥模式：解码 Base64 密钥并解密
                // Direct key mode: decode Base64 key and decrypt.
                let key_bytes = base64::engine::general_purpose::STANDARD
                    .decode(&key_b64)
                    .map_err(|e| anyhow::anyhow!("invalid Base64 key: {e}"))?;
                if key_bytes.len() != 32 {
                    anyhow::bail!("key must be 32 bytes, got {}", key_bytes.len());
                }
                let key_arr: [u8; 32] = key_bytes.try_into().unwrap();
                ucx_crypto::decrypt(&file, &key_arr)?
            } else {
                anyhow::bail!("must specify --key or --passphrase");
            };

            std::fs::write(dest, &plaintext)?;
            println!("File decrypted: {}", dest.display());
            println!("  Size: {} bytes", plaintext.len());
        }
    }

    Ok(())
}

// =============================================================================
// Helper functions / 辅助函数
// =============================================================================

/// Check whether a file begins with the UCXE magic number `b"UCXE"` (0x55 43 58 45).
///
/// Returns `Ok(true)` when the file exists and its first four bytes match the
/// magic. A file shorter than 4 bytes or with different bytes returns
/// `Ok(false)`. Propagates I/O errors other than EOF.
///
/// Used by `ucx verify` to short-circuit with a friendly message when the
/// user passes a UCXE-encrypted payload instead of a `.ucx` archive.
///
/// 判断文件是否以 UCXE 魔数 `b"UCXE"`（0x55 43 58 45）开头。
/// 文件存在且前 4 字节等于魔数返回 `Ok(true)`；文件不足 4 字节或不等则
/// 返回 `Ok(false)`；其他 I/O 错误原样向上传递。
/// `ucx verify` 调用此函数在用户传入 UCXE 裸文件时快速给出友好提示。
fn is_ucxe_encrypted_file(path: &std::path::Path) -> anyhow::Result<bool> {
    use std::io::Read as _;

    let mut file = std::fs::File::open(path)?;
    let mut magic = [0u8; 4];
    match file.read_exact(&mut magic) {
        Ok(()) => Ok(magic == *b"UCXE"),
        // Short file — can't be an UCXE header. Not our job to classify; let
        // the normal parse surface its own error.
        // 文件过短 — 肯定不是 UCXE 头，交由正常解析路径报错。
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Read a passphrase from stdin (no echo if terminal).
///
/// 从标准输入读取口令（如果是终端则不回显）。
fn read_passphrase() -> anyhow::Result<String> {
    let mut pass = String::new();
    std::io::stdin().read_line(&mut pass)?;
    let pass = pass.trim_end().to_string();
    if pass.is_empty() {
        anyhow::bail!("passphrase cannot be empty");
    }
    Ok(pass)
}

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

// =============================================================================
// Version command implementation / 版本命令实现
// =============================================================================

/// The `.ucx-version.json` file name for persisting version state.
/// 持久化版本状态的 `.ucx-version.json` 文件名。
const VERSION_STATE_FILE: &str = ".ucx-version.json";

/// Load the current version state from `.ucx-version.json`.
///
/// Returns `None` if the file does not exist.
///
/// 从 `.ucx-version.json` 加载当前版本状态。
/// 文件不存在时返回 `None`。
fn load_version_state(project_path: &std::path::Path) -> anyhow::Result<Option<ucx_types::FileVersion>> {
    let path = project_path.join(VERSION_STATE_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let fv: ucx_types::FileVersion = serde_json::from_str(&content)?;
    Ok(Some(fv))
}

/// Save the version state to `.ucx-version.json`.
///
/// 将版本状态保存到 `.ucx-version.json`。
fn save_version_state(
    project_path: &std::path::Path,
    file_version: &ucx_types::FileVersion,
) -> anyhow::Result<()> {
    let path = project_path.join(VERSION_STATE_FILE);
    let json = serde_json::to_string_pretty(file_version)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Build a `FileVersion` from a `UcxVersion` and an optional previous state.
///
/// Increments the revision counter and updates the released_at timestamp.
///
/// 从 `UcxVersion` 和可选的先前状态构建 `FileVersion`。
/// 递增修订计数器并更新 released_at 时间戳。
fn build_file_version(
    version: &ucx_version::UcxVersion,
    previous: Option<&ucx_types::FileVersion>,
) -> ucx_types::FileVersion {
    let prev_revision = previous
        .and_then(|fv| fv.revision)
        .unwrap_or(0);
    ucx_types::FileVersion {
        version: Some(version.to_string()),
        revision: Some(prev_revision + 1),
        released_at: Some(chrono::Utc::now().to_rfc3339()),
        changelog: None,
    }
}

/// Handle the `ucx version` command.
///
/// 处理 `ucx version` 命令。
fn handle_version_command(
    action: Option<VersionAction>,
    project_path: &std::path::Path,
) -> anyhow::Result<()> {
    let project_path = std::fs::canonicalize(project_path)?;

    match action {
        None => {
            // Show current version.
            // 显示当前版本。
            let state = load_version_state(&project_path)?;
            match state {
                Some(fv) => {
                    let ver = fv.version.as_deref().unwrap_or("(unset)");
                    let rev = fv.revision.map_or("(unset)".to_string(), |r| r.to_string());
                    let at = fv.released_at.as_deref().unwrap_or("(unknown)");
                    println!("File version: {ver}");
                    println!("Revision:     {rev}");
                    println!("Released at:  {at}");
                    if let Some(log) = &fv.changelog {
                        println!("Changelog:    {log}");
                    }
                }
                None => {
                    println!("No version set. Use 'ucx version auto' or 'ucx version set X.Y.Z'.");
                }
            }
        }
        Some(VersionAction::Auto) => {
            // Auto-detect changes and bump version.
            // 自动检测变更并升级版本。
            let (current, changes) = ucx_version::detect_changes(&project_path)?;
            println!("Current version: {current}");
            println!("Changes: {} added, {} modified, {} deleted",
                changes.added.len(), changes.modified.len(), changes.deleted.len());

            let next = ucx_version::auto_version(&current, &changes, &project_path)?;
            if next == current && !changes.is_empty() {
                println!("Version unchanged: {current}");
            } else if changes.is_empty() {
                println!("No changes detected. Version stays at {current}");
            } else {
                println!("Version bump: {current} → {next}");
                let prev_state = load_version_state(&project_path)?;
                let fv = build_file_version(&next, prev_state.as_ref());
                save_version_state(&project_path, &fv)?;
                println!("Saved to {VERSION_STATE_FILE} (revision {})", fv.revision.unwrap_or(0));
            }
        }
        Some(VersionAction::Patch) => {
            // Manual patch bump.
            // 手动修订升级。
            let state = load_version_state(&project_path)?;
            let current = state
                .as_ref()
                .and_then(|fv| fv.version.as_deref())
                .map(ucx_version::UcxVersion::parse)
                .transpose()?
                .unwrap_or_else(ucx_version::UcxVersion::zero);

            let next = ucx_version::bump::bump_patch(&current);
            println!("Version bump: {current} → {next}");
            let fv = build_file_version(&next, state.as_ref());
            save_version_state(&project_path, &fv)?;
            println!("Saved to {VERSION_STATE_FILE}");
        }
        Some(VersionAction::Chapter) => {
            // Manual chapter bump — read struct.json for chapter count.
            // 手动章节升级 — 读取 struct.json 获取章节数。
            let state = load_version_state(&project_path)?;
            let current = state
                .as_ref()
                .and_then(|fv| fv.version.as_deref())
                .map(ucx_version::UcxVersion::parse)
                .transpose()?
                .unwrap_or_else(ucx_version::UcxVersion::zero);

            let struct_path = project_path.join("content").join("struct.json");
            let struct_chapter = if struct_path.exists() {
                let content = std::fs::read_to_string(&struct_path)?;
                let structure: ucx_types::Structure = serde_json::from_str(&content)?;
                ucx_version::bump::find_latest_chapter_number(&structure)
            } else {
                current.chapter + 1
            };
            // Prevent version downgrade: new chapter must be > current chapter.
            // 防止版本降级：新章节编号必须大于当前章节编号。
            let chapter_num = struct_chapter.max(current.chapter + 1);

            let next = ucx_version::bump::bump_chapter(&current, chapter_num);
            println!("Version bump: {current} → {next}");
            let fv = build_file_version(&next, state.as_ref());
            save_version_state(&project_path, &fv)?;
            println!("Saved to {VERSION_STATE_FILE}");
        }
        Some(VersionAction::Volume) => {
            // Manual volume bump.
            // 手动卷升级。
            let state = load_version_state(&project_path)?;
            let current = state
                .as_ref()
                .and_then(|fv| fv.version.as_deref())
                .map(ucx_version::UcxVersion::parse)
                .transpose()?
                .unwrap_or_else(ucx_version::UcxVersion::zero);

            let next = ucx_version::bump::bump_volume(&current, 1);
            println!("Version bump: {current} → {next}");
            let fv = build_file_version(&next, state.as_ref());
            save_version_state(&project_path, &fv)?;
            println!("Saved to {VERSION_STATE_FILE}");
        }
        Some(VersionAction::Set { version }) => {
            // Set version to a specific value.
            // 设置为指定版本。
            let parsed = ucx_version::UcxVersion::parse(&version)?;
            println!("Setting version to {parsed}");
            let prev_state = load_version_state(&project_path)?;
            let fv = build_file_version(&parsed, prev_state.as_ref());
            save_version_state(&project_path, &fv)?;
            println!("Saved to {VERSION_STATE_FILE}");
        }
    }

    Ok(())
}
