//! Bookmark (saved posts) management
//!
//! This module provides functionality for saving and managing bookmarked posts.
//! Bookmarks allow users to save posts for later viewing.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::feeds::PostView;
use atproto_client::xrpc::XrpcClient;
use atproto_client::XrpcRequest;

/// Maximum bookmarks per page
const PAGE_SIZE: u32 = 25;

/// Errors that can occur during bookmark operations
#[derive(Debug, thiserror::Error)]
pub enum BookmarkError {
    /// API error
    #[error("API error: {0}")]
    ApiError(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    /// Bookmark not found
    #[error("Bookmark not found: {0}")]
    NotFound(String),

    /// No active session
    #[error("No active session")]
    NoSession,
}

/// Result type for bookmark operations
pub type Result<T> = std::result::Result<T, BookmarkError>;

/// A reference to a bookmarked subject (post)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkSubject {
    /// Post URI
    pub uri: String,

    /// Post CID
    pub cid: String,
}

/// A bookmark entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    /// When the bookmark was created
    pub created_at: String,

    /// The bookmarked subject reference
    pub subject: BookmarkSubject,

    /// The bookmarked post (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<PostView>,
}

impl Bookmark {
    /// Get the URI of the bookmarked post
    pub fn post_uri(&self) -> &str {
        &self.subject.uri
    }

    /// Get the CID of the bookmarked post
    pub fn post_cid(&self) -> &str {
        &self.subject.cid
    }

    /// Check if the post content is available
    pub fn has_content(&self) -> bool {
        self.item.is_some()
    }
}

/// Response from getBookmarks API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBookmarksResponse {
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// List of bookmarks
    pub bookmarks: Vec<Bookmark>,
}

impl GetBookmarksResponse {
    /// Check if there are more pages
    pub fn has_more(&self) -> bool {
        self.cursor.is_some()
    }

    /// Get the number of bookmarks in this page
    pub fn count(&self) -> usize {
        self.bookmarks.len()
    }
}

/// Parameters for creating a bookmark
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBookmarkParams {
    /// URI of the post to bookmark
    pub uri: String,

    /// CID of the post to bookmark
    pub cid: String,
}

/// Parameters for deleting a bookmark
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteBookmarkParams {
    /// URI of the post to unbookmark
    pub uri: String,
}

/// Bookmark service for managing saved posts
pub struct BookmarkService {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl BookmarkService {
    /// Create a new bookmark service
    pub fn new(client: Arc<RwLock<XrpcClient>>) -> Self {
        BookmarkService { client }
    }

    /// Get bookmarks with pagination
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::bookmarks::BookmarkService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let service = BookmarkService::new(client);
    ///
    /// // Get first page of bookmarks
    /// let page = service.get_bookmarks(None, None).await?;
    /// for bookmark in page.bookmarks {
    ///     println!("Saved: {}", bookmark.post_uri());
    /// }
    ///
    /// // Get next page if available
    /// if let Some(cursor) = page.cursor {
    ///     let next_page = service.get_bookmarks(Some(cursor), None).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_bookmarks(
        &self,
        cursor: Option<String>,
        limit: Option<u32>,
    ) -> Result<GetBookmarksResponse> {
        let client = self.client.read().await;

        let mut request = XrpcRequest::query("app.bsky.bookmark.getBookmarks")
            .param("limit", limit.unwrap_or(PAGE_SIZE).to_string());

        if let Some(c) = cursor {
            request = request.param("cursor", c);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| BookmarkError::ApiError(e.to_string()))?;

        let data: GetBookmarksResponse = serde_json::from_value(response.data)?;
        Ok(data)
    }

