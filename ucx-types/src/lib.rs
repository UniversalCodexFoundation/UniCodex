//! UCX Shared Types — Common data structures for all UCX modules.
//!
//! This crate defines the data structures shared across multiple UCX modules,
//! solving the type-ownership problem identified in ADR-001. Instead of each
//! module independently defining types like `Codex` or `Manifest`, they all
//! depend on this single crate for consistent definitions.
//!
//! UCX 公共类型 — 所有 UCX 模块共享的数据结构。
//! 本 crate 定义多个 UCX 模块共享的数据结构，解决 ADR-001 中识别的类型归属问题。
//! 各模块不再独立定义 `Codex`、`Manifest` 等类型，而是统一依赖本 crate。
//!
//! # Modules / 模块
//!
//! - [`codex`] — Codex metadata types (codex.json) / 作品核心元数据类型
//! - [`structure`] — Content structure types (struct.json) / 内容结构类型
//! - [`manifest`] — MANIFEST.MF types / 资源清单类型
//! - [`project`] — Project configuration types (unicodex.toml) / 项目配置类型
//! - [`ucx_id`] — UCX unique identifier / UCX 唯一标识
//! - [`path_safety`] — Shared safe-relative-path validation / 共享安全相对路径校验

// --- Sub-modules / 子模块 ---
pub mod codex;
pub mod manifest;
pub mod path_safety;
pub mod project;
pub mod structure;
pub mod ucx_id;

// --- Re-exports for convenience / 便捷重导出 ---
pub use codex::{Codex, FileVersion};
pub use manifest::{HashAlgorithm, Manifest, ManifestEntry};
pub use project::{ProjectConfig, VersionSection};
pub use structure::{Structure, StructureNode};
pub use ucx_id::UcxId;
