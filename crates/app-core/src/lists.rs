//! User list management
//!
//! This module provides functionality for creating and managing user lists,
//! including curate lists (for organizing users) and moderation lists
//! (for blocking/muting groups of users).

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::feeds::Label;
use crate::posts::Facet;
use crate::profiles::ProfileViewBasic;
use atproto_client::xrpc::XrpcClient;
use atproto_client::XrpcRequest;

/// Maximum items per page when fetching list members
const PAGE_SIZE: u32 = 30;

/// Maximum pages to fetch when getting all members
const MAX_PAGES: usize = 6;

/// Errors that can occur during list operations
#[derive(Debug, thiserror::Error)]
pub enum ListError {
    /// API error
    #[error("API error: {0}")]
    ApiError(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    /// List not found
    #[error("List not found: {0}")]
    NotFound(String),

    /// Not authorized
    #[error("Not authorized: {0}")]
    NotAuthorized(String),

    /// Invalid list purpose
    #[error("Invalid list purpose: {0}")]
    InvalidPurpose(String),

    /// No active session
    #[error("No active session")]
    NoSession,
}

/// Result type for list operations
pub type Result<T> = std::result::Result<T, ListError>;

/// Purpose of a list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ListPurpose {
    /// Curate list - for organizing users to follow
    #[serde(rename = "app.bsky.graph.defs#curatelist")]
    Curatelist,
    /// Moderation list - for muting/blocking groups
    #[serde(rename = "app.bsky.graph.defs#modlist")]
    Modlist,
    /// Reference list - for referencing users
    #[serde(rename = "app.bsky.graph.defs#referencelist")]
    Referencelist,
}

impl ListPurpose {
    /// Convert to AT Protocol string
    pub fn as_str(&self) -> &'static str {
        match self {
            ListPurpose::Curatelist => "app.bsky.graph.defs#curatelist",
            ListPurpose::Modlist => "app.bsky.graph.defs#modlist",
            ListPurpose::Referencelist => "app.bsky.graph.defs#referencelist",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "app.bsky.graph.defs#curatelist" => Some(ListPurpose::Curatelist),
            "app.bsky.graph.defs#modlist" => Some(ListPurpose::Modlist),
            "app.bsky.graph.defs#referencelist" => Some(ListPurpose::Referencelist),
            _ => None,
        }
    }

    /// Check if this is a moderation list
    pub fn is_modlist(&self) -> bool {
        matches!(self, ListPurpose::Modlist)
    }

    /// Check if this is a curate list
    pub fn is_curatelist(&self) -> bool {
        matches!(self, ListPurpose::Curatelist)
    }
}

impl std::fmt::Display for ListPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Viewer state for a list
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListViewerState {
    /// Whether the list is muted by the viewer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,

    /// URI of the block record if the list is blocked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
}

impl ListViewerState {
    /// Check if the list is muted
    pub fn is_muted(&self) -> bool {
        self.muted.unwrap_or(false)
    }

    /// Check if the list is blocked
    pub fn is_blocked(&self) -> bool {
        self.blocked.is_some()
    }
}

/// Basic list view
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListViewBasic {
    /// List URI
    pub uri: String,

    /// List CID
    pub cid: String,

    /// List name
    pub name: String,

    /// List purpose
    pub purpose: String,

    /// Avatar URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,

    /// Number of items in the list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_item_count: Option<u32>,

    /// Labels on the list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,

    /// Viewer state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<ListViewerState>,

    /// When the list was indexed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<String>,
}

impl ListViewBasic {
    /// Get the list purpose as an enum
    pub fn purpose_enum(&self) -> Option<ListPurpose> {
        ListPurpose::from_str(&self.purpose)
    }
}

