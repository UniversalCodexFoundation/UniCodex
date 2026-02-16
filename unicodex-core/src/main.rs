//! Unicodex CLI — unified novel file standard tool.
//!
//! This is the main entry point for the `ucx` command-line tool.
//! It delegates all business logic to the corresponding `ucx-*` modules.
//!
//! Unicodex CLI —— 统一小说文件标准工具。
//! 这是 `ucx` 命令行工具的主入口。
//! 所有业务逻辑委托给对应的 `ucx-*` 模块处理。

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
    Init,

    /// Build (pack) a UCX file from project directory.
    /// 从项目目录构建（打包）UCX 文件。
    Build,

    /// Unpack a UCX file to a directory.
    /// 将 UCX 文件解包到目录。
    Unpack,

    /// Display information about a UCX file.
    /// 显示 UCX 文件的信息。
    Info,

    /// Verify the integrity and signatures of a UCX file.
    /// 验证 UCX 文件的完整性和签名。
    Verify,

    /// Sign a UCX file.
    /// 对 UCX 文件签名。
    Sign,

    /// Manage version numbers.
    /// 管理版本号。
    Version,

    /// Encrypt chapters or resources.
    /// 加密章节或资源。
    Encrypt,

    /// Decrypt chapters or resources.
    /// 解密章节或资源。
    Decrypt,
}

// =============================================================================
// Main entry point / 主入口
// =============================================================================

fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber for structured logging.
    // 初始化 tracing 订阅器以进行结构化日志输出。
    tracing_subscriber::fmt::init();

    // Parse CLI arguments.
    // 解析命令行参数。
    let cli = Cli::parse();

    // Dispatch to the corresponding module based on the subcommand.
    // 根据子命令分发到对应的模块。
    match cli.command {
        Commands::Init => {
            // TODO: Delegate to ucx_init module.
            // TODO: 委托给 ucx_init 模块。
            println!("ucx init: not yet implemented");
        }
        Commands::Build => {
            // TODO: Delegate to ucx_build module.
            // TODO: 委托给 ucx_build 模块。
            println!("ucx build: not yet implemented");
        }
        Commands::Unpack => {
            // TODO: Delegate to ucx_parse module.
            // TODO: 委托给 ucx_parse 模块。
            println!("ucx unpack: not yet implemented");
        }
        Commands::Info => {
            // TODO: Delegate to ucx_parse module.
            // TODO: 委托给 ucx_parse 模块。
            println!("ucx info: not yet implemented");
        }
        Commands::Verify => {
            // TODO: Delegate to ucx_verify module.
            // TODO: 委托给 ucx_verify 模块。
            println!("ucx verify: not yet implemented");
        }
        Commands::Sign => {
            // TODO: Delegate to ucx_sign module.
            // TODO: 委托给 ucx_sign 模块。
            println!("ucx sign: not yet implemented");
        }
        Commands::Version => {
            // TODO: Delegate to ucx_version module.
            // TODO: 委托给 ucx_version 模块。
            println!("ucx version: not yet implemented");
        }
        Commands::Encrypt => {
            // TODO: Delegate to ucx_crypto module.
            // TODO: 委托给 ucx_crypto 模块。
            println!("ucx encrypt: not yet implemented");
        }
        Commands::Decrypt => {
            // TODO: Delegate to ucx_crypto module.
            // TODO: 委托给 ucx_crypto 模块。
            println!("ucx decrypt: not yet implemented");
        }
    }

    Ok(())
}
