//! Path safety validation — shared safe-relative-path checks for UCX archives.
//!
//! UCX archives are ZIP containers whose entry names (and the `file` references
//! inside `content/struct.json`) are **untrusted**: a malicious `.ucx` can carry
//! crafted entry names to escape the extraction directory (Zip-Slip / arbitrary
//! file write). This module centralizes the single source of truth for "is this
//! entry/path a safe relative path?" so that the **producer** side
//! (`ucx-build`, when packing) and the **consumer** side (`ucx-parse::extract_to`,
//! when unpacking) apply *identical* rules and can never drift apart (ADR-001
//! type-ownership rationale applied to validation logic).
//!
//! 路径安全校验 — UCX 归档的共享"安全相对路径"检查。
//!
//! UCX 归档是 ZIP 容器，其条目名（以及 `content/struct.json` 内的 `file` 引用）
//! 都是**不可信**输入：恶意 `.ucx` 可携带精心构造的条目名以逃逸解压目录
//! （Zip-Slip / 任意文件写）。本模块集中提供"该条目/路径是否为安全相对路径？"
//! 的**唯一真实来源**，使**生产侧**（`ucx-build` 打包时）与**消费侧**
//! （`ucx-parse::extract_to` 解包时）应用**完全一致**的规则，杜绝两侧漂移。
//!
//! # Rejected forms / 拒绝的形式
//!
//! - Empty string / 空串
//! - NUL byte or any control character / NUL 字节或任意控制字符
//!   （同时中和针对终端/日志的注入，以及跨平台文件名风险）
//! - Backslash `\` (forward-slash only; also preempts `C:\` drive attacks) /
//!   反斜杠（仅允许正斜杠；同时预防 `C:\` 盘符攻击）
//! - Absolute paths: leading `/`, Windows drive `C:`, or any `Path::is_absolute` /
//!   绝对路径：以 `/` 开头、Windows 盘符 `C:`、或任何 `Path::is_absolute`
//! - `..` parent-traversal segments / `..` 父级穿越段
//! - Windows reserved device names (CON, PRN, AUX, NUL, COM1-9, LPT1-9) /
//!   Windows 保留设备名
//! - Path segments ending in a dot or space (silently stripped by Windows) /
//!   以点或空格结尾的路径段（Windows 会静默剥除，导致路径混淆）

use thiserror::Error;

/// Reason a path was rejected as an unsafe relative path.
///
/// 路径被判定为不安全相对路径的原因。
///
/// Note: every variant stores the offending path **already escaped** via
/// [`str::escape_debug`], so rendering the error can never re-emit raw control
/// characters (which would itself be a log/terminal-injection vector — the same
/// vulnerability class this module defends against).
///
/// 注意：每个变体存储的违规路径都已通过 [`str::escape_debug`] **转义**，
/// 因此渲染错误时绝不会重新输出原始控制字符（否则将构成日志/终端注入——
/// 正是本模块所防御的同一漏洞类别）。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathSafetyError {
    /// The path was empty. / 路径为空。
    #[error("path is empty / 路径为空")]
    Empty,

    /// The path contained a NUL byte or other control character. /
    /// 路径含 NUL 字节或其他控制字符。
    #[error("path contains a NUL or control character / 路径含 NUL 或控制字符: '{0}'")]
    ControlChar(String),

    /// The path contained a backslash. / 路径含反斜杠。
    #[error("path contains a backslash, use forward slashes only / 路径含反斜杠，仅允许正斜杠: '{0}'")]
    Backslash(String),

    /// The path was absolute (must be relative). / 路径为绝对路径（必须相对）。
    #[error("path is absolute, must be relative / 路径为绝对路径，必须相对: '{0}'")]
    Absolute(String),

    /// The path contained a `..` parent-traversal segment. / 路径含 `..` 父级穿越段。
    #[error("path contains a '..' parent-traversal segment / 路径含 '..' 父级穿越段: '{0}'")]
    ParentTraversal(String),

    /// A segment matched a Windows reserved device name. / 某段为 Windows 保留设备名。
    #[error("path uses a Windows reserved device name / 路径使用 Windows 保留设备名: '{0}'")]
    ReservedName(String),

    /// A segment ended with a dot or space. / 某段以点或空格结尾。
    #[error("path segment ends with a dot or space (ambiguous on Windows) / 路径段以点或空格结尾（Windows 下有歧义）: '{0}'")]
    TrailingDotOrSpace(String),
}

