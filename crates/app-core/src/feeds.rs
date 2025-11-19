//! Feed management
//!
//! This module provides feed functionality including the following feed,
//! feed pagination, and real-time updates.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::profiles::ProfileViewBasic;
use atproto_client::xrpc::XrpcClient;

/// Errors that can occur during feed operations
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    /// Network or API error
    #[error("API error: {0}")]
    ApiError(String),

    /// JSON parsing error
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    /// Lock error
    #[error("Lock error")]
    LockError,
}

/// Result type for feed operations
pub type Result<T> = std::result::Result<T, FeedError>;

/// A post view in a feed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostView {
    /// URI of the post
    pub uri: String,

    /// CID of the post
    pub cid: String,

    /// Author of the post
    pub author: ProfileViewBasic,

    /// Post record (text, facets, etc.)
    pub record: serde_json::Value,

    /// Embed content (images, videos, quotes, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed: Option<serde_json::Value>,

    /// Reply count
    #[serde(rename = "replyCount", skip_serializing_if = "Option::is_none")]
    pub reply_count: Option<u32>,

    /// Repost count
    #[serde(rename = "repostCount", skip_serializing_if = "Option::is_none")]
    pub repost_count: Option<u32>,

    /// Like count
    #[serde(rename = "likeCount", skip_serializing_if = "Option::is_none")]
    pub like_count: Option<u32>,

    /// Quote count
    #[serde(rename = "quoteCount", skip_serializing_if = "Option::is_none")]
    pub quote_count: Option<u32>,

    /// Timestamp when indexed
    #[serde(rename = "indexedAt")]
    pub indexed_at: String,

    /// Viewer state (like URI, repost URI, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<ViewerState>,

    /// Labels applied to the post
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,

    /// Thread gate (who can reply)
    #[serde(rename = "threadgate", skip_serializing_if = "Option::is_none")]
    pub threadgate: Option<serde_json::Value>,
}

/// Viewer's state relative to a post
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewerState {
    /// URI of the viewer's like, if they liked this post
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like: Option<String>,

    /// URI of the viewer's repost, if they reposted this post
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repost: Option<String>,

    /// Whether the viewer has muted this thread
    #[serde(rename = "threadMuted", skip_serializing_if = "Option::is_none")]
    pub thread_muted: Option<bool>,

    /// Whether the post is embedded as a quote
    #[serde(rename = "embeddingDisabled", skip_serializing_if = "Option::is_none")]
    pub embedding_disabled: Option<bool>,

    /// Whether the viewer is pinned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
}

/// Label applied to content for moderation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    /// Version of the label schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ver: Option<u32>,

    /// Source DID that created the label
    pub src: String,

    /// URI of the labeled content
    pub uri: String,

    /// CID of the labeled content (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,

    /// The label value
    pub val: String,

    /// Negation flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neg: Option<bool>,

    /// When the label was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cts: Option<String>,

    /// Expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<String>,

    /// Signature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sig: Option<Vec<u8>>,
}

/// Reply reference in a feed post
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplyRef {
    /// Root post in the thread
    pub root: ReplyRefPost,

    /// Immediate parent post
    pub parent: ReplyRefPost,

    /// Grandparent author (if available)
    #[serde(rename = "grandparentAuthor", skip_serializing_if = "Option::is_none")]
    pub grandparent_author: Option<ProfileViewBasic>,
}

/// Post reference in a reply chain (can be a full post or a stub)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReplyRefPost {
    /// Full post view
    PostView(Box<PostView>),
    /// Not found stub
    NotFoundPost(NotFoundPost),
    /// Blocked post stub
    BlockedPost(BlockedPost),
}

/// Stub for a post that wasn't found
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotFoundPost {
    /// URI of the not found post
    pub uri: String,
    /// Indicates this is a not found post
    #[serde(rename = "notFound")]
    pub not_found: bool,
}

/// Stub for a blocked post
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedPost {
    /// URI of the blocked post
    pub uri: String,
    /// Indicates this is blocked
    pub blocked: bool,
    /// Author of the blocked post
    pub author: BlockedAuthor,
}

/// Author of a blocked post
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedAuthor {
    /// DID of the blocked author
    pub did: String,
    /// Viewer state showing block
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<serde_json::Value>,
}

/// Reason a post appears in the feed (repost)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum FeedReason {
    /// Post was reposted by someone
    #[serde(rename = "app.bsky.feed.defs#reasonRepost")]
    Repost {
        /// The user who reposted
        by: Box<ProfileViewBasic>,
        /// When it was reposted
        #[serde(rename = "indexedAt")]
        indexed_at: String,
    },
    /// Post was pinned
    #[serde(rename = "app.bsky.feed.defs#reasonPin")]
    Pin,
}

/// A post in a feed with context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedViewPost {
    /// The post itself
    pub post: PostView,

    /// Reply context (parent/root posts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<ReplyRef>,

    /// Reason this post appears in the feed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<FeedReason>,

    /// Feed-specific context string
    #[serde(rename = "feedContext", skip_serializing_if = "Option::is_none")]
    pub feed_context: Option<String>,
}

/// Parameters for fetching a feed
#[derive(Debug, Clone, Default)]
pub struct FeedParams {
    /// Pagination cursor
    pub cursor: Option<String>,

    /// Number of items to fetch (default 50, max 100)
    pub limit: u32,
}

/// Response from fetching a feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedResponse {
    /// Cursor for pagination (next page)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// Feed posts
    pub feed: Vec<FeedViewPost>,
}

/// Following feed service
pub struct FollowingFeed {
    client: Arc<RwLock<XrpcClient>>,
}

impl FollowingFeed {
    /// Create a new following feed service
    pub fn new(client: Arc<RwLock<XrpcClient>>) -> Self {
        Self { client }
    }

    /// Fetch the following feed (timeline)
    ///
    /// This returns posts from accounts the user follows in reverse chronological order.
    ///
    /// # Arguments
    ///
    /// * `params` - Feed parameters including cursor and limit
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::feeds::{FollowingFeed, FeedParams};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let feed = FollowingFeed::new(client);
    /// let params = FeedParams {
    ///     cursor: None,
    ///     limit: 50,
    /// };
    /// let response = feed.fetch(params).await?;
    /// println!("Got {} posts", response.feed.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch(&self, params: FeedParams) -> Result<FeedResponse> {
        let client = self.client.read().await;

        let mut request = atproto_client::XrpcRequest::query("app.bsky.feed.getTimeline")
            .param("limit", params.limit.to_string());

        if let Some(cursor) = params.cursor {
            request = request.param("cursor", cursor);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| FeedError::ApiError(e.to_string()))?;

        let feed_response: FeedResponse =
            serde_json::from_value(response.data).map_err(FeedError::ParseError)?;

        Ok(feed_response)
    }

    /// Peek at the latest post without affecting pagination
    ///
    /// This is useful for checking if there are new posts available
    /// without consuming them from the feed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::feeds::FollowingFeed;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let feed = FollowingFeed::new(client);
    /// if let Some(latest) = feed.peek_latest().await? {
    ///     println!("New post available: {}", latest.post.uri);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn peek_latest(&self) -> Result<Option<FeedViewPost>> {
        let params = FeedParams { cursor: None, limit: 1 };

        let response = self.fetch(params).await?;
        Ok(response.feed.into_iter().next())
    }
}

/// Feed ranking/sorting options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeedSort {
    /// Reverse chronological (newest first)
    ReverseChronological,
    /// Algorithmic ranking
    Algorithmic,
}

/// Feed merge configuration for combining multiple feeds
#[derive(Debug, Clone)]
pub struct FeedMergeConfig {
    /// Source feed URIs to merge
    pub sources: Vec<String>,

    /// How to sort the merged feed
    pub sort: FeedSort,

    /// Weight for each source (for algorithmic sorting)
    pub weights: Vec<f32>,
}

/// Deduplication strategy for feed items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupeStrategy {
    /// No deduplication
    None,
    /// Dedupe by post URI
    ByUri,
    /// Dedupe by thread root URI (show only latest in thread)
    ByThread,
}

