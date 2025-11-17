//! Post management and rich text parsing
//!
//! This module provides functionality for working with posts, including
//! rich text parsing with facets for links, mentions, and hashtags.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use thiserror::Error;

/// Rich text error types
#[derive(Debug, Error)]
pub enum RichTextError {
    /// Invalid UTF-8 byte indices
    #[error("Invalid UTF-8 byte indices: start={0}, end={1}")]
    InvalidByteIndices(usize, usize),

    /// Invalid facet
    #[error("Invalid facet: {0}")]
    InvalidFacet(String),
}

/// Result type for rich text operations
pub type Result<T> = std::result::Result<T, RichTextError>;

/// Byte range index for facet positions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByteSlice {
    /// Start position in UTF-8 bytes
    pub byte_start: usize,
    /// End position in UTF-8 bytes
    pub byte_end: usize,
}

/// Link feature
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// URI of the link
    pub uri: String,
}

/// Mention feature
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mention {
    /// DID of the mentioned user
    pub did: String,
}

/// Tag (hashtag) feature
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    /// Tag name without the # symbol
    pub tag: String,
}

/// Facet feature types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum FacetFeature {
    /// Link feature
    #[serde(rename = "app.bsky.richtext.facet#link")]
    Link(Link),
    /// Mention feature
    #[serde(rename = "app.bsky.richtext.facet#mention")]
    Mention(Mention),
    /// Tag (hashtag) feature
    #[serde(rename = "app.bsky.richtext.facet#tag")]
    Tag(Tag),
}

/// A facet represents a span of text with special meaning (link, mention, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facet {
    /// Byte range of the facet in the text
    pub index: ByteSlice,
    /// Features associated with this facet
    pub features: Vec<FacetFeature>,
}

/// Rich text with detected facets
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RichText {
    /// The text content
    pub text: String,
    /// Detected facets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<Vec<Facet>>,
}

impl RichText {
    /// Create a new RichText instance
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            facets: None,
        }
    }

    /// Detect facets in the text
    ///
    /// This automatically detects:
    /// - HTTP/HTTPS URLs
    /// - Mentions (@handle)
    /// - Hashtags (#tag)
    pub fn detect_facets(&mut self) {
        let mut facets = Vec::new();

        // Detect links
        facets.extend(detect_links(&self.text));

        // Detect mentions
        facets.extend(detect_mentions(&self.text));

        // Detect hashtags
        facets.extend(detect_tags(&self.text));

        // Sort facets by byte position
        facets.sort_by_key(|f| f.index.byte_start);

        if !facets.is_empty() {
            self.facets = Some(facets);
        }
    }

    /// Get the text content
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the facets
    pub fn facets(&self) -> Option<&[Facet]> {
        self.facets.as_deref()
    }

    /// Extract a substring by byte range
    pub fn substring(&self, byte_start: usize, byte_end: usize) -> Result<&str> {
        self.text
            .get(byte_start..byte_end)
            .ok_or(RichTextError::InvalidByteIndices(byte_start, byte_end))
    }
}

/// Detect links in text
fn detect_links(text: &str) -> Vec<Facet> {
    static LINK_REGEX: OnceLock<Regex> = OnceLock::new();
    let re = LINK_REGEX.get_or_init(|| {
        // Matches:
        // - https://example.com
        // - http://example.com
        // - example.com (with valid TLD)
        Regex::new(
            r"(?:^|\s|\()(https?://[^\s]+|(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,}(?:/[^\s]*)?)"
        )
        .unwrap()
    });

    let mut facets = Vec::new();

    for cap in re.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let mut url = m.as_str().to_string();
            let start = m.start();

            // Strip trailing punctuation
            while url.ends_with(&['.', ',', ';', '!', '?', ')', ']', '}'][..]) {
                // Don't strip ) if there's a matching (
                if url.ends_with(')') && url.contains('(') {
                    break;
                }
                url.pop();
            }

            // Add https:// if no protocol
            let uri = if !url.starts_with("http://") && !url.starts_with("https://") {
                format!("https://{}", url)
            } else {
                url.clone()
            };

            // Get byte positions
            let byte_start = text[..start].len() + (m.as_str().len() - url.len());
            let byte_end = byte_start + url.len();

            facets.push(Facet {
                index: ByteSlice {
                    byte_start,
                    byte_end,
                },
                features: vec![FacetFeature::Link(Link { uri })],
            });
        }
    }

    facets
}

/// Detect mentions in text
fn detect_mentions(text: &str) -> Vec<Facet> {
    static MENTION_REGEX: OnceLock<Regex> = OnceLock::new();
    let re = MENTION_REGEX.get_or_init(|| {
        // Matches @handle.domain or @handle
        Regex::new(r"(?:^|\s|\()@([a-zA-Z0-9][a-zA-Z0-9.-]*[a-zA-Z0-9])").unwrap()
    });

    let mut facets = Vec::new();

    for cap in re.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            let full_match = m.as_str();
            let mention_text = full_match.trim_start_matches(&[' ', '('][..]);
            let handle = mention_text.trim_start_matches('@');

            // Calculate byte position
            let match_start = m.start();
            let prefix_len = full_match.len() - mention_text.len();
            let byte_start = text[..match_start].len() + prefix_len;
            let byte_end = byte_start + mention_text.len();

            // For now, we don't resolve the handle to a DID
            // This would typically require an API call
            // The DID would be filled in by the caller
            facets.push(Facet {
                index: ByteSlice {
                    byte_start,
                    byte_end,
                },
                features: vec![FacetFeature::Mention(Mention {
                    did: format!("did:placeholder:{}", handle),
                })],
            });
        }
    }

    facets
}

