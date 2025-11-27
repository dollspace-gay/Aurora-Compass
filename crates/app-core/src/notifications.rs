//! Notification management
//!
//! This module provides functionality for fetching, grouping, and managing
//! activity notifications including likes, reposts, follows, mentions, quotes,
//! replies, and starter pack joins.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::feeds::{Label, PostView};
use crate::profiles::ProfileViewBasic;
use atproto_client::xrpc::XrpcClient;
use atproto_client::XrpcRequest;

/// Time window for grouping notifications (48 hours in milliseconds)
const GROUPING_WINDOW_MS: i64 = 1000 * 60 * 60 * 48;

/// Maximum notifications per page
const PAGE_SIZE: u32 = 30;

/// Notification reasons that can be grouped together
const GROUPABLE_REASONS: &[&str] = &[
    "like",
    "repost",
    "follow",
    "like-via-repost",
    "repost-via-repost",
    "subscribed-post",
];

/// Errors that can occur during notification operations
#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    /// Network or API error
    #[error("API error: {0}")]
    ApiError(String),

    /// JSON parsing error
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    /// No active session
    #[error("No active session")]
    NoSession,

    /// Lock acquisition failed
    #[error("Lock error")]
    LockError,
}

/// Result type for notification operations
pub type Result<T> = std::result::Result<T, NotificationError>;

/// Types of notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationType {
    /// Someone liked your post
    PostLike,
    /// Someone reposted your post
    Repost,
    /// Someone mentioned you
    Mention,
    /// Someone replied to your post
    Reply,
    /// Someone quoted your post
    Quote,
    /// Someone followed you
    Follow,
    /// Someone liked your feed generator
    FeedgenLike,
    /// Someone joined a starter pack
    StarterpackJoined,
    /// Account verified
    Verified,
    /// Account unverified
    Unverified,
    /// Like via repost
    LikeViaRepost,
    /// Repost via repost
    RepostViaRepost,
    /// Subscribed post
    SubscribedPost,
    /// Unknown notification type
    Unknown,
}

impl NotificationType {
    /// Check if this notification type can be grouped with others
    pub fn is_groupable(&self) -> bool {
        GROUPABLE_REASONS.contains(&self.to_reason())
    }

    /// Convert from AT Protocol reason string
    pub fn from_reason(reason: &str, reason_subject: Option<&str>) -> Self {
        match reason {
            "like" => {
                if reason_subject
                    .map(|s| s.contains("feed.generator"))
                    .unwrap_or(false)
                {
                    NotificationType::FeedgenLike
                } else {
                    NotificationType::PostLike
                }
            }
            "repost" => NotificationType::Repost,
            "mention" => NotificationType::Mention,
            "reply" => NotificationType::Reply,
            "quote" => NotificationType::Quote,
            "follow" => NotificationType::Follow,
            "starterpack-joined" => NotificationType::StarterpackJoined,
            "verified" => NotificationType::Verified,
            "unverified" => NotificationType::Unverified,
            "like-via-repost" => NotificationType::LikeViaRepost,
            "repost-via-repost" => NotificationType::RepostViaRepost,
            "subscribed-post" => NotificationType::SubscribedPost,
            _ => NotificationType::Unknown,
        }
    }

    /// Convert to AT Protocol reason string
    pub fn to_reason(&self) -> &'static str {
        match self {
            NotificationType::PostLike => "like",
            NotificationType::Repost => "repost",
            NotificationType::Mention => "mention",
            NotificationType::Reply => "reply",
            NotificationType::Quote => "quote",
            NotificationType::Follow => "follow",
            NotificationType::FeedgenLike => "like",
            NotificationType::StarterpackJoined => "starterpack-joined",
            NotificationType::Verified => "verified",
            NotificationType::Unverified => "unverified",
            NotificationType::LikeViaRepost => "like-via-repost",
            NotificationType::RepostViaRepost => "repost-via-repost",
            NotificationType::SubscribedPost => "subscribed-post",
            NotificationType::Unknown => "unknown",
        }
    }
}

