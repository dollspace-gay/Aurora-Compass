//! Post management and rich text parsing
//!
//! This module provides functionality for working with posts, including
//! rich text parsing with facets for links, mentions, and hashtags, and
//! reply handling for threaded conversations.

use atproto_client::lexicon::BlobRef;
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

/// Aspect ratio for images
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AspectRatio {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

/// Image in an embed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedImage {
    /// Blob reference to the image
    pub image: BlobRef,
    /// Alt text for accessibility
    pub alt: String,
    /// Aspect ratio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<AspectRatio>,
}

/// Images embed (up to 4 images)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImagesEmbed {
    /// The images
    pub images: Vec<EmbedImage>,
    /// Embed type
    #[serde(rename = "$type")]
    pub embed_type: String,
}

impl ImagesEmbed {
    /// Create a new images embed
    pub fn new(images: Vec<EmbedImage>) -> Self {
        Self {
            images,
            embed_type: "app.bsky.embed.images".to_string(),
        }
    }
}

/// Main embed union type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Embed {
    /// Images embed
    Images(ImagesEmbed),
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
    /// Optional embed (images, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed: Option<Embed>,
    /// Optional language tags (BCP-47 language codes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub langs: Option<Vec<String>>,
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
            embed: None,
            langs: None,
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

/// Post composer error types
#[derive(Debug, Error)]
pub enum PostError {
    /// XRPC error
    #[error("XRPC error: {0}")]
    Xrpc(String),

    /// No session
    #[error("No active session")]
    NoSession,

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Text too long
    #[error("Text exceeds maximum length of {max} characters (got {actual})")]
    TextTooLong {
        /// Actual text length
        actual: usize,
        /// Maximum allowed length
        max: usize,
    },

    /// Too many images
    #[error("Too many images: {count} (maximum is 4)")]
    TooManyImages {
        /// Number of images
        count: usize,
    },

    /// Image processing error
    #[error("Image processing error: {0}")]
    ImageError(String),

    /// Invalid language code
    #[error("Invalid language code: {0}")]
    InvalidLanguage(String),

    /// Empty post
    #[error("Post cannot be empty (no text and no images)")]
    EmptyPost,

    /// Post not found
    #[error("Post not found: {0}")]
    NotFound(String),

    /// Not authorized to delete post
    #[error("Not authorized to delete this post")]
    NotAuthorized,

    /// Invalid post URI
    #[error("Invalid post URI: {0}")]
    InvalidUri(String),
}

/// Result type for post operations
pub type PostResult<T> = std::result::Result<T, PostError>;

/// Maximum post text length (grapheme count)
pub const MAX_POST_LENGTH: usize = 300;

/// Maximum number of images per post
pub const MAX_IMAGES_PER_POST: usize = 4;

/// Post composer for creating posts
///
/// Provides methods for composing and creating posts with text, images, and metadata.
///
/// # Example
///
/// ```rust,no_run
/// use app_core::posts::{PostComposer, RichText};
/// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create XRPC client (with auth)
///     let config = XrpcClientConfig::new("https://bsky.social");
///     let client = XrpcClient::new(config);
///     let composer = PostComposer::new(client);
///
///     // Create a post with text
///     let mut text = RichText::new("Hello Bluesky! #introduction");
///     text.detect_facets();
///
///     let (uri, cid) = composer.create_post(&text).await?;
///     println!("Created post: {} ({})", uri, cid);
///
///     Ok(())
/// }
/// ```
pub struct PostComposer {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl PostComposer {
    /// Create a new post composer
    pub fn new(client: XrpcClient) -> Self {
        Self { client: Arc::new(RwLock::new(client)) }
    }

    /// Create a post with text only
    ///
    /// # Arguments
    ///
    /// * `text` - The post text with facets
    ///
    /// # Returns
    ///
    /// Tuple of (uri, cid) for the created post
    ///
    /// # Errors
    ///
    /// - `PostError::EmptyPost` - Text is empty
    /// - `PostError::TextTooLong` - Text exceeds 300 graphemes
    /// - `PostError::NoSession` - No active session
    /// - `PostError::Xrpc` - XRPC error
    pub async fn create_post(&self, text: &RichText) -> PostResult<(String, String)> {
        self.create_post_with_options(text, None, None).await
    }