/// Full list view with creator info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListView {
    /// List URI
    pub uri: String,

    /// List CID
    pub cid: String,

    /// List creator
    pub creator: ProfileViewBasic,

    /// List name
    pub name: String,

    /// List purpose
    pub purpose: String,

    /// List description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Description facets (rich text)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_facets: Option<Vec<Facet>>,

    /// Avatar URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,

    /// Number of items in the list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_item_count: Option<u32>,

    /// Labels on the list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Label>>,

    /// Viewer state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<ListViewerState>,

    /// When the list was indexed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<String>,
}

impl ListView {
    /// Get the list purpose as an enum
    pub fn purpose_enum(&self) -> Option<ListPurpose> {
        ListPurpose::from_str(&self.purpose)
    }

    /// Check if the current user owns this list
    pub fn is_owned_by(&self, did: &str) -> bool {
        self.creator.did == did
    }
}

/// A list item (member of a list)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItemView {
    /// List item URI
    pub uri: String,

    /// The user in the list
    pub subject: ProfileViewBasic,
}

/// Response from getList API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetListResponse {
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// The list
    pub list: ListView,

    /// Items in the list
    pub items: Vec<ListItemView>,
}

/// Response from getLists API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetListsResponse {
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,

    /// Lists
    pub lists: Vec<ListView>,
}

/// Parameters for creating a list
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateListParams {
    /// List purpose
    pub purpose: ListPurpose,

    /// List name
    pub name: String,

    /// List description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Description facets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_facets: Option<Vec<Facet>>,

    /// Avatar blob reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<serde_json::Value>,
}

/// Parameters for updating a list
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateListParams {
    /// List URI
    pub uri: String,

    /// New name
    pub name: String,

    /// New description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// New description facets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_facets: Option<Vec<Facet>>,

    /// New avatar blob reference (None to keep, Some(None) to remove)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Option<serde_json::Value>>,
}

/// Record for creating a list item
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItemRecord {
    /// Subject DID
    pub subject: String,

    /// List URI
    pub list: String,

    /// Creation timestamp
    pub created_at: String,
}

/// Response from record creation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecordResponse {
    /// Record URI
    pub uri: String,

    /// Record CID
    pub cid: String,
}

/// List service for managing user lists
pub struct ListService {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl ListService {
    /// Create a new list service
    pub fn new(client: Arc<RwLock<XrpcClient>>) -> Self {
        ListService { client }
    }