/// A raw notification from the API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    /// Notification URI
    pub uri: String,

    /// Notification CID
    pub cid: String,

    /// Author of the notification
    pub author: ProfileViewBasic,

    /// Reason for the notification (like, repost, follow, etc.)
    pub reason: String,

    /// Subject of the notification (e.g., the post that was liked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_subject: Option<String>,

    /// The notification record content
    pub record: serde_json::Value,

    /// Whether the notification has been read
    pub is_read: bool,

    /// When the notification was indexed
    pub indexed_at: String,

    /// Labels on the notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,
}

impl Notification {
    /// Get the notification type
    pub fn notification_type(&self) -> NotificationType {
        NotificationType::from_reason(&self.reason, self.reason_subject.as_deref())
    }

    /// Get the subject URI for this notification
    pub fn subject_uri(&self) -> Option<String> {
        let notif_type = self.notification_type();
        match notif_type {
            NotificationType::Reply
            | NotificationType::Quote
            | NotificationType::Mention
            | NotificationType::SubscribedPost => Some(self.uri.clone()),
            NotificationType::PostLike
            | NotificationType::Repost
            | NotificationType::LikeViaRepost
            | NotificationType::RepostViaRepost => {
                // Extract subject URI from record
                self.record
                    .get("subject")
                    .and_then(|s| s.get("uri"))
                    .and_then(|u| u.as_str())
                    .map(String::from)
            }
            NotificationType::FeedgenLike => self.reason_subject.clone(),
            _ => None,
        }
    }

    /// Parse the indexed_at timestamp to milliseconds since epoch
    pub fn indexed_at_ms(&self) -> i64 {
        chrono::DateTime::parse_from_rfc3339(&self.indexed_at)
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(0)
    }
}

/// Subject of a notification (either a post or a starter pack)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NotificationSubject {
    /// Post subject
    Post(Box<PostView>),
    /// Starter pack subject
    StarterPack(Box<StarterPackViewBasic>),
}

/// Basic view of a starter pack
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StarterPackViewBasic {
    /// Starter pack URI
    pub uri: String,

    /// Starter pack CID
    pub cid: String,

    /// Creator of the starter pack
    pub creator: ProfileViewBasic,

    /// Starter pack record
    pub record: serde_json::Value,

    /// Number of users who joined via this pack
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined_week_count: Option<u32>,

    /// All-time join count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined_all_time_count: Option<u32>,

    /// Labels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,

    /// When the starter pack was indexed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<String>,
}

/// A grouped notification for display in the feed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedNotification {
    /// Unique key for React rendering
    pub react_key: String,

    /// Type of notification
    pub notification_type: NotificationType,

    /// Primary notification
    pub notification: Notification,

    /// Additional grouped notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional: Option<Vec<Notification>>,

    /// Subject URI (post or starter pack)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_uri: Option<String>,

    /// Subject content (post or starter pack)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<NotificationSubject>,
}

impl FeedNotification {
    /// Create a new feed notification from a raw notification
    pub fn new(notification: Notification) -> Self {
        let notification_type = notification.notification_type();
        let subject_uri = notification.subject_uri();
        let react_key = format!("notif-{}-{}", notification.uri, notification.reason);

        FeedNotification {
            react_key,
            notification_type,
            notification,
            additional: None,
            subject_uri,
            subject: None,
        }
    }

    /// Get total count of notifications in this group
    pub fn count(&self) -> usize {
        1 + self.additional.as_ref().map(|a| a.len()).unwrap_or(0)
    }

    /// Get all authors involved in this notification group
    pub fn authors(&self) -> Vec<&ProfileViewBasic> {
        let mut authors = vec![&self.notification.author];
        if let Some(ref additional) = self.additional {
            for notif in additional {
                authors.push(&notif.author);
            }
        }
        authors
    }

