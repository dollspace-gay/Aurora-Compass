//! Lexicon schema core structures
//!
//! This module defines the top-level Lexicon document structure and all definition types.

use super::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level Lexicon document
///
/// Reference: <https://atproto.com/specs/lexicon>
///
/// # Example
///
/// ```json
/// {
///   "lexicon": 1,
///   "id": "com.example.getRecord",
///   "description": "Get a record by URI",
///   "defs": {
///     "main": {
///       "type": "query",
///       "parameters": { ... },
///       "output": { ... }
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LexiconDoc {
    /// Lexicon language version (must be 1)
    pub lexicon: u32,

    /// NSID identifier for this lexicon
    pub id: String,

    /// Revision number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u32>,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Map of named definitions
    pub defs: HashMap<String, LexiconDef>,
}

impl LexiconDoc {
    /// Create a new Lexicon document
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            lexicon: 1,
            id: id.into(),
            revision: None,
            description: None,
            defs: HashMap::new(),
        }
    }

    /// Add a definition to the document
    pub fn with_def(mut self, name: impl Into<String>, def: LexiconDef) -> Self {
        self.defs.insert(name.into(), def);
        self
    }

    /// Get the main definition (if it exists)
    pub fn main_def(&self) -> Option<&LexiconDef> {
        self.defs.get("main")
    }
}

/// Record key type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordKeyType {
    /// Timestamp Identifier (TID)
    Tid,

    /// Any valid record key
    Any,
}

fn default_record_type() -> String {
    "record".to_string()
}

/// Record definition (storable in repository)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexRecord {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_record_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Record key type
    pub key: RecordKeyType,

    /// Record schema (must be an object)
    pub record: LexObject,
}

fn default_params_type() -> String {
    "params".to_string()
}

/// Query parameters (HTTP query string params)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexParams {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_params_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Properties map (limited to boolean, integer, string, unknown)
    #[serde(default)]
    pub properties: HashMap<String, LexType>,

    /// Required property names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// XRPC input/output body
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexBody {
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Encoding (MIME type, e.g., "application/json")
    pub encoding: String,

    /// Schema for the body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Box<LexType>>,
}

/// XRPC error definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexXrpcError {
    /// Error name
    pub name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_query_type() -> String {
    "query".to_string()
}

/// Query definition (HTTP GET endpoint)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexQuery {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_query_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Query parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<LexParams>,

    /// Output body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<LexBody>,

    /// Possible errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<LexXrpcError>>,
}

fn default_procedure_type() -> String {
    "procedure".to_string()
}

/// Procedure definition (HTTP POST endpoint)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexProcedure {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_procedure_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Query parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<LexParams>,

    /// Input body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<LexBody>,

    /// Output body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<LexBody>,

    /// Possible errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<LexXrpcError>>,
}

fn default_subscription_type() -> String {
    "subscription".to_string()
}

/// Subscription definition (WebSocket event stream)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LexSubscription {
    /// Type discriminator
    #[serde(skip_deserializing, default = "default_subscription_type")]
    pub type_name: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Query parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<LexParams>,

    /// Message schema (union of possible message types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Box<LexType>>,

    /// Possible errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<LexXrpcError>>,
}

