//! Block and mute functionality
//!
//! This module provides functionality for blocking and muting accounts,
//! as well as managing block/mute lists via the AT Protocol.
//!
//! # Overview
//!
//! The blocking system provides two levels of moderation:
//!
//! - **Blocking**: Prevents an account from viewing your profile, posts, and interacting with you.
//!   Creates a `app.bsky.graph.block` record in your repository.
//!
//! - **Muting**: Hides an account's posts from your feeds without notifying them.
//!   Uses the `app.bsky.graph.muteActor` endpoint.
//!
//! # Example
//!
//! ```rust,no_run
//! use moderation::blocking::BlockService;
//! use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = XrpcClientConfig::new("https://bsky.social");
//!     let client = XrpcClient::new(config);
//!     let service = BlockService::new(client);
//!
//!     // Block a user
//!     let block_uri = service.block("did:plc:example123").await?;
//!     println!("Blocked user, record URI: {}", block_uri);
//!
//!     // Mute a user
//!     service.mute("did:plc:example456").await?;
//!     println!("Muted user");
//!
//!     Ok(())
//! }
//! ```

use atproto_client::xrpc::{XrpcClient, XrpcRequest};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors that can occur during block/mute operations
#[derive(Debug, Error)]
pub enum BlockError {
    /// XRPC/API error
    #[error("API error: {0}")]
    ApiError(String),

    /// Invalid DID format
    #[error("Invalid DID: {0}")]
    InvalidDid(String),

    /// Block/mute record not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// No active session
    #[error("No active session")]
    NoSession,

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid URI format
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
}

/// Result type for block/mute operations
pub type Result<T> = std::result::Result<T, BlockError>;

/// Maximum items per page when fetching blocks/mutes
const PAGE_SIZE: u32 = 50;

/// Maximum pages to fetch when getting all blocks/mutes
const MAX_PAGES: usize = 10;

/// A blocked profile view (returned from getBlocks)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedProfileView {
    /// DID of the blocked user
    pub did: String,
    /// Handle of the blocked user
    pub handle: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Avatar URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Labels on the profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<serde_json::Value>>,
    /// Viewer state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<BlockedViewerState>,
}

/// Viewer state for a blocked profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BlockedViewerState {
    /// URI of the block record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking: Option<String>,
    /// Whether blocked by this user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<bool>,
}

/// A muted profile view (returned from getMutes)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutedProfileView {
    /// DID of the muted user
    pub did: String,
    /// Handle of the muted user
    pub handle: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Avatar URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Labels on the profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<serde_json::Value>>,
    /// Viewer state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<MutedViewerState>,
}

/// Viewer state for a muted profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MutedViewerState {
    /// Whether this profile is muted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    /// URI of the list if muted by list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted_by_list: Option<String>,
}

/// Response from getBlocks API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlocksResponse {
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// List of blocked profiles
    pub blocks: Vec<BlockedProfileView>,
}

/// Response from getMutes API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMutesResponse {
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// List of muted profiles
    pub mutes: Vec<MutedProfileView>,
}

/// Response from creating a record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecordResponse {
    /// URI of the created record
    pub uri: String,
    /// CID of the created record
    pub cid: String,
}

/// Block service for managing account blocks and mutes
///
/// Provides methods for blocking, unblocking, muting, and unmuting accounts,
/// as well as fetching lists of blocked/muted accounts.
///
/// # Example
///
/// ```rust,no_run
/// use moderation::blocking::BlockService;
/// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = XrpcClientConfig::new("https://bsky.social");
///     let client = XrpcClient::new(config);
///     let service = BlockService::new(client);
///
///     // Block a user
///     let block_uri = service.block("did:plc:example123").await?;
///
///     // Get all blocked users
///     let blocked = service.get_blocks(None, None).await?;
///     for profile in blocked.blocks {
///         println!("Blocked: @{}", profile.handle);
///     }
///
///     // Unblock the user
///     service.unblock(&block_uri).await?;
///
///     Ok(())
/// }
/// ```
pub struct BlockService {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl BlockService {
    /// Create a new block service
    ///
    /// # Arguments
    ///
    /// * `client` - XRPC client for making API calls
    pub fn new(client: XrpcClient) -> Self {
        Self { client: Arc::new(RwLock::new(client)) }
    }