    /// Check if this notification has been read
    pub fn is_read(&self) -> bool {
        self.notification.is_read
    }

    /// Set whether this notification is read based on seen_at time
    pub fn update_read_status(&mut self, seen_at: &chrono::DateTime<chrono::Utc>) {
        if let Ok(indexed) = chrono::DateTime::parse_from_rfc3339(&self.notification.indexed_at) {
            self.notification.is_read = seen_at > &indexed.with_timezone(&chrono::Utc);
        }
    }
}

/// A page of notifications
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPage {
    /// Cursor for the next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// When the user last saw notifications
    pub seen_at: chrono::DateTime<chrono::Utc>,

    /// Notifications in this page
    pub items: Vec<FeedNotification>,

    /// Whether this page has priority notifications
    pub priority: bool,
}

impl NotificationPage {
    /// Create an empty notification page
    pub fn empty() -> Self {
        NotificationPage {
            cursor: None,
            seen_at: chrono::Utc::now(),
            items: Vec::new(),
            priority: false,
        }
    }
}

/// Cached notification page for unread checking
#[derive(Debug, Clone)]
pub struct CachedNotificationPage {
    /// Whether this cache is fresh enough to use
    pub usable_in_feed: bool,

    /// When the cache was synced
    pub synced_at: chrono::DateTime<chrono::Utc>,

    /// The cached page data
    pub data: Option<NotificationPage>,

    /// Unread count at time of sync
    pub unread_count: u32,
}

impl Default for CachedNotificationPage {
    fn default() -> Self {
        CachedNotificationPage {
            usable_in_feed: false,
            synced_at: chrono::Utc::now(),
            data: None,
            unread_count: 0,
        }
    }
}

/// Parameters for listing notifications
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListNotificationsParams {
    /// Maximum number of notifications to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// Filter by specific reasons
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasons: Option<Vec<String>>,

    /// Only show notifications from priority accounts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<bool>,
}

/// Response from listNotifications API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotificationsResponse {
    /// Cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// Notifications
    pub notifications: Vec<Notification>,

    /// When the user last saw notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seen_at: Option<String>,

    /// Whether priority notifications are available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<bool>,
}

/// Parameters for updating seen time
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSeenParams {
    /// Timestamp when notifications were seen
    pub seen_at: String,
}

/// Response from getUnreadCount API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadCountResponse {
    /// Number of unread notifications
    pub count: u32,
}

/// Filter for notification feeds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationFilter {
    /// All notifications
    All,
    /// Only mentions (replies, quotes, mentions)
    Mentions,
}

impl NotificationFilter {
    /// Get the reasons to filter by for this filter type
    pub fn reasons(&self) -> Option<Vec<String>> {
        match self {
            NotificationFilter::All => None,
            NotificationFilter::Mentions => {
                Some(vec![
                    "mention".to_string(),
                    "reply".to_string(),
                    "quote".to_string(),
                ])
            }
        }
    }
}

/// Groups notifications by reason and subject within a time window
pub fn group_notifications(notifications: Vec<Notification>) -> Vec<FeedNotification> {
    let mut grouped: Vec<FeedNotification> = Vec::new();

    for notif in notifications {
        let ts = notif.indexed_at_ms();
        let notif_type = notif.notification_type();
        let mut was_grouped = false;

        // Check if this notification can be grouped
        if notif_type.is_groupable() {
            for existing in grouped.iter_mut() {
                let existing_ts = existing.notification.indexed_at_ms();

                // Check if within time window and same type/subject
                if (existing_ts - ts).abs() < GROUPING_WINDOW_MS
                    && notif.reason == existing.notification.reason
                    && notif.reason_subject == existing.notification.reason_subject
                {
                    // Don't group if same author (unless subscribed-post)
                    let same_author = notif.author.did == existing.notification.author.did;
                    let is_subscribed = notif.reason == "subscribed-post";

                    if !same_author || is_subscribed {
                        // Check for follow-back (don't group follow-backs)
                        let next_is_follow_back = notif.reason == "follow"
                            && notif.author.viewer.as_ref().map(|v| v.following.is_some()).unwrap_or(false);
                        let prev_is_follow_back = existing.notification.reason == "follow"
                            && existing.notification.author.viewer.as_ref().map(|v| v.following.is_some()).unwrap_or(false);

                        if !next_is_follow_back && !prev_is_follow_back {
                            // Add to existing group
                            if existing.additional.is_none() {
                                existing.additional = Some(Vec::new());
                            }
                            existing.additional.as_mut().unwrap().push(notif.clone());
                            was_grouped = true;
                            break;
                        }
                    }
                }
            }
        }

        if !was_grouped {
            grouped.push(FeedNotification::new(notif));
        }
    }

    grouped
}