    /// Create a post with text and images
    ///
    /// # Arguments
    ///
    /// * `text` - The post text with facets
    /// * `images` - Image embeds (up to 4)
    ///
    /// # Returns
    ///
    /// Tuple of (uri, cid) for the created post
    ///
    /// # Errors
    ///
    /// - `PostError::TooManyImages` - More than 4 images
    /// - `PostError::TextTooLong` - Text exceeds 300 graphemes
    /// - `PostError::NoSession` - No active session
    /// - `PostError::Xrpc` - XRPC error
    pub async fn create_post_with_images(
        &self,
        text: &RichText,
        images: Vec<EmbedImage>,
    ) -> PostResult<(String, String)> {
        if images.len() > MAX_IMAGES_PER_POST {
            return Err(PostError::TooManyImages { count: images.len() });
        }

        let embed = if !images.is_empty() {
            Some(Embed::Images(ImagesEmbed::new(images)))
        } else {
            None
        };

        self.create_post_with_options(text, embed, None).await
    }

    /// Create a post with full options
    ///
    /// # Arguments
    ///
    /// * `text` - The post text with facets
    /// * `embed` - Optional embed (images, etc.)
    /// * `langs` - Optional language codes (BCP-47 format, e.g., ["en", "es"])
    ///
    /// # Returns
    ///
    /// Tuple of (uri, cid) for the created post
    ///
    /// # Errors
    ///
    /// - `PostError::EmptyPost` - No text and no embed
    /// - `PostError::TextTooLong` - Text exceeds 300 graphemes
    /// - `PostError::NoSession` - No active session
    /// - `PostError::Xrpc` - XRPC error
    pub async fn create_post_with_options(
        &self,
        text: &RichText,
        embed: Option<Embed>,
        langs: Option<Vec<String>>,
    ) -> PostResult<(String, String)> {
        // Validate text length using grapheme count
        let grapheme_count =
            unicode_segmentation::UnicodeSegmentation::graphemes(text.text.as_str(), true).count();

        if grapheme_count > MAX_POST_LENGTH {
            return Err(PostError::TextTooLong { actual: grapheme_count, max: MAX_POST_LENGTH });
        }

        // Check that post has content
        if text.text.is_empty() && embed.is_none() {
            return Err(PostError::EmptyPost);
        }

        let now = Utc::now().to_rfc3339();

        let record = PostRecord {
            text: text.text.clone(),
            created_at: now,
            reply: None,
            facets: text.facets.clone(),
            embed,
            langs,
            record_type: "app.bsky.feed.post".to_string(),
        };

        let body = serde_json::json!({
            "repo": "self",
            "collection": "app.bsky.feed.post",
            "record": record,
        });

        let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .json_body(&body)
            .map_err(|e| PostError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| PostError::Xrpc(e.to_string()))?;

        let create_response: CreatePostResponse =
            serde_json::from_value(response.data).map_err(PostError::Serialization)?;

        Ok((create_response.uri, create_response.cid))
    }

    /// Create a simple text post (convenience method)
    ///
    /// This method automatically detects facets in the text
    ///
    /// # Arguments
    ///
    /// * `text` - Plain text for the post
    ///
    /// # Returns
    ///
    /// Tuple of (uri, cid) for the created post
    pub async fn post_text(&self, text: impl Into<String>) -> PostResult<(String, String)> {
        let mut rich_text = RichText::new(text);
        rich_text.detect_facets();
        self.create_post(&rich_text).await
    }

    /// Create a post with language specified
    ///
    /// # Arguments
    ///
    /// * `text` - Plain text for the post
    /// * `langs` - Language codes (BCP-47 format, e.g., vec!["en", "es"])
    ///
    /// # Returns
    ///
    /// Tuple of (uri, cid) for the created post
    pub async fn post_text_with_langs(
        &self,
        text: impl Into<String>,
        langs: Vec<String>,
    ) -> PostResult<(String, String)> {
        let mut rich_text = RichText::new(text);
        rich_text.detect_facets();
        self.create_post_with_options(&rich_text, None, Some(langs))
            .await
    }

    /// Validate text length
    ///
    /// Returns the grapheme count and whether it's within limits
    pub fn validate_text_length(text: &str) -> (usize, bool) {
        let count = unicode_segmentation::UnicodeSegmentation::graphemes(text, true).count();
        (count, count <= MAX_POST_LENGTH)
    }

    /// Get remaining characters for a given text
    pub fn chars_remaining(text: &str) -> i32 {
        let count = unicode_segmentation::UnicodeSegmentation::graphemes(text, true).count();
        MAX_POST_LENGTH as i32 - count as i32
    }

