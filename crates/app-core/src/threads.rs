//! Thread viewing and management
//!
//! This module provides functionality for viewing post threads, including
//! parent context, replies, and thread navigation.

use atproto_client::xrpc::{XrpcClient, XrpcRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::profiles::ProfileViewBasic;

/// Thread service error types
#[derive(Debug, Error)]
pub enum ThreadError {
    /// XRPC error
    #[error("XRPC error: {0}")]
    Xrpc(String),

    /// Post not found
    #[error("Post not found: {0}")]
    NotFound(String),

    /// Thread blocked
    #[error("Thread blocked")]
    Blocked,

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid URI
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
}

/// Result type for thread operations
pub type Result<T> = std::result::Result<T, ThreadError>;

/// Sort order for thread replies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThreadSortOrder {
    /// Oldest first
    Oldest,
    /// Newest first
    Newest,
    /// Most likes first
    MostLikes,
    /// Random order
    Random,
}

impl Default for ThreadSortOrder {
    fn default() -> Self {
        Self::Oldest
    }
}

/// View mode for thread display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreadViewMode {
    /// Tree view with nested replies
    Tree,
    /// Linear view with all replies at same level
    Linear,
}

impl Default for ThreadViewMode {
    fn default() -> Self {
        Self::Tree
    }
}

/// Post record data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostRecord {
    /// Post text
    pub text: String,
    /// Created at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Reply reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<ReplyRef>,
    /// Embed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed: Option<serde_json::Value>,
    /// Facets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facets: Option<Vec<serde_json::Value>>,
    /// Record type
    #[serde(rename = "$type")]
    pub record_type: String,
}

/// Reply reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyRef {
    /// Root post reference
    pub root: StrongRef,
    /// Parent post reference
    pub parent: StrongRef,
}

/// Strong reference to a record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrongRef {
    /// URI of the record
    pub uri: String,
    /// CID of the record
    pub cid: String,
}

/// Post viewer state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerState {
    /// Whether the current user has liked this post
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like: Option<String>,
    /// Whether the current user has reposted this post
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repost: Option<String>,
    /// Whether the current user can reply to this post
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threadgated: Option<bool>,
}

/// Post view with full data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostView {
    /// Post URI
    pub uri: String,
    /// Post CID
    pub cid: String,
    /// Post author
    pub author: ProfileViewBasic,
    /// Post record
    pub record: PostRecord,
    /// Reply count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_count: Option<u32>,
    /// Repost count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repost_count: Option<u32>,
    /// Like count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like_count: Option<u32>,
    /// Indexed at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<String>,
    /// Viewer state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<ViewerState>,
}

/// Thread item post data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadItemPost {
    /// Post view
    pub post: PostView,
    /// Whether this is part of the OP's thread
    #[serde(default)]
    pub op_thread: bool,
    /// Whether there are more parents above
    #[serde(default)]
    pub more_parents: bool,
    /// Number of additional replies not shown
    #[serde(default)]
    pub more_replies: u32,
    /// Whether hidden by threadgate
    #[serde(default)]
    pub hidden_by_threadgate: bool,
    /// Whether muted by viewer
    #[serde(default)]
    pub muted_by_viewer: bool,
}

/// Thread item not found
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadItemNotFound {
    /// URI of the not found post
    pub uri: String,
    /// Whether this was a not found response
    pub not_found: bool,
}

/// Thread item blocked
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadItemBlocked {
    /// URI of the blocked post
    pub uri: String,
    /// Whether this was blocked
    pub blocked: bool,
}