    /// Create a bookmark for a post
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::bookmarks::{BookmarkService, CreateBookmarkParams};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let service = BookmarkService::new(client);
    ///
    /// service.create_bookmark(CreateBookmarkParams {
    ///     uri: "at://did:plc:abc/app.bsky.feed.post/123".to_string(),
    ///     cid: "bafyreib...".to_string(),
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_bookmark(&self, params: CreateBookmarkParams) -> Result<()> {
        let client = self.client.read().await;

        let request = XrpcRequest::procedure("app.bsky.bookmark.createBookmark")
            .json_body(&params)
            .map_err(|e| BookmarkError::ApiError(e.to_string()))?;

        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| BookmarkError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Delete a bookmark
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::bookmarks::{BookmarkService, DeleteBookmarkParams};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let service = BookmarkService::new(client);
    ///
    /// service.delete_bookmark(DeleteBookmarkParams {
    ///     uri: "at://did:plc:abc/app.bsky.feed.post/123".to_string(),
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete_bookmark(&self, params: DeleteBookmarkParams) -> Result<()> {
        let client = self.client.read().await;

        let request = XrpcRequest::procedure("app.bsky.bookmark.deleteBookmark")
            .json_body(&params)
            .map_err(|e| BookmarkError::ApiError(e.to_string()))?;

        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| BookmarkError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Toggle bookmark status for a post
    ///
    /// Returns true if the post is now bookmarked, false if unbookmarked.
    pub async fn toggle_bookmark(&self, uri: &str, cid: &str, is_bookmarked: bool) -> Result<bool> {
        if is_bookmarked {
            self.delete_bookmark(DeleteBookmarkParams { uri: uri.to_string() })
                .await?;
            Ok(false)
        } else {
            self.create_bookmark(CreateBookmarkParams {
                uri: uri.to_string(),
                cid: cid.to_string(),
            })
            .await?;
            Ok(true)
        }
    }

    /// Check if a post is bookmarked
    ///
    /// Note: This requires fetching bookmarks, which may not be efficient.
    /// Consider using cached state from PostView.viewer.bookmarked instead.
    pub async fn is_bookmarked(&self, uri: &str) -> Result<bool> {
        let mut cursor: Option<String> = None;

        // Search through all bookmarks (up to 10 pages)
        for _ in 0..10 {
            let response = self.get_bookmarks(cursor, Some(50)).await?;

            for bookmark in &response.bookmarks {
                if bookmark.subject.uri == uri {
                    return Ok(true);
                }
            }

            if response.cursor.is_none() {
                break;
            }
            cursor = response.cursor;
        }

        Ok(false)
    }

    /// Get all bookmarks (paginating through all results)
    ///
    /// Warning: This may make multiple API calls for users with many bookmarks.
    pub async fn get_all_bookmarks(&self) -> Result<Vec<Bookmark>> {
        let mut all_bookmarks = Vec::new();
        let mut cursor: Option<String> = None;

        // Limit to 20 pages (500 bookmarks) to prevent infinite loops
        for _ in 0..20 {
            let response = self.get_bookmarks(cursor, Some(PAGE_SIZE)).await?;
            all_bookmarks.extend(response.bookmarks);

            if response.cursor.is_none() {
                break;
            }
            cursor = response.cursor;
        }

        Ok(all_bookmarks)
    }
}

/// Optimistic update helper for bookmark state
#[derive(Debug, Clone, Default)]
pub struct BookmarkCache {
    /// URIs of bookmarked posts (for quick lookup)
    bookmarked_uris: std::collections::HashSet<String>,
}

impl BookmarkCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        BookmarkCache { bookmarked_uris: std::collections::HashSet::new() }
    }

    /// Add a bookmark to the cache
    pub fn add(&mut self, uri: &str) {
        self.bookmarked_uris.insert(uri.to_string());
    }

    /// Remove a bookmark from the cache
    pub fn remove(&mut self, uri: &str) {
        self.bookmarked_uris.remove(uri);
    }

    /// Check if a URI is bookmarked
    pub fn contains(&self, uri: &str) -> bool {
        self.bookmarked_uris.contains(uri)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.bookmarked_uris.clear();
    }

    /// Get the number of cached bookmarks
    pub fn len(&self) -> usize {
        self.bookmarked_uris.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.bookmarked_uris.is_empty()
    }