/// Validate that `path` is a safe **relative** path suitable for use as a ZIP
/// entry name or a `struct.json` `file` reference.
///
/// Returns `Ok(())` if safe, or the specific [`PathSafetyError`] otherwise.
/// The check is intentionally strict and identical on both the producing and
/// consuming sides; any path it rejects must never be written to disk relative
/// to a base directory, because `base.join(unsafe_path)` can escape `base`.
///
/// 校验 `path` 是否为可安全用作 ZIP 条目名或 `struct.json` `file` 引用的
/// **相对**路径。安全则返回 `Ok(())`，否则返回具体的 [`PathSafetyError`]。
/// 该检查刻意严格，且生产/消费两侧完全一致；任何被它拒绝的路径都绝不能相对
/// 某基目录写入磁盘，因为 `base.join(不安全路径)` 可能逃逸 `base`。
pub fn validate_safe_relative_path(path: &str) -> Result<(), PathSafetyError> {
    // The escaped rendering of `path`, used for every error so that control
    // characters embedded in a malicious path can never reach a log/terminal raw.
    // `path` 的转义渲染，用于所有错误，使恶意路径中的控制字符绝不会原样到达日志/终端。
    let shown = || path.escape_debug().to_string();

    // 1. Empty string. / 空串。
    if path.is_empty() {
        return Err(PathSafetyError::Empty);
    }

    // 2. NUL or control characters (defense-in-depth + injection neutralization).
    // 2. NUL 或控制字符（纵深防御 + 中和注入）。
    if path.chars().any(|c| c.is_control()) {
        return Err(PathSafetyError::ControlChar(shown()));
    }

    // 3. Backslash — forward-slash only. Also preempts `C:\foo` drive attacks.
    // 3. 反斜杠 — 仅允许正斜杠。同时预防 `C:\foo` 盘符攻击。
    if path.contains('\\') {
        return Err(PathSafetyError::Backslash(shown()));
    }

    // 4. Absolute: leading '/'.
    // 4. 绝对路径：以 '/' 开头。
    if path.starts_with('/') {
        return Err(PathSafetyError::Absolute(shown()));
    }

    // 5. Windows drive-absolute: first char ASCII alpha, second char ':'.
    //    (e.g. `C:/Windows/Temp/x`, `d:relative`). `base.join` of such a path
    //    discards `base` on Windows, landing the file at an attacker location.
    // 5. Windows 盘符绝对路径：首字符为 ASCII 字母、第二字符为 ':'。
    //    （如 `C:/Windows/Temp/x`、`d:relative`）。在 Windows 上 `base.join`
    //    这样的路径会丢弃 `base`，使文件落到攻击者指定位置。
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Err(PathSafetyError::Absolute(shown()));
    }

    // 6. Backstop: any platform-recognized absolute path. Catches forms the
    //    explicit checks above might miss on a given OS.
    // 6. 兜底：任何被平台识别为绝对的路径。捕获上面显式检查在特定 OS 上可能遗漏的形式。
    if std::path::Path::new(path).is_absolute() {
        return Err(PathSafetyError::Absolute(shown()));
    }

    // 7. Per-segment checks (split on '/').
    // 7. 逐段检查（按 '/' 分割）。
    for segment in path.split('/') {
        // 7a. No `..` parent-traversal segment.
        // 7a. 禁止 `..` 父级穿越段。
        if segment == ".." {
            return Err(PathSafetyError::ParentTraversal(shown()));
        }

        // 7b. No Windows reserved device name (CON, NUL, COM1, ...).
        // 7b. 禁止 Windows 保留设备名（CON、NUL、COM1 ...）。
        if is_windows_reserved_name(segment) {
            return Err(PathSafetyError::ReservedName(shown()));
        }

        // 7c. No segment ending with a dot or space (Windows strips these,
        //     creating two paths that resolve to the same file). The lone "."
        //     current-directory segment is harmless and allowed.
        // 7c. 禁止以点或空格结尾的段（Windows 会剥除，导致两个路径解析到同一文件）。
        //     单独的 "." 当前目录段无害，允许。
        if segment != "." && (segment.ends_with('.') || segment.ends_with(' ')) {
            return Err(PathSafetyError::TrailingDotOrSpace(shown()));
        }
    }

    Ok(())
}