/// Thread item - represents different types of items in a thread
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum ThreadItem {
    /// Regular post in thread
    #[serde(rename = "app.bsky.unspecced.defs#threadItemPost")]
    Post {
        /// URI of the post
        uri: String,
        /// Depth in thread (0 = anchor)
        depth: u32,
        /// Post data
        #[serde(flatten)]
        value: Box<ThreadItemPost>,
    },
    /// Post not found
    #[serde(rename = "app.bsky.unspecced.defs#threadItemNotFound")]
    NotFound {
        /// URI of the post
        uri: String,
        /// Depth in thread
        depth: u32,
        /// Not found data
        #[serde(flatten)]
        value: ThreadItemNotFound,
    },
    /// Post blocked
    #[serde(rename = "app.bsky.unspecced.defs#threadItemBlocked")]
    Blocked {
        /// URI of the post
        uri: String,
        /// Depth in thread
        depth: u32,
        /// Blocked data
        #[serde(flatten)]
        value: ThreadItemBlocked,
    },
}

impl ThreadItem {
    /// Get the URI of this thread item
    pub fn uri(&self) -> &str {
        match self {
            ThreadItem::Post { uri, .. } => uri,
            ThreadItem::NotFound { uri, .. } => uri,
            ThreadItem::Blocked { uri, .. } => uri,
        }
    }

    /// Get the depth of this thread item
    pub fn depth(&self) -> u32 {
        match self {
            ThreadItem::Post { depth, .. } => *depth,
            ThreadItem::NotFound { depth, .. } => *depth,
            ThreadItem::Blocked { depth, .. } => *depth,
        }
    }

    /// Check if this is a post item
    pub fn is_post(&self) -> bool {
        matches!(self, ThreadItem::Post { .. })
    }

    /// Get the post view if this is a post item
    pub fn as_post(&self) -> Option<&PostView> {
        match self {
            ThreadItem::Post { value, .. } => Some(&value.post),
            _ => None,
        }
    }
}

/// Thread response from API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadResponse {
    /// Thread items
    #[serde(default)]
    pub thread: Vec<ThreadItem>,
    /// Whether there are other replies available
    #[serde(default)]
    pub has_other_replies: bool,
    /// Threadgate view
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threadgate: Option<serde_json::Value>,
}

/// Parameters for fetching a thread
#[derive(Debug, Clone)]
pub struct ThreadParams {
    /// URI of the anchor post
    pub anchor: String,
    /// Sort order for replies
    pub sort: ThreadSortOrder,
    /// View mode
    pub view: ThreadViewMode,
    /// Whether to prioritize followed users
    pub prioritize_followed_users: bool,
    /// Number of levels below anchor to fetch
    pub below: u32,
    /// Branching factor (replies per level)
    pub branching_factor: u32,
}

impl Default for ThreadParams {
    fn default() -> Self {
        Self {
            anchor: String::new(),
            sort: ThreadSortOrder::default(),
            view: ThreadViewMode::default(),
            prioritize_followed_users: false,
            below: 20,
            branching_factor: 10,
        }
    }
}

impl ThreadParams {
    /// Create new thread params with an anchor URI
    pub fn new(anchor: impl Into<String>) -> Self {
        Self {
            anchor: anchor.into(),
            ..Default::default()
        }
    }

    /// Set the sort order
    pub fn with_sort(mut self, sort: ThreadSortOrder) -> Self {
        self.sort = sort;
        self
    }

    /// Set the view mode
    pub fn with_view(mut self, view: ThreadViewMode) -> Self {
        self.view = view;
        self
    }

    /// Set whether to prioritize followed users
    pub fn with_prioritize_followed(mut self, prioritize: bool) -> Self {
        self.prioritize_followed_users = prioritize;
        self
    }

    /// Set the number of levels below to fetch
    pub fn with_below(mut self, below: u32) -> Self {
        self.below = below;
        self
    }

    /// Set the branching factor
    pub fn with_branching_factor(mut self, branching_factor: u32) -> Self {
        self.branching_factor = branching_factor;
        self
    }
}

