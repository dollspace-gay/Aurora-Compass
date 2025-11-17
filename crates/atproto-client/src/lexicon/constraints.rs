//! Field constraints for Lexicon schema validation

use serde::{Deserialize, Serialize};

/// Constraints for string fields
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StringConstraints {
    /// Maximum length in UTF-8 bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,

    /// Minimum length in UTF-8 bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,

    /// Maximum length in Unicode grapheme clusters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_graphemes: Option<usize>,

    /// Minimum length in Unicode grapheme clusters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_graphemes: Option<usize>,

    /// Allowed values (closed set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<String>>,

    /// Constant value (field must always have this value)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#const: Option<String>,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Known values (open set, not enforced)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub known_values: Option<Vec<String>>,
}

/// Constraints for integer fields
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntegerConstraints {
    /// Maximum value (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i64>,

    /// Minimum value (inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i64>,

    /// Allowed values (closed set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#enum: Option<Vec<i64>>,

    /// Constant value (field must always have this value)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#const: Option<i64>,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<i64>,
}

/// Constraints for boolean fields
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BooleanConstraints {
    /// Constant value (field must always have this value)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#const: Option<bool>,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

/// Constraints for array fields
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArrayConstraints {
    /// Maximum number of items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,

    /// Minimum number of items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
}

/// Constraints for blob fields
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlobConstraints {
    /// Accepted MIME types (supports glob patterns like `image/*`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept: Option<Vec<String>>,

    /// Maximum size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<usize>,
}

/// Constraints for bytes fields
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BytesConstraints {
    /// Maximum length in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,

    /// Minimum length in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_constraints_serde() {
        let constraints = StringConstraints {
            max_length: Some(100),
            min_length: Some(1),
            max_graphemes: Some(50),
            r#enum: Some(vec!["a".to_string(), "b".to_string()]),
            ..Default::default()
        };

        let json = serde_json::to_string(&constraints).unwrap();
        let deserialized: StringConstraints = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, constraints);
    }

    #[test]
    fn test_integer_constraints_serde() {
        let constraints = IntegerConstraints {
            maximum: Some(100),
            minimum: Some(0),
            default: Some(50),
            ..Default::default()
        };

        let json = serde_json::to_string(&constraints).unwrap();
        let deserialized: IntegerConstraints = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, constraints);
    }

    #[test]
    fn test_boolean_constraints_serde() {
        let constraints = BooleanConstraints {
            default: Some(true),
            ..Default::default()
        };

        let json = serde_json::to_string(&constraints).unwrap();
        let deserialized: BooleanConstraints = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, constraints);
    }

    #[test]
    fn test_array_constraints_serde() {
        let constraints = ArrayConstraints {
            max_length: Some(10),
            min_length: Some(1),
        };

        let json = serde_json::to_string(&constraints).unwrap();
        let deserialized: ArrayConstraints = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, constraints);
    }

    #[test]
    fn test_blob_constraints_serde() {
        let constraints = BlobConstraints {
            accept: Some(vec!["image/*".to_string(), "video/mp4".to_string()]),
            max_size: Some(5_000_000),
        };

        let json = serde_json::to_string(&constraints).unwrap();
        let deserialized: BlobConstraints = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, constraints);
    }

    #[test]
    fn test_bytes_constraints_serde() {
        let constraints = BytesConstraints {
            max_length: Some(1024),
            min_length: Some(16),
        };

        let json = serde_json::to_string(&constraints).unwrap();
        let deserialized: BytesConstraints = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, constraints);
    }
}
