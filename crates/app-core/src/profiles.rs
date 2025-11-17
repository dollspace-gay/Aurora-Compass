//! Profile viewing and management
//!
//! This module provides functionality for viewing and managing user profiles,
//! including fetching profile data, stats, and profile-related operations.

use atproto_client::xrpc::{XrpcClient, XrpcRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Profile service error types
#[derive(Debug, Error)]
pub enum ProfileError {
    /// XRPC error
    #[error("XRPC error: {0}")]
    Xrpc(String),

    /// Profile not found
    #[error("Profile not found: {0}")]
    NotFound(String),

    /// Invalid actor identifier
    #[error("Invalid actor identifier: {0}")]
    InvalidActor(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// No active session
    #[error("No active session")]
    NoSession,
}

/// Result type for profile operations
pub type Result<T> = std::result::Result<T, ProfileError>;

/// Image blob reference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageBlob {
    /// Content type (e.g., "image/jpeg")
    #[serde(rename = "$type")]
    pub blob_type: String,
    /// MIME type
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Size in bytes
    pub size: u64,
    /// CID reference
    #[serde(rename = "ref")]
    pub reference: serde_json::Value,
}

/// Viewer state for a profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileViewerState {
    /// Whether the current user is muting this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    /// URI of the mute list if muted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted_by_list: Option<String>,
    /// Whether the current user is blocking this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking: Option<String>,
    /// Whether this profile is blocking the current user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<bool>,
    /// URI of the follow record if following
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<String>,
    /// URI of the follow record if followed by this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followed_by: Option<String>,
}

/// Associated profile information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssociatedProfile {
    /// DID
    pub did: String,
    /// Handle
    pub handle: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Avatar image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Labels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<serde_json::Value>>,
}

/// Profile associated information (e.g., labeler info)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAssociated {
    /// Chat configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat: Option<serde_json::Value>,
    /// Labeler information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labeler: Option<bool>,
    /// Feed generator information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedgens: Option<i32>,
    /// Starter packs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starter_packs: Option<i32>,
}

/// Basic profile view
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileViewBasic {
    /// DID
    pub did: String,
    /// Handle
    pub handle: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Avatar image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Associated information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub associated: Option<ProfileAssociated>,
    /// Viewer state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<ProfileViewerState>,
    /// Labels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<serde_json::Value>>,
    /// Created at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// Standard profile view
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileView {
    /// DID
    pub did: String,
    /// Handle
    pub handle: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Profile description/bio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Avatar image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Associated information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub associated: Option<ProfileAssociated>,
    /// Indexed at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<String>,
    /// Viewer state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<ProfileViewerState>,
    /// Labels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<serde_json::Value>>,
    /// Created at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// Detailed profile view with stats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileViewDetailed {
    /// DID
    pub did: String,
    /// Handle
    pub handle: String,
    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Profile description/bio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Avatar image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Banner image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    /// Number of followers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub followers_count: Option<i64>,
    /// Number of accounts following
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follows_count: Option<i64>,
    /// Number of posts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posts_count: Option<i64>,
    /// Associated information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub associated: Option<ProfileAssociated>,
    /// Pinned post URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_post: Option<String>,
    /// Indexed at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<String>,
    /// Viewer state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<ProfileViewerState>,
    /// Labels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<serde_json::Value>>,
    /// Created at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// Response from getProfile API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProfileResponse {
    /// Profile data
    #[serde(flatten)]
    pub profile: ProfileViewDetailed,
}

/// Response from getProfiles API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProfilesResponse {
    /// List of profiles
    pub profiles: Vec<ProfileViewDetailed>,
}

/// Profile service for fetching and managing profiles
///
/// Provides methods for fetching profile data, including basic profile info,
/// detailed stats, and batch profile fetching.
///
/// # Example
///
/// ```rust,no_run
/// use app_core::profiles::ProfileService;
/// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create XRPC client (with auth)
///     let config = XrpcClientConfig::new("https://bsky.social");
///     let client = XrpcClient::new(config);
///     let service = ProfileService::new(client);
///
///     // Fetch a profile
///     let profile = service.get_profile("alice.bsky.social").await?;
///     println!("Profile: {} (@{})", profile.display_name.unwrap_or_default(), profile.handle);
///     println!("Followers: {}", profile.followers_count.unwrap_or(0));
///
///     Ok(())
/// }
/// ```
pub struct ProfileService {
    client: Arc<RwLock<XrpcClient>>,
}

impl ProfileService {
    /// Create a new profile service
    ///
    /// # Arguments
    ///
    /// * `client` - XRPC client for making API calls
    pub fn new(client: XrpcClient) -> Self {
        Self {
            client: Arc::new(RwLock::new(client)),
        }
    }

    /// Get a single profile by actor (DID or handle)
    ///
    /// # Arguments
    ///
    /// * `actor` - Actor identifier (DID or handle)
    ///
    /// # Returns
    ///
    /// Detailed profile view with stats
    ///
    /// # Errors
    ///
    /// - `ProfileError::NotFound` - Profile not found
    /// - `ProfileError::InvalidActor` - Invalid actor identifier
    /// - `ProfileError::Network` - Network error
    pub async fn get_profile(&self, actor: &str) -> Result<ProfileViewDetailed> {
        if actor.is_empty() {
            return Err(ProfileError::InvalidActor("Actor cannot be empty".to_string()));
        }

        let request = XrpcRequest::query("app.bsky.actor.getProfile")
            .param("actor", actor);

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        let profile: ProfileViewDetailed = serde_json::from_value(response.data)
            .map_err(ProfileError::Serialization)?;

        Ok(profile)
    }