    /// Get a list by URI
    pub async fn get_list(
        &self,
        uri: &str,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<GetListResponse> {
        let client = self.client.read().await;

        let mut request = XrpcRequest::query("app.bsky.graph.getList")
            .param("list", uri.to_string())
            .param("limit", limit.unwrap_or(PAGE_SIZE).to_string());

        if let Some(c) = cursor {
            request = request.param("cursor", c);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        let data: GetListResponse = serde_json::from_value(response.data)?;
        Ok(data)
    }

    /// Get lists created by a user
    pub async fn get_lists(
        &self,
        actor: &str,
        limit: Option<u32>,
        cursor: Option<String>,
    ) -> Result<GetListsResponse> {
        let client = self.client.read().await;

        let mut request = XrpcRequest::query("app.bsky.graph.getLists")
            .param("actor", actor.to_string())
            .param("limit", limit.unwrap_or(PAGE_SIZE).to_string());

        if let Some(c) = cursor {
            request = request.param("cursor", c);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        let data: GetListsResponse = serde_json::from_value(response.data)?;
        Ok(data)
    }

    /// Get all members of a list (up to MAX_PAGES worth)
    pub async fn get_all_members(&self, uri: &str) -> Result<Vec<ListItemView>> {
        let mut all_items = Vec::new();
        let mut cursor: Option<String> = None;

        for _ in 0..MAX_PAGES {
            let response = self.get_list(uri, Some(50), cursor).await?;
            all_items.extend(response.items);

            if response.cursor.is_none() {
                break;
            }
            cursor = response.cursor;
        }

        Ok(all_items)
    }

    /// Create a new list
    pub async fn create_list(
        &self,
        repo: &str,
        params: CreateListParams,
    ) -> Result<CreateRecordResponse> {
        let client = self.client.read().await;

        let record = serde_json::json!({
            "$type": "app.bsky.graph.list",
            "purpose": params.purpose.as_str(),
            "name": params.name,
            "description": params.description,
            "descriptionFacets": params.description_facets,
            "avatar": params.avatar,
            "createdAt": chrono::Utc::now().to_rfc3339(),
        });

        let body = serde_json::json!({
            "repo": repo,
            "collection": "app.bsky.graph.list",
            "record": record,
        });

        let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .json_body(&body)
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        let response = client
            .procedure(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        let data: CreateRecordResponse = serde_json::from_value(response.data)?;
        Ok(data)
    }

    /// Add a user to a list
    pub async fn add_to_list(
        &self,
        repo: &str,
        list_uri: &str,
        subject_did: &str,
    ) -> Result<CreateRecordResponse> {
        let client = self.client.read().await;

        let record = ListItemRecord {
            subject: subject_did.to_string(),
            list: list_uri.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let body = serde_json::json!({
            "repo": repo,
            "collection": "app.bsky.graph.listitem",
            "record": record,
        });

        let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .json_body(&body)
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        let response = client
            .procedure(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        let data: CreateRecordResponse = serde_json::from_value(response.data)?;
        Ok(data)
    }

    /// Remove a user from a list
    pub async fn remove_from_list(&self, repo: &str, list_item_uri: &str) -> Result<()> {
        let client = self.client.read().await;

        // Parse the URI to get the rkey
        let rkey = list_item_uri
            .rsplit('/')
            .next()
            .ok_or_else(|| ListError::ApiError("Invalid list item URI".to_string()))?;

        let body = serde_json::json!({
            "repo": repo,
            "collection": "app.bsky.graph.listitem",
            "rkey": rkey,
        });

        let request = XrpcRequest::procedure("com.atproto.repo.deleteRecord")
            .json_body(&body)
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Delete a list and all its items
    pub async fn delete_list(&self, repo: &str, list_uri: &str) -> Result<()> {
        // First, get all list items
        let items = self.get_all_members(list_uri).await?;

        let client = self.client.read().await;

        // Build writes array for batch delete
        let mut writes: Vec<serde_json::Value> = Vec::new();

        // Add list items to delete
        for item in items {
            let rkey = item
                .uri
                .rsplit('/')
                .next()
                .ok_or_else(|| ListError::ApiError("Invalid item URI".to_string()))?;

            writes.push(serde_json::json!({
                "$type": "com.atproto.repo.applyWrites#delete",
                "collection": "app.bsky.graph.listitem",
                "rkey": rkey,
            }));
        }

        // Add the list itself
        let list_rkey = list_uri
            .rsplit('/')
            .next()
            .ok_or_else(|| ListError::ApiError("Invalid list URI".to_string()))?;

        writes.push(serde_json::json!({
            "$type": "com.atproto.repo.applyWrites#delete",
            "collection": "app.bsky.graph.list",
            "rkey": list_rkey,
        }));

        // Apply in chunks of 10
        for chunk in writes.chunks(10) {
            let body = serde_json::json!({
                "repo": repo,
                "writes": chunk,
            });

            let request = XrpcRequest::procedure("com.atproto.repo.applyWrites")
                .json_body(&body)
                .map_err(|e| ListError::ApiError(e.to_string()))?;

            client
                .procedure::<serde_json::Value>(request)
                .await
                .map_err(|e| ListError::ApiError(e.to_string()))?;
        }

        Ok(())
    }

    /// Mute a moderation list
    pub async fn mute_list(&self, list_uri: &str) -> Result<()> {
        let client = self.client.read().await;

        let body = serde_json::json!({
            "list": list_uri,
        });

        let request = XrpcRequest::procedure("app.bsky.graph.muteActorList")
            .json_body(&body)
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Unmute a moderation list
    pub async fn unmute_list(&self, list_uri: &str) -> Result<()> {
        let client = self.client.read().await;

        let body = serde_json::json!({
            "list": list_uri,
        });

        let request = XrpcRequest::procedure("app.bsky.graph.unmuteActorList")
            .json_body(&body)
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Block a moderation list
    pub async fn block_list(&self, repo: &str, list_uri: &str) -> Result<CreateRecordResponse> {
        let client = self.client.read().await;

        let record = serde_json::json!({
            "$type": "app.bsky.graph.listblock",
            "subject": list_uri,
            "createdAt": chrono::Utc::now().to_rfc3339(),
        });

        let body = serde_json::json!({
            "repo": repo,
            "collection": "app.bsky.graph.listblock",
            "record": record,
        });

        let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .json_body(&body)
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        let response = client
            .procedure(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        let data: CreateRecordResponse = serde_json::from_value(response.data)?;
        Ok(data)
    }

    /// Unblock a moderation list
    pub async fn unblock_list(&self, repo: &str, block_uri: &str) -> Result<()> {
        let client = self.client.read().await;

        let rkey = block_uri
            .rsplit('/')
            .next()
            .ok_or_else(|| ListError::ApiError("Invalid block URI".to_string()))?;

        let body = serde_json::json!({
            "repo": repo,
            "collection": "app.bsky.graph.listblock",
            "rkey": rkey,
        });

        let request = XrpcRequest::procedure("com.atproto.repo.deleteRecord")
            .json_body(&body)
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Update list metadata
    pub async fn update_list(&self, repo: &str, params: UpdateListParams) -> Result<()> {
        let client = self.client.read().await;

        let rkey = params
            .uri
            .rsplit('/')
            .next()
            .ok_or_else(|| ListError::ApiError("Invalid list URI".to_string()))?;

        // First get the current record
        let get_request = XrpcRequest::query("com.atproto.repo.getRecord")
            .param("repo", repo.to_string())
            .param("collection", "app.bsky.graph.list".to_string())
            .param("rkey", rkey.to_string());

        let get_response = client
            .query::<serde_json::Value>(get_request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        let mut record: serde_json::Value = get_response
            .data
            .get("value")
            .cloned()
            .ok_or_else(|| ListError::NotFound(params.uri.clone()))?;

        // Update fields
        record["name"] = serde_json::Value::String(params.name);
        record["description"] = params
            .description
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null);
        record["descriptionFacets"] = params
            .description_facets
            .map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null))
            .unwrap_or(serde_json::Value::Null);

        if let Some(avatar_opt) = params.avatar {
            record["avatar"] = avatar_opt.unwrap_or(serde_json::Value::Null);
        }

        let body = serde_json::json!({
            "repo": repo,
            "collection": "app.bsky.graph.list",
            "rkey": rkey,
            "record": record,
        });

        let request = XrpcRequest::procedure("com.atproto.repo.putRecord")
            .json_body(&body)
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| ListError::ApiError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_purpose_as_str() {
        assert_eq!(ListPurpose::Curatelist.as_str(), "app.bsky.graph.defs#curatelist");
        assert_eq!(ListPurpose::Modlist.as_str(), "app.bsky.graph.defs#modlist");
        assert_eq!(ListPurpose::Referencelist.as_str(), "app.bsky.graph.defs#referencelist");
    }

    #[test]
    fn test_list_purpose_from_str() {
        assert_eq!(
            ListPurpose::from_str("app.bsky.graph.defs#curatelist"),
            Some(ListPurpose::Curatelist)
        );
        assert_eq!(
            ListPurpose::from_str("app.bsky.graph.defs#modlist"),
            Some(ListPurpose::Modlist)
        );
        assert_eq!(
            ListPurpose::from_str("app.bsky.graph.defs#referencelist"),
            Some(ListPurpose::Referencelist)
        );
        assert_eq!(ListPurpose::from_str("invalid"), None);
    }

    #[test]
    fn test_list_purpose_is_modlist() {
        assert!(!ListPurpose::Curatelist.is_modlist());
        assert!(ListPurpose::Modlist.is_modlist());
        assert!(!ListPurpose::Referencelist.is_modlist());
    }

    #[test]
    fn test_list_purpose_is_curatelist() {
        assert!(ListPurpose::Curatelist.is_curatelist());
        assert!(!ListPurpose::Modlist.is_curatelist());
        assert!(!ListPurpose::Referencelist.is_curatelist());
    }

    #[test]
    fn test_list_viewer_state_default() {
        let state = ListViewerState::default();
        assert!(!state.is_muted());
        assert!(!state.is_blocked());
    }

    #[test]
    fn test_list_viewer_state_muted() {
        let state = ListViewerState { muted: Some(true), blocked: None };
        assert!(state.is_muted());
        assert!(!state.is_blocked());
    }

    #[test]
    fn test_list_viewer_state_blocked() {
        let state = ListViewerState {
            muted: None,
            blocked: Some("at://did:plc:abc/app.bsky.graph.listblock/123".to_string()),
        };
        assert!(!state.is_muted());
        assert!(state.is_blocked());
    }

    #[test]
    fn test_list_view_basic_purpose_enum() {
        let list = ListViewBasic {
            uri: "at://did:plc:abc/app.bsky.graph.list/123".to_string(),
            cid: "bafyreib".to_string(),
            name: "Test List".to_string(),
            purpose: "app.bsky.graph.defs#curatelist".to_string(),
            avatar: None,
            list_item_count: Some(5),
            labels: None,
            viewer: None,
            indexed_at: None,
        };

        assert_eq!(list.purpose_enum(), Some(ListPurpose::Curatelist));
    }

    #[test]
    fn test_list_view_is_owned_by() {
        let list = ListView {
            uri: "at://did:plc:abc/app.bsky.graph.list/123".to_string(),
            cid: "bafyreib".to_string(),
            creator: ProfileViewBasic {
                did: "did:plc:owner123".to_string(),
                handle: "owner.bsky.social".to_string(),
                display_name: None,
                avatar: None,
                associated: None,
                viewer: None,
                labels: None,
                created_at: None,
            },
            name: "Test List".to_string(),
            purpose: "app.bsky.graph.defs#modlist".to_string(),
            description: Some("A test list".to_string()),
            description_facets: None,
            avatar: None,
            list_item_count: Some(10),
            labels: None,
            viewer: None,
            indexed_at: None,
        };

        assert!(list.is_owned_by("did:plc:owner123"));
        assert!(!list.is_owned_by("did:plc:other456"));
    }

    #[test]
    fn test_list_purpose_display() {
        assert_eq!(format!("{}", ListPurpose::Curatelist), "app.bsky.graph.defs#curatelist");
        assert_eq!(format!("{}", ListPurpose::Modlist), "app.bsky.graph.defs#modlist");
    }

    #[test]
    fn test_create_list_params_serialization() {
        let params = CreateListParams {
            purpose: ListPurpose::Curatelist,
            name: "My List".to_string(),
            description: Some("A cool list".to_string()),
            description_facets: None,
            avatar: None,
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["name"], "My List");
        assert_eq!(json["description"], "A cool list");
    }

    #[test]
    fn test_list_item_record_serialization() {
        let record = ListItemRecord {
            subject: "did:plc:user123".to_string(),
            list: "at://did:plc:abc/app.bsky.graph.list/123".to_string(),
            created_at: "2024-01-15T10:00:00Z".to_string(),
        };

        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["subject"], "did:plc:user123");
        assert_eq!(json["list"], "at://did:plc:abc/app.bsky.graph.list/123");
    }
}
