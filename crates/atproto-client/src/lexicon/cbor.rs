//! CBOR (DAG-CBOR) encoding and decoding for AT Protocol records
//!
//! This module implements DAG-CBOR encoding/decoding for Lexicon records.
//! DAG-CBOR is a strict subset of CBOR that ensures deterministic encoding
//! and supports IPLD data structures like CID links.
//!
//! Reference: <https://ipld.io/specs/codecs/dag-cbor/>
//!
//! # Example
//!
//! ```rust
//! use atproto_client::lexicon::cbor::{encode_record, decode_record};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
//! struct Post {
//!     text: String,
//!     created_at: String,
//! }
//!
//! let post = Post {
//!     text: "Hello, AT Protocol!".to_string(),
//!     created_at: "2024-01-01T00:00:00Z".to_string(),
//! };
//!
//! // Encode to DAG-CBOR
//! let bytes = encode_record(&post).unwrap();
//!
//! // Decode back
//! let decoded: Post = decode_record(&bytes).unwrap();
//! assert_eq!(post, decoded);
//! ```

use serde::{Deserialize, Serialize};
use serde_ipld_dagcbor::{from_slice, to_vec};
use thiserror::Error;

/// Errors that can occur during CBOR encoding/decoding
#[derive(Debug, Error)]
pub enum CborError {
    /// Error encoding to CBOR
    #[error("CBOR encoding error: {0}")]
    EncodingError(String),

    /// Error decoding from CBOR
    #[error("CBOR decoding error: {0}")]
    DecodingError(String),

    /// Invalid CID format
    #[error("Invalid CID: {0}")]
    InvalidCid(String),

    /// Invalid blob reference
    #[error("Invalid blob reference: {0}")]
    InvalidBlob(String),
}

/// Result type for CBOR operations
pub type Result<T> = std::result::Result<T, CborError>;

/// Encode a value to DAG-CBOR bytes
///
/// This function serializes any Serde-serializable type to DAG-CBOR format.
/// The encoding is deterministic, meaning the same input always produces
/// the same byte sequence.
///
/// # Arguments
///
/// * `value` - The value to encode (must implement `Serialize`)
///
/// # Example
///
/// ```rust
/// use atproto_client::lexicon::cbor::encode_record;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     text: String,
/// }
///
/// let record = Record {
///     text: "Hello".to_string(),
/// };
///
/// let bytes = encode_record(&record).unwrap();
/// assert!(!bytes.is_empty());
/// ```
pub fn encode_record<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    to_vec(value).map_err(|e| CborError::EncodingError(e.to_string()))
}

/// Decode a value from DAG-CBOR bytes
///
/// This function deserializes a value from DAG-CBOR format.
///
/// # Arguments
///
/// * `bytes` - The DAG-CBOR encoded bytes
///
/// # Example
///
/// ```rust
/// use atproto_client::lexicon::cbor::{encode_record, decode_record};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, PartialEq, Serialize, Deserialize)]
/// struct Record {
///     text: String,
/// }
///
/// let original = Record {
///     text: "Hello".to_string(),
/// };
///
/// let bytes = encode_record(&original).unwrap();
/// let decoded: Record = decode_record(&bytes).unwrap();
///
/// assert_eq!(original, decoded);
/// ```
pub fn decode_record<T>(bytes: &[u8]) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    from_slice(bytes).map_err(|e| CborError::DecodingError(e.to_string()))
}

/// CID link representation
///
/// CID (Content Identifier) links are represented in DAG-CBOR as a special
/// tagged value. This struct provides a way to work with CIDs in records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CidLink {
    /// The CID string (e.g., "bafyreib...")
    #[serde(rename = "/")]
    pub cid: String,
}

impl CidLink {
    /// Create a new CID link
    pub fn new(cid: impl Into<String>) -> Self {
        Self { cid: cid.into() }
    }