/// Thread service for managing thread operations
///
/// Provides methods for fetching and navigating post threads.
///
/// # Example
///
/// ```rust,no_run
/// use app_core::threads::{ThreadService, ThreadParams, ThreadSortOrder};
/// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create XRPC client (with auth)
///     let config = XrpcClientConfig::new("https://bsky.social");
///     let client = XrpcClient::new(config);
///     let service = ThreadService::new(client);
///
///     // Fetch a thread
///     let params = ThreadParams::new("at://did:plc:abc123/app.bsky.feed.post/xyz456")
///         .with_sort(ThreadSortOrder::Newest);
///
///     let thread = service.get_thread(params).await?;
///     println!("Thread has {} items", thread.thread.len());
///
///     Ok(())
/// }
/// ```
pub struct ThreadService {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl ThreadService {
    /// Create a new thread service
    pub fn new(client: XrpcClient) -> Self {
        Self {
            client: Arc::new(RwLock::new(client)),
        }
    }

    /// Get a post thread
    ///
    /// # Arguments
    ///
    /// * `params` - Thread fetch parameters
    ///
    /// # Returns
    ///
    /// Thread response with items and metadata
    ///
    /// # Errors
    ///
    /// - `ThreadError::NotFound` - Post not found
    /// - `ThreadError::Blocked` - Thread is blocked
    /// - `ThreadError::Network` - Network error
    pub async fn get_thread(&self, params: ThreadParams) -> Result<ThreadResponse> {
        if params.anchor.is_empty() {
            return Err(ThreadError::InvalidUri("Anchor URI cannot be empty".to_string()));
        }

        let request = XrpcRequest::query("app.bsky.unspecced.getPostThreadV2")
            .param("anchor", &params.anchor)
            .param("below", params.below.to_string())
            .param("branchingFactor", params.branching_factor.to_string())
            .param("sort", match params.sort {
                ThreadSortOrder::Oldest => "oldest",
                ThreadSortOrder::Newest => "newest",
                ThreadSortOrder::MostLikes => "most-likes",
                ThreadSortOrder::Random => "random",
            })
            .param("prioritizeFollowedUsers", params.prioritize_followed_users.to_string());

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ThreadError::Xrpc(e.to_string()))?;

        let thread_response: ThreadResponse = serde_json::from_value(response.data)
            .map_err(ThreadError::Serialization)?;

        Ok(thread_response)
    }

    /// Get additional thread replies not included in main response
    ///
    /// # Arguments
    ///
    /// * `anchor` - URI of the anchor post
    /// * `prioritize_followed_users` - Whether to prioritize followed users
    ///
    /// # Returns
    ///
    /// Thread response with additional items
    pub async fn get_other_replies(
        &self,
        anchor: &str,
        prioritize_followed_users: bool,
    ) -> Result<ThreadResponse> {
        if anchor.is_empty() {
            return Err(ThreadError::InvalidUri("Anchor URI cannot be empty".to_string()));
        }

        let request = XrpcRequest::query("app.bsky.unspecced.getPostThreadOtherV2")
            .param("anchor", anchor)
            .param("prioritizeFollowedUsers", prioritize_followed_users.to_string());

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ThreadError::Xrpc(e.to_string()))?;

        let thread_response: ThreadResponse = serde_json::from_value(response.data)
            .map_err(ThreadError::Serialization)?;

