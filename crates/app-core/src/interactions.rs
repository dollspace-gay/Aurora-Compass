//! Post interactions (like, repost, quote)
//!
//! This module provides functionality for interacting with posts, including
//! liking, unliking, reposting, unreposting, and quoting posts.

use atproto_client::xrpc::{XrpcClient, XrpcRequest};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Interaction service error types
#[derive(Debug, Error)]
pub enum InteractionError {
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
}

/// Result type for interaction operations
pub type Result<T> = std::result::Result<T, InteractionError>;

/// Strong reference to a record (used in like/repost subjects)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubjectRef {
    /// URI of the record
    pub uri: String,
    /// CID of the record
    pub cid: String,
}

/// Via reference for tracking (used when liking/reposting through a repost)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViaRef {
    /// URI of the repost
    pub uri: String,
    /// CID of the repost
    pub cid: String,
}

/// Like record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikeRecord {
    /// Subject of the like (the post being liked)
    pub subject: SubjectRef,
    /// Created at timestamp
    pub created_at: String,
    /// Optional via reference (if liked through a repost)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<ViaRef>,
}

/// Repost record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepostRecord {
    /// Subject of the repost (the post being reposted)
    pub subject: SubjectRef,
    /// Created at timestamp
    pub created_at: String,
    /// Optional via reference (if reposted through a repost)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<ViaRef>,
}

/// Embed record reference (for quote posts)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbedRecord {
    /// URI of the embedded post
    pub uri: String,
    /// CID of the embedded post
    pub cid: String,
}

/// Quote post embed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteEmbed {
    /// Type discriminator
    #[serde(rename = "$type")]
    pub embed_type: String,
    /// Embedded record
    pub record: EmbedRecord,
}

/// Response from creating a record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateRecordResponse {
    /// URI of the created record
    pub uri: String,
    /// CID of the created record
    pub cid: String,
}

/// Interaction counts for a post
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionCounts {
    /// Number of likes
    #[serde(default)]
    pub like_count: u32,
    /// Number of reposts
    #[serde(default)]
    pub repost_count: u32,
    /// Number of replies
    #[serde(default)]
    pub reply_count: u32,
    /// Number of quotes
    #[serde(default)]
    pub quote_count: u32,
}

impl InteractionCounts {
    /// Create new interaction counts
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment like count
    pub fn increment_likes(mut self) -> Self {
        self.like_count = self.like_count.saturating_add(1);
        self
    }

    /// Decrement like count
    pub fn decrement_likes(mut self) -> Self {
        self.like_count = self.like_count.saturating_sub(1);
        self
    }

    /// Increment repost count
    pub fn increment_reposts(mut self) -> Self {
        self.repost_count = self.repost_count.saturating_add(1);
        self
    }

    /// Decrement repost count
    pub fn decrement_reposts(mut self) -> Self {
        self.repost_count = self.repost_count.saturating_sub(1);
        self
    }

    /// Increment reply count
    pub fn increment_replies(mut self) -> Self {
        self.reply_count = self.reply_count.saturating_add(1);
        self
    }

    /// Increment quote count
    pub fn increment_quotes(mut self) -> Self {
        self.quote_count = self.quote_count.saturating_add(1);
        self
    }
}

/// Interaction service for managing post interactions
///
/// Provides methods for liking, unliking, reposting, unreposting, and quoting posts.
///
/// # Example
///
/// ```rust,no_run
/// use app_core::interactions::{InteractionService, SubjectRef};
/// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create XRPC client (with auth)
///     let config = XrpcClientConfig::new("https://bsky.social");
///     let client = XrpcClient::new(config);
///     let service = InteractionService::new(client);
///
///     // Like a post
///     let subject = SubjectRef {
///         uri: "at://did:plc:abc123/app.bsky.feed.post/xyz456".to_string(),
///         cid: "bafytest123".to_string(),
///     };
///     let like_uri = service.like(&subject, None).await?;
///     println!("Liked post, like URI: {}", like_uri);
///
///     // Unlike the post
///     service.unlike(&like_uri).await?;
///     println!("Unliked post");
///
///     Ok(())
/// }
/// ```
pub struct InteractionService {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl InteractionService {
    /// Create a new interaction service
    pub fn new(client: XrpcClient) -> Self {
        Self {
            client: Arc::new(RwLock::new(client)),
        }
    }

    /// Like a post
    ///
    /// # Arguments
    ///
    /// * `subject` - The post to like (URI and CID)
    /// * `via` - Optional via reference (if liking through a repost)
    ///
    /// # Returns
    ///
    /// URI of the created like record
    ///
    /// # Errors
    ///
    /// - `InteractionError::NoSession` - No active session
    /// - `InteractionError::Xrpc` - XRPC error
    pub async fn like(&self, subject: &SubjectRef, via: Option<ViaRef>) -> Result<String> {
        if subject.uri.is_empty() {
            return Err(InteractionError::InvalidUri("Subject URI cannot be empty".to_string()));
        }

        if subject.cid.is_empty() {
            return Err(InteractionError::InvalidCid("Subject CID cannot be empty".to_string()));
        }

        let now = Utc::now().to_rfc3339();
        let like_record = LikeRecord {
            subject: subject.clone(),
            created_at: now,
            via,
        };

        let body = serde_json::json!({
            "repo": "self",
            "collection": "app.bsky.feed.like",
            "record": {
                "subject": like_record.subject,
                "createdAt": like_record.created_at,
                "via": like_record.via,
                "$type": "app.bsky.feed.like"
            }
        });

        let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .json_body(&body)
            .map_err(|e| InteractionError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| InteractionError::Xrpc(e.to_string()))?;

        let create_response: CreateRecordResponse = serde_json::from_value(response.data)
            .map_err(InteractionError::Serialization)?;

        Ok(create_response.uri)
    }