/// Check whether a path segment corresponds to a Windows reserved device name.
///
/// The reserved names are: CON, PRN, AUX, NUL, COM1..COM9, LPT1..LPT9.
/// The check is case-insensitive and applied to the segment both as-is and
/// with any trailing extension removed (`NUL.txt` is also reserved).
///
/// 判断路径段是否为 Windows 保留设备名。
/// 保留名：CON、PRN、AUX、NUL、COM1..COM9、LPT1..LPT9。
/// 检查不区分大小写，同时对原段和去扩展名后的段进行匹配（`NUL.txt` 也视为保留）。
pub fn is_windows_reserved_name(segment: &str) -> bool {
    // Strip the extension (everything from the first '.') for comparison.
    // 去除扩展名（从第一个 '.' 开始）用于比较。
    let stem = segment.split('.').next().unwrap_or(segment);
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL"
            | "COM1" | "COM2" | "COM3" | "COM4" | "COM5"
            | "COM6" | "COM7" | "COM8" | "COM9"
            | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5"
            | "LPT6" | "LPT7" | "LPT8" | "LPT9"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Safe relative paths are accepted. / 安全相对路径应通过。
    #[test]
    fn accepts_safe_relative_paths() {
        for p in [
            "chapter-001.md",
            "content/chapter-001.md",
            "META-INF/MANIFEST.MF",
            "a/b/c/d.txt",
            "./local.md",
            "中文章节.md",
        ] {
            assert!(
                validate_safe_relative_path(p).is_ok(),
                "expected safe: {p:?}"
            );
        }
    }

    /// Empty path rejected. / 空路径拒绝。
    #[test]
    fn rejects_empty() {
        assert_eq!(validate_safe_relative_path(""), Err(PathSafetyError::Empty));
    }

    /// Leading-slash absolute path rejected. / 以斜杠开头的绝对路径拒绝。
    #[test]
    fn rejects_posix_absolute() {
        assert!(matches!(
            validate_safe_relative_path("/etc/passwd"),
            Err(PathSafetyError::Absolute(_))
        ));
    }

    /// Windows drive-absolute paths rejected (the Zip-Slip C-1 vector). /
    /// Windows 盘符绝对路径拒绝（Zip-Slip C-1 攻击向量）。
    #[test]
    fn rejects_windows_drive_absolute() {
        for p in ["C:/Windows/Temp/x.txt", "d:/dir/x", "Z:relative"] {
            assert!(
                matches!(
                    validate_safe_relative_path(p),
                    Err(PathSafetyError::Absolute(_))
                ),
                "expected absolute-reject: {p:?}"
            );
        }
    }

    /// Backslash paths rejected (incl. `C:\` and UNC-ish forms). /
    /// 反斜杠路径拒绝（含 `C:\` 与类 UNC 形式）。
    #[test]
    fn rejects_backslash() {
        for p in ["sub\\chapter.md", "C:\\Windows\\x", "a\\b"] {
            assert!(
                matches!(
                    validate_safe_relative_path(p),
                    Err(PathSafetyError::Backslash(_)) | Err(PathSafetyError::Absolute(_))
                ),
                "expected reject: {p:?}"
            );
        }
    }

    /// `..` traversal segments rejected. / `..` 穿越段拒绝。
    #[test]
    fn rejects_parent_traversal() {
        for p in ["../outside.md", "a/../../etc", "content/../../../x"] {
            assert!(
                matches!(
                    validate_safe_relative_path(p),
                    Err(PathSafetyError::ParentTraversal(_))
                ),
                "expected traversal-reject: {p:?}"
            );
        }
    }

    /// Windows reserved device names rejected (case-insensitive, with ext). /
    /// Windows 保留设备名拒绝（不区分大小写，含扩展名）。
    #[test]
    fn rejects_reserved_names() {
        for p in ["NUL", "con.txt", "a/COM1.md", "lpt9"] {
            assert!(
                matches!(
                    validate_safe_relative_path(p),
                    Err(PathSafetyError::ReservedName(_))
                ),
                "expected reserved-reject: {p:?}"
            );
        }
    }

    /// NUL and control characters rejected, and the error never carries them raw. /
    /// NUL 与控制字符拒绝，且错误绝不原样携带它们。
    #[test]
    fn rejects_control_chars_and_escapes_message() {
        let evil = "ok\nInjected: line\u{0}.md";
        let err = validate_safe_relative_path(evil).unwrap_err();
        assert!(matches!(err, PathSafetyError::ControlChar(_)));
        // The rendered message must not contain a raw newline or NUL.
        // 渲染后的消息不得含原始换行或 NUL。
        let msg = err.to_string();
        assert!(!msg.contains('\n'), "message leaked a raw newline: {msg:?}");
        assert!(!msg.contains('\u{0}'), "message leaked a raw NUL: {msg:?}");
    }

    /// Trailing dot/space segments rejected; lone "." allowed. /
    /// 尾随点/空格的段拒绝；单独 "." 允许。
    #[test]
    fn rejects_trailing_dot_or_space() {
        for p in ["chapter.", "a/b ", "dir./x.md"] {
            assert!(
                matches!(
                    validate_safe_relative_path(p),
                    Err(PathSafetyError::TrailingDotOrSpace(_))
                ),
                "expected trailing-reject: {p:?}"
            );
        }
        assert!(validate_safe_relative_path("./ok.md").is_ok());
    }
}