    /// Populate the cache from a list of bookmarks
    pub fn populate(&mut self, bookmarks: &[Bookmark]) {
        self.bookmarked_uris.clear();
        for bookmark in bookmarks {
            self.bookmarked_uris.insert(bookmark.subject.uri.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bookmark(uri: &str, cid: &str, created_at: &str) -> Bookmark {
        Bookmark {
            created_at: created_at.to_string(),
            subject: BookmarkSubject { uri: uri.to_string(), cid: cid.to_string() },
            item: None,
        }
    }

    #[test]
    fn test_bookmark_post_uri() {
        let bookmark = make_bookmark(
            "at://did:plc:abc/app.bsky.feed.post/123",
            "bafyreib",
            "2024-01-15T10:00:00Z",
        );

        assert_eq!(bookmark.post_uri(), "at://did:plc:abc/app.bsky.feed.post/123");
        assert_eq!(bookmark.post_cid(), "bafyreib");
    }

    #[test]
    fn test_bookmark_has_content() {
        let bookmark = make_bookmark(
            "at://did:plc:abc/app.bsky.feed.post/123",
            "bafyreib",
            "2024-01-15T10:00:00Z",
        );

        assert!(!bookmark.has_content());
    }

    #[test]
    fn test_get_bookmarks_response_has_more() {
        let response = GetBookmarksResponse {
            cursor: Some("cursor123".to_string()),
            bookmarks: vec![make_bookmark(
                "at://did:plc:abc/app.bsky.feed.post/123",
                "bafyreib",
                "2024-01-15T10:00:00Z",
            )],
        };

        assert!(response.has_more());
        assert_eq!(response.count(), 1);

        let no_more = GetBookmarksResponse { cursor: None, bookmarks: vec![] };

        assert!(!no_more.has_more());
        assert_eq!(no_more.count(), 0);
    }

    #[test]
    fn test_create_bookmark_params_serialization() {
        let params = CreateBookmarkParams {
            uri: "at://did:plc:abc/app.bsky.feed.post/123".to_string(),
            cid: "bafyreib".to_string(),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["uri"], "at://did:plc:abc/app.bsky.feed.post/123");
        assert_eq!(json["cid"], "bafyreib");
    }

    #[test]
    fn test_delete_bookmark_params_serialization() {
        let params = DeleteBookmarkParams {
            uri: "at://did:plc:abc/app.bsky.feed.post/123".to_string(),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["uri"], "at://did:plc:abc/app.bsky.feed.post/123");
    }

    #[test]
    fn test_bookmark_cache_new() {
        let cache = BookmarkCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_bookmark_cache_add_remove() {
        let mut cache = BookmarkCache::new();

        cache.add("at://did:plc:abc/app.bsky.feed.post/123");
        assert!(cache.contains("at://did:plc:abc/app.bsky.feed.post/123"));
        assert!(!cache.contains("at://did:plc:abc/app.bsky.feed.post/456"));
        assert_eq!(cache.len(), 1);

        cache.add("at://did:plc:abc/app.bsky.feed.post/456");
        assert_eq!(cache.len(), 2);

        cache.remove("at://did:plc:abc/app.bsky.feed.post/123");
        assert!(!cache.contains("at://did:plc:abc/app.bsky.feed.post/123"));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_bookmark_cache_clear() {
        let mut cache = BookmarkCache::new();

        cache.add("at://did:plc:abc/app.bsky.feed.post/123");
        cache.add("at://did:plc:abc/app.bsky.feed.post/456");
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_bookmark_cache_populate() {
        let mut cache = BookmarkCache::new();

        let bookmarks = vec![
            make_bookmark(
                "at://did:plc:abc/app.bsky.feed.post/123",
                "bafyreib1",
                "2024-01-15T10:00:00Z",
            ),
            make_bookmark(
                "at://did:plc:abc/app.bsky.feed.post/456",
                "bafyreib2",
                "2024-01-15T11:00:00Z",
            ),
        ];

        cache.populate(&bookmarks);
        assert_eq!(cache.len(), 2);
        assert!(cache.contains("at://did:plc:abc/app.bsky.feed.post/123"));
        assert!(cache.contains("at://did:plc:abc/app.bsky.feed.post/456"));
    }

    #[test]
    fn test_bookmark_subject_serialization() {
        let subject = BookmarkSubject {
            uri: "at://did:plc:abc/app.bsky.feed.post/123".to_string(),
            cid: "bafyreib".to_string(),
        };

        let json = serde_json::to_string(&subject).unwrap();
        let parsed: BookmarkSubject = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.uri, subject.uri);
        assert_eq!(parsed.cid, subject.cid);
    }

    #[test]
    fn test_bookmark_cache_default() {
        let cache: BookmarkCache = Default::default();
        assert!(cache.is_empty());
    }
}