/// Feed tuner for filtering and deduplicating feed items
pub struct FeedTuner {
    seen_uris: std::collections::HashSet<String>,
    seen_thread_roots: std::collections::HashSet<String>,
}

impl FeedTuner {
    /// Create a new feed tuner
    pub fn new() -> Self {
        Self {
            seen_uris: std::collections::HashSet::new(),
            seen_thread_roots: std::collections::HashSet::new(),
        }
    }

    /// Apply deduplication to a list of feed posts
    pub fn dedupe(
        &mut self,
        posts: Vec<FeedViewPost>,
        strategy: DedupeStrategy,
    ) -> Vec<FeedViewPost> {
        match strategy {
            DedupeStrategy::None => posts,
            DedupeStrategy::ByUri => self.dedupe_by_uri(posts),
            DedupeStrategy::ByThread => self.dedupe_by_thread(posts),
        }
    }

    fn dedupe_by_uri(&mut self, posts: Vec<FeedViewPost>) -> Vec<FeedViewPost> {
        posts
            .into_iter()
            .filter(|post| {
                if self.seen_uris.contains(&post.post.uri) {
                    false
                } else {
                    self.seen_uris.insert(post.post.uri.clone());
                    true
                }
            })
            .collect()
    }

    fn dedupe_by_thread(&mut self, posts: Vec<FeedViewPost>) -> Vec<FeedViewPost> {
        posts
            .into_iter()
            .filter(|post| {
                // Get the root URI of the thread
                let root_uri = if let Some(reply) = &post.reply {
                    match &reply.root {
                        ReplyRefPost::PostView(p) => &p.uri,
                        ReplyRefPost::NotFoundPost(nf) => &nf.uri,
                        ReplyRefPost::BlockedPost(b) => &b.uri,
                    }
                } else {
                    &post.post.uri
                };

                if self.seen_thread_roots.contains(root_uri) {
                    false
                } else {
                    self.seen_thread_roots.insert(root_uri.clone());
                    true
                }
            })
            .collect()
    }

    /// Filter out replies that don't meet following feed criteria
    ///
    /// In the following feed, we only show replies if:
    /// - The reply author is someone you follow
    /// - AND at least one of: you follow the parent author, the root author, or it's a self-thread
    pub fn filter_followed_replies_only(
        &self,
        posts: Vec<FeedViewPost>,
        user_did: &str,
    ) -> Vec<FeedViewPost> {
        posts
            .into_iter()
            .filter(|post| {
                // If it's not a reply, always show it
                let Some(reply) = &post.reply else {
                    return true;
                };

                let author = &post.post.author;

                // Only show replies from self or people you follow
                if !Self::is_self_or_following(author, user_did) {
                    return false;
                }

                // Check if it's a self-thread
                let parent_author = match &reply.parent {
                    ReplyRefPost::PostView(p) => Some(&p.author),
                    _ => None,
                };

                let root_author = match &reply.root {
                    ReplyRefPost::PostView(p) => Some(&p.author),
                    _ => None,
                };

                // Always show self-threads
                if parent_author.map(|p| p.did == author.did).unwrap_or(true)
                    && root_author.map(|r| r.did == author.did).unwrap_or(true)
                    && reply
                        .grandparent_author
                        .as_ref()
                        .map(|g| g.did == author.did)
                        .unwrap_or(true)
                {
                    return true;
                }

                // From this point on we need at least one more reason to show it
                if let Some(parent) = parent_author {
                    if parent.did != author.did && Self::is_self_or_following(parent, user_did) {
                        return true;
                    }
                }

                if let Some(grandparent) = &reply.grandparent_author {
                    if grandparent.did != author.did
                        && Self::is_self_or_following(grandparent, user_did)
                    {
                        return true;
                    }
                }

                if let Some(root) = root_author {
                    if root.did != author.did && Self::is_self_or_following(root, user_did) {
                        return true;
                    }
                }

                false
            })
            .collect()
    }

    fn is_self_or_following(profile: &ProfileViewBasic, user_did: &str) -> bool {
        profile.did == user_did
            || profile
                .viewer
                .as_ref()
                .and_then(|v| v.following.as_ref())
                .is_some()
    }
}

impl Default for FeedTuner {
    fn default() -> Self {
        Self::new()
    }
}

/// Feed generator view (custom algorithm feed)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratorView {
    /// URI of the feed generator
    pub uri: String,

    /// CID of the feed generator record
    pub cid: String,

    /// DID of the creator
    pub did: String,

    /// Creator profile
    pub creator: ProfileViewBasic,

    /// Display name of the feed
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Description of the feed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Description facets (links, mentions, etc.)
    #[serde(rename = "descriptionFacets", skip_serializing_if = "Option::is_none")]
    pub description_facets: Option<Vec<serde_json::Value>>,

    /// Avatar image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,

    /// Number of likes
    #[serde(rename = "likeCount", skip_serializing_if = "Option::is_none")]
    pub like_count: Option<u32>,

    /// Whether this feed accepts interactions
    #[serde(
        rename = "acceptsInteractions",
        skip_serializing_if = "Option::is_none"
    )]
    pub accepts_interactions: Option<bool>,

    /// Labels applied to the feed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,

    /// Viewer state for this feed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<GeneratorViewerState>,

    /// When the feed was indexed
    #[serde(rename = "indexedAt")]
    pub indexed_at: String,
}

/// Viewer state for a feed generator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorViewerState {
    /// URI of viewer's like if they liked this feed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like: Option<String>,
}

/// Feed preferences
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedPreferences {
    /// User's language preferences for content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_languages: Option<Vec<String>>,

    /// User's interests/topics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interests: Option<Vec<String>>,
}

impl FeedPreferences {
    /// Create preferences with content languages
    pub fn with_languages(languages: Vec<String>) -> Self {
        Self {
            content_languages: Some(languages),
            interests: None,
        }
    }

    /// Create preferences with interests
    pub fn with_interests(interests: Vec<String>) -> Self {
        Self {
            content_languages: None,
            interests: Some(interests),
        }
    }

    /// Get content languages as a comma-separated string
    pub fn content_languages_header(&self) -> String {
        self.content_languages
            .as_ref()
            .map(|langs| langs.join(","))
            .unwrap_or_default()
    }

    /// Get interests as a comma-separated string
    pub fn interests_header(&self) -> String {
        self.interests
            .as_ref()
            .map(|interests| interests.join(","))
            .unwrap_or_default()
    }
}

/// Custom feed (algorithm feed) service
pub struct CustomFeed {
    client: Arc<RwLock<XrpcClient>>,
    feed_uri: String,
    preferences: FeedPreferences,
}

impl CustomFeed {
    /// Create a new custom feed service
    ///
    /// # Arguments
    ///
    /// * `client` - XRPC client for API requests
    /// * `feed_uri` - AT URI of the feed generator
    /// * `preferences` - Feed preferences (languages, interests)
    pub fn new(
        client: Arc<RwLock<XrpcClient>>,
        feed_uri: impl Into<String>,
        preferences: FeedPreferences,
    ) -> Self {
        Self { client, feed_uri: feed_uri.into(), preferences }
    }

    /// Fetch posts from the custom feed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::feeds::{CustomFeed, FeedParams, FeedPreferences};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let preferences = FeedPreferences::with_languages(vec!["en".to_string()]);
    /// let feed = CustomFeed::new(
    ///     client,
    ///     "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot",
    ///     preferences,
    /// );
    /// let params = FeedParams {
    ///     cursor: None,
    ///     limit: 50,
    /// };
    /// let response = feed.fetch(params).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch(&self, params: FeedParams) -> Result<FeedResponse> {
        let client = self.client.read().await;

        let mut request = atproto_client::XrpcRequest::query("app.bsky.feed.getFeed")
            .param("feed", &self.feed_uri)
            .param("limit", params.limit.to_string());

        if let Some(cursor) = params.cursor {
            request = request.param("cursor", cursor);
        }

        // Add language preferences
        let content_langs = self.preferences.content_languages_header();
        if !content_langs.is_empty() {
            request = request.header("Accept-Language", content_langs);
        }

        // Add interests header for Bluesky-owned feeds
        let interests = self.preferences.interests_header();
        if !interests.is_empty() && self.is_bluesky_owned() {
            request = request.header("X-Bsky-Topics", interests);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| FeedError::ApiError(e.to_string()))?;