    /// Unlike a post
    ///
    /// # Arguments
    ///
    /// * `like_uri` - URI of the like record to delete
    ///
    /// # Errors
    ///
    /// - `InteractionError::NoSession` - No active session
    /// - `InteractionError::Xrpc` - XRPC error
    pub async fn unlike(&self, like_uri: &str) -> Result<()> {
        if like_uri.is_empty() {
            return Err(InteractionError::InvalidUri("Like URI cannot be empty".to_string()));
        }

        // Parse the AT URI to extract repo and rkey
        // Format: at://did:plc:xyz/app.bsky.feed.like/rkey
        let uri_parts: Vec<&str> = like_uri.trim_start_matches("at://").split('/').collect();
        if uri_parts.len() < 3 {
            return Err(InteractionError::InvalidUri(format!(
                "Invalid like URI format: {}",
                like_uri
            )));
        }

        let repo = uri_parts[0];
        let rkey = uri_parts[2];

        let request = XrpcRequest::procedure("com.atproto.repo.deleteRecord")
            .json_body(&serde_json::json!({
                "repo": repo,
                "collection": "app.bsky.feed.like",
                "rkey": rkey,
            }))
            .map_err(|e| InteractionError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| InteractionError::Xrpc(e.to_string()))?;

        Ok(())
    }

    /// Repost a post
    ///
    /// # Arguments
    ///
    /// * `subject` - The post to repost (URI and CID)
    /// * `via` - Optional via reference (if reposting through a repost)
    ///
    /// # Returns
    ///
    /// URI of the created repost record
    ///
    /// # Errors
    ///
    /// - `InteractionError::NoSession` - No active session
    /// - `InteractionError::Xrpc` - XRPC error
    pub async fn repost(&self, subject: &SubjectRef, via: Option<ViaRef>) -> Result<String> {
        if subject.uri.is_empty() {
            return Err(InteractionError::InvalidUri("Subject URI cannot be empty".to_string()));
        }

        if subject.cid.is_empty() {
            return Err(InteractionError::InvalidCid("Subject CID cannot be empty".to_string()));
        }

        let now = Utc::now().to_rfc3339();
        let repost_record = RepostRecord {
            subject: subject.clone(),
            created_at: now,
            via,
        };

        let body = serde_json::json!({
            "repo": "self",
            "collection": "app.bsky.feed.repost",
            "record": {
                "subject": repost_record.subject,
                "createdAt": repost_record.created_at,
                "via": repost_record.via,
                "$type": "app.bsky.feed.repost"
            }
        });

        let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .json_body(&body)
            .map_err(|e| InteractionError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| InteractionError::Xrpc(e.to_string()))?;

        let create_response: CreateRecordResponse = serde_json::from_value(response.data)
            .map_err(InteractionError::Serialization)?;

        Ok(create_response.uri)
    }

    /// Unrepost a post
    ///
    /// # Arguments
    ///
    /// * `repost_uri` - URI of the repost record to delete
    ///
    /// # Errors
    ///
    /// - `InteractionError::NoSession` - No active session
    /// - `InteractionError::Xrpc` - XRPC error
    pub async fn unrepost(&self, repost_uri: &str) -> Result<()> {
        if repost_uri.is_empty() {
            return Err(InteractionError::InvalidUri("Repost URI cannot be empty".to_string()));
        }

        // Parse the AT URI to extract repo and rkey
        // Format: at://did:plc:xyz/app.bsky.feed.repost/rkey
        let uri_parts: Vec<&str> = repost_uri.trim_start_matches("at://").split('/').collect();
        if uri_parts.len() < 3 {
            return Err(InteractionError::InvalidUri(format!(
                "Invalid repost URI format: {}",
                repost_uri
            )));
        }

        let repo = uri_parts[0];
        let rkey = uri_parts[2];

        let request = XrpcRequest::procedure("com.atproto.repo.deleteRecord")
            .json_body(&serde_json::json!({
                "repo": repo,
                "collection": "app.bsky.feed.repost",
                "rkey": rkey,
            }))
            .map_err(|e| InteractionError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| InteractionError::Xrpc(e.to_string()))?;

        Ok(())
    }

