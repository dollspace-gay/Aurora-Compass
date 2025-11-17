//! Lexicon schema parsing and validation
//!
//! This module implements the AT Protocol Lexicon schema system, which is used
//! to define the structure of records, XRPC endpoints, and event streams.
//!
//! Reference: <https://atproto.com/specs/lexicon>
//!
//! # Overview
//!
//! Lexicon is a schema definition language similar to JSON Schema and OpenAPI,
//! but with AT Protocol-specific features. A Lexicon document defines:
//!
//! - **Records**: Objects stored in repositories
//! - **Queries**: HTTP GET endpoints (XRPC)
//! - **Procedures**: HTTP POST endpoints (XRPC)
//! - **Subscriptions**: WebSocket event streams
//!
//! # Example
//!
//! ```rust
//! use atproto_client::lexicon::{LexiconDoc, StringFormat};
//!
//! // Parse a lexicon schema from JSON
//! let json = r#"{
//!   "lexicon": 1,
//!   "id": "com.example.getRecord",
//!   "defs": {
//!     "main": {
//!       "type": "query",
//!       "parameters": {
//!         "type": "params",
//!         "properties": {
//!           "uri": {
//!             "type": "string",
//!             "format": "at-uri"
//!           }
//!         }
//!       }
//!     }
//!   }
//! }"#;
//!
//! // Parse would be implemented in the parsing module
//! // let doc: LexiconDoc = serde_json::from_str(json)?;
//! ```

pub mod cbor;
pub mod constraints;
pub mod formats;
pub mod parsing;
pub mod resolution;
pub mod schema;
pub mod types;
pub mod validation;

// Re-export commonly used types
pub use constraints::*;
pub use formats::*;
pub use schema::*;
pub use types::*;

// Re-export parsing types (excluding Result to avoid ambiguity)
pub use parsing::LexiconParseError;

// Re-export resolution types (excluding Result to avoid ambiguity)
pub use resolution::{parse_ref, RefResolutionError, SchemaRegistry};

// Re-export validation types (excluding Result to avoid ambiguity)
pub use validation::{
    validate_array_length, validate_integer, validate_string, ValidationError,
};

// Re-export CBOR types (excluding Result to avoid ambiguity)
pub use cbor::{encode_record, decode_record, CidLink, BlobRef, TypedRecord, CborError};