    /// Delete a post
    ///
    /// Deletes a post from the user's feed. Only the post author can delete their own posts.
    ///
    /// # Arguments
    ///
    /// * `uri` - The AT-URI of the post to delete (e.g., "at://did:plc:abc123/app.bsky.feed.post/abc123")
    ///
    /// # Returns
    ///
    /// `Ok(())` if the post was successfully deleted
    ///
    /// # Errors
    ///
    /// - `PostError::InvalidUri` - Invalid or malformed post URI
    /// - `PostError::NotFound` - Post does not exist
    /// - `PostError::NotAuthorized` - User is not the post author
    /// - `PostError::NoSession` - No active session
    /// - `PostError::Xrpc` - XRPC error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::posts::PostComposer;
    /// # use atproto_client::xrpc::XrpcClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = XrpcClient::new(Default::default());
    /// let composer = PostComposer::new(client);
    ///
    /// // Delete a post
    /// composer.delete_post("at://did:plc:abc123/app.bsky.feed.post/xyz789").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_post(&self, uri: &str) -> PostResult<()> {
        // Validate URI format
        if !uri.starts_with("at://") {
            return Err(PostError::InvalidUri(format!("URI must start with 'at://': {}", uri)));
        }

        // Parse the URI to extract repo and rkey
        // AT-URI format: at://did:plc:abc123/app.bsky.feed.post/rkey
        let parts: Vec<&str> = uri.trim_start_matches("at://").split('/').collect();
        if parts.len() != 3 {
            return Err(PostError::InvalidUri(format!(
                "Invalid AT-URI format, expected 'at://repo/collection/rkey': {}",
                uri
            )));
        }

        let repo = parts[0];
        let collection = parts[1];
        let rkey = parts[2];

        // Verify this is a post collection
        if collection != "app.bsky.feed.post" {
            return Err(PostError::InvalidUri(format!(
                "URI must be for app.bsky.feed.post collection, got: {}",
                collection
            )));
        }

        // Build the deleteRecord request
        let body = serde_json::json!({
            "repo": repo,
            "collection": collection,
            "rkey": rkey,
        });

        let request = XrpcRequest::procedure("com.atproto.repo.deleteRecord")
            .json_body(&body)
            .map_err(|e| PostError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        let result = client.procedure::<serde_json::Value>(request).await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let err_msg = e.to_string();
                // Map common error responses
                if err_msg.contains("RecordNotFound") || err_msg.contains("not found") {
                    Err(PostError::NotFound(uri.to_string()))
                } else if err_msg.contains("NotAuthorized")
                    || err_msg.contains("not authorized")
                    || err_msg.contains("permission")
                {
                    Err(PostError::NotAuthorized)
                } else {
                    Err(PostError::Xrpc(err_msg))
                }
            }
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
            embed: None,
            langs: None,
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
            embed: None,
            langs: None,
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

    #[test]
    fn test_aspect_ratio() {
        let ratio = AspectRatio { width: 1920, height: 1080 };
        assert_eq!(ratio.width, 1920);
        assert_eq!(ratio.height, 1080);
    }

    #[test]
    fn test_embed_image() {
        let blob_ref = BlobRef::new("image/jpeg", 500000, "bafytest123");

        let embed_image = EmbedImage {
            image: blob_ref.clone(),
            alt: "Test image".to_string(),
            aspect_ratio: Some(AspectRatio { width: 1920, height: 1080 }),
        };

        assert_eq!(embed_image.alt, "Test image");
        assert_eq!(embed_image.image.mime_type, "image/jpeg");
        assert!(embed_image.aspect_ratio.is_some());
    }

    #[test]
    fn test_images_embed() {
        let blob_ref = BlobRef::new("image/jpeg", 500000, "bafytest123");

        let embed_image = EmbedImage {
            image: blob_ref,
            alt: "Test image".to_string(),
            aspect_ratio: None,
        };

        let images_embed = ImagesEmbed::new(vec![embed_image]);
        assert_eq!(images_embed.images.len(), 1);
        assert_eq!(images_embed.embed_type, "app.bsky.embed.images");
    }

    #[test]
    fn test_images_embed_serialization() {
        let blob_ref = BlobRef::new("image/jpeg", 500000, "bafytest123");

        let embed_image = EmbedImage {
            image: blob_ref,
            alt: "Test alt text".to_string(),
            aspect_ratio: Some(AspectRatio { width: 1000, height: 500 }),
        };

        let images_embed = ImagesEmbed::new(vec![embed_image]);
        let json = serde_json::to_string(&images_embed).unwrap();

        assert!(json.contains("app.bsky.embed.images"));
        assert!(json.contains("Test alt text"));
        assert!(json.contains("bafytest123"));
    }