    /// Create a new block service with a shared client
    ///
    /// # Arguments
    ///
    /// * `client` - Shared XRPC client
    pub fn with_shared_client(client: Arc<RwLock<XrpcClient>>) -> Self {
        Self { client }
    }

    // =========================================================================
    // Block Operations
    // =========================================================================

    /// Block an account
    ///
    /// Creates a block record in your repository, preventing the blocked account
    /// from viewing your profile, posts, and interacting with you.
    ///
    /// # Arguments
    ///
    /// * `did` - DID of the account to block
    ///
    /// # Returns
    ///
    /// URI of the created block record (needed for unblocking)
    ///
    /// # Errors
    ///
    /// - `BlockError::InvalidDid` - Empty or invalid DID
    /// - `BlockError::ApiError` - API error
    /// - `BlockError::NoSession` - No active session
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::blocking::BlockService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = BlockService::new(client);
    /// let block_uri = service.block("did:plc:example123").await?;
    /// println!("Created block record: {}", block_uri);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn block(&self, did: &str) -> Result<String> {
        self.validate_did(did)?;

        let now = Utc::now().to_rfc3339();

        let body = serde_json::json!({
            "repo": "self",
            "collection": "app.bsky.graph.block",
            "record": {
                "$type": "app.bsky.graph.block",
                "subject": did,
                "createdAt": now
            }
        });

        let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .json_body(&body)
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        let create_response: CreateRecordResponse =
            serde_json::from_value(response.data).map_err(BlockError::Serialization)?;

        Ok(create_response.uri)
    }

    /// Unblock an account
    ///
    /// Deletes the block record, allowing the previously blocked account to
    /// view your profile and interact with you again.
    ///
    /// # Arguments
    ///
    /// * `block_uri` - URI of the block record to delete
    ///
    /// # Errors
    ///
    /// - `BlockError::InvalidUri` - Empty or invalid URI
    /// - `BlockError::ApiError` - API error
    /// - `BlockError::NotFound` - Block record not found
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::blocking::BlockService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = BlockService::new(client);
    /// service.unblock("at://did:plc:abc/app.bsky.graph.block/123").await?;
    /// println!("User unblocked");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unblock(&self, block_uri: &str) -> Result<()> {
        let (repo, rkey) = self.parse_record_uri(block_uri, "app.bsky.graph.block")?;

        let body = serde_json::json!({
            "repo": repo,
            "collection": "app.bsky.graph.block",
            "rkey": rkey
        });

        let request = XrpcRequest::procedure("com.atproto.repo.deleteRecord")
            .json_body(&body)
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Get blocked accounts with pagination
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results (default 50, max 100)
    /// * `cursor` - Pagination cursor
    ///
    /// # Returns
    ///
    /// Response containing blocked profiles and pagination cursor
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::blocking::BlockService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = BlockService::new(client);
    /// let response = service.get_blocks(Some(25), None).await?;
    /// for blocked in response.blocks {
    ///     println!("Blocked: @{} ({})", blocked.handle, blocked.did);
    /// }
    ///
    /// // Get next page
    /// if let Some(cursor) = response.cursor {
    ///     let next_page = service.get_blocks(Some(25), Some(cursor)).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_blocks(
        &self,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<GetBlocksResponse> {
        let client = self.client.read().await;

        let mut request = XrpcRequest::query("app.bsky.graph.getBlocks")
            .param("limit", limit.unwrap_or(PAGE_SIZE).min(100).to_string());

        if let Some(c) = cursor {
            request = request.param("cursor", c);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        let data: GetBlocksResponse =
            serde_json::from_value(response.data).map_err(BlockError::Serialization)?;

        Ok(data)
    }

    /// Get all blocked accounts (up to MAX_PAGES worth)
    ///
    /// Automatically paginates through all blocked accounts.
    ///
    /// # Returns
    ///
    /// Vector of all blocked profiles
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::blocking::BlockService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = BlockService::new(client);
    /// let all_blocked = service.get_all_blocks().await?;
    /// println!("Total blocked accounts: {}", all_blocked.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_all_blocks(&self) -> Result<Vec<BlockedProfileView>> {
        let mut all_blocks = Vec::new();
        let mut cursor: Option<String> = None;

        for _ in 0..MAX_PAGES {
            let response = self.get_blocks(Some(100), cursor).await?;
            all_blocks.extend(response.blocks);

            if response.cursor.is_none() {
                break;
            }
            cursor = response.cursor;
        }

        Ok(all_blocks)
    }

