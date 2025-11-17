//! JSON parsing for Lexicon schemas

use super::schema::LexiconDoc;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during Lexicon parsing
#[derive(Debug, Error)]
pub enum LexiconParseError {
    /// Invalid JSON syntax
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// IO error reading file
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid lexicon version
    #[error("Invalid lexicon version: expected 1, got {0}")]
    InvalidVersion(u32),

    /// Invalid NSID format
    #[error("Invalid NSID format: {0}")]
    InvalidNsid(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Result type for Lexicon parsing operations
pub type Result<T> = std::result::Result<T, LexiconParseError>;

impl LexiconDoc {
    /// Parse a Lexicon document from a JSON string
    ///
    /// # Examples
    ///
    /// ```
    /// use atproto_client::lexicon::LexiconDoc;
    ///
    /// let json = r#"{
    ///   "lexicon": 1,
    ///   "id": "com.example.test",
    ///   "defs": {
    ///     "main": {
    ///       "type": "token"
    ///     }
    ///   }
    /// }"#;
    ///
    /// let doc = LexiconDoc::from_json(json).unwrap();
    /// assert_eq!(doc.id, "com.example.test");
    /// ```
    pub fn from_json(json: &str) -> Result<Self> {
        let doc: LexiconDoc = serde_json::from_str(json)?;

        // Validate lexicon version
        if doc.lexicon != 1 {
            return Err(LexiconParseError::InvalidVersion(doc.lexicon));
        }

        // Validate NSID format (basic validation)
        if !is_valid_nsid(&doc.id) {
            return Err(LexiconParseError::InvalidNsid(doc.id.clone()));
        }

        Ok(doc)
    }

    /// Parse a Lexicon document from a JSON file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use atproto_client::lexicon::LexiconDoc;
    ///
    /// let doc = LexiconDoc::from_file("lexicons/app.bsky.feed.post.json").unwrap();
    /// println!("Loaded lexicon: {}", doc.id);
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        Self::from_json(&json)
    }
}

/// Validate NSID format
///
/// NSID format: reverse-DNS style with camelCase allowed in last segment
/// Examples: "com.example.recordType", "app.bsky.feed.post"
pub(crate) fn is_valid_nsid(nsid: &str) -> bool {
    // Basic validation: must have at least one dot and valid characters
    if !nsid.contains('.') {
        return false;
    }

    // Must not start or end with dot
    if nsid.starts_with('.') || nsid.ends_with('.') {
        return false;
    }

    let segments: Vec<&str> = nsid.split('.').collect();
    if segments.is_empty() {
        return false;
    }

    // Validate each segment
    for (i, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            return false;
        }

        // Must start with a letter
        if !segment.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
            return false;
        }

        // Last segment can use camelCase, others must be lowercase
        let is_last = i == segments.len() - 1;
        for ch in segment.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' {
                return false;
            }
            // Non-last segments must be lowercase
            if !is_last && ch.is_ascii_uppercase() {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_nsid() {
        assert!(is_valid_nsid("com.example.test"));
        assert!(is_valid_nsid("app.bsky.feed.post"));
        assert!(is_valid_nsid("com.atproto.repo.strongRef"));

        assert!(!is_valid_nsid("invalid"));
        assert!(!is_valid_nsid("Invalid.Example"));
        assert!(!is_valid_nsid(".com.example"));
        assert!(!is_valid_nsid("com.example."));
        assert!(!is_valid_nsid("com..example"));
    }

    #[test]
    fn test_from_json_simple() {
        let json = r#"{
            "lexicon": 1,
            "id": "com.example.test",
            "defs": {
                "main": {
                    "type": "token"
                }
            }
        }"#;

        let result = LexiconDoc::from_json(json);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let doc = result.unwrap();
        assert_eq!(doc.lexicon, 1);
        assert_eq!(doc.id, "com.example.test");
        assert_eq!(doc.defs.len(), 1);
    }

    #[test]
    fn test_from_json_invalid_version() {
        let json = r#"{
            "lexicon": 2,
            "id": "com.example.test",
            "defs": {}
        }"#;

        let result = LexiconDoc::from_json(json);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LexiconParseError::InvalidVersion(2)
        ));
    }

    #[test]
    fn test_from_json_invalid_nsid() {
        let json = r#"{
            "lexicon": 1,
            "id": "Invalid.NSID",
            "defs": {}
        }"#;

        let result = LexiconDoc::from_json(json);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LexiconParseError::InvalidNsid(_)
        ));
    }

    #[test]
    fn test_from_json_malformed() {
        let json = r#"{ invalid json }"#;

        let result = LexiconDoc::from_json(json);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LexiconParseError::InvalidJson(_)
        ));
    }

    #[test]
    fn test_from_json_complex() {
        let json = r#"{
            "lexicon": 1,
            "id": "app.bsky.feed.post",
            "description": "A social media post",
            "defs": {
                "main": {
                    "type": "record",
                    "key": "tid",
                    "record": {
                        "type": "object",
                        "properties": {
                            "text": {
                                "type": "string",
                                "maxGraphemes": 300
                            }
                        },
                        "required": ["text"]
                    }
                }
            }
        }"#;

        let result = LexiconDoc::from_json(json);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let doc = result.unwrap();
        assert_eq!(doc.id, "app.bsky.feed.post");
        assert_eq!(doc.description, Some("A social media post".to_string()));
    }
}