        let mut feed_response: FeedResponse =
            serde_json::from_value(response.data).map_err(FeedError::ParseError)?;

        // Some custom feeds fail to enforce pagination limits, so truncate manually
        if feed_response.feed.len() > params.limit as usize {
            feed_response.feed.truncate(params.limit as usize);
        }

        // Clear cursor if feed is empty
        if feed_response.feed.is_empty() {
            feed_response.cursor = None;
        }

        Ok(feed_response)
    }

    /// Peek at the latest post in the feed
    pub async fn peek_latest(&self) -> Result<Option<FeedViewPost>> {
        let params = FeedParams { cursor: None, limit: 1 };

        let response = self.fetch(params).await?;
        Ok(response.feed.into_iter().next())
    }

    /// Check if this feed is owned by Bluesky
    ///
    /// Bluesky-owned feeds receive special treatment like the X-Bsky-Topics header
    fn is_bluesky_owned(&self) -> bool {
        // Known Bluesky feed owner DIDs
        const BLUESKY_FEED_OWNERS: &[&str] = &[
            "did:plc:z72i7hdynmk6r22z27h6tvur", // bsky.app
            "did:plc:q6gjnaw2blty4crticxkmujt", // other official feeds
        ];

        // Parse the feed URI to extract the DID
        if let Some(did_end) = self.feed_uri.find("/app.bsky.feed.generator/") {
            let did_start = self.feed_uri.find("did:").unwrap_or(0);
            let did = &self.feed_uri[did_start..did_end];
            BLUESKY_FEED_OWNERS.contains(&did)
        } else {
            false
        }
    }

    /// Get the feed URI
    pub fn uri(&self) -> &str {
        &self.feed_uri
    }
}

/// Get feed generator information
pub struct FeedGeneratorService {
    client: Arc<RwLock<XrpcClient>>,
}

impl FeedGeneratorService {
    /// Create a new feed generator service
    pub fn new(client: Arc<RwLock<XrpcClient>>) -> Self {
        Self { client }
    }

    /// Get information about a feed generator
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::feeds::FeedGeneratorService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let service = FeedGeneratorService::new(client);
    /// let generator = service.get_feed_generator(
    ///     "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot"
    /// ).await?;
    /// println!("Feed: {}", generator.display_name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_feed_generator(&self, uri: &str) -> Result<GeneratorView> {
        let client = self.client.read().await;

        let request =
            atproto_client::XrpcRequest::query("app.bsky.feed.getFeedGenerator").param("feed", uri);

        let response = client
            .query(request)
            .await
            .map_err(|e| FeedError::ApiError(e.to_string()))?;

        #[derive(Deserialize)]
        struct GetFeedGeneratorResponse {
            view: GeneratorView,
        }

        let generator_response: GetFeedGeneratorResponse =
            serde_json::from_value(response.data).map_err(FeedError::ParseError)?;

        Ok(generator_response.view)
    }

    /// Get multiple feed generators in a single request
    pub async fn get_feed_generators(&self, uris: &[String]) -> Result<Vec<GeneratorView>> {
        let client = self.client.read().await;

        let mut request = atproto_client::XrpcRequest::query("app.bsky.feed.getFeedGenerators");

        for uri in uris {
            request = request.param("feeds", uri);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| FeedError::ApiError(e.to_string()))?;

        #[derive(Deserialize)]
        struct GetFeedGeneratorsResponse {
            feeds: Vec<GeneratorView>,
        }

        let generators_response: GetFeedGeneratorsResponse =
            serde_json::from_value(response.data).map_err(FeedError::ParseError)?;

        Ok(generators_response.feeds)
    }
}

/// Sort order for hashtag feed results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HashtagFeedSort {
    /// Most relevant results first (default)
    #[default]
    Top,
    /// Most recent results first
    Latest,
}

impl HashtagFeedSort {
    /// Convert to API string value
    pub fn as_str(&self) -> &'static str {
        match self {
            HashtagFeedSort::Top => "top",
            HashtagFeedSort::Latest => "latest",
        }
    }
}

/// Parameters for fetching a hashtag feed
#[derive(Debug, Clone)]
pub struct HashtagFeedParams {
    /// Hashtag to search for (with or without # prefix)
    pub hashtag: String,

    /// Pagination cursor
    pub cursor: Option<String>,

    /// Number of items to fetch (default 50, max 100)
    pub limit: u32,

    /// Sort order for results
    pub sort: HashtagFeedSort,
}

impl HashtagFeedParams {
    /// Create new hashtag feed parameters
    ///
    /// # Arguments
    ///
    /// * `hashtag` - The hashtag to search for (with or without # prefix)
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::HashtagFeedParams;
    /// let params = HashtagFeedParams::new("rust");
    /// assert_eq!(params.hashtag, "rust");
    /// ```
    pub fn new(hashtag: impl Into<String>) -> Self {
        let hashtag = hashtag.into();
        // Remove # prefix if present
        let hashtag = if let Some(stripped) = hashtag.strip_prefix('#') {
            stripped.to_string()
        } else {
            hashtag
        };

        Self {
            hashtag,
            cursor: None,
            limit: 50,
            sort: HashtagFeedSort::default(),
        }
    }

    /// Set the pagination cursor
    pub fn with_cursor(mut self, cursor: Option<String>) -> Self {
        self.cursor = cursor;
        self
    }

    /// Set the result limit
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit.min(100); // Cap at 100 per API limits
        self
    }

    /// Set the sort order
    pub fn with_sort(mut self, sort: HashtagFeedSort) -> Self {
        self.sort = sort;
        self
    }
}

/// Hashtag feed service
///
/// Provides feed functionality for posts containing specific hashtags.
/// Uses the Bluesky search API to find posts with hashtags.
pub struct HashtagFeed {
    client: Arc<RwLock<XrpcClient>>,
}

impl HashtagFeed {
    /// Create a new hashtag feed service
    ///
    /// # Arguments
    ///
    /// * `client` - Shared XRPC client
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::feeds::HashtagFeed;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let feed = HashtagFeed::new(client);
    /// # }
    /// ```
    pub fn new(client: Arc<RwLock<XrpcClient>>) -> Self {
        Self { client }
    }

    /// Fetch posts for a specific hashtag
    ///
    /// This returns posts containing the specified hashtag, using the Bluesky
    /// search API. Results can be sorted by relevance (top) or recency (latest).
    ///
    /// # Arguments
    ///
    /// * `params` - Hashtag feed parameters including hashtag, cursor, limit, and sort
    ///
    /// # Returns
    ///
    /// A `FeedResponse` containing posts with the hashtag and pagination cursor.
    ///
    /// # Errors
    ///
    /// Returns `FeedError::ApiError` if the network request fails or
    /// `FeedError::ParseError` if the response cannot be parsed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::feeds::{HashtagFeed, HashtagFeedParams, HashtagFeedSort};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let feed = HashtagFeed::new(client);
    /// let params = HashtagFeedParams::new("rust")
    ///     .with_sort(HashtagFeedSort::Latest)
    ///     .with_limit(25);
    /// let response = feed.fetch(params).await?;
    /// println!("Got {} posts with #rust", response.feed.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch(&self, params: HashtagFeedParams) -> Result<FeedResponse> {
        if params.hashtag.trim().is_empty() {
            return Err(FeedError::ApiError("Hashtag cannot be empty".to_string()));
        }

        let client = self.client.read().await;

        // Build search query with hashtag prefix
        let query = format!("#{}", params.hashtag.trim());

        let mut request = atproto_client::XrpcRequest::query("app.bsky.feed.searchPosts")
            .param("q", query)
            .param("limit", params.limit.to_string())
            .param("sort", params.sort.as_str());

        if let Some(cursor) = params.cursor {
            request = request.param("cursor", cursor);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| FeedError::ApiError(e.to_string()))?;

        // Parse the search response
        #[derive(Deserialize)]
        struct SearchResponse {
            posts: Vec<PostView>,
            #[serde(skip_serializing_if = "Option::is_none")]
            cursor: Option<String>,
        }