    /// Get the CID string
    pub fn as_str(&self) -> &str {
        &self.cid
    }
}

/// Blob reference representation
///
/// Blobs in AT Protocol are represented as objects with metadata about
/// the binary data (MIME type, size, and a CID reference).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobRef {
    /// MIME type of the blob
    pub mime_type: String,

    /// Size of the blob in bytes
    pub size: usize,

    /// CID reference to the blob data
    #[serde(rename = "ref")]
    pub ref_link: CidLink,
}

impl BlobRef {
    /// Create a new blob reference
    pub fn new(mime_type: impl Into<String>, size: usize, cid: impl Into<String>) -> Self {
        Self {
            mime_type: mime_type.into(),
            size,
            ref_link: CidLink::new(cid),
        }
    }
}

/// Record representation with type field
///
/// AT Protocol records include a `$type` field that identifies the record type.
/// This is a convenience type for working with typed records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypedRecord<T> {
    /// The record type (NSID)
    #[serde(rename = "$type")]
    pub record_type: String,

    /// The record data
    #[serde(flatten)]
    pub data: T,
}

impl<T> TypedRecord<T> {
    /// Create a new typed record
    pub fn new(record_type: impl Into<String>, data: T) -> Self {
        Self {
            record_type: record_type.into(),
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct SimpleRecord {
        text: String,
        count: i64,
    }

    #[test]
    fn test_encode_decode_simple() {
        let record = SimpleRecord {
            text: "Hello, World!".to_string(),
            count: 42,
        };

        let bytes = encode_record(&record).unwrap();
        assert!(!bytes.is_empty());

        let decoded: SimpleRecord = decode_record(&bytes).unwrap();
        assert_eq!(record, decoded);
    }

    #[test]
    fn test_encode_decode_with_optional() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct RecordWithOptional {
            text: String,
            optional: Option<String>,
        }

        let record = RecordWithOptional {
            text: "Hello".to_string(),
            optional: Some("World".to_string()),
        };

        let bytes = encode_record(&record).unwrap();
        let decoded: RecordWithOptional = decode_record(&bytes).unwrap();
        assert_eq!(record, decoded);

        let record_none = RecordWithOptional {
            text: "Hello".to_string(),
            optional: None,
        };

        let bytes = encode_record(&record_none).unwrap();
        let decoded: RecordWithOptional = decode_record(&bytes).unwrap();
        assert_eq!(record_none, decoded);
    }

    #[test]
    fn test_encode_decode_array() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct RecordWithArray {
            items: Vec<String>,
        }

        let record = RecordWithArray {
            items: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };

        let bytes = encode_record(&record).unwrap();
        let decoded: RecordWithArray = decode_record(&bytes).unwrap();
        assert_eq!(record, decoded);
    }

    #[test]
    fn test_encode_decode_nested() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Inner {
            value: i64,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Outer {
            inner: Inner,
            text: String,
        }

        let record = Outer {
            inner: Inner { value: 123 },
            text: "test".to_string(),
        };

        let bytes = encode_record(&record).unwrap();
        let decoded: Outer = decode_record(&bytes).unwrap();
        assert_eq!(record, decoded);
    }

    #[test]
    fn test_cid_link() {
        let cid = CidLink::new("bafyreibjo4xmgaevkgud7mbifn3dzp4v4lyaui4yvqp3f2bqwtxcjrdqg4");
        assert_eq!(
            cid.as_str(),
            "bafyreibjo4xmgaevkgud7mbifn3dzp4v4lyaui4yvqp3f2bqwtxcjrdqg4"
        );

        let bytes = encode_record(&cid).unwrap();
        let decoded: CidLink = decode_record(&bytes).unwrap();
        assert_eq!(cid, decoded);
    }