/// Notification service for fetching and managing notifications
pub struct NotificationService {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,

    /// Cached first page for unread checking
    cached_page: Arc<RwLock<CachedNotificationPage>>,

    /// Last known unread count
    unread_count: Arc<RwLock<u32>>,
}

impl NotificationService {
    /// Create a new notification service
    pub fn new(client: Arc<RwLock<XrpcClient>>) -> Self {
        NotificationService {
            client,
            cached_page: Arc::new(RwLock::new(CachedNotificationPage::default())),
            unread_count: Arc::new(RwLock::new(0)),
        }
    }

    /// List notifications with pagination
    pub async fn list_notifications(
        &self,
        params: ListNotificationsParams,
    ) -> Result<ListNotificationsResponse> {
        let client = self.client.read().await;

        let mut request = XrpcRequest::query("app.bsky.notification.listNotifications");

        if let Some(limit) = params.limit {
            request = request.param("limit", limit.to_string());
        }
        if let Some(ref cursor) = params.cursor {
            request = request.param("cursor", cursor.clone());
        }
        if let Some(priority) = params.priority {
            request = request.param("priority", priority.to_string());
        }
        if let Some(ref reasons) = params.reasons {
            for reason in reasons {
                request = request.param("reasons", reason.clone());
            }
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| NotificationError::ApiError(e.to_string()))?;

        let data: ListNotificationsResponse = serde_json::from_value(response.data)?;
        Ok(data)
    }

    /// Fetch a page of notifications with grouping and subject resolution
    pub async fn fetch_page(
        &self,
        cursor: Option<String>,
        filter: NotificationFilter,
        fetch_subjects: bool,
    ) -> Result<NotificationPage> {
        let params = ListNotificationsParams {
            limit: Some(PAGE_SIZE),
            cursor,
            reasons: filter.reasons(),
            priority: None,
        };

        let response = self.list_notifications(params).await?;

        // Group notifications
        let grouped = group_notifications(response.notifications);

        // Optionally fetch subjects
        let items = if fetch_subjects {
            self.resolve_subjects(grouped).await?
        } else {
            grouped
        };

        // Parse seen_at
        let seen_at = response
            .seen_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        Ok(NotificationPage {
            cursor: response.cursor,
            seen_at,
            items,
            priority: response.priority.unwrap_or(false),
        })
    }