    /// Create a quote post embed
    ///
    /// This creates an embed structure for quoting a post.
    /// The actual post creation should use the regular post creation API
    /// with this embed included.
    ///
    /// # Arguments
    ///
    /// * `subject` - The post to quote (URI and CID)
    ///
    /// # Returns
    ///
    /// QuoteEmbed structure to include in post record
    pub fn create_quote_embed(&self, subject: &SubjectRef) -> QuoteEmbed {
        QuoteEmbed {
            embed_type: "app.bsky.embed.record".to_string(),
            record: EmbedRecord {
                uri: subject.uri.clone(),
                cid: subject.cid.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subject_ref() {
        let subject = SubjectRef {
            uri: "at://did:plc:test/app.bsky.feed.post/123".to_string(),
            cid: "bafytest123".to_string(),
        };

        assert_eq!(subject.uri, "at://did:plc:test/app.bsky.feed.post/123");
        assert_eq!(subject.cid, "bafytest123");
    }

    #[test]
    fn test_like_record_serialization() {
        let like_record = LikeRecord {
            subject: SubjectRef {
                uri: "at://did:plc:test/app.bsky.feed.post/123".to_string(),
                cid: "bafytest123".to_string(),
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            via: None,
        };

        let json = serde_json::to_string(&like_record).unwrap();
        let deserialized: LikeRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, like_record);
    }

    #[test]
    fn test_like_record_with_via() {
        let like_record = LikeRecord {
            subject: SubjectRef {
                uri: "at://did:plc:test/app.bsky.feed.post/123".to_string(),
                cid: "bafytest123".to_string(),
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            via: Some(ViaRef {
                uri: "at://did:plc:test/app.bsky.feed.repost/456".to_string(),
                cid: "bafyrepost456".to_string(),
            }),
        };

        let json = serde_json::to_string(&like_record).unwrap();
        assert!(json.contains("via"));
        assert!(json.contains("bafyrepost456"));
    }

    #[test]
    fn test_repost_record_serialization() {
        let repost_record = RepostRecord {
            subject: SubjectRef {
                uri: "at://did:plc:test/app.bsky.feed.post/123".to_string(),
                cid: "bafytest123".to_string(),
            },
            created_at: "2024-01-01T00:00:00Z".to_string(),
            via: None,
        };

        let json = serde_json::to_string(&repost_record).unwrap();
        let deserialized: RepostRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, repost_record);
    }

    #[test]
    fn test_quote_embed_serialization() {
        let quote_embed = QuoteEmbed {
            embed_type: "app.bsky.embed.record".to_string(),
            record: EmbedRecord {
                uri: "at://did:plc:test/app.bsky.feed.post/123".to_string(),
                cid: "bafytest123".to_string(),
            },
        };

        let json = serde_json::to_string(&quote_embed).unwrap();
        assert!(json.contains("app.bsky.embed.record"));
        assert!(json.contains("$type"));
    }

    #[test]
    fn test_interaction_counts_default() {
        let counts = InteractionCounts::default();
        assert_eq!(counts.like_count, 0);
        assert_eq!(counts.repost_count, 0);
        assert_eq!(counts.reply_count, 0);
        assert_eq!(counts.quote_count, 0);
    }

    #[test]
    fn test_interaction_counts_increment() {
        let counts = InteractionCounts::new()
            .increment_likes()
            .increment_reposts()
            .increment_replies()
            .increment_quotes();

        assert_eq!(counts.like_count, 1);
        assert_eq!(counts.repost_count, 1);
        assert_eq!(counts.reply_count, 1);
        assert_eq!(counts.quote_count, 1);
    }

    #[test]
    fn test_interaction_counts_decrement() {
        let counts = InteractionCounts {
            like_count: 5,
            repost_count: 3,
            reply_count: 2,
            quote_count: 1,
        };

        let updated = counts.decrement_likes().decrement_reposts();

        assert_eq!(updated.like_count, 4);
        assert_eq!(updated.repost_count, 2);
    }

    #[test]
    fn test_interaction_counts_saturation() {
        let counts = InteractionCounts {
            like_count: 0,
            repost_count: u32::MAX,
            reply_count: 0,
            quote_count: 0,
        };

        // Test underflow protection
        let decremented = counts.decrement_likes();
        assert_eq!(decremented.like_count, 0);

        // Test overflow protection
        let incremented = counts.increment_reposts();
        assert_eq!(incremented.repost_count, u32::MAX);
    }

    #[test]
    fn test_create_quote_embed() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = InteractionService::new(client);

        let subject = SubjectRef {
            uri: "at://did:plc:test/app.bsky.feed.post/123".to_string(),
            cid: "bafytest123".to_string(),
        };

        let quote_embed = service.create_quote_embed(&subject);

        assert_eq!(quote_embed.embed_type, "app.bsky.embed.record");
        assert_eq!(quote_embed.record.uri, subject.uri);
        assert_eq!(quote_embed.record.cid, subject.cid);
    }

    #[test]
    fn test_create_record_response_deserialization() {
        let json = r#"{"uri":"at://did:plc:test/app.bsky.feed.like/abc","cid":"bafytest"}"#;
        let response: CreateRecordResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.uri, "at://did:plc:test/app.bsky.feed.like/abc");
        assert_eq!(response.cid, "bafytest");
    }
}
