//! Post management and rich text parsing
//!
//! This module provides functionality for working with posts, including
//! rich text parsing with facets for links, mentions, and hashtags, and
//! reply handling for threaded conversations.

use atproto_client::xrpc::{XrpcClient, XrpcRequest};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use thiserror::Error;
use tokio::sync::RwLock;

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
        Self { text: text.into(), facets: None }
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
                index: ByteSlice { byte_start, byte_end },
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
                index: ByteSlice { byte_start, byte_end },
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
                index: ByteSlice { byte_start, byte_end },
                features: vec![FacetFeature::Tag(Tag { tag: tag_name.to_string() })],
            });
        }
    }

    facets
}

/// Strong reference to a post (URI + CID)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrongRef {
    /// URI of the post
    pub uri: String,
    /// CID of the post
    pub cid: String,
}

/// Reply reference containing root and parent posts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplyRef {
    /// Reference to the root post of the thread
    pub root: StrongRef,
    /// Reference to the immediate parent post
    pub parent: StrongRef,
}

impl ReplyRef {
    /// Create a reply reference for replying to a top-level post
    ///
    /// When replying to a top-level post, root and parent are the same
    pub fn to_post(uri: impl Into<String>, cid: impl Into<String>) -> Self {
        let strong_ref = StrongRef { uri: uri.into(), cid: cid.into() };

        Self { root: strong_ref.clone(), parent: strong_ref }
    }

    /// Create a reply reference for replying within a thread
    ///
    /// # Arguments
    ///
    /// * `root_uri` - URI of the root post
    /// * `root_cid` - CID of the root post
    /// * `parent_uri` - URI of the immediate parent post
    /// * `parent_cid` - CID of the immediate parent post
    pub fn in_thread(
        root_uri: impl Into<String>,
        root_cid: impl Into<String>,
        parent_uri: impl Into<String>,
        parent_cid: impl Into<String>,
    ) -> Self {
        Self {
            root: StrongRef { uri: root_uri.into(), cid: root_cid.into() },
            parent: StrongRef { uri: parent_uri.into(), cid: parent_cid.into() },
        }
    }

    /// Check if this is a reply to a top-level post
    pub fn is_top_level_reply(&self) -> bool {
        self.root.uri == self.parent.uri && self.root.cid == self.parent.cid
    }

    /// Get the root post URI
    pub fn root_uri(&self) -> &str {
        &self.root.uri
    }

    /// Get the parent post URI
    pub fn parent_uri(&self) -> &str {
        &self.parent.uri
    }
}

/// Reply error types
#[derive(Debug, Error)]
pub enum ReplyError {
    /// XRPC error
    #[error("XRPC error: {0}")]
    Xrpc(String),

    /// Post not found
    #[error("Post not found: {0}")]
    NotFound(String),

    /// No session
    #[error("No active session")]
    NoSession,

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid URI
    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    /// Invalid CID
    #[error("Invalid CID: {0}")]
    InvalidCid(String),

    /// Reply not allowed
    #[error("Replies not allowed: {0}")]
    NotAllowed(String),
}

/// Result type for reply operations
pub type ReplyResult<T> = std::result::Result<T, ReplyError>;

/// Post record for creating posts with replies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostRecord {
    /// Post text
    pub text: String,
    /// Created at timestamp
    pub created_at: String,
    /// Optional reply reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<ReplyRef>,
    /// Optional facets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<Vec<Facet>>,
    /// Record type
    #[serde(rename = "$type")]
    pub record_type: String,
}

/// Response from creating a post
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatePostResponse {
    /// URI of the created post
    pub uri: String,
    /// CID of the created post
    pub cid: String,
}

/// Reply composer for creating reply posts
///
/// Provides methods for composing and creating reply posts with proper
/// parent and root references.
///
/// # Example
///
/// ```rust,no_run
/// use app_core::posts::{ReplyComposer, ReplyRef, RichText};
/// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create XRPC client (with auth)
///     let config = XrpcClientConfig::new("https://bsky.social");
///     let client = XrpcClient::new(config);
///     let composer = ReplyComposer::new(client);
///
///     // Create a reply reference
///     let reply_ref = ReplyRef::to_post(
///         "at://did:plc:abc123/app.bsky.feed.post/xyz456",
///         "bafytest123",
///     );
///
///     // Compose and create a reply
///     let mut text = RichText::new("This is a reply!");
///     text.detect_facets();
///
///     let (uri, cid) = composer.create_reply(&text, &reply_ref).await?;
///     println!("Created reply: {} ({})", uri, cid);
///
///     Ok(())
/// }
/// ```
pub struct ReplyComposer {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl ReplyComposer {
    /// Create a new reply composer
    pub fn new(client: XrpcClient) -> Self {
        Self { client: Arc::new(RwLock::new(client)) }
    }

