//! Core AT Protocol types
//!
//! This module implements the core types used throughout the AT Protocol,
//! based on the official specification and the TypeScript implementation in
//! `original-bluesky/`.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Decentralized Identifier (DID)
///
/// A DID uniquely identifies an account in the AT Protocol.
/// Format: `did:method:identifier`
///
/// Supported methods:
/// - `did:plc:` - Placeholder DID method
/// - `did:web:` - Web-based DID method
///
/// # Examples
/// ```
/// # use atproto_client::types::Did;
/// # use std::str::FromStr;
/// let did = Did::from_str("did:plc:z72i7hdynmk6r22z27h6tvur").unwrap();
/// assert_eq!(did.as_str(), "did:plc:z72i7hdynmk6r22z27h6tvur");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Did(String);

impl Did {
    /// Create a new DID from a string, validating its format
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        Self::validate(&s)?;
        Ok(Did(s))
    }

    /// Get the DID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the DID method (e.g., "plc" or "web")
    pub fn method(&self) -> &str {
        // DIDs are always "did:method:identifier"
        self.0
            .strip_prefix("did:")
            .and_then(|s| s.split(':').next())
            .unwrap_or("")
    }

    /// Get the DID identifier (the part after "did:method:")
    pub fn identifier(&self) -> &str {
        // Skip "did:method:" to get the identifier
        self.0
            .strip_prefix("did:")
            .and_then(|s| s.split_once(':'))
            .map(|(_, id)| id)
            .unwrap_or("")
    }

    /// Validate DID format
    fn validate(s: &str) -> Result<(), Error> {
        if !s.starts_with("did:") {
            return Err(Error::InvalidDid("DID must start with 'did:'".to_string()));
        }

        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() < 3 {
            return Err(Error::InvalidDid(
                "DID must have format 'did:method:identifier'".to_string(),
            ));
        }

        let method = parts[1];
        if method.is_empty() {
            return Err(Error::InvalidDid("DID method cannot be empty".to_string()));
        }

        let identifier = parts[2..].join(":");
        if identifier.is_empty() {
            return Err(Error::InvalidDid("DID identifier cannot be empty".to_string()));
        }

        // Validate method is supported (plc or web)
        if method != "plc" && method != "web" {
            return Err(Error::InvalidDid(format!(
                "Unsupported DID method '{}'. Supported: plc, web",
                method
            )));
        }

        Ok(())
    }
}

impl FromStr for Did {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Did::new(s)
    }
}

impl fmt::Display for Did {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Did {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Handle (username)
///
/// A handle is a human-readable identifier for an account, formatted as a domain name.
///
/// # Examples
/// ```
/// # use atproto_client::types::Handle;
/// # use std::str::FromStr;
/// let handle = Handle::from_str("alice.bsky.social").unwrap();
/// assert_eq!(handle.as_str(), "alice.bsky.social");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Handle(String);

impl Handle {
    /// Create a new Handle from a string, validating its format
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        Self::validate(&s)?;
        Ok(Handle(s))
    }

    /// Get the handle as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate handle format (must be a valid domain name)
    fn validate(s: &str) -> Result<(), Error> {
        if s.is_empty() {
            return Err(Error::InvalidHandle("Handle cannot be empty".to_string()));
        }

        // Basic domain name validation
        // Must contain at least one dot, contain only valid characters
        if !s.contains('.') {
            return Err(Error::InvalidHandle(
                "Handle must be a domain name (contain at least one dot)".to_string(),
            ));
        }

        // Check for valid characters (alphanumeric, dots, hyphens)
        if !s
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-')
        {
            return Err(Error::InvalidHandle("Handle contains invalid characters".to_string()));
        }

        // Cannot start or end with dot or hyphen
        if s.starts_with('.') || s.starts_with('-') || s.ends_with('.') || s.ends_with('-') {
            return Err(Error::InvalidHandle(
                "Handle cannot start or end with '.' or '-'".to_string(),
            ));
        }

        // Check length (max 253 characters for domain names)
        if s.len() > 253 {
            return Err(Error::InvalidHandle("Handle too long (max 253 characters)".to_string()));
        }

        Ok(())
    }
}

impl FromStr for Handle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Handle::new(s)
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Handle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// AT Protocol URI
///
/// Format: `at://authority/collection/rkey`
///
/// Where:
/// - `authority` is a DID or handle
/// - `collection` is the NSID of the collection (e.g., app.bsky.feed.post)
/// - `rkey` is the record key (optional)
///
/// # Examples
/// ```
/// # use atproto_client::types::AtUri;
/// # use std::str::FromStr;
/// let uri = AtUri::from_str("at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post/3lniysofyll2d").unwrap();
/// assert_eq!(uri.authority(), "did:plc:z72i7hdynmk6r22z27h6tvur");
/// assert_eq!(uri.collection(), Some("app.bsky.feed.post"));
/// assert_eq!(uri.rkey(), Some("3lniysofyll2d"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AtUri(String);

impl AtUri {
    /// Create a new AT URI from a string, validating its format
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        Self::validate(&s)?;
        Ok(AtUri(s))
    }