    /// Check if an account is blocked
    ///
    /// Searches through blocked accounts to find a specific DID.
    ///
    /// # Arguments
    ///
    /// * `did` - DID of the account to check
    ///
    /// # Returns
    ///
    /// `Some(block_uri)` if blocked, `None` otherwise
    ///
    /// # Note
    ///
    /// This method fetches all blocks and searches through them, which may be
    /// slow for accounts with many blocks. For checking block status of a
    /// specific user, consider using the profile viewer state instead.
    pub async fn find_block(&self, did: &str) -> Result<Option<String>> {
        let blocks = self.get_all_blocks().await?;

        for block in blocks {
            if block.did == did {
                if let Some(viewer) = block.viewer {
                    return Ok(viewer.blocking);
                }
            }
        }

        Ok(None)
    }

    // =========================================================================
    // Mute Operations
    // =========================================================================

    /// Mute an account
    ///
    /// Muting hides an account's posts from your feeds without notifying them.
    /// Unlike blocking, muted accounts can still view your profile and posts.
    ///
    /// # Arguments
    ///
    /// * `did` - DID of the account to mute
    ///
    /// # Errors
    ///
    /// - `BlockError::InvalidDid` - Empty or invalid DID
    /// - `BlockError::ApiError` - API error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::blocking::BlockService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = BlockService::new(client);
    /// service.mute("did:plc:example123").await?;
    /// println!("User muted");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn mute(&self, did: &str) -> Result<()> {
        self.validate_did(did)?;

        let body = serde_json::json!({
            "actor": did
        });

        let request = XrpcRequest::procedure("app.bsky.graph.muteActor")
            .json_body(&body)
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Unmute an account
    ///
    /// Removes the mute, allowing the account's posts to appear in your feeds again.
    ///
    /// # Arguments
    ///
    /// * `did` - DID of the account to unmute
    ///
    /// # Errors
    ///
    /// - `BlockError::InvalidDid` - Empty or invalid DID
    /// - `BlockError::ApiError` - API error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::blocking::BlockService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = BlockService::new(client);
    /// service.unmute("did:plc:example123").await?;
    /// println!("User unmuted");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unmute(&self, did: &str) -> Result<()> {
        self.validate_did(did)?;

        let body = serde_json::json!({
            "actor": did
        });

