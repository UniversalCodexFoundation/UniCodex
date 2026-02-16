//! UCX unique identifier.
//!
//! Defines the `UcxId` type representing a globally unique identifier for UCX works.
//! Format: `urn:ucx:{UUID v4}`
//!
//! UCX 唯一标识符。
//! 定义 `UcxId` 类型，代表 UCX 作品的全局唯一标识。
//! 格式：`urn:ucx:{UUID v4}`

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The prefix for all UCX identifiers.
/// UCX 标识符的固定前缀。
const UCX_ID_PREFIX: &str = "urn:ucx:";

// =============================================================================
// UcxId type / UcxId 类型
// =============================================================================

/// A globally unique identifier for a UCX work.
///
/// Format: `urn:ucx:{UUID v4}` (e.g., `urn:ucx:550e8400-e29b-41d4-a716-446655440000`).
/// Once generated, it must not change — the same work across different versions
/// shares the same `UcxId`.
///
/// UCX 作品的全局唯一标识符。
/// 格式：`urn:ucx:{UUID v4}`。
/// 一旦生成不可变更，同一作品的不同版本共享相同的 `UcxId`。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UcxId(String);

impl UcxId {
    /// Generate a new random UCX ID using UUID v4.
    ///
    /// 使用 UUID v4 生成新的随机 UCX ID。
    ///
    /// # Examples / 示例
    ///
    /// ```
    /// use ucx_types::UcxId;
    ///
    /// let id = UcxId::new();
    /// assert!(id.as_str().starts_with("urn:ucx:"));
    /// ```
    pub fn new() -> Self {
        let uuid = Uuid::new_v4();
        Self(format!("{UCX_ID_PREFIX}{uuid}"))
    }

    /// Parse a UCX ID from a string.
    ///
    /// The string must start with `urn:ucx:` followed by a valid UUID.
    ///
    /// 从字符串解析 UCX ID。
    /// 字符串必须以 `urn:ucx:` 开头，后跟有效的 UUID。
    ///
    /// # Errors / 错误
    ///
    /// Returns an error if the format is invalid.
    /// 如果格式无效则返回错误。
    pub fn parse(s: &str) -> Result<Self, UcxIdError> {
        // Check the prefix.
        // 检查前缀。
        let uuid_str = s
            .strip_prefix(UCX_ID_PREFIX)
            .ok_or_else(|| UcxIdError::InvalidPrefix(s.to_string()))?;

        // Validate the UUID portion.
        // 验证 UUID 部分。
        Uuid::parse_str(uuid_str)
            .map_err(|e| UcxIdError::InvalidUuid(e.to_string()))?;

        Ok(Self(s.to_string()))
    }

    /// Return the UCX ID as a string slice.
    ///
    /// 将 UCX ID 作为字符串切片返回。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the UUID portion (without the `urn:ucx:` prefix).
    ///
    /// 提取 UUID 部分（不含 `urn:ucx:` 前缀）。
    pub fn uuid_str(&self) -> &str {
        // Safe to unwrap: UcxId is always constructed with the prefix.
        // 安全的 unwrap：UcxId 总是带前缀构造的。
        self.0.strip_prefix(UCX_ID_PREFIX).unwrap_or(&self.0)
    }
}

impl Default for UcxId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UcxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// =============================================================================
// Error type / 错误类型
// =============================================================================

/// Errors that can occur when parsing a UCX ID.
///
/// 解析 UCX ID 时可能发生的错误。
#[derive(Debug, thiserror::Error)]
pub enum UcxIdError {
    /// The string does not start with `urn:ucx:`.
    /// 字符串不以 `urn:ucx:` 开头。
    #[error("invalid UCX ID prefix (expected 'urn:ucx:'): {0}")]
    InvalidPrefix(String),

    /// The UUID portion is not valid.
    /// UUID 部分无效。
    #[error("invalid UUID in UCX ID: {0}")]
    InvalidUuid(String),
}

// =============================================================================
// Tests / 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_ucx_id_has_correct_prefix() {
        // A newly generated UcxId should start with "urn:ucx:".
        // 新生成的 UcxId 应以 "urn:ucx:" 开头。
        let id = UcxId::new();
        assert!(id.as_str().starts_with("urn:ucx:"));
    }

    #[test]
    fn test_new_ucx_id_has_valid_uuid() {
        // The UUID portion should be parseable.
        // UUID 部分应可解析。
        let id = UcxId::new();
        let uuid_str = id.uuid_str();
        assert!(Uuid::parse_str(uuid_str).is_ok());
    }

    #[test]
    fn test_parse_valid_ucx_id() {
        // A valid UCX ID string should parse successfully.
        // 有效的 UCX ID 字符串应解析成功。
        let s = "urn:ucx:550e8400-e29b-41d4-a716-446655440000";
        let id = UcxId::parse(s).unwrap();
        assert_eq!(id.as_str(), s);
        assert_eq!(id.uuid_str(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_parse_invalid_prefix() {
        // A string without the correct prefix should fail.
        // 没有正确前缀的字符串应失败。
        let result = UcxId::parse("invalid:550e8400-e29b-41d4-a716-446655440000");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_uuid() {
        // A string with a bad UUID should fail.
        // UUID 无效的字符串应失败。
        let result = UcxId::parse("urn:ucx:not-a-valid-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn test_serde_round_trip() {
        // Serialize and deserialize should produce the same value.
        // 序列化和反序列化应产生相同的值。
        let id = UcxId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: UcxId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_display() {
        // Display should show the full URN.
        // Display 应显示完整的 URN。
        let id = UcxId::parse("urn:ucx:550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(format!("{id}"), "urn:ucx:550e8400-e29b-41d4-a716-446655440000");
    }
}