    /// Create a reply post
    ///
    /// # Arguments
    ///
    /// * `text` - The reply text with facets
    /// * `reply_ref` - Reference to the parent and root posts
    ///
    /// # Returns
    ///
    /// Tuple of (uri, cid) for the created reply
    ///
    /// # Errors
    ///
    /// - `ReplyError::NoSession` - No active session
    /// - `ReplyError::Xrpc` - XRPC error
    pub async fn create_reply(
        &self,
        text: &RichText,
        reply_ref: &ReplyRef,
    ) -> ReplyResult<(String, String)> {
        if text.text.is_empty() {
            return Err(ReplyError::InvalidUri("Reply text cannot be empty".to_string()));
        }

        let now = Utc::now().to_rfc3339();

        let record = PostRecord {
            text: text.text.clone(),
            created_at: now,
            reply: Some(reply_ref.clone()),
            facets: text.facets.clone(),
            record_type: "app.bsky.feed.post".to_string(),
        };

        let body = serde_json::json!({
            "repo": "self",
            "collection": "app.bsky.feed.post",
            "record": record,
        });

        let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .json_body(&body)
            .map_err(|e| ReplyError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| ReplyError::Xrpc(e.to_string()))?;

        let create_response: CreatePostResponse =
            serde_json::from_value(response.data).map_err(ReplyError::Serialization)?;

        Ok((create_response.uri, create_response.cid))
    }

    /// Create a simple text reply (convenience method)
    ///
    /// This method automatically detects facets in the text
    ///
    /// # Arguments
    ///
    /// * `text` - Plain text for the reply
    /// * `reply_ref` - Reference to the parent and root posts
    ///
    /// # Returns
    ///
    /// Tuple of (uri, cid) for the created reply
    pub async fn reply_with_text(
        &self,
        text: impl Into<String>,
        reply_ref: &ReplyRef,
    ) -> ReplyResult<(String, String)> {
        let mut rich_text = RichText::new(text);
        rich_text.detect_facets();
        self.create_reply(&rich_text, reply_ref).await
    }

    /// Get thread context for displaying reply indicator
    ///
    /// Returns information about the post being replied to
    ///
    /// # Arguments
    ///
    /// * `reply_ref` - Reply reference
    ///
    /// # Returns
    ///
    /// Human-readable description of the reply context
    pub fn get_reply_context(&self, reply_ref: &ReplyRef) -> String {
        if reply_ref.is_top_level_reply() {
            format!("Replying to {}", reply_ref.parent_uri())
        } else {
            format!("Replying to {} (in thread {})", reply_ref.parent_uri(), reply_ref.root_uri())
        }
    }
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
            rt.substring(mention_facet.index.byte_start, mention_facet.index.byte_end)
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
            index: ByteSlice { byte_start: 0, byte_end: 10 },
            features: vec![FacetFeature::Link(Link { uri: "https://example.com".to_string() })],
        };