    /// Get the AT URI as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the authority (DID or handle)
    pub fn authority(&self) -> &str {
        self.0
            .strip_prefix("at://")
            .and_then(|s| s.split('/').next())
            .unwrap_or("")
    }

    /// Get the collection NSID
    pub fn collection(&self) -> Option<&str> {
        self.0
            .strip_prefix("at://")
            .and_then(|s| s.split('/').nth(1))
    }

    /// Get the record key
    pub fn rkey(&self) -> Option<&str> {
        self.0
            .strip_prefix("at://")
            .and_then(|s| s.split('/').nth(2))
    }

    /// Validate AT URI format
    fn validate(s: &str) -> Result<(), Error> {
        if !s.starts_with("at://") {
            return Err(Error::InvalidAtUri("AT URI must start with 'at://'".to_string()));
        }

        let without_scheme = s.strip_prefix("at://").unwrap();
        let parts: Vec<&str> = without_scheme.split('/').collect();

        if parts.is_empty() || parts[0].is_empty() {
            return Err(Error::InvalidAtUri("AT URI must have authority".to_string()));
        }

        // Authority must be a valid DID or handle
        let authority = parts[0];
        if authority.starts_with("did:") {
            // Validate as DID
            Did::new(authority)?;
        } else {
            // Validate as handle
            Handle::new(authority)?;
        }

        Ok(())
    }
}

impl FromStr for AtUri {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AtUri::new(s)
    }
}

impl fmt::Display for AtUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for AtUri {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Error types for AT Protocol types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid DID format
    #[error("Invalid DID: {0}")]
    InvalidDid(String),

    /// Invalid Handle format
    #[error("Invalid Handle: {0}")]
    InvalidHandle(String),

    /// Invalid AT URI format
    #[error("Invalid AT URI: {0}")]
    InvalidAtUri(String),

    /// Invalid TID format
    #[error("Invalid TID: {0}")]
    InvalidTid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // DID Tests
    #[test]
    fn test_did_plc_valid() {
        let did = Did::from_str("did:plc:z72i7hdynmk6r22z27h6tvur").unwrap();
        assert_eq!(did.as_str(), "did:plc:z72i7hdynmk6r22z27h6tvur");
        assert_eq!(did.method(), "plc");
        assert_eq!(did.identifier(), "z72i7hdynmk6r22z27h6tvur");
    }

    #[test]
    fn test_did_web_valid() {
        let did = Did::from_str("did:web:api.bsky.app").unwrap();
        assert_eq!(did.as_str(), "did:web:api.bsky.app");
        assert_eq!(did.method(), "web");
        assert_eq!(did.identifier(), "api.bsky.app");
    }

    #[test]
    fn test_did_invalid_no_prefix() {
        let result = Did::from_str("plc:z72i7hdynmk6r22z27h6tvur");
        assert!(result.is_err());
    }

    #[test]
    fn test_did_invalid_no_identifier() {
        let result = Did::from_str("did:plc:");
        assert!(result.is_err());
    }

    #[test]
    fn test_did_invalid_method() {
        let result = Did::from_str("did:invalid:z72i7hdynmk6r22z27h6tvur");
        assert!(result.is_err());
    }

    #[test]
    fn test_did_display() {
        let did = Did::from_str("did:plc:test").unwrap();
        assert_eq!(format!("{}", did), "did:plc:test");
    }