        let search_response: SearchResponse =
            serde_json::from_value(response.data).map_err(FeedError::ParseError)?;

        // Convert PostView to FeedViewPost for feed consistency
        let feed = search_response
            .posts
            .into_iter()
            .map(|post| FeedViewPost {
                post,
                reply: None,
                reason: None,
                feed_context: None,
            })
            .collect();

        Ok(FeedResponse {
            cursor: search_response.cursor,
            feed,
        })
    }

    /// Peek at the latest post for a hashtag without affecting pagination
    ///
    /// This is useful for checking if there are new posts with this hashtag
    /// without consuming them from the feed.
    ///
    /// # Arguments
    ///
    /// * `hashtag` - The hashtag to check (with or without # prefix)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::feeds::HashtagFeed;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let feed = HashtagFeed::new(client);
    /// if let Some(latest) = feed.peek_latest("rust").await? {
    ///     println!("New post with #rust: {}", latest.post.uri);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn peek_latest(&self, hashtag: impl Into<String>) -> Result<Option<FeedViewPost>> {
        let params = HashtagFeedParams::new(hashtag)
            .with_limit(1)
            .with_sort(HashtagFeedSort::Latest);

        let response = self.fetch(params).await?;
        Ok(response.feed.into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_post(uri: &str, author_did: &str) -> PostView {
        PostView {
            uri: uri.to_string(),
            cid: "test-cid".to_string(),
            author: ProfileViewBasic {
                did: author_did.to_string(),
                handle: "test.bsky.social".to_string(),
                display_name: Some("Test User".to_string()),
                avatar: None,
                associated: None,
                viewer: None,
                labels: None,
                created_at: None,
            },
            record: serde_json::json!({"text": "test"}),
            embed: None,
            reply_count: Some(0),
            repost_count: Some(0),
            like_count: Some(0),
            quote_count: Some(0),
            indexed_at: "2024-01-01T00:00:00Z".to_string(),
            viewer: None,
            labels: None,
            threadgate: None,
        }
    }

    fn create_test_feed_post(uri: &str, author_did: &str) -> FeedViewPost {
        FeedViewPost {
            post: create_test_post(uri, author_did),
            reply: None,
            reason: None,
            feed_context: None,
        }
    }

    #[test]
    fn test_dedupe_by_uri() {
        let mut tuner = FeedTuner::new();

        let posts = vec![
            create_test_feed_post("at://did:plc:abc/app.bsky.feed.post/1", "did:plc:abc"),
            create_test_feed_post("at://did:plc:abc/app.bsky.feed.post/2", "did:plc:abc"),
            create_test_feed_post("at://did:plc:abc/app.bsky.feed.post/1", "did:plc:abc"), // duplicate
        ];

        let deduped = tuner.dedupe(posts, DedupeStrategy::ByUri);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].post.uri, "at://did:plc:abc/app.bsky.feed.post/1");
        assert_eq!(deduped[1].post.uri, "at://did:plc:abc/app.bsky.feed.post/2");
    }

    #[test]
    fn test_dedupe_by_thread() {
        let mut tuner = FeedTuner::new();

        let root_post = create_test_post("at://did:plc:abc/app.bsky.feed.post/1", "did:plc:abc");

        let posts = vec![
            FeedViewPost {
                post: create_test_post("at://did:plc:abc/app.bsky.feed.post/2", "did:plc:abc"),
                reply: Some(ReplyRef {
                    root: ReplyRefPost::PostView(Box::new(root_post.clone())),
                    parent: ReplyRefPost::PostView(Box::new(root_post.clone())),
                    grandparent_author: None,
                }),
                reason: None,
                feed_context: None,
            },
            FeedViewPost {
                post: create_test_post("at://did:plc:abc/app.bsky.feed.post/3", "did:plc:abc"),
                reply: Some(ReplyRef {
                    root: ReplyRefPost::PostView(Box::new(root_post.clone())),
                    parent: ReplyRefPost::PostView(Box::new(root_post.clone())),
                    grandparent_author: None,
                }),
                reason: None,
                feed_context: None,
            },
            create_test_feed_post("at://did:plc:xyz/app.bsky.feed.post/1", "did:plc:xyz"), // different thread
        ];

        let deduped = tuner.dedupe(posts, DedupeStrategy::ByThread);
        // Should keep only the first post from the thread and the post from a different thread
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_filter_followed_replies_self_thread() {
        let tuner = FeedTuner::new();
        let user_did = "did:plc:user";

        let root_post = create_test_post("at://did:plc:user/app.bsky.feed.post/1", user_did);

        let posts = vec![FeedViewPost {
            post: create_test_post("at://did:plc:user/app.bsky.feed.post/2", user_did),
            reply: Some(ReplyRef {
                root: ReplyRefPost::PostView(Box::new(root_post.clone())),
                parent: ReplyRefPost::PostView(Box::new(root_post.clone())),
                grandparent_author: None,
            }),
            reason: None,
            feed_context: None,
        }];

        let filtered = tuner.filter_followed_replies_only(posts, user_did);
        // Self-threads should always be shown
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_feed_params_default() {
        let params = FeedParams::default();
        assert_eq!(params.cursor, None);
        assert_eq!(params.limit, 0);
    }

    #[test]
    fn test_viewer_state_serialization() {
        let viewer = ViewerState {
            like: Some("at://did:plc:abc/app.bsky.feed.like/123".to_string()),
            repost: None,
            thread_muted: Some(false),
            embedding_disabled: None,
            pinned: None,
        };

        let json = serde_json::to_string(&viewer).unwrap();
        let deserialized: ViewerState = serde_json::from_str(&json).unwrap();
        assert_eq!(viewer, deserialized);
    }

    #[test]
    fn test_feed_reason_repost() {
        let reason = FeedReason::Repost {
            by: Box::new(ProfileViewBasic {
                did: "did:plc:abc".to_string(),
                handle: "user.bsky.social".to_string(),
                display_name: Some("User".to_string()),
                avatar: None,
                associated: None,
                viewer: None,
                labels: None,
                created_at: None,
            }),
            indexed_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("app.bsky.feed.defs#reasonRepost"));
    }

    #[test]
    fn test_reply_ref_post_variants() {
        let not_found = ReplyRefPost::NotFoundPost(NotFoundPost {
            uri: "at://did:plc:abc/app.bsky.feed.post/1".to_string(),
            not_found: true,
        });

        let blocked = ReplyRefPost::BlockedPost(BlockedPost {
            uri: "at://did:plc:xyz/app.bsky.feed.post/1".to_string(),
            blocked: true,
            author: BlockedAuthor { did: "did:plc:xyz".to_string(), viewer: None },
        });

        // These should serialize/deserialize correctly
        let nf_json = serde_json::to_string(&not_found).unwrap();
        let b_json = serde_json::to_string(&blocked).unwrap();

        let _nf: ReplyRefPost = serde_json::from_str(&nf_json).unwrap();
        let _b: ReplyRefPost = serde_json::from_str(&b_json).unwrap();
    }

    #[test]
    fn test_feed_preferences_languages() {
        let prefs = FeedPreferences::with_languages(vec!["en".to_string(), "es".to_string()]);
        assert_eq!(prefs.content_languages_header(), "en,es");
        assert!(prefs.interests.is_none());
    }

    #[test]
    fn test_feed_preferences_interests() {
        let prefs = FeedPreferences::with_interests(vec!["tech".to_string(), "sports".to_string()]);
        assert_eq!(prefs.interests_header(), "tech,sports");
        assert!(prefs.content_languages.is_none());
    }

    #[test]
    fn test_feed_preferences_default() {
        let prefs = FeedPreferences::default();
        assert_eq!(prefs.content_languages_header(), "");
        assert_eq!(prefs.interests_header(), "");
    }

    #[test]
    fn test_generator_view_serialization() {
        let generator = GeneratorView {
            uri: "at://did:plc:abc/app.bsky.feed.generator/test".to_string(),
            cid: "test-cid".to_string(),
            did: "did:plc:abc".to_string(),
            creator: ProfileViewBasic {
                did: "did:plc:abc".to_string(),
                handle: "user.bsky.social".to_string(),
                display_name: Some("User".to_string()),
                avatar: None,
                associated: None,
                viewer: None,
                labels: None,
                created_at: None,
            },
            display_name: "Test Feed".to_string(),
            description: Some("A test feed".to_string()),
            description_facets: None,
            avatar: Some("https://example.com/avatar.jpg".to_string()),
            like_count: Some(100),
            accepts_interactions: Some(true),
            labels: None,
            viewer: Some(GeneratorViewerState {
                like: Some("at://did:plc:abc/app.bsky.feed.like/123".to_string()),
            }),
            indexed_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&generator).unwrap();
        let deserialized: GeneratorView = serde_json::from_str(&json).unwrap();
        assert_eq!(generator, deserialized);
        assert!(json.contains("displayName"));
        assert!(json.contains("Test Feed"));
    }

    #[test]
    fn test_generator_viewer_state() {
        let viewer_state = GeneratorViewerState {
            like: Some("at://did:plc:abc/app.bsky.feed.like/123".to_string()),
        };

        let json = serde_json::to_string(&viewer_state).unwrap();
        let deserialized: GeneratorViewerState = serde_json::from_str(&json).unwrap();
        assert_eq!(viewer_state, deserialized);
    }

    #[test]
    fn test_custom_feed_is_bluesky_owned() {
        use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let config = XrpcClientConfig::default();
        let client = Arc::new(RwLock::new(XrpcClient::new(config)));

        // Bluesky-owned feed
        let bluesky_feed = CustomFeed::new(
            client.clone(),
            "at://did:plc:z72i7hdynmk6r22z27h6tvur/app.bsky.feed.generator/whats-hot",
            FeedPreferences::default(),
        );
        assert!(bluesky_feed.is_bluesky_owned());

        // Non-Bluesky feed
        let custom_feed = CustomFeed::new(
            client,
            "at://did:plc:abc123/app.bsky.feed.generator/my-feed",
            FeedPreferences::default(),
        );
        assert!(!custom_feed.is_bluesky_owned());
    }

    #[test]
    fn test_custom_feed_uri() {
        use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let config = XrpcClientConfig::default();
        let client = Arc::new(RwLock::new(XrpcClient::new(config)));
        let feed_uri = "at://did:plc:abc/app.bsky.feed.generator/test";

        let feed = CustomFeed::new(client, feed_uri, FeedPreferences::default());

        assert_eq!(feed.uri(), feed_uri);
    }

    // Hashtag Feed Tests

    #[test]
    fn test_hashtag_feed_params_new() {
        let params = HashtagFeedParams::new("rust");
        assert_eq!(params.hashtag, "rust");
        assert_eq!(params.cursor, None);
        assert_eq!(params.limit, 50);
        assert_eq!(params.sort, HashtagFeedSort::Top);
    }

    #[test]
    fn test_hashtag_feed_params_strips_hash() {
        let params = HashtagFeedParams::new("#rust");
        assert_eq!(params.hashtag, "rust");
    }

    #[test]
    fn test_hashtag_feed_params_with_cursor() {
        let params = HashtagFeedParams::new("rust")
            .with_cursor(Some("cursor123".to_string()));
        assert_eq!(params.cursor, Some("cursor123".to_string()));
    }

    #[test]
    fn test_hashtag_feed_params_with_limit() {
        let params = HashtagFeedParams::new("rust").with_limit(25);
        assert_eq!(params.limit, 25);
    }

    #[test]
    fn test_hashtag_feed_params_limit_cap() {
        // Limit should be capped at 100
        let params = HashtagFeedParams::new("rust").with_limit(150);
        assert_eq!(params.limit, 100);
    }

    #[test]
    fn test_hashtag_feed_params_with_sort() {
        let params = HashtagFeedParams::new("rust")
            .with_sort(HashtagFeedSort::Latest);
        assert_eq!(params.sort, HashtagFeedSort::Latest);
    }

    #[test]
    fn test_hashtag_feed_params_builder_chain() {
        let params = HashtagFeedParams::new("#bluesky")
            .with_cursor(Some("abc".to_string()))
            .with_limit(10)
            .with_sort(HashtagFeedSort::Latest);

        assert_eq!(params.hashtag, "bluesky");
        assert_eq!(params.cursor, Some("abc".to_string()));
        assert_eq!(params.limit, 10);
        assert_eq!(params.sort, HashtagFeedSort::Latest);
    }

    #[test]
    fn test_hashtag_feed_sort_as_str() {
        assert_eq!(HashtagFeedSort::Top.as_str(), "top");
        assert_eq!(HashtagFeedSort::Latest.as_str(), "latest");
    }

    #[test]
    fn test_hashtag_feed_sort_default() {
        let sort = HashtagFeedSort::default();
        assert_eq!(sort, HashtagFeedSort::Top);
    }

    #[tokio::test]
    async fn test_hashtag_feed_empty_hashtag_error() {
        use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let config = XrpcClientConfig::default();
        let client = Arc::new(RwLock::new(XrpcClient::new(config)));
        let feed = HashtagFeed::new(client);

        let params = HashtagFeedParams::new("");
        let result = feed.fetch(params).await;

        assert!(result.is_err());
        match result {
            Err(FeedError::ApiError(msg)) => {
                assert_eq!(msg, "Hashtag cannot be empty");
            }
            _ => panic!("Expected ApiError for empty hashtag"),
        }
    }

    #[tokio::test]
    async fn test_hashtag_feed_whitespace_hashtag_error() {
        use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let config = XrpcClientConfig::default();
        let client = Arc::new(RwLock::new(XrpcClient::new(config)));
        let feed = HashtagFeed::new(client);

        let params = HashtagFeedParams::new("   ");
        let result = feed.fetch(params).await;

        assert!(result.is_err());
        match result {
            Err(FeedError::ApiError(msg)) => {
                assert_eq!(msg, "Hashtag cannot be empty");
            }
            _ => panic!("Expected ApiError for whitespace hashtag"),
        }
    }

    #[test]
    fn test_hashtag_feed_new() {
        use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let config = XrpcClientConfig::default();
        let client = Arc::new(RwLock::new(XrpcClient::new(config)));

        let _feed = HashtagFeed::new(client);
        // Test passes if HashtagFeed::new doesn't panic
    }

    #[test]
    fn test_hashtag_params_multiple_words() {
        let params = HashtagFeedParams::new("rust programming");
        assert_eq!(params.hashtag, "rust programming");
    }

    #[test]
    fn test_hashtag_params_special_chars() {
        let params = HashtagFeedParams::new("rust-lang");
        assert_eq!(params.hashtag, "rust-lang");
    }

    #[test]
    fn test_hashtag_feed_sort_equality() {
        assert_eq!(HashtagFeedSort::Top, HashtagFeedSort::Top);
        assert_eq!(HashtagFeedSort::Latest, HashtagFeedSort::Latest);
        assert_ne!(HashtagFeedSort::Top, HashtagFeedSort::Latest);
    }

    #[test]
    fn test_hashtag_feed_sort_clone() {
        let sort1 = HashtagFeedSort::Latest;
        let sort2 = sort1;
        assert_eq!(sort1, sort2);
    }

    #[test]
    fn test_hashtag_params_clone() {
        let params1 = HashtagFeedParams::new("rust")
            .with_limit(25)
            .with_sort(HashtagFeedSort::Latest);

        let params2 = params1.clone();

        assert_eq!(params1.hashtag, params2.hashtag);
        assert_eq!(params1.limit, params2.limit);
        assert_eq!(params1.sort, params2.sort);
        assert_eq!(params1.cursor, params2.cursor);
    }
}

// ============================================================================
// Pinned Feeds System
// ============================================================================

/// Pinned feed metadata
///
/// Represents a feed that has been pinned by the user for quick access.
/// Pinned feeds appear in a customizable order at the top of the feed list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedFeed {
    /// AT URI of the feed (for custom feeds) or special identifier (like "following")
    pub uri: String,

    /// Display name of the feed
    pub display_name: Option<String>,

    /// Feed type for UI differentiation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feed_type: Option<PinnedFeedType>,

    /// Position in the pinned feeds list (0-indexed)
    pub position: usize,

    /// Timestamp when the feed was pinned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<String>,
}