    /// Resolve subjects (posts, starter packs) for notifications
    async fn resolve_subjects(
        &self,
        mut notifications: Vec<FeedNotification>,
    ) -> Result<Vec<FeedNotification>> {
        // Collect URIs to fetch
        let mut post_uris: Vec<String> = Vec::new();
        let mut pack_uris: Vec<String> = Vec::new();

        for notif in &notifications {
            if let Some(ref uri) = notif.subject_uri {
                if uri.contains("app.bsky.feed.post") {
                    post_uris.push(uri.clone());
                } else if notif.notification.reason_subject
                    .as_ref()
                    .map(|s| s.contains("app.bsky.graph.starterpack"))
                    .unwrap_or(false)
                {
                    if let Some(ref reason_subject) = notif.notification.reason_subject {
                        pack_uris.push(reason_subject.clone());
                    }
                }
            }
        }

        // Fetch posts in batches of 25
        let posts = self.fetch_posts(&post_uris).await?;
        let packs = self.fetch_starter_packs(&pack_uris).await?;

        // Attach subjects to notifications
        for notif in &mut notifications {
            if let Some(ref uri) = notif.subject_uri {
                if notif.notification_type == NotificationType::StarterpackJoined {
                    if let Some(ref reason_subject) = notif.notification.reason_subject {
                        if let Some(pack) = packs.get(reason_subject) {
                            notif.subject = Some(NotificationSubject::StarterPack(Box::new(pack.clone())));
                        }
                    }
                } else if let Some(post) = posts.get(uri) {
                    notif.subject = Some(NotificationSubject::Post(Box::new(post.clone())));
                }
            }
        }

        Ok(notifications)
    }

    /// Fetch posts by URIs
    async fn fetch_posts(&self, uris: &[String]) -> Result<HashMap<String, PostView>> {
        let mut result = HashMap::new();

        if uris.is_empty() {
            return Ok(result);
        }

        let client = self.client.read().await;

        // Fetch in chunks of 25
        for chunk in uris.chunks(25) {
            let mut request = XrpcRequest::query("app.bsky.feed.getPosts");
            for uri in chunk {
                request = request.param("uris", uri.clone());
            }

            match client.query::<serde_json::Value>(request).await {
                Ok(response) => {
                    if let Some(posts) = response.data.get("posts").and_then(|p| p.as_array()) {
                        for post_value in posts {
                            if let Ok(post) = serde_json::from_value::<PostView>(post_value.clone())
                            {
                                result.insert(post.uri.clone(), post);
                            }
                        }
                    }
                }
                Err(e) => {
                    // Log error but continue - some posts might be deleted
                    tracing::warn!("Failed to fetch posts: {}", e);
                }
            }
        }

        Ok(result)
    }