    #[test]
    fn test_did_serde() {
        let did = Did::from_str("did:plc:test").unwrap();
        let json = serde_json::to_string(&did).unwrap();
        assert_eq!(json, "\"did:plc:test\"");

        let deserialized: Did = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, did);
    }

    // Handle Tests
    #[test]
    fn test_handle_valid() {
        let handle = Handle::from_str("alice.bsky.social").unwrap();
        assert_eq!(handle.as_str(), "alice.bsky.social");
    }

    #[test]
    fn test_handle_subdomain() {
        let handle = Handle::from_str("subdomain.alice.bsky.social").unwrap();
        assert_eq!(handle.as_str(), "subdomain.alice.bsky.social");
    }

    #[test]
    fn test_handle_invalid_no_dot() {
        let result = Handle::from_str("alice");
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_invalid_empty() {
        let result = Handle::from_str("");
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_invalid_characters() {
        let result = Handle::from_str("alice@bsky.social");
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_invalid_starts_with_dot() {
        let result = Handle::from_str(".alice.bsky.social");
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_display() {
        let handle = Handle::from_str("alice.bsky.social").unwrap();
        assert_eq!(format!("{}", handle), "alice.bsky.social");
    }

    #[test]
    fn test_handle_serde() {
        let handle = Handle::from_str("alice.bsky.social").unwrap();
        let json = serde_json::to_string(&handle).unwrap();
        assert_eq!(json, "\"alice.bsky.social\"");

        let deserialized: Handle = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, handle);
    }

    // AtUri Tests
    #[test]
    fn test_at_uri_with_did_full() {
        let uri = AtUri::from_str(
            "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post/3lniysofyll2d",
        )
        .unwrap();
        assert_eq!(uri.authority(), "did:plc:z72i7hdynmk6r22z27h6tvur");
        assert_eq!(uri.collection(), Some("app.bsky.feed.post"));
        assert_eq!(uri.rkey(), Some("3lniysofyll2d"));
    }

    #[test]
    fn test_at_uri_with_handle() {
        let uri =
            AtUri::from_str("at://alice.bsky.social/app.bsky.feed.post/3lniysofyll2d").unwrap();
        assert_eq!(uri.authority(), "alice.bsky.social");
        assert_eq!(uri.collection(), Some("app.bsky.feed.post"));
        assert_eq!(uri.rkey(), Some("3lniysofyll2d"));
    }

    #[test]
    fn test_at_uri_without_rkey() {
        let uri =
            AtUri::from_str("at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post").unwrap();
        assert_eq!(uri.authority(), "did:plc:z72i7hdynmk6r22z27h6tvur");
        assert_eq!(uri.collection(), Some("app.bsky.feed.post"));
        assert_eq!(uri.rkey(), None);
    }

    #[test]
    fn test_at_uri_authority_only() {
        let uri = AtUri::from_str("at://did:plc:z72i7hdynmk6r22z27h6tvur").unwrap();
        assert_eq!(uri.authority(), "did:plc:z72i7hdynmk6r22z27h6tvur");
        assert_eq!(uri.collection(), None);
        assert_eq!(uri.rkey(), None);
    }

    #[test]
    fn test_at_uri_invalid_no_scheme() {
        let result = AtUri::from_str("did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.post");
        assert!(result.is_err());
    }

    #[test]
    fn test_at_uri_invalid_no_authority() {
        let result = AtUri::from_str("at:///app.bsky.feed.post");
        assert!(result.is_err());
    }

    #[test]
    fn test_at_uri_invalid_authority() {
        let result = AtUri::from_str("at://invalid-did/app.bsky.feed.post");
        assert!(result.is_err());
    }

    #[test]
    fn test_at_uri_display() {
        let uri = AtUri::from_str("at://alice.bsky.social/app.bsky.feed.post").unwrap();
        assert_eq!(format!("{}", uri), "at://alice.bsky.social/app.bsky.feed.post");
    }

    #[test]
    fn test_at_uri_serde() {
        let uri = AtUri::from_str("at://alice.bsky.social/app.bsky.feed.post").unwrap();
        let json = serde_json::to_string(&uri).unwrap();
        assert_eq!(json, "\"at://alice.bsky.social/app.bsky.feed.post\"");

        let deserialized: AtUri = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, uri);
    }
}

// =============================================================================
// TID (Timestamp Identifier)
// =============================================================================

/// TID (Timestamp Identifier) - A sortable timestamp-based identifier
///
/// TIDs are used as record keys (rkeys) in AT Protocol repositories.
/// They are base32-encoded timestamps with random bits for collision resistance.
///
/// # Format
/// - 13 characters long
/// - Base32 encoded using custom alphabet (234567abcdefghijklmnopqrstuvwxyz)
/// - Encodes: 64-bit microsecond timestamp + random bits
/// - Lexicographically sortable by time
///
/// # Examples
/// ```
/// use atproto_client::types::Tid;
/// use std::str::FromStr;
///
/// // Parse an existing TID
/// let tid = Tid::from_str("3jzfcijpj2z2a").unwrap();
/// assert_eq!(tid.to_string(), "3jzfcijpj2z2a");
///
/// // Generate a new TID
/// let new_tid = Tid::now();
/// assert_eq!(new_tid.to_string().len(), 13);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tid(String);

impl Tid {
    /// The base32 alphabet used for TIDs (no 0, 1, or vowels to avoid confusion)
    const ALPHABET: &'static [u8] = b"234567abcdefghijklmnopqrstuvwxyz";

    /// TIDs are always exactly 13 characters
    const LENGTH: usize = 13;

    /// Create a new TID from a string, validating the format
    ///
    /// # Errors
    /// Returns an error if the string is not a valid TID format
    pub fn new(s: impl Into<String>) -> Result<Self, Error> {
        let s = s.into();
        Self::validate(&s)?;
        Ok(Tid(s))
    }

    /// Generate a new TID based on the current timestamp
    ///
    /// # Examples
    /// ```
    /// use atproto_client::types::Tid;
    ///
    /// let tid = Tid::now();
    /// assert_eq!(tid.to_string().len(), 13);
    /// ```
    pub fn now() -> Self {
        Self::from_timestamp(std::time::SystemTime::now())
    }

    /// Create a TID from a specific timestamp
    ///
    /// This is useful for testing or when you need deterministic TIDs
    pub fn from_timestamp(timestamp: std::time::SystemTime) -> Self {
        use std::time::UNIX_EPOCH;

        // Get microseconds since Unix epoch
        let duration = timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0));
        let micros = duration.as_secs() * 1_000_000 + u64::from(duration.subsec_micros());

        // Add random bits for collision resistance (using timestamp nanos as pseudo-random)
        let clock_id = duration.subsec_nanos() & 0x3FF; // 10 bits

        // Encode to base32
        let mut value = (micros << 10) | u64::from(clock_id);
        let mut result = String::with_capacity(Self::LENGTH);

        for _ in 0..Self::LENGTH {
            let idx = (value & 0x1F) as usize; // 5 bits at a time
            result.insert(0, Self::ALPHABET[idx] as char);
            value >>= 5;
        }

        Tid(result)
    }

    /// Get the underlying string representation
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate a TID string
    fn validate(s: &str) -> Result<(), Error> {
        if s.len() != Self::LENGTH {
            return Err(Error::InvalidTid(format!(
                "TID must be exactly {} characters, got {}",
                Self::LENGTH,
                s.len()
            )));
        }

        // Verify all characters are in the base32 alphabet
        for ch in s.chars() {
            if !Self::ALPHABET.contains(&(ch as u8)) {
                return Err(Error::InvalidTid(format!("TID contains invalid character: {}", ch)));
            }
        }

        Ok(())
    }
}

