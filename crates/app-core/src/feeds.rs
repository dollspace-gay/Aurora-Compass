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
}
