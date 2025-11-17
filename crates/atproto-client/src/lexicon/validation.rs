//! Lexicon schema validation
//!
//! This module implements validation for Lexicon schemas and values according
//! to the AT Protocol specification.
//!
//! # Example
//!
//! ```rust
//! use atproto_client::lexicon::{LexString, StringFormat, StringConstraints, validate_string};
//!
//! let lex_string = LexString {
//!     type_name: "string".to_string(),
//!     description: None,
//!     format: Some(StringFormat::Did),
//!     constraints: StringConstraints {
//!         max_length: Some(100),
//!         ..Default::default()
//!     },
//! };
//!
//! // Validate a DID value
//! let result = validate_string("did:plc:abc123", &lex_string);
//! ```

use super::constraints::*;
use super::formats::StringFormat;
use super::types::LexString;
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;

/// Errors that can occur during validation
#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    /// String is too long
    #[error("String exceeds maximum length: {actual} > {max}")]
    StringTooLong {
        /// Actual length
        actual: usize,
        /// Maximum allowed length
        max: usize,
    },

    /// String is too short
    #[error("String is shorter than minimum length: {actual} < {min}")]
    StringTooShort {
        /// Actual length
        actual: usize,
        /// Minimum required length
        min: usize,
    },

    /// String has too many graphemes
    #[error("String exceeds maximum graphemes: {actual} > {max}")]
    TooManyGraphemes {
        /// Actual grapheme count
        actual: usize,
        /// Maximum allowed graphemes
        max: usize,
    },

    /// String has too few graphemes
    #[error("String has fewer than minimum graphemes: {actual} < {min}")]
    TooFewGraphemes {
        /// Actual grapheme count
        actual: usize,
        /// Minimum required graphemes
        min: usize,
    },

    /// Value not in enum
    #[error("Value '{value}' not in allowed enum values")]
    NotInEnum {
        /// The value that was provided
        value: String,
    },

    /// Value does not match constant
    #[error("Value '{actual}' does not match required constant '{expected}'")]
    ConstMismatch {
        /// The value that was provided
        actual: String,
        /// The expected constant value
        expected: String,
    },

    /// Invalid string format
    #[error("Invalid {format} format: {value}")]
    InvalidFormat {
        /// The format that was expected
        format: String,
        /// The value that failed validation
        value: String,
    },

    /// Integer out of range
    #[error("Integer {value} is outside allowed range [{min}..{max}]")]
    IntegerOutOfRange {
        /// The value that was provided
        value: i64,
        /// Minimum allowed value
        min: i64,
        /// Maximum allowed value
        max: i64,
    },

    /// Array has too many items
    #[error("Array has too many items: {actual} > {max}")]
    ArrayTooLong {
        /// Actual length
        actual: usize,
        /// Maximum allowed length
        max: usize,
    },

    /// Array has too few items
    #[error("Array has too few items: {actual} < {min}")]
    ArrayTooShort {
        /// Actual length
        actual: usize,
        /// Minimum required length
        min: usize,
    },

    /// Missing required field
    #[error("Missing required field: {field}")]
    MissingRequiredField {
        /// The field name
        field: String,
    },

    /// Invalid NSID format
    #[error("Invalid NSID format: {0}")]
    InvalidNsid(String),

    /// Blob size exceeds maximum
    #[error("Blob size {actual} exceeds maximum {max}")]
    BlobTooLarge {
        /// Actual size in bytes
        actual: usize,
        /// Maximum allowed size
        max: usize,
    },

    /// Blob MIME type not accepted
    #[error("Blob MIME type '{mime_type}' not in accepted types")]
    BlobMimeTypeNotAccepted {
        /// The MIME type
        mime_type: String,
    },
}

/// Result type for validation operations
pub type Result<T> = std::result::Result<T, ValidationError>;

/// Validate a string value against a string type definition
///
/// # Arguments
///
/// * `value` - The string value to validate
/// * `def` - The string type definition with format and constraints
///
/// # Example
///
/// ```rust
/// use atproto_client::lexicon::{LexString, StringFormat, StringConstraints, validate_string};
///
/// let def = LexString {
///     type_name: "string".to_string(),
///     description: None,
///     format: Some(StringFormat::Handle),
///     constraints: StringConstraints {
///         max_length: Some(253),
///         ..Default::default()
///     },
/// };
///
/// assert!(validate_string("user.bsky.social", &def).is_ok());
/// assert!(validate_string("invalid handle!", &def).is_err());
/// ```
pub fn validate_string(value: &str, def: &LexString) -> Result<()> {
    // Validate format first
    if let Some(format) = &def.format {
        validate_string_format(value, format)?;
    }

    // Validate constraints
    validate_string_constraints(value, &def.constraints)?;

    Ok(())
}