    #[test]
    fn test_blob_ref() {
        let blob = BlobRef::new(
            "image/png",
            12345,
            "bafyreibjo4xmgaevkgud7mbifn3dzp4v4lyaui4yvqp3f2bqwtxcjrdqg4",
        );

        assert_eq!(blob.mime_type, "image/png");
        assert_eq!(blob.size, 12345);

        let bytes = encode_record(&blob).unwrap();
        let decoded: BlobRef = decode_record(&bytes).unwrap();
        assert_eq!(blob, decoded);
    }

    #[test]
    fn test_typed_record() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Post {
            text: String,
            created_at: String,
        }

        let typed = TypedRecord::new(
            "app.bsky.feed.post",
            Post {
                text: "Hello, AT Protocol!".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            },
        );

        let bytes = encode_record(&typed).unwrap();
        let decoded: TypedRecord<Post> = decode_record(&bytes).unwrap();
        assert_eq!(typed, decoded);
        assert_eq!(decoded.record_type, "app.bsky.feed.post");
    }

    #[test]
    fn test_record_with_cid_link() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct RecordWithCid {
            text: String,
            link: CidLink,
        }

        let record = RecordWithCid {
            text: "Reference".to_string(),
            link: CidLink::new("bafyreibjo4xmgaevkgud7mbifn3dzp4v4lyaui4yvqp3f2bqwtxcjrdqg4"),
        };

        let bytes = encode_record(&record).unwrap();
        let decoded: RecordWithCid = decode_record(&bytes).unwrap();
        assert_eq!(record, decoded);
    }

    #[test]
    fn test_record_with_blob() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct RecordWithBlob {
            text: String,
            image: BlobRef,
        }

        let record = RecordWithBlob {
            text: "Post with image".to_string(),
            image: BlobRef::new(
                "image/jpeg",
                524288,
                "bafyreibjo4xmgaevkgud7mbifn3dzp4v4lyaui4yvqp3f2bqwtxcjrdqg4",
            ),
        };

        let bytes = encode_record(&record).unwrap();
        let decoded: RecordWithBlob = decode_record(&bytes).unwrap();
        assert_eq!(record, decoded);
    }

    #[test]
    fn test_deterministic_encoding() {
        let record = SimpleRecord {
            text: "Deterministic".to_string(),
            count: 100,
        };

        let bytes1 = encode_record(&record).unwrap();
        let bytes2 = encode_record(&record).unwrap();

        // Same input should produce identical bytes
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn test_btreemap_ordering() {
        use std::collections::BTreeMap;

        let mut map = BTreeMap::new();
        map.insert("z".to_string(), 1);
        map.insert("a".to_string(), 2);
        map.insert("m".to_string(), 3);

        let bytes = encode_record(&map).unwrap();
        let decoded: BTreeMap<String, i32> = decode_record(&bytes).unwrap();

        // BTreeMap maintains sorted order
        assert_eq!(map, decoded);
        let keys: Vec<_> = decoded.keys().collect();
        assert_eq!(keys, vec!["a", "m", "z"]);
    }

    #[test]
    fn test_complex_at_protocol_record() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Facet {
            index: Index,
            features: Vec<Feature>,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Index {
            byte_start: usize,
            byte_end: usize,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "$type", rename_all = "camelCase")]
        enum Feature {
            #[serde(rename = "app.bsky.richtext.facet#mention")]
            Mention { did: String },
            #[serde(rename = "app.bsky.richtext.facet#link")]
            Link { uri: String },
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Post {
            text: String,
            facets: Option<Vec<Facet>>,
            created_at: String,
        }

        let post = Post {
            text: "Hello @alice.bsky.social!".to_string(),
            facets: Some(vec![Facet {
                index: Index {
                    byte_start: 6,
                    byte_end: 25,
                },
                features: vec![Feature::Mention {
                    did: "did:plc:abc123".to_string(),
                }],
            }]),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let bytes = encode_record(&post).unwrap();
        let decoded: Post = decode_record(&bytes).unwrap();
        assert_eq!(post, decoded);
    }
}