    /// Fetch starter packs by URIs
    async fn fetch_starter_packs(
        &self,
        uris: &[String],
    ) -> Result<HashMap<String, StarterPackViewBasic>> {
        let mut result = HashMap::new();

        if uris.is_empty() {
            return Ok(result);
        }

        let client = self.client.read().await;

        // Fetch in chunks of 25
        for chunk in uris.chunks(25) {
            let mut request = XrpcRequest::query("app.bsky.graph.getStarterPacks");
            for uri in chunk {
                request = request.param("uris", uri.clone());
            }

            match client.query::<serde_json::Value>(request).await {
                Ok(response) => {
                    if let Some(packs) = response.data.get("starterPacks").and_then(|p| p.as_array()) {
                        for pack_value in packs {
                            if let Ok(pack) =
                                serde_json::from_value::<StarterPackViewBasic>(pack_value.clone())
                            {
                                result.insert(pack.uri.clone(), pack);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch starter packs: {}", e);
                }
            }
        }

        Ok(result)
    }

    /// Get the unread notification count
    pub async fn get_unread_count(&self) -> Result<u32> {
        let client = self.client.read().await;

        let request = XrpcRequest::query("app.bsky.notification.getUnreadCount");

        let response = client
            .query(request)
            .await
            .map_err(|e| NotificationError::ApiError(e.to_string()))?;

        let data: UnreadCountResponse = serde_json::from_value(response.data)?;

        // Update cached count
        let mut count = self.unread_count.write().await;
        *count = data.count;

        Ok(data.count)
    }

    /// Mark all notifications as read (up to the given timestamp)
    pub async fn mark_all_read(&self) -> Result<()> {
        let client = self.client.read().await;
        let now = chrono::Utc::now().to_rfc3339();

        let params = UpdateSeenParams { seen_at: now };

        let request = XrpcRequest::procedure("app.bsky.notification.updateSeen")
            .json_body(&params)
            .map_err(|e| NotificationError::ApiError(e.to_string()))?;

        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| NotificationError::ApiError(e.to_string()))?;

        // Reset unread count
        let mut count = self.unread_count.write().await;
        *count = 0;

        Ok(())
    }

    /// Mark all notifications as read up to a specific time
    pub async fn mark_read_until(&self, seen_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        let client = self.client.read().await;

        let params = UpdateSeenParams {
            seen_at: seen_at.to_rfc3339(),
        };

        let request = XrpcRequest::procedure("app.bsky.notification.updateSeen")
            .json_body(&params)
            .map_err(|e| NotificationError::ApiError(e.to_string()))?;

        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| NotificationError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Check for new notifications and update cache
    ///
    /// If `invalidate` is true, the cached page will be marked as invalid
    /// and the query will be forced to refetch.
    pub async fn check_unread(&self, _invalidate: bool) -> Result<u32> {
        // Fetch first page
        let page = self.fetch_page(None, NotificationFilter::All, true).await?;

        // Update cache
        let mut cache = self.cached_page.write().await;
        cache.synced_at = chrono::Utc::now();
        cache.usable_in_feed = true;

        // Count unread
        let unread = page
            .items
            .iter()
            .filter(|n| !n.is_read())
            .count() as u32;

        cache.unread_count = unread;
        cache.data = Some(page);

        // Update unread count
        let mut count = self.unread_count.write().await;
        *count = unread;

        Ok(unread)
    }

    /// Get cached page if available and fresh
    pub async fn get_cached_page(&self) -> Option<NotificationPage> {
        let cache = self.cached_page.read().await;
        if cache.usable_in_feed {
            cache.data.clone()
        } else {
            None
        }
    }

    /// Get the last known unread count
    pub async fn last_unread_count(&self) -> u32 {
        *self.unread_count.read().await
    }

    /// Invalidate the notification cache
    pub async fn invalidate_cache(&self) {
        let mut cache = self.cached_page.write().await;
        cache.usable_in_feed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_notification(
        uri: &str,
        reason: &str,
        author_did: &str,
        indexed_at: &str,
        reason_subject: Option<&str>,
    ) -> Notification {
        Notification {
            uri: uri.to_string(),
            cid: "bafyreib".to_string(),
            author: ProfileViewBasic {
                did: author_did.to_string(),
                handle: format!("{}.bsky.social", author_did.split(':').last().unwrap_or("user")),
                display_name: None,
                avatar: None,
                associated: None,
                viewer: None,
                labels: None,
                created_at: None,
            },
            reason: reason.to_string(),
            reason_subject: reason_subject.map(String::from),
            record: serde_json::json!({}),
            is_read: false,
            indexed_at: indexed_at.to_string(),
            labels: None,
        }
    }

    #[test]
    fn test_notification_type_from_reason() {
        assert_eq!(
            NotificationType::from_reason("like", None),
            NotificationType::PostLike
        );
        assert_eq!(
            NotificationType::from_reason("like", Some("at://did:plc:abc/app.bsky.feed.generator/xyz")),
            NotificationType::FeedgenLike
        );
        assert_eq!(
            NotificationType::from_reason("repost", None),
            NotificationType::Repost
        );
        assert_eq!(
            NotificationType::from_reason("mention", None),
            NotificationType::Mention
        );
        assert_eq!(
            NotificationType::from_reason("reply", None),
            NotificationType::Reply
        );
        assert_eq!(
            NotificationType::from_reason("quote", None),
            NotificationType::Quote
        );
        assert_eq!(
            NotificationType::from_reason("follow", None),
            NotificationType::Follow
        );
        assert_eq!(
            NotificationType::from_reason("starterpack-joined", None),
            NotificationType::StarterpackJoined
        );
        assert_eq!(
            NotificationType::from_reason("unknown-reason", None),
            NotificationType::Unknown
        );
    }

    #[test]
    fn test_notification_type_is_groupable() {
        assert!(NotificationType::PostLike.is_groupable());
        assert!(NotificationType::Repost.is_groupable());
        assert!(NotificationType::Follow.is_groupable());
        assert!(!NotificationType::Reply.is_groupable());
        assert!(!NotificationType::Mention.is_groupable());
        assert!(!NotificationType::Quote.is_groupable());
    }

    #[test]
    fn test_group_notifications_single() {
        let notifications = vec![make_notification(
            "at://did:plc:abc/app.bsky.feed.like/123",
            "like",
            "did:plc:user1",
            "2024-01-15T10:00:00Z",
            Some("at://did:plc:abc/app.bsky.feed.post/456"),
        )];

        let grouped = group_notifications(notifications);
        assert_eq!(grouped.len(), 1);
        assert!(grouped[0].additional.is_none());
    }

    #[test]
    fn test_group_notifications_multiple_likes() {
        let notifications = vec![
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/123",
                "like",
                "did:plc:user1",
                "2024-01-15T10:00:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/124",
                "like",
                "did:plc:user2",
                "2024-01-15T10:05:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/125",
                "like",
                "did:plc:user3",
                "2024-01-15T10:10:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
        ];

        let grouped = group_notifications(notifications);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].count(), 3);
    }

    #[test]
    fn test_group_notifications_different_subjects() {
        let notifications = vec![
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/123",
                "like",
                "did:plc:user1",
                "2024-01-15T10:00:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/124",
                "like",
                "did:plc:user2",
                "2024-01-15T10:05:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/789"), // Different post
            ),
        ];

        let grouped = group_notifications(notifications);
        assert_eq!(grouped.len(), 2); // Not grouped because different subjects
    }

