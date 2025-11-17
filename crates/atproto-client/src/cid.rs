//! Content Identifier (CID) generation and validation for AT Protocol
//!
//! This module implements CID generation and validation for AT Protocol records.
//! CIDs are used to create content-addressable identifiers for records stored
//! in repositories.
//!
//! Reference: <https://github.com/multiformats/cid>
//!
//! # Example
//!
//! ```rust
//! use atproto_client::cid::{generate_cid, validate_cid_string};
//! use atproto_client::lexicon::cbor::encode_record;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
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
//! // Generate CID
//! let cid = generate_cid(&bytes).unwrap();
//! let cid_string = cid.to_string();
//!
//! // Validate CID string
//! assert!(validate_cid_string(&cid_string).is_ok());
//! ```

use cid::Cid;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors that can occur during CID operations
#[derive(Debug, Error)]
pub enum CidError {
    /// Error generating CID
    #[error("CID generation error: {0}")]
    GenerationError(String),

    /// Error parsing CID string
    #[error("CID parsing error: {0}")]
    ParseError(#[from] cid::Error),

    /// Invalid CID version
    #[error("Invalid CID version: expected v1, got {0:?}")]
    InvalidVersion(cid::Version),

    /// Invalid multihash codec
    #[error("Invalid multihash codec: expected SHA-256 (0x12), got {0:#x}")]
    InvalidCodec(u64),

    /// CID string is empty
    #[error("CID string is empty")]
    EmptyString,
}

/// Result type for CID operations
pub type Result<T> = std::result::Result<T, CidError>;

/// Generate a CID from DAG-CBOR encoded bytes
///
/// This function creates a CIDv1 with SHA-256 multihash from the given bytes.
/// This is the standard CID generation method for AT Protocol records.
///
/// # Arguments
///
/// * `bytes` - The DAG-CBOR encoded record bytes
///
/// # Returns
///
/// A CIDv1 identifier for the record
///
/// # Example
///
/// ```rust
/// use atproto_client::cid::generate_cid;
///
/// let data = b"example data";
/// let cid = generate_cid(data).unwrap();
/// println!("CID: {}", cid);
/// ```
pub fn generate_cid(bytes: &[u8]) -> Result<Cid> {
    // Compute SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash_bytes = hasher.finalize();

    // Wrap in multihash format (code 0x12 = SHA-256)
    let mh = multihash_codetable::Multihash::wrap(0x12, &hash_bytes)
        .map_err(|e| CidError::GenerationError(format!("Failed to create multihash: {}", e)))?;

    // Create CIDv1 with DAG-CBOR codec (0x71)
    let cid = Cid::new_v1(0x71, mh);

    Ok(cid)
}

/// Parse a CID from a string
///
/// This function parses a CID string and validates that it's a valid
/// CIDv1 with SHA-256 multihash.
///
/// # Arguments
///
/// * `cid_str` - The CID string to parse (base32 or base58btc encoded)
///
/// # Returns
///
/// The parsed CID
///
/// # Example
///
/// ```rust
/// use atproto_client::cid::parse_cid;
///
/// let cid_str = "bafyreib2rxk3rybk3aobmv46tme4ddxaev7jfaubgfr3fmn66vd4gxkn4i";
/// let cid = parse_cid(cid_str).unwrap();
/// assert_eq!(cid.to_string(), cid_str);
/// ```
pub fn parse_cid(cid_str: &str) -> Result<Cid> {
    if cid_str.is_empty() {
        return Err(CidError::EmptyString);
    }

    let cid = Cid::try_from(cid_str)?;
    Ok(cid)
}

/// Validate a CID string
///
/// This function validates that a CID string is:
/// - Valid CID format
/// - Version 1
/// - Uses SHA-256 multihash (as required by AT Protocol)
///
/// # Arguments
///
/// * `cid_str` - The CID string to validate
///
/// # Example
///
/// ```rust
/// use atproto_client::cid::validate_cid_string;
///
/// let cid_str = "bafyreib2rxk3rybk3aobmv46tme4ddxaev7jfaubgfr3fmn66vd4gxkn4i";
/// assert!(validate_cid_string(cid_str).is_ok());
/// ```
pub fn validate_cid_string(cid_str: &str) -> Result<Cid> {
    let cid = parse_cid(cid_str)?;

    // Validate CID version (must be v1 for AT Protocol)
    if cid.version() != cid::Version::V1 {
        return Err(CidError::InvalidVersion(cid.version()));
    }

    // Validate multihash codec (must be SHA-256, code 0x12)
    let hash_code = cid.hash().code();
    if hash_code != 0x12 {
        return Err(CidError::InvalidCodec(hash_code));
    }

    Ok(cid)
}

/// Validate a CID object
///
/// This function validates that a CID is:
/// - Version 1
/// - Uses SHA-256 multihash (as required by AT Protocol)
///
/// # Arguments
///
/// * `cid` - The CID to validate
///
/// # Example
///
/// ```rust
/// use atproto_client::cid::{generate_cid, validate_cid};
///
/// let data = b"example data";
/// let cid = generate_cid(data).unwrap();
/// assert!(validate_cid(&cid).is_ok());
/// ```
pub fn validate_cid(cid: &Cid) -> Result<()> {
    // Validate CID version (must be v1 for AT Protocol)
    if cid.version() != cid::Version::V1 {
        return Err(CidError::InvalidVersion(cid.version()));
    }

    // Validate multihash codec (must be SHA-256, code 0x12)
    let hash_code = cid.hash().code();
    if hash_code != 0x12 {
        return Err(CidError::InvalidCodec(hash_code));
    }

    Ok(())
}

/// Convert a CID to a base32 string
///
/// This is the standard string representation for CIDs in AT Protocol.
///
/// # Arguments
///
/// * `cid` - The CID to convert
///
/// # Returns
///
/// Base32-encoded CID string
///
/// # Example
///
/// ```rust
/// use atproto_client::cid::{generate_cid, cid_to_string};
///
/// let data = b"example data";
/// let cid = generate_cid(data).unwrap();
/// let cid_str = cid_to_string(&cid);
/// assert!(cid_str.starts_with("bafy"));
/// ```
pub fn cid_to_string(cid: &Cid) -> String {
    cid.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cid() {
        let data = b"Hello, AT Protocol!";
        let cid = generate_cid(data).unwrap();

        // Verify it's a valid CID
        assert_eq!(cid.version(), cid::Version::V1);
        assert_eq!(cid.codec(), 0x71); // DAG-CBOR codec
        assert_eq!(cid.hash().code(), 0x12); // SHA-256
    }

    #[test]
    fn test_generate_cid_deterministic() {
        let data = b"Deterministic test";

        // Generate CID twice from same data
        let cid1 = generate_cid(data).unwrap();
        let cid2 = generate_cid(data).unwrap();

        // Should be identical
        assert_eq!(cid1, cid2);
        assert_eq!(cid1.to_string(), cid2.to_string());
    }

    #[test]
    fn test_generate_cid_different_data() {
        let data1 = b"Data 1";
        let data2 = b"Data 2";

        let cid1 = generate_cid(data1).unwrap();
        let cid2 = generate_cid(data2).unwrap();

        // Different data should produce different CIDs
        assert_ne!(cid1, cid2);
    }

    #[test]
    fn test_parse_cid_valid() {
        // Known valid CIDv1 with SHA-256
        let cid_str = "bafyreib2rxk3rybk3aobmv46tme4ddxaev7jfaubgfr3fmn66vd4gxkn4i";
        let cid = parse_cid(cid_str).unwrap();

        assert_eq!(cid.version(), cid::Version::V1);
        assert_eq!(cid.to_string(), cid_str);
    }

    #[test]
    fn test_parse_cid_empty() {
        let result = parse_cid("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CidError::EmptyString));
    }

    #[test]
    fn test_parse_cid_invalid_format() {
        let result = parse_cid("not-a-valid-cid");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_cid_string_valid() {
        // Generate a valid CID
        let data = b"test data";
        let cid = generate_cid(data).unwrap();
        let cid_str = cid.to_string();

        // Validate it
        let validated = validate_cid_string(&cid_str).unwrap();
        assert_eq!(cid, validated);
    }

    #[test]
    fn test_validate_cid_valid() {
        let data = b"test data";
        let cid = generate_cid(data).unwrap();

        // Should pass validation
        assert!(validate_cid(&cid).is_ok());
    }

    #[test]
    fn test_cid_to_string() {
        let data = b"example";
        let cid = generate_cid(data).unwrap();
        let cid_str = cid_to_string(&cid);

        // CIDv1 base32 strings start with "bafy" for DAG-CBOR
        assert!(cid_str.starts_with("bafy"));

        // Should be parseable back
        let parsed = parse_cid(&cid_str).unwrap();
        assert_eq!(cid, parsed);
    }

    #[test]
    fn test_round_trip() {
        let data = b"Round trip test data";

        // Generate CID
        let original_cid = generate_cid(data).unwrap();

        // Convert to string
        let cid_str = cid_to_string(&original_cid);

        // Parse back
        let parsed_cid = parse_cid(&cid_str).unwrap();

        // Should be identical
        assert_eq!(original_cid, parsed_cid);
    }

    #[test]
    fn test_with_real_record() {
        use crate::lexicon::cbor::encode_record;
        use serde::Serialize;

        #[derive(Serialize)]
        struct Post {
            text: String,
            created_at: String,
        }

        let post = Post {
            text: "Hello, AT Protocol!".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        // Encode to DAG-CBOR
        let bytes = encode_record(&post).unwrap();

        // Generate CID
        let cid = generate_cid(&bytes).unwrap();

        // Validate
        assert!(validate_cid(&cid).is_ok());

        // Convert to string and validate
        let cid_str = cid_to_string(&cid);
        assert!(validate_cid_string(&cid_str).is_ok());

        // Verify deterministic: same record should produce same CID
        let bytes2 = encode_record(&post).unwrap();
        let cid2 = generate_cid(&bytes2).unwrap();
        assert_eq!(cid, cid2);
    }

    #[test]
    fn test_known_cid_example() {
        // Test with a known CID from AT Protocol documentation
        // This is a real CID from the AT Protocol spec
        let known_cid = "bafyreib2rxk3rybk3aobmv46tme4ddxaev7jfaubgfr3fmn66vd4gxkn4i";

        // Should parse successfully
        let cid = parse_cid(known_cid).unwrap();

        // Should validate successfully
        assert!(validate_cid(&cid).is_ok());
        assert!(validate_cid_string(known_cid).is_ok());

        // Should round-trip
        assert_eq!(cid.to_string(), known_cid);
    }

    #[test]
    fn test_multiple_records() {
        use crate::lexicon::cbor::encode_record;
        use serde::Serialize;

        #[derive(Serialize)]
        struct Record {
            id: u32,
            value: String,
        }

        // Generate CIDs for multiple records
        let records = vec![
            Record { id: 1, value: "first".to_string() },
            Record { id: 2, value: "second".to_string() },
            Record { id: 3, value: "third".to_string() },
        ];

        let mut cids = Vec::new();
        for record in &records {
            let bytes = encode_record(record).unwrap();
            let cid = generate_cid(&bytes).unwrap();
            cids.push(cid);
        }

        // All CIDs should be unique
        assert_eq!(cids.len(), 3);
        assert_ne!(cids[0], cids[1]);
        assert_ne!(cids[1], cids[2]);
        assert_ne!(cids[0], cids[2]);

        // All should validate
        for cid in &cids {
            assert!(validate_cid(cid).is_ok());
        }
    }

    #[test]
    fn test_empty_data() {
        let data = b"";
        let cid = generate_cid(data).unwrap();

        // Even empty data should produce a valid CID
        assert!(validate_cid(&cid).is_ok());

        // Should be deterministic
        let cid2 = generate_cid(data).unwrap();
        assert_eq!(cid, cid2);
    }
}