/// All possible Lexicon definition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LexiconDef {
    /// Record definition
    Record(LexRecord),

    /// Query definition (GET endpoint)
    Query(LexQuery),

    /// Procedure definition (POST endpoint)
    Procedure(LexProcedure),

    /// Subscription definition (WebSocket)
    Subscription(LexSubscription),

    /// Token (named symbolic value)
    Token(LexToken),

    /// Object (reusable object definition)
    Object(LexObject),

    /// Array (reusable array definition)
    Array(LexArray),

    /// String (reusable string definition)
    String(LexString),

    /// Integer (reusable integer definition)
    Integer(LexInteger),

    /// Boolean (reusable boolean definition)
    Boolean(LexBoolean),

    /// Bytes (reusable bytes definition)
    Bytes(LexBytes),

    /// CID link (reusable CID link definition)
    #[serde(rename = "cid-link")]
    CidLink(LexCidLink),

    /// Blob (reusable blob definition)
    Blob(LexBlob),

    /// Union (reusable union definition)
    Union(LexUnion),

    /// Unknown (reusable unknown definition)
    Unknown(LexUnknown),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexicon_doc_new() {
        let doc = LexiconDoc::new("com.example.test");
        assert_eq!(doc.lexicon, 1);
        assert_eq!(doc.id, "com.example.test");
        assert!(doc.defs.is_empty());
    }

    #[test]
    fn test_lexicon_doc_with_def() {
        let token = LexToken {
            type_name: "token".to_string(),
            description: Some("Test token".to_string()),
        };

        let doc = LexiconDoc::new("com.example.test")
            .with_def("testToken", LexiconDef::Token(token));

        assert_eq!(doc.defs.len(), 1);
        assert!(doc.defs.contains_key("testToken"));
    }

    #[test]
    fn test_lexicon_doc_main_def() {
        let token = LexToken {
            type_name: "token".to_string(),
            description: None,
        };

        let doc = LexiconDoc::new("com.example.test")
            .with_def("main", LexiconDef::Token(token.clone()))
            .with_def("other", LexiconDef::Token(token));

        assert!(doc.main_def().is_some());
        assert!(matches!(doc.main_def(), Some(LexiconDef::Token(_))));
    }

    #[test]
    fn test_record_key_type_serde() {
        let tid = RecordKeyType::Tid;
        let json = serde_json::to_string(&tid).unwrap();
        assert_eq!(json, "\"tid\"");

        let any = RecordKeyType::Any;
        let json = serde_json::to_string(&any).unwrap();
        assert_eq!(json, "\"any\"");
    }

    #[test]
    fn test_lex_query_serde() {
        let query = LexiconDef::Query(LexQuery {
            type_name: "query".to_string(),
            description: Some("Test query".to_string()),
            parameters: None,
            output: Some(LexBody {
                description: None,
                encoding: "application/json".to_string(),
                schema: None,
            }),
            errors: None,
        });

        let json = serde_json::to_value(&query).unwrap();
        assert_eq!(json["type"], "query");
        assert_eq!(json["output"]["encoding"], "application/json");
    }

    #[test]
    fn test_lex_procedure_serde() {
        let procedure = LexiconDef::Procedure(LexProcedure {
            type_name: "procedure".to_string(),
            description: Some("Test procedure".to_string()),
            parameters: None,
            input: Some(LexBody {
                description: None,
                encoding: "application/json".to_string(),
                schema: None,
            }),
            output: None,
            errors: None,
        });

        let json = serde_json::to_value(&procedure).unwrap();
        assert_eq!(json["type"], "procedure");
        assert_eq!(json["input"]["encoding"], "application/json");
    }

    #[test]
    fn test_lexicon_doc_serde() {
        let doc = LexiconDoc {
            lexicon: 1,
            id: "com.example.test".to_string(),
            revision: Some(1),
            description: Some("Test lexicon".to_string()),
            defs: {
                let mut defs = HashMap::new();
                defs.insert(
                    "main".to_string(),
                    LexiconDef::Token(LexToken {
                        type_name: "token".to_string(),
                        description: None,
                    }),
                );
                defs
            },
        };

        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["lexicon"], 1);
        assert_eq!(json["id"], "com.example.test");
        assert_eq!(json["defs"]["main"]["type"], "token");

        // Test full round-trip serialization/deserialization
        let serialized = serde_json::to_string(&doc).unwrap();
        let deserialized: LexiconDoc = serde_json::from_str(&serialized).unwrap();

        // Verify the deserialized doc matches the original
        assert_eq!(deserialized.lexicon, doc.lexicon);
        assert_eq!(deserialized.id, doc.id);
        assert_eq!(deserialized.revision, doc.revision);
        assert_eq!(deserialized.description, doc.description);
        assert_eq!(deserialized.defs.len(), doc.defs.len());

        // Verify the definition was deserialized correctly
        let def = deserialized.defs.get("main").unwrap();
        match def {
            LexiconDef::Token(token) => {
                assert_eq!(token.type_name, "token");
                assert_eq!(token.description, None);
            }
            _ => panic!("Expected Token definition, got {:?}", def),
        }
    }

    #[test]
    fn test_complex_lexicon_doc() {
        // Create a more complex lexicon with multiple definition types
        let mut properties = HashMap::new();
        properties.insert(
            "text".to_string(),
            LexType::String(LexString {
                type_name: "string".to_string(),
                description: Some("Post text".to_string()),
                format: None,
                constraints: super::super::constraints::StringConstraints {
                    max_graphemes: Some(300),
                    ..Default::default()
                },
            }),
        );

        let record = LexRecord {
            type_name: "record".to_string(),
            description: Some("A social media post".to_string()),
            key: RecordKeyType::Tid,
            record: LexObject {
                type_name: "object".to_string(),
                description: None,
                properties,
                required: Some(vec!["text".to_string()]),
                nullable: None,
            },
        };

        let doc = LexiconDoc::new("com.example.post")
            .with_def("main", LexiconDef::Record(record));

        // Serialize and verify
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["defs"]["main"]["type"], "record");
        assert_eq!(json["defs"]["main"]["key"], "tid");
        assert_eq!(
            json["defs"]["main"]["record"]["properties"]["text"]["maxGraphemes"],
            300
        );
    }
}