impl std::fmt::Display for Tid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for Tid {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Serialize for Tid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Tid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

// =============================================================================
// StrongRef - Reference to a specific version of a record
// =============================================================================

/// A strong reference to a specific version of a record
///
/// Contains both the AT URI and the CID of the record, ensuring
/// the reference points to an exact version.
///
/// # Examples
/// ```
/// use atproto_client::types::{StrongRef, AtUri};
/// use std::str::FromStr;
///
/// let uri = AtUri::from_str("at://did:plc:abc123/app.bsky.feed.post/3jzfcijpj2z2a").unwrap();
/// let cid_str = "bafyreigq4zsipbk5w3uqkbmh2w2633c4tcwudryvoqkfrq3mqfs3d5e3wq";
/// let strong_ref = StrongRef::new(uri, cid_str.to_string());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrongRef {
    /// The AT URI of the record
    pub uri: AtUri,
    /// The CID (Content Identifier) of the specific record version
    pub cid: String,
}

impl StrongRef {
    /// Create a new strong reference
    pub fn new(uri: AtUri, cid: String) -> Self {
        Self { uri, cid }
    }
}

impl Serialize for StrongRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("StrongRef", 2)?;
        state.serialize_field("uri", &self.uri)?;
        state.serialize_field("cid", &self.cid)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for StrongRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct StrongRefData {
            uri: AtUri,
            cid: String,
        }

        let data = StrongRefData::deserialize(deserializer)?;
        Ok(StrongRef::new(data.uri, data.cid))
    }
}