/// Type of pinned feed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PinnedFeedType {
    /// Following/timeline feed
    Following,
    /// Custom algorithm feed
    Custom,
    /// List feed
    List,
    /// Hashtag feed
    Hashtag,
}

impl PinnedFeed {
    /// Create a new pinned feed entry
    ///
    /// # Arguments
    ///
    /// * `uri` - Feed URI
    /// * `position` - Position in the pinned feeds list
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeed;
    /// let feed = PinnedFeed::new("at://did:plc:abc/app.bsky.feed.generator/tech", 0);
    /// assert_eq!(feed.position, 0);
    /// ```
    pub fn new(uri: impl Into<String>, position: usize) -> Self {
        Self {
            uri: uri.into(),
            display_name: None,
            feed_type: None,
            position,
            pinned_at: None,
        }
    }

    /// Create a pinned feed with display name
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Create a pinned feed with type
    pub fn with_type(mut self, feed_type: PinnedFeedType) -> Self {
        self.feed_type = Some(feed_type);
        self
    }

    /// Create a pinned feed with timestamp
    pub fn with_timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.pinned_at = Some(timestamp.into());
        self
    }
}

/// Result type for pinned feeds operations
pub type PinnedFeedsResult<T> = std::result::Result<T, PinnedFeedsError>;