        let json = serde_json::to_string(&facet).unwrap();
        let deserialized: Facet = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, facet);
    }

    #[test]
    fn test_mention_feature_serialization() {
        let feature = FacetFeature::Mention(Mention { did: "did:plc:test123".to_string() });

        let json = serde_json::to_string(&feature).unwrap();
        assert!(json.contains("app.bsky.richtext.facet#mention"));
        assert!(json.contains("did:plc:test123"));
    }

    #[test]
    fn test_tag_feature_serialization() {
        let feature = FacetFeature::Tag(Tag { tag: "awesome".to_string() });

        let json = serde_json::to_string(&feature).unwrap();
        assert!(json.contains("app.bsky.richtext.facet#tag"));
        assert!(json.contains("awesome"));
    }

    #[test]
    fn test_strong_ref() {
        let strong_ref = StrongRef {
            uri: "at://did:plc:test/app.bsky.feed.post/123".to_string(),
            cid: "bafytest123".to_string(),
        };

        assert_eq!(strong_ref.uri, "at://did:plc:test/app.bsky.feed.post/123");
        assert_eq!(strong_ref.cid, "bafytest123");
    }

    #[test]
    fn test_reply_ref_to_post() {
        let reply_ref =
            ReplyRef::to_post("at://did:plc:test/app.bsky.feed.post/123", "bafytest123");

        assert!(reply_ref.is_top_level_reply());
        assert_eq!(reply_ref.root.uri, reply_ref.parent.uri);
        assert_eq!(reply_ref.root.cid, reply_ref.parent.cid);
        assert_eq!(reply_ref.root_uri(), "at://did:plc:test/app.bsky.feed.post/123");
        assert_eq!(reply_ref.parent_uri(), "at://did:plc:test/app.bsky.feed.post/123");
    }

    #[test]
    fn test_reply_ref_in_thread() {
        let reply_ref = ReplyRef::in_thread(
            "at://did:plc:test/app.bsky.feed.post/root",
            "bafyroot",
            "at://did:plc:test/app.bsky.feed.post/parent",
            "bafyparent",
        );

        assert!(!reply_ref.is_top_level_reply());
        assert_eq!(reply_ref.root.uri, "at://did:plc:test/app.bsky.feed.post/root");
        assert_eq!(reply_ref.root.cid, "bafyroot");
        assert_eq!(reply_ref.parent.uri, "at://did:plc:test/app.bsky.feed.post/parent");
        assert_eq!(reply_ref.parent.cid, "bafyparent");
        assert_eq!(reply_ref.root_uri(), "at://did:plc:test/app.bsky.feed.post/root");
        assert_eq!(reply_ref.parent_uri(), "at://did:plc:test/app.bsky.feed.post/parent");
    }

    #[test]
    fn test_reply_ref_serialization() {
        let reply_ref =
            ReplyRef::to_post("at://did:plc:test/app.bsky.feed.post/123", "bafytest123");

        let json = serde_json::to_string(&reply_ref).unwrap();
        let deserialized: ReplyRef = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, reply_ref);
    }

    #[test]
    fn test_post_record_with_reply() {
        let reply_ref =
            ReplyRef::to_post("at://did:plc:test/app.bsky.feed.post/parent", "bafyparent");

        let record = PostRecord {
            text: "This is a reply".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            reply: Some(reply_ref.clone()),
            facets: None,
            record_type: "app.bsky.feed.post".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("This is a reply"));
        assert!(json.contains("reply"));
        assert!(json.contains("root"));
        assert!(json.contains("parent"));
        assert!(json.contains("bafyparent"));

        let deserialized: PostRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, record);
    }

    #[test]
    fn test_post_record_without_reply() {
        let record = PostRecord {
            text: "This is a top-level post".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            reply: None,
            facets: None,
            record_type: "app.bsky.feed.post".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("This is a top-level post"));
        assert!(!json.contains("reply"));
    }

    #[test]
    fn test_create_post_response_deserialization() {
        let json = r#"{"uri":"at://did:plc:test/app.bsky.feed.post/abc","cid":"bafytest"}"#;
        let response: CreatePostResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.uri, "at://did:plc:test/app.bsky.feed.post/abc");
        assert_eq!(response.cid, "bafytest");
    }

    #[test]
    fn test_reply_composer_context() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let composer = ReplyComposer::new(client);

        // Top-level reply
        let reply_ref =
            ReplyRef::to_post("at://did:plc:test/app.bsky.feed.post/123", "bafytest123");
        let context = composer.get_reply_context(&reply_ref);
        assert!(context.contains("Replying to"));
        assert!(context.contains("at://did:plc:test/app.bsky.feed.post/123"));

        // Thread reply
        let thread_reply_ref = ReplyRef::in_thread(
            "at://did:plc:test/app.bsky.feed.post/root",
            "bafyroot",
            "at://did:plc:test/app.bsky.feed.post/parent",
            "bafyparent",
        );
        let thread_context = composer.get_reply_context(&thread_reply_ref);
        assert!(thread_context.contains("Replying to"));
        assert!(thread_context.contains("in thread"));
        assert!(thread_context.contains("parent"));
        assert!(thread_context.contains("root"));
    }
}