    #[test]
    fn test_post_record_with_embed() {
        let blob_ref = BlobRef::new("image/jpeg", 500000, "bafytest123");

        let embed_image = EmbedImage {
            image: blob_ref,
            alt: "Test".to_string(),
            aspect_ratio: None,
        };

        let embed = Embed::Images(ImagesEmbed::new(vec![embed_image]));

        let record = PostRecord {
            text: "Check out this image!".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            reply: None,
            facets: None,
            embed: Some(embed),
            langs: None,
            record_type: "app.bsky.feed.post".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("Check out this image!"));
        assert!(json.contains("embed"));
        assert!(json.contains("app.bsky.embed.images"));
    }

    #[test]
    fn test_post_record_with_langs() {
        let record = PostRecord {
            text: "Hello world!".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            reply: None,
            facets: None,
            embed: None,
            langs: Some(vec!["en".to_string(), "es".to_string()]),
            record_type: "app.bsky.feed.post".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("langs"));
        assert!(json.contains("en"));
        assert!(json.contains("es"));
    }

    #[test]
    fn test_post_composer_validate_text_length() {
        let short_text = "Hello world";
        let (count, valid) = PostComposer::validate_text_length(short_text);
        assert_eq!(count, 11);
        assert!(valid);

        // Test with exactly 300 characters
        let max_text = "a".repeat(300);
        let (count, valid) = PostComposer::validate_text_length(&max_text);
        assert_eq!(count, 300);
        assert!(valid);

        // Test with > 300 characters
        let too_long_text = "a".repeat(301);
        let (count, valid) = PostComposer::validate_text_length(&too_long_text);
        assert_eq!(count, 301);
        assert!(!valid);
    }

    #[test]
    fn test_post_composer_validate_text_length_unicode() {
        // Test with emojis (each emoji counts as 1 grapheme)
        let emoji_text = "Hello 👋 World 🌍";
        let (count, valid) = PostComposer::validate_text_length(emoji_text);
        assert_eq!(count, 15); // H e l l o space 👋 space W o r l d space 🌍
        assert!(valid);
    }

    #[test]
    fn test_post_composer_chars_remaining() {
        let text = "Hello world";
        let remaining = PostComposer::chars_remaining(text);
        assert_eq!(remaining, 289); // 300 - 11

        let max_text = "a".repeat(300);
        let remaining = PostComposer::chars_remaining(&max_text);
        assert_eq!(remaining, 0);

        let too_long = "a".repeat(301);
        let remaining = PostComposer::chars_remaining(&too_long);
        assert_eq!(remaining, -1);
    }

    #[test]
    fn test_post_error_display() {
        let err = PostError::TextTooLong { actual: 350, max: 300 };
        let msg = err.to_string();
        assert!(msg.contains("350"));
        assert!(msg.contains("300"));

        let err = PostError::TooManyImages { count: 5 };
        let msg = err.to_string();
        assert!(msg.contains("5"));
        assert!(msg.contains("4"));

        let err = PostError::EmptyPost;
        let msg = err.to_string();
        assert!(msg.contains("empty"));
    }

    #[test]
    fn test_max_post_length_constant() {
        assert_eq!(MAX_POST_LENGTH, 300);
    }

    #[test]
    fn test_max_images_per_post_constant() {
        assert_eq!(MAX_IMAGES_PER_POST, 4);
    }

    #[test]
    fn test_post_composer_creation() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let _composer = PostComposer::new(client);
        // Just testing that it creates successfully
    }

    #[test]
    fn test_embed_image_without_aspect_ratio() {
        let blob_ref = BlobRef::new("image/png", 250000, "bafytest");

        let embed_image = EmbedImage {
            image: blob_ref,
            alt: String::new(),
            aspect_ratio: None,
        };

        let json = serde_json::to_string(&embed_image).unwrap();
        assert!(!json.contains("aspectRatio"));
    }

    #[test]
    fn test_post_record_skips_optional_fields() {
        let record = PostRecord {
            text: "Simple post".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            reply: None,
            facets: None,
            embed: None,
            langs: None,
            record_type: "app.bsky.feed.post".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains("reply"));
        assert!(!json.contains("facets"));
        assert!(!json.contains("embed"));
        assert!(!json.contains("langs"));
        assert!(json.contains("Simple post"));
    }

    #[test]
    fn test_multiple_images_embed() {
        let images: Vec<EmbedImage> = (0..4)
            .map(|i| EmbedImage {
                image: BlobRef::new("image/jpeg", 500000, format!("bafytest{}", i)),
                alt: format!("Image {}", i),
                aspect_ratio: None,
            })
            .collect();

        let images_embed = ImagesEmbed::new(images);
        assert_eq!(images_embed.images.len(), 4);

        let json = serde_json::to_string(&images_embed).unwrap();
        assert!(json.contains("bafytest0"));
        assert!(json.contains("bafytest3"));
        assert!(json.contains("Image 0"));
        assert!(json.contains("Image 3"));
    }