/// Errors that can occur during pinned feeds operations
#[derive(Debug, thiserror::Error)]
pub enum PinnedFeedsError {
    /// Feed is already pinned
    #[error("Feed is already pinned: {0}")]
    AlreadyPinned(String),

    /// Feed is not pinned
    #[error("Feed is not pinned: {0}")]
    NotPinned(String),

    /// Invalid index for reordering
    #[error("Invalid index: {0}")]
    InvalidIndex(usize),

    /// Maximum pinned feeds limit reached
    #[error("Maximum pinned feeds limit reached (max: {0})")]
    LimitReached(usize),

    /// Invalid feed URI
    #[error("Invalid feed URI: {0}")]
    InvalidUri(String),
}

/// Manager for pinned feeds operations
///
/// Provides a high-level API for managing pinned feeds including
/// pinning, unpinning, reordering, and querying pinned status.
///
/// # Example
///
/// ```
/// # use app_core::feeds::PinnedFeedsManager;
/// let manager = PinnedFeedsManager::new();
/// // Pin a feed
/// manager.pin("at://did:plc:abc/app.bsky.feed.generator/tech")?;
/// // Check if pinned
/// assert!(manager.is_pinned("at://did:plc:abc/app.bsky.feed.generator/tech"));
/// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
/// ```
pub struct PinnedFeedsManager {
    /// List of pinned feed URIs in order
    pinned_uris: Vec<String>,

    /// Maximum number of pinned feeds allowed
    max_pinned: usize,
}

impl PinnedFeedsManager {
    /// Default maximum number of pinned feeds
    pub const DEFAULT_MAX_PINNED: usize = 20;

    /// Create a new pinned feeds manager
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let manager = PinnedFeedsManager::new();
    /// assert_eq!(manager.count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            pinned_uris: Vec::new(),
            max_pinned: Self::DEFAULT_MAX_PINNED,
        }
    }

    /// Create a manager with a custom maximum
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let manager = PinnedFeedsManager::with_max(10);
    /// ```
    pub fn with_max(max_pinned: usize) -> Self {
        Self {
            pinned_uris: Vec::new(),
            max_pinned,
        }
    }

    /// Create a manager from existing pinned feeds
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let feeds = vec!["feed1".to_string(), "feed2".to_string()];
    /// let manager = PinnedFeedsManager::from_uris(feeds);
    /// assert_eq!(manager.count(), 2);
    /// ```
    pub fn from_uris(uris: Vec<String>) -> Self {
        Self {
            pinned_uris: uris,
            max_pinned: Self::DEFAULT_MAX_PINNED,
        }
    }

    /// Pin a feed
    ///
    /// Adds the feed to the end of the pinned feeds list.
    ///
    /// # Errors
    ///
    /// Returns `PinnedFeedsError::AlreadyPinned` if the feed is already pinned.
    /// Returns `PinnedFeedsError::LimitReached` if the maximum number of pinned feeds is reached.
    /// Returns `PinnedFeedsError::InvalidUri` if the URI is empty.
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("at://did:plc:abc/app.bsky.feed.generator/tech")?;
    /// assert!(manager.is_pinned("at://did:plc:abc/app.bsky.feed.generator/tech"));
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn pin(&mut self, uri: impl Into<String>) -> PinnedFeedsResult<()> {
        let uri = uri.into();

        // Validate URI
        if uri.trim().is_empty() {
            return Err(PinnedFeedsError::InvalidUri(uri));
        }

        // Check if already pinned
        if self.pinned_uris.contains(&uri) {
            return Err(PinnedFeedsError::AlreadyPinned(uri));
        }

        // Check limit
        if self.pinned_uris.len() >= self.max_pinned {
            return Err(PinnedFeedsError::LimitReached(self.max_pinned));
        }

        self.pinned_uris.push(uri);
        Ok(())
    }

    /// Unpin a feed
    ///
    /// Removes the feed from the pinned feeds list.
    ///
    /// # Errors
    ///
    /// Returns `PinnedFeedsError::NotPinned` if the feed is not pinned.
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("feed1")?;
    /// manager.unpin("feed1")?;
    /// assert!(!manager.is_pinned("feed1"));
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn unpin(&mut self, uri: &str) -> PinnedFeedsResult<()> {
        let pos = self
            .pinned_uris
            .iter()
            .position(|u| u == uri)
            .ok_or_else(|| PinnedFeedsError::NotPinned(uri.to_string()))?;

        self.pinned_uris.remove(pos);
        Ok(())
    }

    /// Check if a feed is pinned
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("feed1")?;
    /// assert!(manager.is_pinned("feed1"));
    /// assert!(!manager.is_pinned("feed2"));
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn is_pinned(&self, uri: &str) -> bool {
        self.pinned_uris.contains(&uri.to_string())
    }

    /// Get the position of a pinned feed (0-indexed)
    ///
    /// Returns `None` if the feed is not pinned.
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("feed1")?;
    /// manager.pin("feed2")?;
    /// assert_eq!(manager.position("feed1"), Some(0));
    /// assert_eq!(manager.position("feed2"), Some(1));
    /// assert_eq!(manager.position("feed3"), None);
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn position(&self, uri: &str) -> Option<usize> {
        self.pinned_uris.iter().position(|u| u == uri)
    }

    /// Get all pinned feed URIs in order
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("feed1")?;
    /// manager.pin("feed2")?;
    /// assert_eq!(manager.list(), &["feed1".to_string(), "feed2".to_string()]);
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn list(&self) -> &[String] {
        &self.pinned_uris
    }

    /// Get the number of pinned feeds
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// assert_eq!(manager.count(), 0);
    /// manager.pin("feed1")?;
    /// assert_eq!(manager.count(), 1);
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn count(&self) -> usize {
        self.pinned_uris.len()
    }

    /// Check if any feeds are pinned
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let manager = PinnedFeedsManager::new();
    /// assert!(manager.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.pinned_uris.is_empty()
    }

    /// Reorder a pinned feed
    ///
    /// Moves a feed from one position to another in the pinned feeds list.
    ///
    /// # Arguments
    ///
    /// * `from_index` - Current position of the feed (0-indexed)
    /// * `to_index` - New position for the feed (0-indexed)
    ///
    /// # Errors
    ///
    /// Returns `PinnedFeedsError::InvalidIndex` if either index is out of bounds.
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("feed1")?;
    /// manager.pin("feed2")?;
    /// manager.pin("feed3")?;
    /// manager.reorder(0, 2)?;
    /// assert_eq!(manager.list(), &["feed2".to_string(), "feed3".to_string(), "feed1".to_string()]);
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn reorder(&mut self, from_index: usize, to_index: usize) -> PinnedFeedsResult<()> {
        if from_index >= self.pinned_uris.len() {
            return Err(PinnedFeedsError::InvalidIndex(from_index));
        }
        if to_index >= self.pinned_uris.len() {
            return Err(PinnedFeedsError::InvalidIndex(to_index));
        }

        let feed = self.pinned_uris.remove(from_index);
        self.pinned_uris.insert(to_index, feed);
        Ok(())
    }

    /// Clear all pinned feeds
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("feed1")?;
    /// manager.pin("feed2")?;
    /// manager.clear();
    /// assert!(manager.is_empty());
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn clear(&mut self) {
        self.pinned_uris.clear();
    }

    /// Export pinned feed URIs for persistence
    ///
    /// Returns a cloned vector of all pinned feed URIs.
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("feed1")?;
    /// let uris = manager.export();
    /// assert_eq!(uris, vec!["feed1".to_string()]);
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn export(&self) -> Vec<String> {
        self.pinned_uris.clone()
    }

    /// Import pinned feeds from persistence
    ///
    /// Replaces all current pinned feeds with the provided URIs.
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.import(vec!["feed1".to_string(), "feed2".to_string()]);
    /// assert_eq!(manager.count(), 2);
    /// ```
    pub fn import(&mut self, uris: Vec<String>) {
        self.pinned_uris = uris;
    }

    /// Toggle a feed's pinned status
    ///
    /// Pins the feed if it's not pinned, unpins it if it is.
    /// Returns `true` if the feed is now pinned, `false` if unpinned.
    ///
    /// # Errors
    ///
    /// Returns `PinnedFeedsError::LimitReached` if trying to pin when at max capacity.
    /// Returns `PinnedFeedsError::InvalidUri` if the URI is empty.
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// let pinned = manager.toggle("feed1")?;
    /// assert!(pinned);
    /// let unpinned = manager.toggle("feed1")?;
    /// assert!(!unpinned);
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn toggle(&mut self, uri: impl Into<String>) -> PinnedFeedsResult<bool> {
        let uri = uri.into();

        if self.is_pinned(&uri) {
            self.unpin(&uri)?;
            Ok(false)
        } else {
            self.pin(uri)?;
            Ok(true)
        }
    }

    /// Move a feed up in the pinned list
    ///
    /// Swaps the feed with the one above it. Does nothing if already at the top.
    ///
    /// # Errors
    ///
    /// Returns `PinnedFeedsError::NotPinned` if the feed is not pinned.
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("feed1")?;
    /// manager.pin("feed2")?;
    /// manager.pin("feed3")?;
    /// manager.move_up("feed3")?;
    /// assert_eq!(manager.position("feed3"), Some(1));
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn move_up(&mut self, uri: &str) -> PinnedFeedsResult<()> {
        let pos = self.position(uri).ok_or_else(|| PinnedFeedsError::NotPinned(uri.to_string()))?;

        if pos > 0 {
            self.reorder(pos, pos - 1)?;
        }
        Ok(())
    }

    /// Move a feed down in the pinned list
    ///
    /// Swaps the feed with the one below it. Does nothing if already at the bottom.
    ///
    /// # Errors
    ///
    /// Returns `PinnedFeedsError::NotPinned` if the feed is not pinned.
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::feeds::PinnedFeedsManager;
    /// let mut manager = PinnedFeedsManager::new();
    /// manager.pin("feed1")?;
    /// manager.pin("feed2")?;
    /// manager.pin("feed3")?;
    /// manager.move_down("feed1")?;
    /// assert_eq!(manager.position("feed1"), Some(1));
    /// # Ok::<(), app_core::feeds::PinnedFeedsError>(())
    /// ```
    pub fn move_down(&mut self, uri: &str) -> PinnedFeedsResult<()> {
        let pos = self.position(uri).ok_or_else(|| PinnedFeedsError::NotPinned(uri.to_string()))?;

        if pos < self.pinned_uris.len() - 1 {
            self.reorder(pos, pos + 1)?;
        }
        Ok(())
    }
}