        Ok(thread_response)
    }

    /// Find the anchor post in a thread
    ///
    /// The anchor post is the post at depth 0
    pub fn find_anchor(thread: &[ThreadItem]) -> Option<&ThreadItem> {
        thread.iter().find(|item| item.depth() == 0)
    }

    /// Get parent posts (posts above the anchor)
    ///
    /// Returns posts sorted from root to anchor parent
    pub fn get_parents<'a>(thread: &'a [ThreadItem], anchor_uri: &str) -> Vec<&'a ThreadItem> {
        // Find the anchor post first
        let anchor = thread.iter().find(|item| item.uri() == anchor_uri);

        if anchor.is_none() {
            return Vec::new();
        }

        // In the V2 API, parents have negative depth or are ordered before the anchor
        // For simplicity, we'll collect all posts that come before the anchor in the array
        // and have depth < anchor depth
        let anchor_depth = anchor.unwrap().depth();
        thread
            .iter()
            .filter(|item| item.depth() < anchor_depth && item.uri() != anchor_uri)
            .collect()
    }

    /// Get reply posts (posts below the anchor)
    ///
    /// Returns posts sorted by the server's ordering
    pub fn get_replies<'a>(thread: &'a [ThreadItem], anchor_uri: &str) -> Vec<&'a ThreadItem> {
        // Find the anchor post first
        let anchor = thread.iter().find(|item| item.uri() == anchor_uri);

        if anchor.is_none() {
            return Vec::new();
        }

        let anchor_depth = anchor.unwrap().depth();
        thread
            .iter()
            .filter(|item| item.depth() > anchor_depth)
            .collect()
    }

    /// Order replies by sort preference
    ///
    /// Note: The server already does most of the sorting, this is for client-side
    /// re-ordering if needed
    pub fn order_replies(mut replies: Vec<&ThreadItem>, sort: ThreadSortOrder) -> Vec<&ThreadItem> {
        match sort {
            ThreadSortOrder::Oldest => {
                // Already in order from server, but ensure by indexed_at
                replies.sort_by(|a, b| {
                    let a_time = a.as_post().and_then(|p| p.indexed_at.as_ref());
                    let b_time = b.as_post().and_then(|p| p.indexed_at.as_ref());
                    a_time.cmp(&b_time)
                });
                replies
            }
            ThreadSortOrder::Newest => {
                replies.sort_by(|a, b| {
                    let a_time = a.as_post().and_then(|p| p.indexed_at.as_ref());
                    let b_time = b.as_post().and_then(|p| p.indexed_at.as_ref());
                    b_time.cmp(&a_time)
                });
                replies
            }
            ThreadSortOrder::MostLikes => {
                replies.sort_by(|a, b| {
                    let a_likes = a.as_post().and_then(|p| p.like_count).unwrap_or(0);
                    let b_likes = b.as_post().and_then(|p| p.like_count).unwrap_or(0);
                    b_likes.cmp(&a_likes)
                });
                replies
            }
            ThreadSortOrder::Random => {
                // Server handles random ordering
                replies
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_params_builder() {
        let params = ThreadParams::new("at://did:plc:test/app.bsky.feed.post/123")
            .with_sort(ThreadSortOrder::Newest)
            .with_view(ThreadViewMode::Linear)
            .with_prioritize_followed(true)
            .with_below(30)
            .with_branching_factor(15);

        assert_eq!(params.anchor, "at://did:plc:test/app.bsky.feed.post/123");
        assert_eq!(params.sort, ThreadSortOrder::Newest);
        assert_eq!(params.view, ThreadViewMode::Linear);
        assert!(params.prioritize_followed_users);
        assert_eq!(params.below, 30);
        assert_eq!(params.branching_factor, 15);
    }

    #[test]
    fn test_thread_sort_order_default() {
        assert_eq!(ThreadSortOrder::default(), ThreadSortOrder::Oldest);
    }

    #[test]
    fn test_thread_view_mode_default() {
        assert_eq!(ThreadViewMode::default(), ThreadViewMode::Tree);
    }

    #[test]
    fn test_thread_item_uri() {
        let post_item = ThreadItem::Post {
            uri: "at://did:plc:test/app.bsky.feed.post/123".to_string(),
            depth: 0,
            value: Box::new(ThreadItemPost {
                post: create_test_post(),
                op_thread: false,
                more_parents: false,
                more_replies: 0,
                hidden_by_threadgate: false,
                muted_by_viewer: false,
            }),
        };

        assert_eq!(post_item.uri(), "at://did:plc:test/app.bsky.feed.post/123");
        assert_eq!(post_item.depth(), 0);
        assert!(post_item.is_post());
    }

    #[test]
    fn test_thread_item_depth() {
        let item = ThreadItem::Post {
            uri: "at://test".to_string(),
            depth: 2,
            value: Box::new(ThreadItemPost {
                post: create_test_post(),
                op_thread: false,
                more_parents: false,
                more_replies: 0,
                hidden_by_threadgate: false,
                muted_by_viewer: false,
            }),
        };

        assert_eq!(item.depth(), 2);
    }

    #[test]
    fn test_find_anchor() {
        let thread = vec![
            create_thread_item("at://parent", 1),
            create_thread_item("at://anchor", 0),
            create_thread_item("at://reply", 2),
        ];

        let anchor = ThreadService::find_anchor(&thread);
        assert!(anchor.is_some());
        assert_eq!(anchor.unwrap().uri(), "at://anchor");
        assert_eq!(anchor.unwrap().depth(), 0);
    }

    #[test]
    fn test_get_parents() {
        let thread = vec![
            create_thread_item("at://root", 0),
            create_thread_item("at://child1", 1),
            create_thread_item("at://child2", 1),
        ];

        let parents = ThreadService::get_parents(&thread, "at://child1");
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].uri(), "at://root");
    }

    #[test]
    fn test_get_replies() {
        let thread = vec![
            create_thread_item("at://anchor", 0),
            create_thread_item("at://reply1", 1),
            create_thread_item("at://reply2", 1),
            create_thread_item("at://reply3", 2),
        ];

        let replies = ThreadService::get_replies(&thread, "at://anchor");
        assert_eq!(replies.len(), 3);
    }

    #[test]
    fn test_thread_item_not_found() {
        let item = ThreadItem::NotFound {
            uri: "at://test".to_string(),
            depth: 1,
            value: ThreadItemNotFound {
                uri: "at://test".to_string(),
                not_found: true,
            },
        };

        assert_eq!(item.uri(), "at://test");
        assert!(!item.is_post());
        assert!(item.as_post().is_none());
    }

    #[test]
    fn test_thread_item_blocked() {
        let item = ThreadItem::Blocked {
            uri: "at://test".to_string(),
            depth: 1,
            value: ThreadItemBlocked {
                uri: "at://test".to_string(),
                blocked: true,
            },
        };

        assert_eq!(item.uri(), "at://test");
        assert!(!item.is_post());
    }

    #[test]
    fn test_post_view_serialization() {
        let post = create_test_post();
        let json = serde_json::to_string(&post).unwrap();
        let deserialized: PostView = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.uri, post.uri);
    }

    #[test]
    fn test_thread_response_serialization() {
        let response = ThreadResponse {
            thread: vec![create_thread_item("at://test", 0)],
            has_other_replies: true,
            threadgate: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ThreadResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.thread.len(), 1);
        assert!(deserialized.has_other_replies);
    }

    // Helper functions for tests
    fn create_test_post() -> PostView {
        PostView {
            uri: "at://did:plc:test/app.bsky.feed.post/123".to_string(),
            cid: "bafytest123".to_string(),
            author: ProfileViewBasic {
                did: "did:plc:test".to_string(),
                handle: "test.bsky.social".to_string(),
                display_name: Some("Test User".to_string()),
                avatar: None,
                associated: None,
                viewer: None,
                labels: None,
                created_at: None,
            },
            record: PostRecord {
                text: "Test post".to_string(),
                created_at: Some("2024-01-01T00:00:00Z".to_string()),
                reply: None,
                embed: None,
                facets: None,
                record_type: "app.bsky.feed.post".to_string(),
            },
            reply_count: Some(0),
            repost_count: Some(0),
            like_count: Some(0),
            indexed_at: Some("2024-01-01T00:00:00Z".to_string()),
            viewer: None,
        }
    }

    fn create_thread_item(uri: &str, depth: u32) -> ThreadItem {
        ThreadItem::Post {
            uri: uri.to_string(),
            depth,
            value: Box::new(ThreadItemPost {
                post: create_test_post(),
                op_thread: false,
                more_parents: false,
                more_replies: 0,
                hidden_by_threadgate: false,
                muted_by_viewer: false,
            }),
        }
    }
}