    // Delete post tests

    #[test]
    fn test_delete_post_invalid_uri_missing_prefix() {
        // Test URI validation - missing at:// prefix
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let client = XrpcClient::new(Default::default());
            let composer = PostComposer::new(client);

            let result = composer
                .delete_post("did:plc:abc123/app.bsky.feed.post/xyz789")
                .await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), PostError::InvalidUri(_)));
        });
    }

    #[test]
    fn test_delete_post_invalid_uri_wrong_format() {
        // Test URI validation - wrong format
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let client = XrpcClient::new(Default::default());
            let composer = PostComposer::new(client);

            // Too few parts
            let result = composer
                .delete_post("at://did:plc:abc123/app.bsky.feed.post")
                .await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), PostError::InvalidUri(_)));

            // Too many parts
            let result = composer
                .delete_post("at://did:plc:abc123/app.bsky.feed.post/xyz789/extra")
                .await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), PostError::InvalidUri(_)));
        });
    }

    #[test]
    fn test_delete_post_invalid_collection() {
        // Test URI validation - wrong collection type
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let client = XrpcClient::new(Default::default());
            let composer = PostComposer::new(client);

            let result = composer
                .delete_post("at://did:plc:abc123/app.bsky.feed.like/xyz789")
                .await;

            assert!(result.is_err());
            match result.unwrap_err() {
                PostError::InvalidUri(msg) => {
                    assert!(msg.contains("app.bsky.feed.post"));
                    assert!(msg.contains("app.bsky.feed.like"));
                }
                _ => panic!("Expected InvalidUri error"),
            }
        });
    }

    #[test]
    fn test_delete_post_valid_uri_format() {
        // Test that a valid URI format passes initial validation
        // Note: This will fail with network error since we don't have a real server,
        // but it should pass URI validation
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let client = XrpcClient::new(Default::default());
            let composer = PostComposer::new(client);

            let result = composer
                .delete_post("at://did:plc:abc123/app.bsky.feed.post/xyz789")
                .await;

            // Should get an XRPC error (network/auth), not URI validation error
            assert!(result.is_err());
            match result.unwrap_err() {
                PostError::Xrpc(_) => {
                    // This is expected - URI validation passed, network request failed
                }
                PostError::InvalidUri(_) => {
                    panic!("URI validation should have passed");
                }
                PostError::NoSession => {
                    // This is also acceptable - means URI validation passed
                }
                _ => {
                    // Other errors are also OK - URI validation passed
                }
            }
        });
    }

    #[test]
    fn test_delete_post_error_display() {
        // Test error message formatting for delete-related errors
        let err = PostError::NotFound("at://did:plc:test/app.bsky.feed.post/123".to_string());
        assert_eq!(err.to_string(), "Post not found: at://did:plc:test/app.bsky.feed.post/123");

        let err = PostError::NotAuthorized;
        assert_eq!(err.to_string(), "Not authorized to delete this post");

        let err = PostError::InvalidUri("bad-uri".to_string());
        assert_eq!(err.to_string(), "Invalid post URI: bad-uri");
    }

    #[test]
    fn test_post_error_types() {
        // Test that error types can be matched
        let err = PostError::NotFound("test".to_string());
        assert!(matches!(err, PostError::NotFound(_)));

        let err = PostError::NotAuthorized;
        assert!(matches!(err, PostError::NotAuthorized));

        let err = PostError::InvalidUri("test".to_string());
        assert!(matches!(err, PostError::InvalidUri(_)));
    }

    #[test]
    fn test_delete_post_uri_parsing() {
        // Test that URI components are correctly extracted
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let client = XrpcClient::new(Default::default());
            let composer = PostComposer::new(client);

            // Valid URI with all components
            let uri = "at://did:plc:abc123xyz/app.bsky.feed.post/3k2k3k4k5k6k7k8k";
            let result = composer.delete_post(uri).await;

            // Should fail with network error, not URI parsing error
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(!matches!(err, PostError::InvalidUri(_)));
        });
    }

    #[test]
    fn test_delete_post_empty_components() {
        // Test URI with empty components
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let client = XrpcClient::new(Default::default());
            let composer = PostComposer::new(client);

            let result = composer.delete_post("at:////").await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), PostError::InvalidUri(_)));
        });
    }
}