/// Validate string format
fn validate_string_format(value: &str, format: &StringFormat) -> Result<()> {
    match format {
        StringFormat::AtIdentifier => validate_at_identifier(value),
        StringFormat::AtUri => validate_at_uri(value),
        StringFormat::Cid => validate_cid(value),
        StringFormat::Datetime => validate_datetime(value),
        StringFormat::Did => validate_did(value),
        StringFormat::Handle => validate_handle(value),
        StringFormat::Nsid => validate_nsid(value),
        StringFormat::Tid => validate_tid(value),
        StringFormat::RecordKey => validate_record_key(value),
        StringFormat::Uri => validate_uri(value),
        StringFormat::Language => validate_language(value),
    }
}

/// Validate AT identifier (DID or handle)
fn validate_at_identifier(value: &str) -> Result<()> {
    if value.starts_with("did:") {
        validate_did(value)
    } else {
        validate_handle(value)
    }
}

/// Validate AT URI
fn validate_at_uri(value: &str) -> Result<()> {
    if !value.starts_with("at://") {
        return Err(ValidationError::InvalidFormat {
            format: "at-uri".to_string(),
            value: value.to_string(),
        });
    }
    // Basic validation - full validation would parse the URI
    Ok(())
}

/// Validate CID (Content Identifier)
fn validate_cid(value: &str) -> Result<()> {
    // CID v1 should start with base32 character (typically 'b')
    // Basic validation - full validation would decode the CID
    if value.is_empty() {
        return Err(ValidationError::InvalidFormat {
            format: "cid".to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

/// Validate datetime (RFC3339 format)
fn validate_datetime(value: &str) -> Result<()> {
    // Basic RFC3339 check - should contain T and Z or timezone offset
    if !value.contains('T')
        || (!value.ends_with('Z') && !value.contains('+') && !value.contains('-'))
    {
        return Err(ValidationError::InvalidFormat {
            format: "datetime".to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

/// Validate DID (Decentralized Identifier)
fn validate_did(value: &str) -> Result<()> {
    if !value.starts_with("did:") {
        return Err(ValidationError::InvalidFormat {
            format: "did".to_string(),
            value: value.to_string(),
        });
    }

    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() < 3 {
        return Err(ValidationError::InvalidFormat {
            format: "did".to_string(),
            value: value.to_string(),
        });
    }

    // Validate method (second part)
    let method = parts[1];
    if method.is_empty()
        || !method
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(ValidationError::InvalidFormat {
            format: "did".to_string(),
            value: value.to_string(),
        });
    }

    // Validate identifier (third part)
    let identifier = parts[2];
    if identifier.is_empty() {
        return Err(ValidationError::InvalidFormat {
            format: "did".to_string(),
            value: value.to_string(),
        });
    }

    Ok(())
}

/// Validate handle (domain name)
fn validate_handle(value: &str) -> Result<()> {
    if value.is_empty() || !value.contains('.') {
        return Err(ValidationError::InvalidFormat {
            format: "handle".to_string(),
            value: value.to_string(),
        });
    }

    // Must not start or end with dot
    if value.starts_with('.') || value.ends_with('.') {
        return Err(ValidationError::InvalidFormat {
            format: "handle".to_string(),
            value: value.to_string(),
        });
    }

    // Validate each segment
    for segment in value.split('.') {
        if segment.is_empty() {
            return Err(ValidationError::InvalidFormat {
                format: "handle".to_string(),
                value: value.to_string(),
            });
        }

        // Must contain only alphanumeric and hyphens
        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(ValidationError::InvalidFormat {
                format: "handle".to_string(),
                value: value.to_string(),
            });
        }

        // Must not start or end with hyphen
        if segment.starts_with('-') || segment.ends_with('-') {
            return Err(ValidationError::InvalidFormat {
                format: "handle".to_string(),
                value: value.to_string(),
            });
        }
    }

    Ok(())
}

/// Validate NSID (Namespaced Identifier)
fn validate_nsid(value: &str) -> Result<()> {
    // Reuse the NSID validation from parsing module
    if !super::parsing::is_valid_nsid(value) {
        return Err(ValidationError::InvalidNsid(value.to_string()));
    }
    Ok(())
}

/// Validate TID (Timestamp Identifier)
fn validate_tid(value: &str) -> Result<()> {
    // TID should be 13 base32 characters
    if value.len() != 13 {
        return Err(ValidationError::InvalidFormat {
            format: "tid".to_string(),
            value: value.to_string(),
        });
    }

    // Should only contain base32 characters (2-7, a-z)
    if !value
        .chars()
        .all(|c| c.is_ascii_lowercase() || ('2'..='7').contains(&c))
    {
        return Err(ValidationError::InvalidFormat {
            format: "tid".to_string(),
            value: value.to_string(),
        });
    }

    Ok(())
}

/// Validate record key
fn validate_record_key(value: &str) -> Result<()> {
    // Record key can be any non-empty alphanumeric string with some special chars
    if value.is_empty() {
        return Err(ValidationError::InvalidFormat {
            format: "record-key".to_string(),
            value: value.to_string(),
        });
    }

    // Allow alphanumeric, hyphens, underscores, periods, tildes
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~')
    {
        return Err(ValidationError::InvalidFormat {
            format: "record-key".to_string(),
            value: value.to_string(),
        });
    }

    Ok(())
}

/// Validate URI
fn validate_uri(value: &str) -> Result<()> {
    // Basic URI validation - must contain ://
    if !value.contains("://") {
        return Err(ValidationError::InvalidFormat {
            format: "uri".to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

/// Validate language tag (BCP 47)
fn validate_language(value: &str) -> Result<()> {
    // Basic BCP 47 validation
    if value.is_empty() {
        return Err(ValidationError::InvalidFormat {
            format: "language".to_string(),
            value: value.to_string(),
        });
    }

    // Language tags are alphanumeric with hyphens
    if !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(ValidationError::InvalidFormat {
            format: "language".to_string(),
            value: value.to_string(),
        });
    }

    Ok(())
}

/// Validate string constraints
fn validate_string_constraints(value: &str, constraints: &StringConstraints) -> Result<()> {
    // Check byte length
    let byte_len = value.len();
    if let Some(max) = constraints.max_length {
        if byte_len > max {
            return Err(ValidationError::StringTooLong { actual: byte_len, max });
        }
    }
    if let Some(min) = constraints.min_length {
        if byte_len < min {
            return Err(ValidationError::StringTooShort { actual: byte_len, min });
        }
    }

    // Check grapheme count (Unicode grapheme clusters)
    let grapheme_count = value.graphemes(true).count();
    if let Some(max) = constraints.max_graphemes {
        if grapheme_count > max {
            return Err(ValidationError::TooManyGraphemes { actual: grapheme_count, max });
        }
    }
    if let Some(min) = constraints.min_graphemes {
        if grapheme_count < min {
            return Err(ValidationError::TooFewGraphemes { actual: grapheme_count, min });
        }
    }

    // Check enum
    if let Some(allowed) = &constraints.r#enum {
        if !allowed.contains(&value.to_string()) {
            return Err(ValidationError::NotInEnum { value: value.to_string() });
        }
    }

    // Check const
    if let Some(constant) = &constraints.r#const {
        if value != constant {
            return Err(ValidationError::ConstMismatch {
                actual: value.to_string(),
                expected: constant.clone(),
            });
        }
    }

    Ok(())
}

/// Validate integer value against integer constraints
pub fn validate_integer(value: i64, constraints: &IntegerConstraints) -> Result<()> {
    // Check range
    if let Some(min) = constraints.minimum {
        if let Some(max) = constraints.maximum {
            if value < min || value > max {
                return Err(ValidationError::IntegerOutOfRange { value, min, max });
            }
        } else if value < min {
            return Err(ValidationError::IntegerOutOfRange { value, min, max: i64::MAX });
        }
    } else if let Some(max) = constraints.maximum {
        if value > max {
            return Err(ValidationError::IntegerOutOfRange { value, min: i64::MIN, max });
        }
    }

    // Check enum
    if let Some(allowed) = &constraints.r#enum {
        if !allowed.contains(&value) {
            return Err(ValidationError::NotInEnum { value: value.to_string() });
        }
    }

    // Check const
    if let Some(constant) = constraints.r#const {
        if value != constant {
            return Err(ValidationError::ConstMismatch {
                actual: value.to_string(),
                expected: constant.to_string(),
            });
        }
    }

    Ok(())
}

/// Validate array length against array constraints
pub fn validate_array_length(length: usize, constraints: &ArrayConstraints) -> Result<()> {
    if let Some(max) = constraints.max_length {
        if length > max {
            return Err(ValidationError::ArrayTooLong { actual: length, max });
        }
    }

    if let Some(min) = constraints.min_length {
        if length < min {
            return Err(ValidationError::ArrayTooShort { actual: length, min });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_did_valid() {
        assert!(validate_did("did:plc:abc123").is_ok());
        assert!(validate_did("did:web:example.com").is_ok());
        assert!(validate_did("did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH").is_ok());
    }

    #[test]
    fn test_validate_did_invalid() {
        assert!(validate_did("not-a-did").is_err());
        assert!(validate_did("did:").is_err());
        assert!(validate_did("did:plc:").is_err());
    }

    #[test]
    fn test_validate_handle_valid() {
        assert!(validate_handle("user.bsky.social").is_ok());
        assert!(validate_handle("example.com").is_ok());
        assert!(validate_handle("subdomain.example.com").is_ok());
    }

    #[test]
    fn test_validate_handle_invalid() {
        assert!(validate_handle("").is_err());
        assert!(validate_handle("nodot").is_err());
        assert!(validate_handle(".startsdot.com").is_err());
        assert!(validate_handle("endsdot.com.").is_err());
        assert!(validate_handle("invalid-.com").is_err());
    }

    #[test]
    fn test_validate_datetime_valid() {
        assert!(validate_datetime("2024-01-01T00:00:00Z").is_ok());
        assert!(validate_datetime("2024-01-01T00:00:00+00:00").is_ok());
        assert!(validate_datetime("2024-01-01T00:00:00-05:00").is_ok());
    }

    #[test]
    fn test_validate_datetime_invalid() {
        assert!(validate_datetime("2024-01-01").is_err());
        assert!(validate_datetime("not a date").is_err());
    }

    #[test]
    fn test_validate_at_uri_valid() {
        assert!(validate_at_uri("at://did:plc:abc123/app.bsky.feed.post/abc").is_ok());
    }

    #[test]
    fn test_validate_at_uri_invalid() {
        assert!(validate_at_uri("https://example.com").is_err());
        assert!(validate_at_uri("not-a-uri").is_err());
    }

    #[test]
    fn test_validate_tid_valid() {
        assert!(validate_tid("3jui7kd54zh2y").is_ok());
    }

    #[test]
    fn test_validate_tid_invalid() {
        assert!(validate_tid("tooshort").is_err()); // Too short
        assert!(validate_tid("waytoolo ngstr").is_err()); // Too long and has space
        assert!(validate_tid("UPPERCASE1234").is_err()); // Uppercase letters
        assert!(validate_tid("invalid!chars").is_err()); // Invalid characters
    }

    #[test]
    fn test_validate_string_length() {
        let constraints = StringConstraints {
            max_length: Some(10),
            min_length: Some(2),
            ..Default::default()
        };

        assert!(validate_string_constraints("hello", &constraints).is_ok());
        assert!(validate_string_constraints("hi", &constraints).is_ok());
        assert!(validate_string_constraints("h", &constraints).is_err()); // Too short
        assert!(validate_string_constraints("this is way too long", &constraints).is_err());
        // Too long
    }

    #[test]
    fn test_validate_string_enum() {
        let constraints = StringConstraints {
            r#enum: Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
            ..Default::default()
        };

        assert!(validate_string_constraints("a", &constraints).is_ok());
        assert!(validate_string_constraints("b", &constraints).is_ok());
        assert!(validate_string_constraints("d", &constraints).is_err());
    }

    #[test]
    fn test_validate_string_const() {
        let constraints = StringConstraints {
            r#const: Some("exact".to_string()),
            ..Default::default()
        };

        assert!(validate_string_constraints("exact", &constraints).is_ok());
        assert!(validate_string_constraints("different", &constraints).is_err());
    }

    #[test]
    fn test_validate_integer_range() {
        let constraints = IntegerConstraints {
            minimum: Some(0),
            maximum: Some(100),
            ..Default::default()
        };

        assert!(validate_integer(50, &constraints).is_ok());
        assert!(validate_integer(0, &constraints).is_ok());
        assert!(validate_integer(100, &constraints).is_ok());
        assert!(validate_integer(-1, &constraints).is_err());
        assert!(validate_integer(101, &constraints).is_err());
    }

    #[test]
    fn test_validate_array_length_constraints() {
        let constraints = ArrayConstraints { max_length: Some(5), min_length: Some(1) };

        assert!(validate_array_length(3, &constraints).is_ok());
        assert!(validate_array_length(1, &constraints).is_ok());
        assert!(validate_array_length(5, &constraints).is_ok());
        assert!(validate_array_length(0, &constraints).is_err());
        assert!(validate_array_length(6, &constraints).is_err());
    }

    #[test]
    fn test_validate_at_identifier_did() {
        assert!(validate_at_identifier("did:plc:abc123").is_ok());
    }

    #[test]
    fn test_validate_at_identifier_handle() {
        assert!(validate_at_identifier("user.bsky.social").is_ok());
    }

    #[test]
    fn test_validate_string_grapheme_count_ascii() {
        let constraints = StringConstraints {
            max_graphemes: Some(5),
            min_graphemes: Some(2),
            ..Default::default()
        };

        // ASCII strings - grapheme count equals char count
        assert!(validate_string_constraints("hello", &constraints).is_ok()); // 5 graphemes
        assert!(validate_string_constraints("hi", &constraints).is_ok()); // 2 graphemes
        assert!(validate_string_constraints("h", &constraints).is_err()); // Too few
        assert!(validate_string_constraints("toolong", &constraints).is_err()); // Too many
    }

    #[test]
    fn test_validate_string_grapheme_count_emoji() {
        let constraints = StringConstraints {
            max_graphemes: Some(3),
            min_graphemes: Some(1),
            ..Default::default()
        };

        // Single emoji counts as 1 grapheme even if multiple code points
        assert!(validate_string_constraints("👍", &constraints).is_ok()); // 1 grapheme
        assert!(validate_string_constraints("🎉", &constraints).is_ok()); // 1 grapheme
        assert!(validate_string_constraints("👍🎉", &constraints).is_ok()); // 2 graphemes
        assert!(validate_string_constraints("👍🎉❤", &constraints).is_ok()); // 3 graphemes
        assert!(validate_string_constraints("👍🎉❤️🔥", &constraints).is_err());
        // 4 graphemes - too many
    }

    #[test]
    fn test_validate_string_grapheme_count_combining_characters() {
        let constraints = StringConstraints {
            max_graphemes: Some(5),
            min_graphemes: Some(1),
            ..Default::default()
        };

        // é can be represented as single code point or e + combining acute accent
        // Both should count as 1 grapheme
        assert!(validate_string_constraints("café", &constraints).is_ok()); // 4 graphemes

        // Multiple combining marks should still count as single grapheme
        assert!(validate_string_constraints("e\u{0301}", &constraints).is_ok()); // e + combining acute = 1 grapheme
        assert!(validate_string_constraints("e\u{0301}\u{0302}", &constraints).is_ok());
        // e + two combining marks = 1 grapheme
    }

    #[test]
    fn test_validate_string_grapheme_count_complex_emoji() {
        let constraints = StringConstraints {
            max_graphemes: Some(5),
            min_graphemes: Some(1),
            ..Default::default()
        };

        // Emoji with skin tone modifier - should count as 1 grapheme
        assert!(validate_string_constraints("👍🏻", &constraints).is_ok()); // 1 grapheme
        assert!(validate_string_constraints("👍🏾", &constraints).is_ok()); // 1 grapheme

        // Family emoji (multiple code points joined with ZWJ) - should count as 1 grapheme
        assert!(validate_string_constraints("👨‍👩‍👧‍👦", &constraints).is_ok()); // 1 grapheme

        // Flag emoji (regional indicator symbols) - should count as 1 grapheme
        assert!(validate_string_constraints("🇺🇸", &constraints).is_ok()); // 1 grapheme
    }

    #[test]
    fn test_validate_string_grapheme_count_mixed_content() {
        let constraints = StringConstraints {
            max_graphemes: Some(10),
            min_graphemes: Some(1),
            ..Default::default()
        };

        // Mix of ASCII, emoji, and combining characters
        assert!(validate_string_constraints("Hello 👋", &constraints).is_ok()); // 7 graphemes: H-e-l-l-o-space-wave
        assert!(validate_string_constraints("café ☕", &constraints).is_ok()); // 6 graphemes
        assert!(validate_string_constraints("🎉Party🎊", &constraints).is_ok());
        // 7 graphemes
    }

    #[test]
    fn test_validate_string_grapheme_vs_char_count() {
        let constraints = StringConstraints {
            max_graphemes: Some(2),
            min_graphemes: Some(1),
            ..Default::default()
        };

        // This string has 3 chars but only 1 grapheme (e + two combining marks)
        let string_with_combining = "e\u{0301}\u{0302}";
        assert_eq!(string_with_combining.chars().count(), 3); // 3 chars
        assert_eq!(string_with_combining.graphemes(true).count(), 1); // 1 grapheme
        assert!(validate_string_constraints(string_with_combining, &constraints).is_ok());

        // This emoji has many code points but is 1 grapheme
        let family_emoji = "👨‍👩‍👧‍👦";
        assert!(family_emoji.chars().count() > 1); // Multiple chars
        assert_eq!(family_emoji.graphemes(true).count(), 1); // 1 grapheme
        assert!(validate_string_constraints(family_emoji, &constraints).is_ok());
    }
}
