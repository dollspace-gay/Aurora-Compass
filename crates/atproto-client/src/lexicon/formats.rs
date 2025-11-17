//! String format types for Lexicon schemas
//!
//! These formats provide additional validation and semantic meaning for string values.

use serde::{Deserialize, Serialize};

/// String format types defined by AT Protocol
///
/// These formats provide semantic meaning and validation rules for string fields.
/// Reference: <https://atproto.com/specs/lexicon#string-formats>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StringFormat {
    /// AT Protocol identifier (DID or handle)
    ///
    /// Examples: `did:plc:abc123`, `alice.bsky.social`
    AtIdentifier,

    /// AT Protocol URI
    ///
    /// Format: `at://authority/collection/rkey`
    AtUri,

    /// Content Identifier (CID)
    ///
    /// Base32-encoded CIDv1 using SHA-256
    Cid,

    /// ISO 8601/RFC 3339 datetime with timezone
    ///
    /// Format: `YYYY-MM-DDTHH:MM:SS.sssZ` or `YYYY-MM-DDTHH:MM:SS.sss±HH:MM`
    ///
    /// Must include timezone and use capital 'T' separator
    Datetime,

    /// Decentralized Identifier (DID)
    ///
    /// Format: `did:method:identifier`
    ///
    /// Examples: `did:plc:abc123`, `did:web:example.com`
    Did,

    /// Domain name handle
    ///
    /// Valid domain name, normalized to lowercase
    ///
    /// Examples: `alice.bsky.social`, `bob.com`
    Handle,

    /// Namespaced Identifier (NSID)
    ///
    /// Reverse-DNS format: `com.example.recordType`
    Nsid,

    /// Timestamp Identifier (TID)
    ///
    /// 13-character base32-encoded timestamp
    Tid,

    /// Record key (rkey)
    ///
    /// Valid record key for repository records
    RecordKey,

    /// Generic URI
    ///
    /// Valid URI according to RFC 3986
    Uri,

    /// ISO 639 language code
    ///
    /// 2 or 3 letter language code, optionally with region
    ///
    /// Examples: `en`, `en-US`, `pt-BR`
    Language,
}

impl StringFormat {
    /// Get the string representation of the format
    pub fn as_str(&self) -> &'static str {
        match self {
            StringFormat::AtIdentifier => "at-identifier",
            StringFormat::AtUri => "at-uri",
            StringFormat::Cid => "cid",
            StringFormat::Datetime => "datetime",
            StringFormat::Did => "did",
            StringFormat::Handle => "handle",
            StringFormat::Nsid => "nsid",
            StringFormat::Tid => "tid",
            StringFormat::RecordKey => "record-key",
            StringFormat::Uri => "uri",
            StringFormat::Language => "language",
        }
    }

    /// Parse a string format from its string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "at-identifier" => Some(StringFormat::AtIdentifier),
            "at-uri" => Some(StringFormat::AtUri),
            "cid" => Some(StringFormat::Cid),
            "datetime" => Some(StringFormat::Datetime),
            "did" => Some(StringFormat::Did),
            "handle" => Some(StringFormat::Handle),
            "nsid" => Some(StringFormat::Nsid),
            "tid" => Some(StringFormat::Tid),
            "record-key" => Some(StringFormat::RecordKey),
            "uri" => Some(StringFormat::Uri),
            "language" => Some(StringFormat::Language),
            _ => None,
        }
    }
}

impl std::fmt::Display for StringFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_format_as_str() {
        assert_eq!(StringFormat::AtUri.as_str(), "at-uri");
        assert_eq!(StringFormat::Did.as_str(), "did");
        assert_eq!(StringFormat::Handle.as_str(), "handle");
        assert_eq!(StringFormat::Datetime.as_str(), "datetime");
    }

    #[test]
    fn test_string_format_from_str() {
        assert_eq!(
            StringFormat::from_str("at-uri"),
            Some(StringFormat::AtUri)
        );
        assert_eq!(StringFormat::from_str("did"), Some(StringFormat::Did));
        assert_eq!(
            StringFormat::from_str("handle"),
            Some(StringFormat::Handle)
        );
        assert_eq!(StringFormat::from_str("invalid"), None);
    }

    #[test]
    fn test_string_format_display() {
        assert_eq!(format!("{}", StringFormat::AtUri), "at-uri");
        assert_eq!(format!("{}", StringFormat::Datetime), "datetime");
    }

    #[test]
    fn test_string_format_serde() {
        let format = StringFormat::AtUri;
        let json = serde_json::to_string(&format).unwrap();
        assert_eq!(json, "\"at-uri\"");

        let deserialized: StringFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, format);
    }

    #[test]
    fn test_all_formats_round_trip() {
        let formats = vec![
            StringFormat::AtIdentifier,
            StringFormat::AtUri,
            StringFormat::Cid,
            StringFormat::Datetime,
            StringFormat::Did,
            StringFormat::Handle,
            StringFormat::Nsid,
            StringFormat::Tid,
            StringFormat::RecordKey,
            StringFormat::Uri,
            StringFormat::Language,
        ];

        for format in formats {
            let s = format.as_str();
            assert_eq!(StringFormat::from_str(s), Some(format));

            let json = serde_json::to_string(&format).unwrap();
            let deserialized: StringFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, format);
        }
    }
}