/// Detect hashtags in text
fn detect_tags(text: &str) -> Vec<Facet> {
    static TAG_REGEX: OnceLock<Regex> = OnceLock::new();
    let re = TAG_REGEX.get_or_init(|| {
        // Matches #tag
        Regex::new(r"(?:^|\s|\()#([a-zA-Z][a-zA-Z0-9_]*)").unwrap()
    });

    let mut facets = Vec::new();

    for cap in re.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            let full_match = m.as_str();
            let tag_text = full_match.trim_start_matches(&[' ', '('][..]);
            let tag_name = tag_text.trim_start_matches('#');

            // Calculate byte position
            let match_start = m.start();
            let prefix_len = full_match.len() - tag_text.len();
            let byte_start = text[..match_start].len() + prefix_len;
            let byte_end = byte_start + tag_text.len();

            facets.push(Facet {
                index: ByteSlice {
                    byte_start,
                    byte_end,
                },
                features: vec![FacetFeature::Tag(Tag {
                    tag: tag_name.to_string(),
                })],
            });
        }
    }

    facets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rich_text_basic() {
        let rt = RichText::new("Hello world");
        assert_eq!(rt.text(), "Hello world");
        assert!(rt.facets().is_none());
    }

    #[test]
    fn test_detect_links_https() {
        let mut rt = RichText::new("Check out https://example.com for more info");
        rt.detect_facets();

        let facets = rt.facets().unwrap();
        assert_eq!(facets.len(), 1);

        assert_eq!(facets[0].index.byte_start, 10);
        assert_eq!(facets[0].index.byte_end, 29);

        match &facets[0].features[0] {
            FacetFeature::Link(link) => {
                assert_eq!(link.uri, "https://example.com");
            }
            _ => panic!("Expected link feature"),
        }
    }

    #[test]
    fn test_detect_links_bare_domain() {
        let mut rt = RichText::new("Visit example.com today");
        rt.detect_facets();

        let facets = rt.facets().unwrap();
        assert_eq!(facets.len(), 1);

        match &facets[0].features[0] {
            FacetFeature::Link(link) => {
                assert_eq!(link.uri, "https://example.com");
            }
            _ => panic!("Expected link feature"),
        }
    }

    #[test]
    fn test_detect_mentions() {
        let mut rt = RichText::new("Hey @alice.bsky.social how are you?");
        rt.detect_facets();

        let facets = rt.facets().unwrap();
        let mention_facet = facets
            .iter()
            .find(|f| matches!(f.features[0], FacetFeature::Mention(_)))
            .unwrap();

        assert_eq!(
            rt.substring(
                mention_facet.index.byte_start,
                mention_facet.index.byte_end
            )
            .unwrap(),
            "@alice.bsky.social"
        );
    }

    #[test]
    fn test_detect_hashtags() {
        let mut rt = RichText::new("This is #awesome and #cool");
        rt.detect_facets();

        let facets = rt.facets().unwrap();
        let tag_facets: Vec<_> = facets
            .iter()
            .filter(|f| matches!(f.features[0], FacetFeature::Tag(_)))
            .collect();

        assert_eq!(tag_facets.len(), 2);

        match &tag_facets[0].features[0] {
            FacetFeature::Tag(tag) => {
                assert_eq!(tag.tag, "awesome");
            }
            _ => panic!("Expected tag feature"),
        }

        match &tag_facets[1].features[0] {
            FacetFeature::Tag(tag) => {
                assert_eq!(tag.tag, "cool");
            }
            _ => panic!("Expected tag feature"),
        }
    }

    #[test]
    fn test_mixed_facets() {
        let mut rt =
            RichText::new("Hey @alice check https://example.com for #news about the #update!");
        rt.detect_facets();

        let facets = rt.facets().unwrap();
        assert!(facets.len() >= 4); // mention, link, 2 tags

        // Verify facets are sorted by position
        for i in 1..facets.len() {
            assert!(facets[i].index.byte_start >= facets[i - 1].index.byte_start);
        }
    }

    #[test]
    fn test_link_with_trailing_punctuation() {
        let mut rt = RichText::new("Visit https://example.com. It's great!");
        rt.detect_facets();

        let facets = rt.facets().unwrap();
        let link_facet = facets
            .iter()
            .find(|f| matches!(f.features[0], FacetFeature::Link(_)))
            .unwrap();

        match &link_facet.features[0] {
            FacetFeature::Link(link) => {
                assert_eq!(link.uri, "https://example.com");
                // Should not include the trailing period
            }
            _ => panic!("Expected link feature"),
        }
    }

    #[test]
    fn test_facet_serialization() {
        let facet = Facet {
            index: ByteSlice {
                byte_start: 0,
                byte_end: 10,
            },
            features: vec![FacetFeature::Link(Link {
                uri: "https://example.com".to_string(),
            })],
        };

        let json = serde_json::to_string(&facet).unwrap();
        let deserialized: Facet = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, facet);
    }

    #[test]
    fn test_mention_feature_serialization() {
        let feature = FacetFeature::Mention(Mention {
            did: "did:plc:test123".to_string(),
        });

        let json = serde_json::to_string(&feature).unwrap();
        assert!(json.contains("app.bsky.richtext.facet#mention"));
        assert!(json.contains("did:plc:test123"));
    }

    #[test]
    fn test_tag_feature_serialization() {
        let feature = FacetFeature::Tag(Tag {
            tag: "awesome".to_string(),
        });

        let json = serde_json::to_string(&feature).unwrap();
        assert!(json.contains("app.bsky.richtext.facet#tag"));
        assert!(json.contains("awesome"));
    }
}