        let request = XrpcRequest::procedure("app.bsky.graph.unmuteActor")
            .json_body(&body)
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Get muted accounts with pagination
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results (default 50, max 100)
    /// * `cursor` - Pagination cursor
    ///
    /// # Returns
    ///
    /// Response containing muted profiles and pagination cursor
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::blocking::BlockService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = BlockService::new(client);
    /// let response = service.get_mutes(Some(25), None).await?;
    /// for muted in response.mutes {
    ///     println!("Muted: @{} ({})", muted.handle, muted.did);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_mutes(
        &self,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<GetMutesResponse> {
        let client = self.client.read().await;

        let mut request = XrpcRequest::query("app.bsky.graph.getMutes")
            .param("limit", limit.unwrap_or(PAGE_SIZE).min(100).to_string());

        if let Some(c) = cursor {
            request = request.param("cursor", c);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        let data: GetMutesResponse =
            serde_json::from_value(response.data).map_err(BlockError::Serialization)?;

        Ok(data)
    }

    /// Get all muted accounts (up to MAX_PAGES worth)
    ///
    /// Automatically paginates through all muted accounts.
    ///
    /// # Returns
    ///
    /// Vector of all muted profiles
    pub async fn get_all_mutes(&self) -> Result<Vec<MutedProfileView>> {
        let mut all_mutes = Vec::new();
        let mut cursor: Option<String> = None;

        for _ in 0..MAX_PAGES {
            let response = self.get_mutes(Some(100), cursor).await?;
            all_mutes.extend(response.mutes);

            if response.cursor.is_none() {
                break;
            }
            cursor = response.cursor;
        }

        Ok(all_mutes)
    }

    /// Check if an account is muted
    ///
    /// Searches through muted accounts to find a specific DID.
    ///
    /// # Arguments
    ///
    /// * `did` - DID of the account to check
    ///
    /// # Returns
    ///
    /// `true` if muted, `false` otherwise
    pub async fn is_muted(&self, did: &str) -> Result<bool> {
        let mutes = self.get_all_mutes().await?;
        Ok(mutes.iter().any(|m| m.did == did))
    }

    // =========================================================================
    // List Muting
    // =========================================================================

    /// Mute a moderation list
    ///
    /// Muting a list will hide posts from all accounts in that list.
    ///
    /// # Arguments
    ///
    /// * `list_uri` - URI of the moderation list to mute
    ///
    /// # Errors
    ///
    /// - `BlockError::InvalidUri` - Empty URI
    /// - `BlockError::ApiError` - API error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::blocking::BlockService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = BlockService::new(client);
    /// service.mute_list("at://did:plc:abc/app.bsky.graph.list/modlist").await?;
    /// println!("List muted");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn mute_list(&self, list_uri: &str) -> Result<()> {
        if list_uri.is_empty() {
            return Err(BlockError::InvalidUri("List URI cannot be empty".to_string()));
        }

        let body = serde_json::json!({
            "list": list_uri
        });

        let request = XrpcRequest::procedure("app.bsky.graph.muteActorList")
            .json_body(&body)
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Unmute a moderation list
    ///
    /// # Arguments
    ///
    /// * `list_uri` - URI of the moderation list to unmute
    ///
    /// # Errors
    ///
    /// - `BlockError::InvalidUri` - Empty URI
    /// - `BlockError::ApiError` - API error
    pub async fn unmute_list(&self, list_uri: &str) -> Result<()> {
        if list_uri.is_empty() {
            return Err(BlockError::InvalidUri("List URI cannot be empty".to_string()));
        }

        let body = serde_json::json!({
            "list": list_uri
        });

        let request = XrpcRequest::procedure("app.bsky.graph.unmuteActorList")
            .json_body(&body)
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| BlockError::ApiError(e.to_string()))?;

        Ok(())
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Validate a DID format
    fn validate_did(&self, did: &str) -> Result<()> {
        if did.is_empty() {
            return Err(BlockError::InvalidDid("DID cannot be empty".to_string()));
        }

        if !did.starts_with("did:") {
            return Err(BlockError::InvalidDid(format!("DID must start with 'did:': {}", did)));
        }

        Ok(())
    }

    /// Parse a record URI into repo and rkey components
    ///
    /// # Arguments
    ///
    /// * `uri` - AT URI (e.g., `at://did:plc:abc/collection/rkey`)
    /// * `expected_collection` - Expected collection name for validation
    ///
    /// # Returns
    ///
    /// Tuple of (repo, rkey)
    fn parse_record_uri(&self, uri: &str, expected_collection: &str) -> Result<(String, String)> {
        if uri.is_empty() {
            return Err(BlockError::InvalidUri("URI cannot be empty".to_string()));
        }

        // Parse the AT URI: at://did:plc:xyz/collection/rkey
        let uri_parts: Vec<&str> = uri.trim_start_matches("at://").split('/').collect();

        if uri_parts.len() < 3 {
            return Err(BlockError::InvalidUri(format!(
                "Invalid URI format, expected at://repo/collection/rkey: {}",
                uri
            )));
        }

        let repo = uri_parts[0];
        let collection = uri_parts[1];
        let rkey = uri_parts[2];

        if collection != expected_collection {
            return Err(BlockError::InvalidUri(format!(
                "Expected collection '{}', got '{}' in URI: {}",
                expected_collection, collection, uri
            )));
        }

        Ok((repo.to_string(), rkey.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Type Serialization Tests
    // =========================================================================

    #[test]
    fn test_blocked_profile_view_serialization() {
        let profile = BlockedProfileView {
            did: "did:plc:test123".to_string(),
            handle: "blocked.test".to_string(),
            display_name: Some("Blocked User".to_string()),
            avatar: Some("https://example.com/avatar.jpg".to_string()),
            labels: None,
            viewer: Some(BlockedViewerState {
                blocking: Some("at://did:plc:me/app.bsky.graph.block/abc".to_string()),
                blocked_by: None,
            }),
        };

        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("did:plc:test123"));
        assert!(json.contains("blocked.test"));
        assert!(json.contains("Blocked User"));

        let deserialized: BlockedProfileView = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, "did:plc:test123");
        assert_eq!(deserialized.handle, "blocked.test");
    }

    #[test]
    fn test_blocked_viewer_state_serialization() {
        let state = BlockedViewerState {
            blocking: Some("at://did:plc:me/app.bsky.graph.block/xyz".to_string()),
            blocked_by: Some(true),
        };

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("blocking"));
        assert!(json.contains("blockedBy"));

        let deserialized: BlockedViewerState = serde_json::from_str(&json).unwrap();
        assert!(deserialized.blocking.is_some());
        assert_eq!(deserialized.blocked_by, Some(true));
    }

    #[test]
    fn test_blocked_viewer_state_default() {
        let state = BlockedViewerState::default();
        assert!(state.blocking.is_none());
        assert!(state.blocked_by.is_none());
    }

    #[test]
    fn test_muted_profile_view_serialization() {
        let profile = MutedProfileView {
            did: "did:plc:muted123".to_string(),
            handle: "muted.test".to_string(),
            display_name: None,
            avatar: None,
            labels: None,
            viewer: Some(MutedViewerState { muted: Some(true), muted_by_list: None }),
        };

        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("did:plc:muted123"));
        assert!(json.contains("muted.test"));

        let deserialized: MutedProfileView = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, "did:plc:muted123");
    }

    #[test]
    fn test_muted_viewer_state_with_list() {
        let state = MutedViewerState {
            muted: Some(true),
            muted_by_list: Some("at://did:plc:abc/app.bsky.graph.list/modlist".to_string()),
        };

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("mutedByList"));
        assert!(json.contains("modlist"));
    }

    #[test]
    fn test_get_blocks_response_serialization() {
        let response = GetBlocksResponse {
            cursor: Some("cursor123".to_string()),
            blocks: vec![BlockedProfileView {
                did: "did:plc:blocked1".to_string(),
                handle: "blocked1.test".to_string(),
                display_name: None,
                avatar: None,
                labels: None,
                viewer: None,
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("cursor"));
        assert!(json.contains("blocks"));

        let deserialized: GetBlocksResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cursor, Some("cursor123".to_string()));
        assert_eq!(deserialized.blocks.len(), 1);
    }

    #[test]
    fn test_get_blocks_response_without_cursor() {
        let response = GetBlocksResponse { cursor: None, blocks: vec![] };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("cursor"));
    }

    #[test]
    fn test_get_mutes_response_serialization() {
        let response = GetMutesResponse {
            cursor: None,
            mutes: vec![
                MutedProfileView {
                    did: "did:plc:muted1".to_string(),
                    handle: "muted1.test".to_string(),
                    display_name: Some("Muted One".to_string()),
                    avatar: None,
                    labels: None,
                    viewer: None,
                },
                MutedProfileView {
                    did: "did:plc:muted2".to_string(),
                    handle: "muted2.test".to_string(),
                    display_name: None,
                    avatar: None,
                    labels: None,
                    viewer: None,
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: GetMutesResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.mutes.len(), 2);
    }

    #[test]
    fn test_create_record_response_serialization() {
        let json = r#"{"uri":"at://did:plc:test/app.bsky.graph.block/abc123","cid":"bafytest"}"#;
        let response: CreateRecordResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.uri, "at://did:plc:test/app.bsky.graph.block/abc123");
        assert_eq!(response.cid, "bafytest");
    }

    // =========================================================================
    // Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_did_empty() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = BlockService::new(client);

        let result = service.validate_did("");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, BlockError::InvalidDid(_)));
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_validate_did_invalid_prefix() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = BlockService::new(client);

        let result = service.validate_did("not-a-did");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("did:"));
    }

    #[test]
    fn test_validate_did_valid() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = BlockService::new(client);

        assert!(service.validate_did("did:plc:test123").is_ok());
        assert!(service.validate_did("did:web:example.com").is_ok());
    }

    #[test]
    fn test_parse_record_uri_empty() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = BlockService::new(client);

        let result = service.parse_record_uri("", "app.bsky.graph.block");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_parse_record_uri_invalid_format() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = BlockService::new(client);

        let result = service.parse_record_uri("at://did:plc:test", "app.bsky.graph.block");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("format"));
    }

    #[test]
    fn test_parse_record_uri_wrong_collection() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = BlockService::new(client);

        let result = service
            .parse_record_uri("at://did:plc:test/app.bsky.feed.like/abc", "app.bsky.graph.block");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Expected collection"));
    }

    #[test]
    fn test_parse_record_uri_valid() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = BlockService::new(client);

        let result = service.parse_record_uri(
            "at://did:plc:abc123/app.bsky.graph.block/rkey456",
            "app.bsky.graph.block",
        );
        assert!(result.is_ok());

        let (repo, rkey) = result.unwrap();
        assert_eq!(repo, "did:plc:abc123");
        assert_eq!(rkey, "rkey456");
    }

    // =========================================================================
    // Error Type Tests
    // =========================================================================

    #[test]
    fn test_block_error_display() {
        let err = BlockError::InvalidDid("bad did".to_string());
        assert!(err.to_string().contains("Invalid DID"));
        assert!(err.to_string().contains("bad did"));

        let err = BlockError::ApiError("network failed".to_string());
        assert!(err.to_string().contains("API error"));

        let err = BlockError::NotFound("record".to_string());
        assert!(err.to_string().contains("Not found"));

        let err = BlockError::NoSession;
        assert!(err.to_string().contains("session"));

        let err = BlockError::InvalidUri("bad uri".to_string());
        assert!(err.to_string().contains("Invalid URI"));
    }

    #[test]
    fn test_block_error_from_serde() {
        let json_err = serde_json::from_str::<BlockedProfileView>("not valid json").unwrap_err();
        let block_err: BlockError = json_err.into();
        assert!(matches!(block_err, BlockError::Serialization(_)));
    }

    // =========================================================================
    // Service Creation Tests
    // =========================================================================

    #[test]
    fn test_block_service_new() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let _service = BlockService::new(client);
        // Service should be created successfully
    }

    #[test]
    fn test_block_service_with_shared_client() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let shared = Arc::new(RwLock::new(client));
        let _service = BlockService::with_shared_client(shared);
        // Service should be created successfully
    }

    // =========================================================================
    // JSON Response Parsing Tests
    // =========================================================================

    #[test]
    fn test_get_blocks_response_from_api() {
        let json = r#"{
            "cursor": "next_page_token",
            "blocks": [
                {
                    "did": "did:plc:blocked1",
                    "handle": "blocked1.bsky.social",
                    "displayName": "Blocked User 1",
                    "viewer": {
                        "blocking": "at://did:plc:me/app.bsky.graph.block/abc"
                    }
                },
                {
                    "did": "did:plc:blocked2",
                    "handle": "blocked2.bsky.social"
                }
            ]
        }"#;

        let response: GetBlocksResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.cursor, Some("next_page_token".to_string()));
        assert_eq!(response.blocks.len(), 2);
        assert_eq!(response.blocks[0].did, "did:plc:blocked1");
        assert_eq!(response.blocks[0].display_name, Some("Blocked User 1".to_string()));
        assert!(response.blocks[0].viewer.is_some());
        assert!(response.blocks[1].viewer.is_none());
    }

    #[test]
    fn test_get_mutes_response_from_api() {
        let json = r#"{
            "mutes": [
                {
                    "did": "did:plc:muted1",
                    "handle": "muted1.bsky.social",
                    "viewer": {
                        "muted": true,
                        "mutedByList": "at://did:plc:xyz/app.bsky.graph.list/modlist"
                    }
                }
            ]
        }"#;

        let response: GetMutesResponse = serde_json::from_str(json).unwrap();
        assert!(response.cursor.is_none());
        assert_eq!(response.mutes.len(), 1);
        assert_eq!(response.mutes[0].did, "did:plc:muted1");

        let viewer = response.mutes[0].viewer.as_ref().unwrap();
        assert_eq!(viewer.muted, Some(true));
        assert!(viewer.muted_by_list.is_some());
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_blocked_profile_minimal() {
        let json = r#"{
            "did": "did:plc:minimal",
            "handle": "minimal.test"
        }"#;

        let profile: BlockedProfileView = serde_json::from_str(json).unwrap();
        assert_eq!(profile.did, "did:plc:minimal");
        assert_eq!(profile.handle, "minimal.test");
        assert!(profile.display_name.is_none());
        assert!(profile.avatar.is_none());
        assert!(profile.labels.is_none());
        assert!(profile.viewer.is_none());
    }

    #[test]
    fn test_muted_profile_minimal() {
        let json = r#"{
            "did": "did:plc:minimal",
            "handle": "minimal.test"
        }"#;

        let profile: MutedProfileView = serde_json::from_str(json).unwrap();
        assert_eq!(profile.did, "did:plc:minimal");
        assert_eq!(profile.handle, "minimal.test");
    }

    #[test]
    fn test_empty_blocks_response() {
        let json = r#"{"blocks": []}"#;

        let response: GetBlocksResponse = serde_json::from_str(json).unwrap();
        assert!(response.cursor.is_none());
        assert!(response.blocks.is_empty());
    }

    #[test]
    fn test_empty_mutes_response() {
        let json = r#"{"mutes": []}"#;

        let response: GetMutesResponse = serde_json::from_str(json).unwrap();
        assert!(response.cursor.is_none());
        assert!(response.mutes.is_empty());
    }

    // =========================================================================
    // Clone and Debug Tests
    // =========================================================================

    #[test]
    fn test_blocked_profile_view_clone() {
        let profile = BlockedProfileView {
            did: "did:plc:test".to_string(),
            handle: "test.bsky.social".to_string(),
            display_name: Some("Test".to_string()),
            avatar: None,
            labels: None,
            viewer: None,
        };

        let cloned = profile.clone();
        assert_eq!(profile, cloned);
    }

    #[test]
    fn test_muted_profile_view_clone() {
        let profile = MutedProfileView {
            did: "did:plc:test".to_string(),
            handle: "test.bsky.social".to_string(),
            display_name: None,
            avatar: None,
            labels: None,
            viewer: Some(MutedViewerState { muted: Some(true), muted_by_list: None }),
        };

        let cloned = profile.clone();
        assert_eq!(profile, cloned);
    }

    #[test]
    fn test_blocked_profile_view_debug() {
        let profile = BlockedProfileView {
            did: "did:plc:test".to_string(),
            handle: "test.bsky.social".to_string(),
            display_name: None,
            avatar: None,
            labels: None,
            viewer: None,
        };

        let debug_str = format!("{:?}", profile);
        assert!(debug_str.contains("BlockedProfileView"));
        assert!(debug_str.contains("did:plc:test"));
    }

    #[test]
    fn test_block_error_debug() {
        let err = BlockError::InvalidDid("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidDid"));
    }

    // =========================================================================
    // Constant Tests
    // =========================================================================

    #[test]
    fn test_page_size_constant() {
        assert_eq!(PAGE_SIZE, 50);
    }

    #[test]
    fn test_max_pages_constant() {
        assert_eq!(MAX_PAGES, 10);
    }
}