    #[test]
    fn test_group_notifications_mixed_types() {
        let notifications = vec![
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/123",
                "like",
                "did:plc:user1",
                "2024-01-15T10:00:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
            make_notification(
                "at://did:plc:abc/app.bsky.feed.post/789",
                "reply",
                "did:plc:user2",
                "2024-01-15T10:05:00Z",
                None,
            ),
            make_notification(
                "at://did:plc:abc/app.bsky.graph.follow/111",
                "follow",
                "did:plc:user3",
                "2024-01-15T10:10:00Z",
                None,
            ),
        ];

        let grouped = group_notifications(notifications);
        assert_eq!(grouped.len(), 3); // All different types, not grouped
    }

    #[test]
    fn test_group_notifications_same_author_not_grouped() {
        let notifications = vec![
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/123",
                "like",
                "did:plc:user1",
                "2024-01-15T10:00:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/124",
                "like",
                "did:plc:user1", // Same author
                "2024-01-15T10:05:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
        ];

        let grouped = group_notifications(notifications);
        assert_eq!(grouped.len(), 2); // Not grouped because same author
    }

    #[test]
    fn test_group_notifications_outside_time_window() {
        let notifications = vec![
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/123",
                "like",
                "did:plc:user1",
                "2024-01-15T10:00:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/124",
                "like",
                "did:plc:user2",
                "2024-01-20T10:00:00Z", // 5 days later, outside 48hr window
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
        ];

        let grouped = group_notifications(notifications);
        assert_eq!(grouped.len(), 2); // Not grouped because outside time window
    }

    #[test]
    fn test_notification_subject_uri() {
        let reply = make_notification(
            "at://did:plc:abc/app.bsky.feed.post/123",
            "reply",
            "did:plc:user1",
            "2024-01-15T10:00:00Z",
            None,
        );
        assert_eq!(
            reply.subject_uri(),
            Some("at://did:plc:abc/app.bsky.feed.post/123".to_string())
        );

        let mut like = make_notification(
            "at://did:plc:abc/app.bsky.feed.like/123",
            "like",
            "did:plc:user1",
            "2024-01-15T10:00:00Z",
            Some("at://did:plc:abc/app.bsky.feed.post/456"),
        );
        like.record = serde_json::json!({
            "subject": {
                "uri": "at://did:plc:abc/app.bsky.feed.post/456",
                "cid": "bafyreib"
            }
        });
        assert_eq!(
            like.subject_uri(),
            Some("at://did:plc:abc/app.bsky.feed.post/456".to_string())
        );
    }

