//! Lexicon type definitions
//!
//! This module defines the core type system for Lexicon schemas.

use super::constraints::*;
use super::formats::StringFormat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A reference to another schema definition
///
/// Can be local (`#defName`) or external (`nsid#defName`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LexRef(pub String);

impl LexRef {
    /// Create a new reference
    pub fn new(s: impl Into<String>) -> Self {
        LexRef(s.into())
    }

    /// Check if this is a local reference (starts with #)
    pub fn is_local(&self) -> bool {
        self.0.starts_with('#')
    }

    /// Get the definition name (part after #)
    pub fn def_name(&self) -> Option<&str> {
        self.0.split('#').nth(1)
    }

    /// Get the NSID part (for external refs)
    pub fn nsid(&self) -> Option<&str> {
        if self.is_local() {
            None
        } else {
            self.0.split('#').next()
        }
    }
}

fn default_ref_type() -> String {
    "ref".to_string()
}

/// String type with optional format and constraints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexString {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_string_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// String format (at-uri, did, handle, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<StringFormat>,

    /// Constraints
    #[serde(flatten)]
    pub constraints: StringConstraints,
}

fn default_string_type() -> String {
    "string".to_string()
}

/// Integer type with constraints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexInteger {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_integer_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Constraints
    #[serde(flatten)]
    pub constraints: IntegerConstraints,
}

fn default_integer_type() -> String {
    "integer".to_string()
}

/// Boolean type with constraints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexBoolean {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_boolean_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Constraints
    #[serde(flatten)]
    pub constraints: BooleanConstraints,
}

fn default_boolean_type() -> String {
    "boolean".to_string()
}

/// Bytes type (base64-encoded binary data)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexBytes {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_bytes_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Constraints
    #[serde(flatten)]
    pub constraints: BytesConstraints,
}

fn default_bytes_type() -> String {
    "bytes".to_string()
}

/// CID link type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexCidLink {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_cid_link_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_cid_link_type() -> String {
    "cid-link".to_string()
}

/// Blob type (binary data with metadata)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexBlob {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_blob_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Constraints
    #[serde(flatten)]
    pub constraints: BlobConstraints,
}

fn default_blob_type() -> String {
    "blob".to_string()
}

/// Array type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexArray {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_array_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Schema for array items
    pub items: Box<LexType>,

    /// Constraints
    #[serde(flatten)]
    pub constraints: ArrayConstraints,
}

fn default_array_type() -> String {
    "array".to_string()
}

/// Token type (named symbolic value with no data representation)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexToken {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_token_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_token_type() -> String {
    "token".to_string()
}

/// Object type with properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexObject {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_object_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Properties map
    #[serde(default)]
    pub properties: HashMap<String, LexType>,

    /// Required property names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    /// Nullable property names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<Vec<String>>,
}

fn default_object_type() -> String {
    "object".to_string()
}

/// Union type (one of several possible types)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexUnion {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_union_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Possible types (must be refs)
    pub refs: Vec<String>,

    /// Whether the union is closed (only listed refs allowed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed: Option<bool>,
}

fn default_union_type() -> String {
    "union".to_string()
}

/// Unknown type (accepts any value)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexUnknown {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_unknown_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_unknown_type() -> String {
    "unknown".to_string()
}

/// Reference to another definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexRefType {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_ref_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Reference string
    #[serde(rename = "ref")]
    pub ref_to: String,
}

/// All possible Lexicon types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LexType {
    /// Null type
    Null,

    /// Boolean type
    Boolean(LexBoolean),

    /// Integer type
    Integer(LexInteger),

    /// String type
    String(LexString),

    /// Bytes type (base64-encoded)
    Bytes(LexBytes),

    /// CID link type
    #[serde(rename = "cid-link")]
    CidLink(LexCidLink),

    /// Blob type
    Blob(LexBlob),

    /// Array type
    Array(LexArray),

    /// Object type
    Object(LexObject),

    /// Token type
    Token(LexToken),

    /// Union type
    Union(LexUnion),

    /// Unknown type
    Unknown(LexUnknown),

    /// Reference to another definition
    #[serde(rename = "ref")]
    Ref(LexRefType),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_ref_local() {
        let ref_type = LexRef::new("#main");
        assert!(ref_type.is_local());
        assert_eq!(ref_type.def_name(), Some("main"));
        assert_eq!(ref_type.nsid(), None);
    }

    #[test]
    fn test_lex_ref_external() {
        let ref_type = LexRef::new("com.atproto.repo.strongRef#main");
        assert!(!ref_type.is_local());
        assert_eq!(ref_type.def_name(), Some("main"));
        assert_eq!(ref_type.nsid(), Some("com.atproto.repo.strongRef"));
    }

    #[test]
    fn test_lex_string_serde() {
        let lex_string = LexType::String(LexString {
            type_name: "string".to_string(),
            description: Some("Test string".to_string()),
            format: Some(StringFormat::AtUri),
            constraints: StringConstraints {
                max_length: Some(100),
                ..Default::default()
            },
        });

        let json = serde_json::to_value(&lex_string).unwrap();
        assert_eq!(json["type"], "string");
        assert_eq!(json["format"], "at-uri");
        assert_eq!(json["maxLength"], 100);
    }

    #[test]
    fn test_lex_integer_serde() {
        let lex_int = LexType::Integer(LexInteger {
            type_name: "integer".to_string(),
            description: None,
            constraints: IntegerConstraints {
                minimum: Some(0),
                maximum: Some(100),
                ..Default::default()
            },
        });

        let json = serde_json::to_value(&lex_int).unwrap();
        assert_eq!(json["type"], "integer");
        assert_eq!(json["minimum"], 0);
        assert_eq!(json["maximum"], 100);
    }

    #[test]
    fn test_lex_array_serde() {
        let lex_array = LexType::Array(LexArray {
            type_name: "array".to_string(),
            description: None,
            items: Box::new(LexType::String(LexString {
                type_name: "string".to_string(),
                description: None,
                format: None,
                constraints: Default::default(),
            })),
            constraints: ArrayConstraints {
                max_length: Some(10),
                min_length: Some(1),
            },
        });

        let json = serde_json::to_value(&lex_array).unwrap();
        assert_eq!(json["type"], "array");
        assert_eq!(json["items"]["type"], "string");
    }

    #[test]
    fn test_lex_object_serde() {
        let mut properties = HashMap::new();
        properties.insert(
            "name".to_string(),
            LexType::String(LexString {
                type_name: "string".to_string(),
                description: None,
                format: None,
                constraints: Default::default(),
            }),
        );

        let lex_object = LexType::Object(LexObject {
            type_name: "object".to_string(),
            description: None,
            properties,
            required: Some(vec!["name".to_string()]),
            nullable: None,
        });

        let json = serde_json::to_value(&lex_object).unwrap();
        assert_eq!(json["type"], "object");
        assert!(json["properties"]["name"].is_object());
        assert_eq!(json["required"][0], "name");
    }
}