impl Default for PinnedFeedsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod pinned_feeds_tests {
    use super::*;

    #[test]
    fn test_new_manager() {
        let manager = PinnedFeedsManager::new();
        assert_eq!(manager.count(), 0);
        assert!(manager.is_empty());
        assert_eq!(manager.max_pinned, PinnedFeedsManager::DEFAULT_MAX_PINNED);
    }

    #[test]
    fn test_with_max() {
        let manager = PinnedFeedsManager::with_max(5);
        assert_eq!(manager.max_pinned, 5);
    }

    #[test]
    fn test_from_uris() {
        let uris = vec!["feed1".to_string(), "feed2".to_string()];
        let manager = PinnedFeedsManager::from_uris(uris);
        assert_eq!(manager.count(), 2);
        assert!(manager.is_pinned("feed1"));
        assert!(manager.is_pinned("feed2"));
    }

    #[test]
    fn test_pin_feed() {
        let mut manager = PinnedFeedsManager::new();
        assert!(manager.pin("feed1").is_ok());
        assert_eq!(manager.count(), 1);
        assert!(manager.is_pinned("feed1"));
    }

    #[test]
    fn test_pin_already_pinned() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        let result = manager.pin("feed1");
        assert!(result.is_err());
        assert!(matches!(result, Err(PinnedFeedsError::AlreadyPinned(_))));
    }

    #[test]
    fn test_pin_empty_uri() {
        let mut manager = PinnedFeedsManager::new();
        let result = manager.pin("");
        assert!(result.is_err());
        assert!(matches!(result, Err(PinnedFeedsError::InvalidUri(_))));
    }

    #[test]
    fn test_pin_limit_reached() {
        let mut manager = PinnedFeedsManager::with_max(2);
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        let result = manager.pin("feed3");
        assert!(result.is_err());
        assert!(matches!(result, Err(PinnedFeedsError::LimitReached(2))));
    }

    #[test]
    fn test_unpin_feed() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        assert!(manager.unpin("feed1").is_ok());
        assert_eq!(manager.count(), 1);
        assert!(!manager.is_pinned("feed1"));
        assert!(manager.is_pinned("feed2"));
    }

    #[test]
    fn test_unpin_not_pinned() {
        let mut manager = PinnedFeedsManager::new();
        let result = manager.unpin("feed1");
        assert!(result.is_err());
        assert!(matches!(result, Err(PinnedFeedsError::NotPinned(_))));
    }

    #[test]
    fn test_is_pinned() {
        let mut manager = PinnedFeedsManager::new();
        assert!(!manager.is_pinned("feed1"));
        manager.pin("feed1").unwrap();
        assert!(manager.is_pinned("feed1"));
        assert!(!manager.is_pinned("feed2"));
    }

    #[test]
    fn test_position() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        manager.pin("feed3").unwrap();
        assert_eq!(manager.position("feed1"), Some(0));
        assert_eq!(manager.position("feed2"), Some(1));
        assert_eq!(manager.position("feed3"), Some(2));
        assert_eq!(manager.position("feed4"), None);
    }

    #[test]
    fn test_list() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        assert_eq!(manager.list(), &["feed1".to_string(), "feed2".to_string()]);
    }

    #[test]
    fn test_count() {
        let mut manager = PinnedFeedsManager::new();
        assert_eq!(manager.count(), 0);
        manager.pin("feed1").unwrap();
        assert_eq!(manager.count(), 1);
        manager.pin("feed2").unwrap();
        assert_eq!(manager.count(), 2);
        manager.unpin("feed1").unwrap();
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_is_empty() {
        let mut manager = PinnedFeedsManager::new();
        assert!(manager.is_empty());
        manager.pin("feed1").unwrap();
        assert!(!manager.is_empty());
        manager.unpin("feed1").unwrap();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_reorder() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        manager.pin("feed3").unwrap();

        // Move first to last
        manager.reorder(0, 2).unwrap();
        assert_eq!(
            manager.list(),
            &["feed2".to_string(), "feed3".to_string(), "feed1".to_string()]
        );

        // Move last to first
        manager.reorder(2, 0).unwrap();
        assert_eq!(
            manager.list(),
            &["feed1".to_string(), "feed2".to_string(), "feed3".to_string()]
        );

        // Move middle up
        manager.reorder(1, 0).unwrap();
        assert_eq!(
            manager.list(),
            &["feed2".to_string(), "feed1".to_string(), "feed3".to_string()]
        );
    }

    #[test]
    fn test_reorder_invalid_from_index() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        let result = manager.reorder(5, 0);
        assert!(result.is_err());
        assert!(matches!(result, Err(PinnedFeedsError::InvalidIndex(5))));
    }

    #[test]
    fn test_reorder_invalid_to_index() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        let result = manager.reorder(0, 5);
        assert!(result.is_err());
        assert!(matches!(result, Err(PinnedFeedsError::InvalidIndex(5))));
    }

    #[test]
    fn test_reorder_same_index() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        manager.reorder(1, 1).unwrap();
        assert_eq!(manager.list(), &["feed1".to_string(), "feed2".to_string()]);
    }

    #[test]
    fn test_clear() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        assert_eq!(manager.count(), 2);
        manager.clear();
        assert_eq!(manager.count(), 0);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_export() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        let exported = manager.export();
        assert_eq!(exported, vec!["feed1".to_string(), "feed2".to_string()]);
    }

    #[test]
    fn test_import() {
        let mut manager = PinnedFeedsManager::new();
        manager.import(vec!["feed1".to_string(), "feed2".to_string(), "feed3".to_string()]);
        assert_eq!(manager.count(), 3);
        assert!(manager.is_pinned("feed1"));
        assert!(manager.is_pinned("feed2"));
        assert!(manager.is_pinned("feed3"));
    }

    #[test]
    fn test_import_replaces_existing() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("old1").unwrap();
        manager.pin("old2").unwrap();
        manager.import(vec!["new1".to_string(), "new2".to_string()]);
        assert_eq!(manager.count(), 2);
        assert!(!manager.is_pinned("old1"));
        assert!(!manager.is_pinned("old2"));
        assert!(manager.is_pinned("new1"));
        assert!(manager.is_pinned("new2"));
    }

    #[test]
    fn test_toggle_pin() {
        let mut manager = PinnedFeedsManager::new();
        let pinned = manager.toggle("feed1").unwrap();
        assert!(pinned);
        assert!(manager.is_pinned("feed1"));
    }

    #[test]
    fn test_toggle_unpin() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        let pinned = manager.toggle("feed1").unwrap();
        assert!(!pinned);
        assert!(!manager.is_pinned("feed1"));
    }

    #[test]
    fn test_toggle_multiple_times() {
        let mut manager = PinnedFeedsManager::new();
        assert!(manager.toggle("feed1").unwrap());
        assert!(!manager.toggle("feed1").unwrap());
        assert!(manager.toggle("feed1").unwrap());
    }

    #[test]
    fn test_move_up() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        manager.pin("feed3").unwrap();

        manager.move_up("feed3").unwrap();
        assert_eq!(manager.position("feed3"), Some(1));
        assert_eq!(
            manager.list(),
            &["feed1".to_string(), "feed3".to_string(), "feed2".to_string()]
        );
    }

    #[test]
    fn test_move_up_at_top() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();

        manager.move_up("feed1").unwrap();
        assert_eq!(manager.position("feed1"), Some(0));
        assert_eq!(manager.list(), &["feed1".to_string(), "feed2".to_string()]);
    }

    #[test]
    fn test_move_up_not_pinned() {
        let mut manager = PinnedFeedsManager::new();
        let result = manager.move_up("feed1");
        assert!(result.is_err());
        assert!(matches!(result, Err(PinnedFeedsError::NotPinned(_))));
    }

    #[test]
    fn test_move_down() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        manager.pin("feed3").unwrap();

        manager.move_down("feed1").unwrap();
        assert_eq!(manager.position("feed1"), Some(1));
        assert_eq!(
            manager.list(),
            &["feed2".to_string(), "feed1".to_string(), "feed3".to_string()]
        );
    }

    #[test]
    fn test_move_down_at_bottom() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();

        manager.move_down("feed2").unwrap();
        assert_eq!(manager.position("feed2"), Some(1));
        assert_eq!(manager.list(), &["feed1".to_string(), "feed2".to_string()]);
    }

    #[test]
    fn test_move_down_not_pinned() {
        let mut manager = PinnedFeedsManager::new();
        let result = manager.move_down("feed1");
        assert!(result.is_err());
        assert!(matches!(result, Err(PinnedFeedsError::NotPinned(_))));
    }

    #[test]
    fn test_default_trait() {
        let manager = PinnedFeedsManager::default();
        assert_eq!(manager.count(), 0);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_pinned_feed_new() {
        let feed = PinnedFeed::new("at://did:plc:abc/app.bsky.feed.generator/tech", 0);
        assert_eq!(feed.uri, "at://did:plc:abc/app.bsky.feed.generator/tech");
        assert_eq!(feed.position, 0);
        assert!(feed.display_name.is_none());
        assert!(feed.feed_type.is_none());
        assert!(feed.pinned_at.is_none());
    }

    #[test]
    fn test_pinned_feed_builder() {
        let feed = PinnedFeed::new("feed1", 0)
            .with_display_name("Tech Feed")
            .with_type(PinnedFeedType::Custom)
            .with_timestamp("2024-01-01T00:00:00Z");

        assert_eq!(feed.display_name, Some("Tech Feed".to_string()));
        assert_eq!(feed.feed_type, Some(PinnedFeedType::Custom));
        assert_eq!(feed.pinned_at, Some("2024-01-01T00:00:00Z".to_string()));
    }

    #[test]
    fn test_pinned_feed_serialization() {
        let feed = PinnedFeed::new("feed1", 0)
            .with_display_name("Test Feed")
            .with_type(PinnedFeedType::Following);

        let json = serde_json::to_string(&feed).unwrap();
        let deserialized: PinnedFeed = serde_json::from_str(&json).unwrap();

        assert_eq!(feed, deserialized);
        assert!(json.contains("displayName"));
        assert!(json.contains("feedType"));
    }

    #[test]
    fn test_pinned_feed_type_serialization() {
        let types = vec![
            PinnedFeedType::Following,
            PinnedFeedType::Custom,
            PinnedFeedType::List,
            PinnedFeedType::Hashtag,
        ];

        for feed_type in types {
            let json = serde_json::to_string(&feed_type).unwrap();
            let deserialized: PinnedFeedType = serde_json::from_str(&json).unwrap();
            assert_eq!(feed_type, deserialized);
        }
    }

    #[test]
    fn test_complex_reordering_sequence() {
        let mut manager = PinnedFeedsManager::new();
        manager.pin("feed1").unwrap();
        manager.pin("feed2").unwrap();
        manager.pin("feed3").unwrap();
        manager.pin("feed4").unwrap();
        manager.pin("feed5").unwrap();

        // Move feed5 to top
        manager.reorder(4, 0).unwrap();
        assert_eq!(manager.position("feed5"), Some(0));

        // Move feed1 to position 2
        manager.reorder(1, 2).unwrap();
        assert_eq!(manager.position("feed1"), Some(2));

        // Verify final order
        assert_eq!(
            manager.list(),
            &[
                "feed5".to_string(),
                "feed2".to_string(),
                "feed1".to_string(),
                "feed3".to_string(),
                "feed4".to_string(),
            ]
        );
    }

    #[test]
    fn test_pin_with_special_characters_in_uri() {
        let mut manager = PinnedFeedsManager::new();
        let uri = "at://did:plc:abc123xyz/app.bsky.feed.generator/tech-news";
        manager.pin(uri).unwrap();
        assert!(manager.is_pinned(uri));
    }

    #[test]
    fn test_error_display() {
        let error = PinnedFeedsError::AlreadyPinned("feed1".to_string());
        assert_eq!(error.to_string(), "Feed is already pinned: feed1");

        let error = PinnedFeedsError::NotPinned("feed2".to_string());
        assert_eq!(error.to_string(), "Feed is not pinned: feed2");

        let error = PinnedFeedsError::InvalidIndex(5);
        assert_eq!(error.to_string(), "Invalid index: 5");

        let error = PinnedFeedsError::LimitReached(20);
        assert_eq!(error.to_string(), "Maximum pinned feeds limit reached (max: 20)");

        let error = PinnedFeedsError::InvalidUri("".to_string());
        assert_eq!(error.to_string(), "Invalid feed URI: ");
    }
}