    /// Get multiple profiles by actors (DIDs or handles)
    ///
    /// # Arguments
    ///
    /// * `actors` - List of actor identifiers (DIDs or handles), max 25
    ///
    /// # Returns
    ///
    /// List of detailed profile views
    ///
    /// # Errors
    ///
    /// - `ProfileError::InvalidActor` - Too many actors (max 25)
    /// - `ProfileError::Network` - Network error
    pub async fn get_profiles(&self, actors: &[String]) -> Result<Vec<ProfileViewDetailed>> {
        if actors.is_empty() {
            return Ok(Vec::new());
        }

        if actors.len() > 25 {
            return Err(ProfileError::InvalidActor(
                "Maximum 25 actors allowed per request".to_string(),
            ));
        }

        let mut request = XrpcRequest::query("app.bsky.actor.getProfiles");
        for actor in actors {
            request = request.param("actors", actor);
        }

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        let profiles_response: GetProfilesResponse = serde_json::from_value(response.data)
            .map_err(ProfileError::Serialization)?;

        Ok(profiles_response.profiles)
    }

    /// Search for profiles by keyword
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    /// * `limit` - Maximum number of results (default 25, max 100)
    ///
    /// # Returns
    ///
    /// List of matching profile views
    ///
    /// # Errors
    ///
    /// - `ProfileError::Network` - Network error
    pub async fn search_profiles(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<ProfileView>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let mut request = XrpcRequest::query("app.bsky.actor.searchActors")
            .param("q", query);

        if let Some(limit) = limit {
            request = request.param("limit", limit.to_string());
        }

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        #[derive(Deserialize)]
        struct SearchResponse {
            actors: Vec<ProfileView>,
        }

        let search_response: SearchResponse = serde_json::from_value(response.data)
            .map_err(ProfileError::Serialization)?;

        Ok(search_response.actors)
    }

    /// Get suggestions for profiles to follow
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of results (default 50, max 100)
    ///
    /// # Returns
    ///
    /// List of suggested profile views
    pub async fn get_suggestions(&self, limit: Option<u32>) -> Result<Vec<ProfileView>> {
        let mut request = XrpcRequest::query("app.bsky.actor.getSuggestions");

        if let Some(limit) = limit {
            request = request.param("limit", limit.to_string());
        }

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        #[derive(Deserialize)]
        struct SuggestionsResponse {
            actors: Vec<ProfileView>,
        }

        let suggestions_response: SuggestionsResponse = serde_json::from_value(response.data)
            .map_err(ProfileError::Serialization)?;

        Ok(suggestions_response.actors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_view_basic_serde() {
        let profile = ProfileViewBasic {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: Some("https://example.com/avatar.jpg".to_string()),
            associated: None,
            viewer: None,
            labels: None,
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: ProfileViewBasic = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, "did:plc:test123");
        assert_eq!(deserialized.handle, "alice.bsky.social");
        assert_eq!(deserialized.display_name, Some("Alice".to_string()));
    }

    #[test]
    fn test_profile_view_detailed_serde() {
        let profile = ProfileViewDetailed {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            description: Some("Software engineer".to_string()),
            avatar: Some("https://example.com/avatar.jpg".to_string()),
            banner: Some("https://example.com/banner.jpg".to_string()),
            followers_count: Some(100),
            follows_count: Some(50),
            posts_count: Some(250),
            associated: None,
            pinned_post: None,
            indexed_at: Some("2024-01-01T00:00:00Z".to_string()),
            viewer: None,
            labels: None,
            created_at: Some("2023-01-01T00:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: ProfileViewDetailed = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, "did:plc:test123");
        assert_eq!(deserialized.followers_count, Some(100));
        assert_eq!(deserialized.follows_count, Some(50));
        assert_eq!(deserialized.posts_count, Some(250));
    }

    #[test]
    fn test_viewer_state_serde() {
        let viewer = ProfileViewerState {
            muted: Some(false),
            muted_by_list: None,
            blocking: None,
            blocked_by: Some(false),
            following: Some("at://did:plc:me/app.bsky.graph.follow/abc123".to_string()),
            followed_by: Some("at://did:plc:them/app.bsky.graph.follow/def456".to_string()),
        };

        let json = serde_json::to_string(&viewer).unwrap();
        let deserialized: ProfileViewerState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.muted, Some(false));
        assert_eq!(
            deserialized.following,
            Some("at://did:plc:me/app.bsky.graph.follow/abc123".to_string())
        );
    }

    #[test]
    fn test_invalid_actor_validation() {
        // This would be tested with actual ProfileService instance
        // but we can validate the error type
        let error = ProfileError::InvalidActor("Test".to_string());
        assert!(error.to_string().contains("Invalid actor"));
    }

    #[test]
    fn test_profile_not_found_error() {
        let error = ProfileError::NotFound("alice.bsky.social".to_string());
        assert!(error.to_string().contains("Profile not found"));
    }
}