    #[test]
    fn test_notification_filter_reasons() {
        assert!(NotificationFilter::All.reasons().is_none());

        let mention_reasons = NotificationFilter::Mentions.reasons().unwrap();
        assert!(mention_reasons.contains(&"mention".to_string()));
        assert!(mention_reasons.contains(&"reply".to_string()));
        assert!(mention_reasons.contains(&"quote".to_string()));
        assert!(!mention_reasons.contains(&"like".to_string()));
    }

    #[test]
    fn test_feed_notification_count() {
        let notif = make_notification(
            "at://did:plc:abc/app.bsky.feed.like/123",
            "like",
            "did:plc:user1",
            "2024-01-15T10:00:00Z",
            Some("at://did:plc:abc/app.bsky.feed.post/456"),
        );

        let mut feed_notif = FeedNotification::new(notif);
        assert_eq!(feed_notif.count(), 1);

        feed_notif.additional = Some(vec![
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/124",
                "like",
                "did:plc:user2",
                "2024-01-15T10:05:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
            make_notification(
                "at://did:plc:abc/app.bsky.feed.like/125",
                "like",
                "did:plc:user3",
                "2024-01-15T10:10:00Z",
                Some("at://did:plc:abc/app.bsky.feed.post/456"),
            ),
        ]);
        assert_eq!(feed_notif.count(), 3);
    }

    #[test]
    fn test_feed_notification_authors() {
        let notif = make_notification(
            "at://did:plc:abc/app.bsky.feed.like/123",
            "like",
            "did:plc:user1",
            "2024-01-15T10:00:00Z",
            Some("at://did:plc:abc/app.bsky.feed.post/456"),
        );

        let mut feed_notif = FeedNotification::new(notif);
        feed_notif.additional = Some(vec![make_notification(
            "at://did:plc:abc/app.bsky.feed.like/124",
            "like",
            "did:plc:user2",
            "2024-01-15T10:05:00Z",
            Some("at://did:plc:abc/app.bsky.feed.post/456"),
        )]);

        let authors = feed_notif.authors();
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].did, "did:plc:user1");
        assert_eq!(authors[1].did, "did:plc:user2");
    }

    #[test]
    fn test_notification_page_empty() {
        let page = NotificationPage::empty();
        assert!(page.cursor.is_none());
        assert!(page.items.is_empty());
        assert!(!page.priority);
    }

    #[test]
    fn test_notification_type_to_reason() {
        assert_eq!(NotificationType::PostLike.to_reason(), "like");
        assert_eq!(NotificationType::FeedgenLike.to_reason(), "like");
        assert_eq!(NotificationType::Repost.to_reason(), "repost");
        assert_eq!(NotificationType::Reply.to_reason(), "reply");
        assert_eq!(NotificationType::Mention.to_reason(), "mention");
        assert_eq!(NotificationType::Quote.to_reason(), "quote");
        assert_eq!(NotificationType::Follow.to_reason(), "follow");
        assert_eq!(NotificationType::StarterpackJoined.to_reason(), "starterpack-joined");
    }

    #[test]
    fn test_cached_notification_page_default() {
        let cache = CachedNotificationPage::default();
        assert!(!cache.usable_in_feed);
        assert!(cache.data.is_none());
        assert_eq!(cache.unread_count, 0);
    }

    #[test]
    fn test_list_notifications_params_serialization() {
        let params = ListNotificationsParams {
            limit: Some(30),
            cursor: Some("cursor123".to_string()),
            reasons: Some(vec!["like".to_string(), "repost".to_string()]),
            priority: Some(true),
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["limit"], 30);
        assert_eq!(json["cursor"], "cursor123");
        assert_eq!(json["priority"], true);
    }
}