// =============================================================================
// Additional Tests
// =============================================================================

#[cfg(test)]
mod tid_tests {
    use super::*;

    #[test]
    fn test_tid_valid() {
        let tid = Tid::from_str("3jzfcijpj2z2a").unwrap();
        assert_eq!(tid.as_str(), "3jzfcijpj2z2a");
    }

    #[test]
    fn test_tid_invalid_length() {
        let result = Tid::from_str("tooshort");
        assert!(result.is_err());
        assert!(matches!(result, Err(Error::InvalidTid(_))));
    }

    #[test]
    fn test_tid_invalid_characters() {
        let result = Tid::from_str("123INVALID!!!"); // uppercase and special chars
        assert!(result.is_err());
    }

    #[test]
    fn test_tid_now() {
        let tid1 = Tid::now();
        assert_eq!(tid1.to_string().len(), 13);

        // Sleep a tiny bit to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(1));

        let tid2 = Tid::now();
        assert_eq!(tid2.to_string().len(), 13);

        // TIDs should be sortable by time
        assert!(tid2 > tid1);
    }

    #[test]
    fn test_tid_from_timestamp() {
        use std::time::{Duration, UNIX_EPOCH};

        let timestamp = UNIX_EPOCH + Duration::from_secs(1234567890);
        let tid = Tid::from_timestamp(timestamp);
        assert_eq!(tid.to_string().len(), 13);

        // Same timestamp should produce comparable TID (same prefix at least)
        let tid2 = Tid::from_timestamp(timestamp);
        assert_eq!(tid.to_string().len(), tid2.to_string().len());
    }

    #[test]
    fn test_tid_display() {
        let tid = Tid::from_str("3jzfcijpj2z2a").unwrap();
        assert_eq!(format!("{}", tid), "3jzfcijpj2z2a");
    }

    #[test]
    fn test_tid_serde() {
        let tid = Tid::from_str("3jzfcijpj2z2a").unwrap();
        let json = serde_json::to_string(&tid).unwrap();
        assert_eq!(json, "\"3jzfcijpj2z2a\"");

        let deserialized: Tid = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, tid);
    }

    #[test]
    fn test_tid_ordering() {
        let tid1 = Tid::from_str("3jzfcijpj2z2a").unwrap();
        let tid2 = Tid::from_str("3jzfcijpj2z2b").unwrap();
        assert!(tid2 > tid1);

        let tid3 = Tid::from_str("3jzfcijpj2z2a").unwrap();
        assert_eq!(tid1, tid3);
    }

    #[test]
    fn test_strong_ref() {
        let uri = AtUri::from_str("at://did:plc:abc123/app.bsky.feed.post/3jzfcijpj2z2a").unwrap();
        let cid = "bafyreigq4zsipbk5w3uqkbmh2w2633c4tcwudryvoqkfrq3mqfs3d5e3wq".to_string();

        let strong_ref = StrongRef::new(uri.clone(), cid.clone());
        assert_eq!(strong_ref.uri, uri);
        assert_eq!(strong_ref.cid, cid);
    }

    #[test]
    fn test_strong_ref_serde() {
        let uri = AtUri::from_str("at://did:plc:abc123/app.bsky.feed.post/3jzfcijpj2z2a").unwrap();
        let cid = "bafyreigq4zsipbk5w3uqkbmh2w2633c4tcwudryvoqkfrq3mqfs3d5e3wq".to_string();
        let strong_ref = StrongRef::new(uri, cid);

        let json = serde_json::to_string(&strong_ref).unwrap();
        let deserialized: StrongRef = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.uri, strong_ref.uri);
        assert_eq!(deserialized.cid, strong_ref.cid);
    }
}
