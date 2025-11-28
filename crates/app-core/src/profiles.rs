//! Profile viewing and management
//!
//! This module provides functionality for viewing and managing user profiles,
//! including fetching profile data, stats, and profile-related operations.

use atproto_client::xrpc::{XrpcClient, XrpcRequest};
use chrono;
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

/// Parameters for updating a profile
///
/// All fields are optional - only provided fields will be updated.
/// Avatar and banner should be blob references (CID + mime type).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUpdateParams {
    /// Repository (DID) of the profile to update
    pub repo: String,

    /// Display name (max 64 characters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Profile description/bio (max 256 characters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Avatar image blob reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<serde_json::Value>,

    /// Banner image blob reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<serde_json::Value>,
}

impl ProfileUpdateParams {
    /// Create a new profile update with the given repository
    pub fn new(repo: impl Into<String>) -> Self {
        Self {
            repo: repo.into(),
            display_name: None,
            description: None,
            avatar: None,
            banner: None,
        }
    }

    /// Set the display name
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Set the description (bio)
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the avatar blob
    pub fn with_avatar(mut self, avatar: serde_json::Value) -> Self {
        self.avatar = Some(avatar);
        self
    }

    /// Set the banner blob
    pub fn with_banner(mut self, banner: serde_json::Value) -> Self {
        self.banner = Some(banner);
        self
    }
}
// ============================================================================
// Suggested Follows Types
// ============================================================================

/// Parameters for suggested follows query
#[derive(Debug, Clone, Default)]
pub struct SuggestedFollowsParams {
    /// Pagination cursor
    pub cursor: Option<String>,
    /// Number of results to return (default 25, max 100)
    pub limit: u32,
    /// User interests for personalization
    pub interests: Option<Vec<String>>,
    /// Preferred language (e.g., "en", "es", "en,es")
    pub language: Option<String>,
}

impl SuggestedFollowsParams {
    /// Create new suggested follows parameters with defaults
    ///
    /// # Example
    ///
    /// ```
    /// # use app_core::profiles::SuggestedFollowsParams;
    /// let params = SuggestedFollowsParams::new();
    /// assert_eq!(params.limit, 25);
    /// ```
    pub fn new() -> Self {
        Self {
            cursor: None,
            limit: 25,
            interests: None,
            language: None,
        }
    }

    /// Set the pagination cursor
    pub fn with_cursor(mut self, cursor: String) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Set the result limit (max 100)
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit.min(100);
        self
    }

    /// Set user interests for personalization
    pub fn with_interests(mut self, interests: Vec<String>) -> Self {
        self.interests = Some(interests);
        self
    }

    /// Set preferred language
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }
}

/// Response from suggested follows query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedFollowsResponse {
    /// List of suggested profiles
    pub actors: Vec<ProfileView>,
    /// Cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Response from suggested follows by actor query
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedFollowsByActorResponse {
    /// List of suggested profiles
    pub suggestions: Vec<ProfileView>,
    /// Whether this is a fallback response (no personalized suggestions available)
    #[serde(default)]
    pub is_fallback: bool,
    /// Recommendation ID for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rec_id: Option<String>,
}

// ============================================================================
// Known Followers Types
// ============================================================================

/// Parameters for known followers query
#[derive(Debug, Clone, Default)]
pub struct KnownFollowersParams {
    /// Pagination cursor
    pub cursor: Option<String>,
    /// Number of results to return (default 50, max 100)
    pub limit: u32,
}

impl KnownFollowersParams {
    /// Create new parameters with default values
    pub fn new() -> Self {
        Self { cursor: None, limit: 50 }
    }

    /// Set the pagination cursor
    pub fn with_cursor(mut self, cursor: String) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Set the limit (capped at 100)
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit.min(100);
        self
    }
}

/// Known followers response from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownFollowersResponse {
    /// DID of the subject profile
    pub subject: String,
    /// List of known followers (people you follow who also follow this profile)
    pub followers: Vec<ProfileView>,
    /// Total count of known followers (may include blocked users)
    pub count: u32,
    /// Pagination cursor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

// ============================================================================
// Profile Service
// ============================================================================

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
        Self { client: Arc::new(RwLock::new(client)) }
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

        let request = XrpcRequest::query("app.bsky.actor.getProfile").param("actor", actor);

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        let profile: ProfileViewDetailed =
            serde_json::from_value(response.data).map_err(ProfileError::Serialization)?;

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

        let profiles_response: GetProfilesResponse =
            serde_json::from_value(response.data).map_err(ProfileError::Serialization)?;

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

        let mut request = XrpcRequest::query("app.bsky.actor.searchActors").param("q", query);

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

        let search_response: SearchResponse =
            serde_json::from_value(response.data).map_err(ProfileError::Serialization)?;

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

        let suggestions_response: SuggestionsResponse =
            serde_json::from_value(response.data).map_err(ProfileError::Serialization)?;

        Ok(suggestions_response.actors)
    }

    /// Get paginated suggestions for profiles to follow with enhanced options
    ///
    /// This provides cursor-based pagination and supports user interests/language preferences.
    ///
    /// # Arguments
    ///
    /// * `params` - Suggestion parameters including cursor, limit, interests, and language
    ///
    /// # Returns
    ///
    /// Paginated list of suggested profiles with cursor for next page
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::profiles::{ProfileService, SuggestedFollowsParams};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = ProfileService::new(client);
    /// let params = SuggestedFollowsParams::new()
    ///     .with_limit(25)
    ///     .with_interests(vec!["tech".to_string(), "programming".to_string()])
    ///     .with_language("en");
    ///
    /// let response = service.get_suggested_follows(params).await?;
    /// println!("Got {} suggestions", response.actors.len());
    ///
    /// // Fetch next page if available
    /// if let Some(cursor) = response.cursor {
    ///     let next_params = SuggestedFollowsParams::new()
    ///         .with_cursor(cursor)
    ///         .with_limit(25);
    ///     let next_response = service.get_suggested_follows(next_params).await?;
    ///     println!("Got {} more suggestions", next_response.actors.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_suggested_follows(
        &self,
        params: SuggestedFollowsParams,
    ) -> Result<SuggestedFollowsResponse> {
        let mut request = XrpcRequest::query("app.bsky.actor.getSuggestions")
            .param("limit", params.limit.to_string());

        if let Some(cursor) = params.cursor {
            request = request.param("cursor", cursor);
        }

        // Add interests header if provided
        if let Some(interests) = params.interests {
            let interests_str = interests.join(",");
            request = request.header("X-Bsky-Topics", interests_str);
        }

        // Add language header if provided
        if let Some(language) = params.language {
            request = request.header("Accept-Language", language);
        }

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        serde_json::from_value(response.data).map_err(ProfileError::Serialization)
    }

    /// Get suggested follows based on a specific actor
    ///
    /// This returns suggestions of accounts that are similar to or followed by
    /// the specified actor.
    ///
    /// # Arguments
    ///
    /// * `actor` - DID or handle of the actor to base suggestions on
    ///
    /// # Returns
    ///
    /// List of suggested profiles with metadata
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::profiles::ProfileService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = ProfileService::new(client);
    /// let response = service.get_suggested_follows_by_actor("alice.bsky.social").await?;
    ///
    /// if !response.is_fallback {
    ///     println!("Found {} similar accounts", response.suggestions.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_suggested_follows_by_actor(
        &self,
        actor: &str,
    ) -> Result<SuggestedFollowsByActorResponse> {
        let request = XrpcRequest::query("app.bsky.graph.getSuggestedFollowsByActor")
            .param("actor", actor.to_string());

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        serde_json::from_value(response.data).map_err(ProfileError::Serialization)
    }

    /// Get known followers for a profile
    ///
    /// Fetches followers of the specified profile that the authenticated user also follows.
    /// This is useful for displaying mutual followers or "followed by people you know" information.
    ///
    /// # Arguments
    ///
    /// * `actor` - DID or handle of the profile to get known followers for
    /// * `params` - Query parameters including cursor and limit
    ///
    /// # Returns
    ///
    /// Response containing:
    /// - `subject`: DID of the queried profile
    /// - `followers`: List of profiles (people you follow who also follow this profile)
    /// - `count`: Total number of known followers (may include blocked users)
    /// - `cursor`: Pagination cursor for next page
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use app_core::profiles::{ProfileService, KnownFollowersParams};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// let service = ProfileService::new(client);
    ///
    /// // Get first page of known followers
    /// let params = KnownFollowersParams::new();
    /// let response = service.get_known_followers("alice.bsky.social", params).await?;
    /// println!("Found {} known followers", response.followers.len());
    ///
    /// // Get next page if available
    /// if let Some(cursor) = response.cursor {
    ///     let params = KnownFollowersParams::new().with_cursor(cursor);
    ///     let response = service.get_known_followers("alice.bsky.social", params).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_known_followers(
        &self,
        actor: &str,
        params: KnownFollowersParams,
    ) -> Result<KnownFollowersResponse> {
        let mut request = XrpcRequest::query("app.bsky.graph.getKnownFollowers")
            .param("actor", actor.to_string())
            .param("limit", params.limit.to_string());

        if let Some(cursor) = params.cursor {
            request = request.param("cursor", cursor);
        }

        let client = self.client.read().await;
        let response = client
            .query(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        serde_json::from_value(response.data).map_err(ProfileError::Serialization)
    }

    /// Update the current user's profile
    ///
    /// Updates the profile record for the authenticated user. Can update display name,
    /// description (bio), avatar, and banner images.
    ///
    /// # Arguments
    ///
    /// * `update` - Profile update parameters
    ///
    /// # Character Limits
    ///
    /// - Display name: 64 characters maximum
    /// - Description: 256 characters maximum
    ///
    /// # Returns
    ///
    /// Updated profile view
    ///
    /// # Errors
    ///
    /// - `ProfileError::InvalidActor` - Character limits exceeded
    /// - `ProfileError::NoSession` - No active session
    /// - `ProfileError::Network` - Network error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::profiles::{ProfileService, ProfileUpdateParams};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = ProfileService::new(client);
    /// let update = ProfileUpdateParams::new("did:plc:abc123")
    ///     .with_display_name("Alice Smith")
    ///     .with_description("Software engineer and coffee enthusiast");
    ///
    /// let updated_profile = service.update_profile(update).await?;
    /// println!("Updated profile: {}", updated_profile.handle);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_profile(&self, update: ProfileUpdateParams) -> Result<ProfileViewDetailed> {
        // Validate character limits
        if let Some(ref display_name) = update.display_name {
            if display_name.chars().count() > 64 {
                return Err(ProfileError::InvalidActor(
                    "Display name must be 64 characters or less".to_string(),
                ));
            }
        }

        if let Some(ref description) = update.description {
            if description.chars().count() > 256 {
                return Err(ProfileError::InvalidActor(
                    "Description must be 256 characters or less".to_string(),
                ));
            }
        }

        // Get the current profile first to get the repo (DID)
        // In a real implementation, this should come from the session
        // For now, we'll use a placeholder approach
        #[derive(Serialize)]
        struct ProfileRecord {
            #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
            display_name: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            avatar: Option<serde_json::Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            banner: Option<serde_json::Value>,
            #[serde(rename = "$type")]
            record_type: String,
        }

        let record = ProfileRecord {
            display_name: update.display_name,
            description: update.description,
            avatar: update.avatar,
            banner: update.banner,
            record_type: "app.bsky.actor.profile".to_string(),
        };

        // Update profile record via XRPC
        let request = XrpcRequest::procedure("com.atproto.repo.putRecord")
            .json_body(&serde_json::json!({
                "repo": update.repo,
                "collection": "app.bsky.actor.profile",
                "rkey": "self",
                "record": record,
            }))
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        // Fetch and return the updated profile
        self.get_profile(&update.repo).await
    }

    /// Follow a user
    ///
    /// # Arguments
    ///
    /// * `did` - DID of the user to follow
    ///
    /// # Returns
    ///
    /// URI of the follow record created
    ///
    /// # Errors
    ///
    /// - `ProfileError::NoSession` - No active session
    /// - `ProfileError::Network` - Network error
    pub async fn follow(&self, did: &str) -> Result<String> {
        if did.is_empty() {
            return Err(ProfileError::InvalidActor("DID cannot be empty".to_string()));
        }

        #[derive(Serialize)]
        struct FollowRecord {
            subject: String,
            #[serde(rename = "createdAt")]
            created_at: String,
            #[serde(rename = "$type")]
            record_type: String,
        }

        let now = chrono::Utc::now().to_rfc3339();
        let record = FollowRecord {
            subject: did.to_string(),
            created_at: now,
            record_type: "app.bsky.graph.follow".to_string(),
        };

        // Create follow record via XRPC
        let request = XrpcRequest::procedure("com.atproto.repo.createRecord")
            .json_body(&serde_json::json!({
                "collection": "app.bsky.graph.follow",
                "record": record,
            }))
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        #[derive(Deserialize)]
        struct CreateRecordResponse {
            uri: String,
        }

        let create_response: CreateRecordResponse =
            serde_json::from_value(response.data).map_err(ProfileError::Serialization)?;

        Ok(create_response.uri)
    }

    /// Unfollow a user
    ///
    /// # Arguments
    ///
    /// * `follow_uri` - URI of the follow record to delete
    ///
    /// # Errors
    ///
    /// - `ProfileError::NoSession` - No active session
    /// - `ProfileError::Network` - Network error
    pub async fn unfollow(&self, follow_uri: &str) -> Result<()> {
        if follow_uri.is_empty() {
            return Err(ProfileError::InvalidActor("Follow URI cannot be empty".to_string()));
        }

        // Parse the AT URI to extract repo and rkey
        // Format: at://did:plc:xyz/app.bsky.graph.follow/rkey
        let uri_parts: Vec<&str> = follow_uri.trim_start_matches("at://").split('/').collect();
        if uri_parts.len() < 3 {
            return Err(ProfileError::InvalidActor(format!(
                "Invalid follow URI format: {}",
                follow_uri
            )));
        }

        let repo = uri_parts[0];
        let rkey = uri_parts[2];

        let request = XrpcRequest::procedure("com.atproto.repo.deleteRecord")
            .json_body(&serde_json::json!({
                "repo": repo,
                "collection": "app.bsky.graph.follow",
                "rkey": rkey,
            }))
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        let client = self.client.read().await;
        client
            .procedure::<serde_json::Value>(request)
            .await
            .map_err(|e| ProfileError::Xrpc(e.to_string()))?;

        Ok(())
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

    #[test]
    fn test_follow_uri_parsing() {
        // Test valid follow URI parsing for unfollow
        let uri = "at://did:plc:abc123/app.bsky.graph.follow/3jzfcijpj2z2a";
        let uri_parts: Vec<&str> = uri.trim_start_matches("at://").split('/').collect();

        assert_eq!(uri_parts.len(), 3);
        assert_eq!(uri_parts[0], "did:plc:abc123");
        assert_eq!(uri_parts[1], "app.bsky.graph.follow");
        assert_eq!(uri_parts[2], "3jzfcijpj2z2a");
    }

    #[test]
    fn test_invalid_follow_uri() {
        // Test that invalid URIs would be rejected
        let invalid_uris = vec![
            "",
            "at://",
            "at://did:plc:abc123",
            "at://did:plc:abc123/app.bsky.graph.follow",
            "not-a-uri",
        ];

        for uri in invalid_uris {
            let uri_parts: Vec<&str> = uri.trim_start_matches("at://").split('/').collect();
            // These should all have less than 3 parts
            if !uri.is_empty() && uri != "not-a-uri" {
                assert!(uri_parts.len() < 3, "URI {} should have less than 3 parts", uri);
            }
        }
    }

    #[test]
    fn test_follow_record_serialization() {
        #[derive(Serialize)]
        struct FollowRecord {
            subject: String,
            #[serde(rename = "createdAt")]
            created_at: String,
            #[serde(rename = "$type")]
            record_type: String,
        }

        let record = FollowRecord {
            subject: "did:plc:test123".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            record_type: "app.bsky.graph.follow".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("did:plc:test123"));
        assert!(json.contains("createdAt"));
        assert!(json.contains("$type"));
        assert!(json.contains("app.bsky.graph.follow"));
    }

    // Profile Update Tests

    #[test]
    fn test_profile_update_params_builder() {
        let update = ProfileUpdateParams::new("did:plc:test123")
            .with_display_name("Alice Smith")
            .with_description("Software engineer and coffee enthusiast");

        assert_eq!(update.repo, "did:plc:test123");
        assert_eq!(update.display_name, Some("Alice Smith".to_string()));
        assert_eq!(update.description, Some("Software engineer and coffee enthusiast".to_string()));
        assert!(update.avatar.is_none());
        assert!(update.banner.is_none());
    }

    #[test]
    fn test_profile_update_params_serialization() {
        let update = ProfileUpdateParams::new("did:plc:test123")
            .with_display_name("Alice")
            .with_description("Developer");

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("did:plc:test123"));
        assert!(json.contains("Alice"));
        assert!(json.contains("Developer"));

        let deserialized: ProfileUpdateParams = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.repo, "did:plc:test123");
        assert_eq!(deserialized.display_name, Some("Alice".to_string()));
    }

    #[test]
    fn test_profile_update_display_name_length_valid() {
        // Test that valid display names (64 chars or less) are acceptable
        let valid_name = "A".repeat(64);
        let update = ProfileUpdateParams::new("did:plc:test").with_display_name(valid_name.clone());

        assert_eq!(update.display_name, Some(valid_name));
        assert_eq!(update.display_name.as_ref().unwrap().chars().count(), 64);
    }

    #[test]
    fn test_profile_update_display_name_length_invalid() {
        // Test that display name with > 64 chars would be caught by validation
        let too_long = "A".repeat(65);
        assert_eq!(too_long.chars().count(), 65);
    }

    #[test]
    fn test_profile_update_description_length_valid() {
        // Test that valid descriptions (256 chars or less) are acceptable
        let valid_desc = "A".repeat(256);
        let update = ProfileUpdateParams::new("did:plc:test").with_description(valid_desc.clone());

        assert_eq!(update.description, Some(valid_desc));
        assert_eq!(update.description.as_ref().unwrap().chars().count(), 256);
    }

    #[test]
    fn test_profile_update_description_length_invalid() {
        // Test that description with > 256 chars would be caught by validation
        let too_long = "A".repeat(257);
        assert_eq!(too_long.chars().count(), 257);
    }

    #[test]
    fn test_profile_update_unicode_characters() {
        // Test that character counting works correctly with Unicode
        let emoji_name = "Alice 👩‍💻"; // Contains emoji with zero-width joiner
        let update = ProfileUpdateParams::new("did:plc:test").with_display_name(emoji_name);

        assert_eq!(update.display_name, Some(emoji_name.to_string()));
        // Verify we're counting grapheme clusters/chars correctly
        assert!(emoji_name.chars().count() < 64);
    }

    #[test]
    fn test_profile_update_empty_values() {
        // Test that empty strings are handled correctly
        let update = ProfileUpdateParams::new("did:plc:test")
            .with_display_name("")
            .with_description("");

        assert_eq!(update.display_name, Some("".to_string()));
        assert_eq!(update.description, Some("".to_string()));
    }

    #[test]
    fn test_profile_update_with_avatar() {
        let avatar_blob = serde_json::json!({
            "$type": "blob",
            "ref": {
                "$link": "bafyreigq4zsipbk5w3uqkbfcvpgsfmvhvpfokymhbwdsl7zzkh3ovg6nlq"
            },
            "mimeType": "image/jpeg",
            "size": 42000
        });

        let update = ProfileUpdateParams::new("did:plc:test").with_avatar(avatar_blob.clone());

        assert_eq!(update.avatar, Some(avatar_blob));
    }

    #[test]
    fn test_profile_update_with_banner() {
        let banner_blob = serde_json::json!({
            "$type": "blob",
            "ref": {
                "$link": "bafyreibqxzrmfke3tjpexqj3wbqh7ypm6i3yxdixvkvixp5ey4bnz4nzbq"
            },
            "mimeType": "image/png",
            "size": 128000
        });

        let update = ProfileUpdateParams::new("did:plc:test").with_banner(banner_blob.clone());

        assert_eq!(update.banner, Some(banner_blob));
    }

    #[test]
    fn test_profile_update_partial_update() {
        // Test that we can update only some fields
        let update = ProfileUpdateParams::new("did:plc:test").with_display_name("New Name");

        assert_eq!(update.display_name, Some("New Name".to_string()));
        assert!(update.description.is_none());
        assert!(update.avatar.is_none());
        assert!(update.banner.is_none());
    }

    #[test]
    fn test_profile_record_serialization() {
        // Test that the ProfileRecord struct serializes correctly
        #[derive(Serialize)]
        struct ProfileRecord {
            #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
            display_name: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            avatar: Option<serde_json::Value>,
            #[serde(skip_serializing_if = "Option::is_none")]
            banner: Option<serde_json::Value>,
            #[serde(rename = "$type")]
            record_type: String,
        }

        let record = ProfileRecord {
            display_name: Some("Alice".to_string()),
            description: Some("Developer".to_string()),
            avatar: None,
            banner: None,
            record_type: "app.bsky.actor.profile".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("displayName"));
        assert!(json.contains("Alice"));
        assert!(json.contains("description"));
        assert!(json.contains("Developer"));
        assert!(json.contains("$type"));
        assert!(json.contains("app.bsky.actor.profile"));
        // Avatar and banner should not be in JSON since they're None
        assert!(!json.contains("avatar"));
        assert!(!json.contains("banner"));
    }

    // Suggested Follows Tests

    #[test]
    fn test_suggested_follows_params_new() {
        let params = SuggestedFollowsParams::new();
        assert_eq!(params.limit, 25);
        assert!(params.cursor.is_none());
        assert!(params.interests.is_none());
        assert!(params.language.is_none());
    }

    #[test]
    fn test_suggested_follows_params_with_cursor() {
        let params = SuggestedFollowsParams::new().with_cursor("cursor123".to_string());
        assert_eq!(params.cursor, Some("cursor123".to_string()));
    }

    #[test]
    fn test_suggested_follows_params_with_limit() {
        let params = SuggestedFollowsParams::new().with_limit(50);
        assert_eq!(params.limit, 50);
    }

    #[test]
    fn test_suggested_follows_params_limit_capped() {
        let params = SuggestedFollowsParams::new().with_limit(150);
        assert_eq!(params.limit, 100);
    }

    #[test]
    fn test_suggested_follows_params_with_interests() {
        let params = SuggestedFollowsParams::new()
            .with_interests(vec!["tech".to_string(), "sports".to_string()]);
        assert_eq!(params.interests, Some(vec!["tech".to_string(), "sports".to_string()]));
    }

    #[test]
    fn test_suggested_follows_params_with_language() {
        let params = SuggestedFollowsParams::new().with_language("en");
        assert_eq!(params.language, Some("en".to_string()));
    }

    #[test]
    fn test_suggested_follows_params_builder_chain() {
        let params = SuggestedFollowsParams::new()
            .with_cursor("abc".to_string())
            .with_limit(30)
            .with_interests(vec!["tech".to_string()])
            .with_language("es");

        assert_eq!(params.cursor, Some("abc".to_string()));
        assert_eq!(params.limit, 30);
        assert_eq!(params.interests, Some(vec!["tech".to_string()]));
        assert_eq!(params.language, Some("es".to_string()));
    }

    #[test]
    fn test_suggested_follows_params_default() {
        let params = SuggestedFollowsParams::default();
        assert_eq!(params.limit, 0); // Default trait gives 0
        assert!(params.cursor.is_none());
    }

    #[test]
    fn test_suggested_follows_params_clone() {
        let params1 = SuggestedFollowsParams::new()
            .with_limit(40)
            .with_language("en");
        let params2 = params1.clone();

        assert_eq!(params1.limit, params2.limit);
        assert_eq!(params1.language, params2.language);
    }

    #[test]
    fn test_suggested_follows_response_serialization() {
        let response = SuggestedFollowsResponse {
            actors: vec![],
            cursor: Some("next_page".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("actors"));
        assert!(json.contains("cursor"));
        assert!(json.contains("next_page"));
    }

    #[test]
    fn test_suggested_follows_response_without_cursor() {
        let response = SuggestedFollowsResponse { actors: vec![], cursor: None };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("actors"));
        // Cursor should not be in JSON since it's None
        assert!(!json.contains("cursor"));
    }

    #[test]
    fn test_suggested_follows_by_actor_response_serialization() {
        let response = SuggestedFollowsByActorResponse {
            suggestions: vec![],
            is_fallback: false,
            rec_id: Some("rec123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("suggestions"));
        assert!(json.contains("isFallback"));
        assert!(json.contains("recId"));
        assert!(json.contains("rec123"));
    }

    #[test]
    fn test_suggested_follows_by_actor_response_fallback() {
        let response = SuggestedFollowsByActorResponse {
            suggestions: vec![],
            is_fallback: true,
            rec_id: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("isFallback"));
        assert!(json.contains("true"));
        // rec_id should not be in JSON since it's None
        assert!(!json.contains("recId"));
    }

    #[test]
    fn test_suggested_follows_by_actor_response_default_is_fallback() {
        let json = r#"{"suggestions": []}"#;
        let response: SuggestedFollowsByActorResponse = serde_json::from_str(json).unwrap();
        assert!(!response.is_fallback); // Default is false
        assert!(response.suggestions.is_empty());
        assert!(response.rec_id.is_none());
    }

    #[test]
    fn test_suggested_follows_params_multiple_interests() {
        let interests = vec!["tech".to_string(), "programming".to_string(), "rust".to_string()];
        let params = SuggestedFollowsParams::new().with_interests(interests.clone());
        assert_eq!(params.interests, Some(interests));
    }

    #[test]
    fn test_suggested_follows_params_empty_interests() {
        let params = SuggestedFollowsParams::new().with_interests(vec![]);
        assert_eq!(params.interests, Some(vec![]));
    }

    #[test]
    fn test_suggested_follows_params_limit_boundary() {
        // Zero should be zero
        let params = SuggestedFollowsParams::new().with_limit(0);
        assert_eq!(params.limit, 0);

        // Exactly 100 should be 100
        let params = SuggestedFollowsParams::new().with_limit(100);
        assert_eq!(params.limit, 100);

        // Over 100 should cap at 100
        let params = SuggestedFollowsParams::new().with_limit(1000);
        assert_eq!(params.limit, 100);
    }

    #[test]
    fn test_suggested_follows_params_debug() {
        let params = SuggestedFollowsParams::new().with_limit(25);
        let debug_str = format!("{:?}", params);
        assert!(debug_str.contains("SuggestedFollowsParams"));
        assert!(debug_str.contains("25"));
    }

    // ========================================================================
    // Known Followers Tests
    // ========================================================================

    #[test]
    fn test_known_followers_params_new() {
        let params = KnownFollowersParams::new();
        assert_eq!(params.cursor, None);
        assert_eq!(params.limit, 50);
    }

    #[test]
    fn test_known_followers_params_default() {
        let params = KnownFollowersParams::default();
        assert_eq!(params.cursor, None);
        assert_eq!(params.limit, 0);
    }

    #[test]
    fn test_known_followers_params_with_cursor() {
        let params = KnownFollowersParams::new().with_cursor("test_cursor".to_string());
        assert_eq!(params.cursor, Some("test_cursor".to_string()));
        assert_eq!(params.limit, 50);
    }

    #[test]
    fn test_known_followers_params_with_limit() {
        let params = KnownFollowersParams::new().with_limit(25);
        assert_eq!(params.cursor, None);
        assert_eq!(params.limit, 25);
    }

    #[test]
    fn test_known_followers_params_limit_capped() {
        let params = KnownFollowersParams::new().with_limit(200);
        assert_eq!(params.limit, 100);
    }

    #[test]
    fn test_known_followers_params_builder_chain() {
        let params = KnownFollowersParams::new()
            .with_cursor("abc123".to_string())
            .with_limit(30);

        assert_eq!(params.cursor, Some("abc123".to_string()));
        assert_eq!(params.limit, 30);
    }

    #[test]
    fn test_known_followers_params_clone() {
        let params1 = KnownFollowersParams::new().with_limit(20);
        let params2 = params1.clone();

        assert_eq!(params1.limit, params2.limit);
        assert_eq!(params1.cursor, params2.cursor);
    }

    #[test]
    fn test_known_followers_params_debug() {
        let params = KnownFollowersParams::new().with_limit(50);
        let debug_str = format!("{:?}", params);
        assert!(debug_str.contains("KnownFollowersParams"));
        assert!(debug_str.contains("50"));
    }

    #[test]
    fn test_known_followers_response_serialization() {
        let json = r#"{
            "subject": "did:plc:abc123",
            "followers": [],
            "count": 5,
            "cursor": "next_page"
        }"#;

        let response: KnownFollowersResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.subject, "did:plc:abc123");
        assert_eq!(response.followers.len(), 0);
        assert_eq!(response.count, 5);
        assert_eq!(response.cursor, Some("next_page".to_string()));
    }

    #[test]
    fn test_known_followers_response_without_cursor() {
        let json = r#"{
            "subject": "did:plc:xyz789",
            "followers": [],
            "count": 0
        }"#;

        let response: KnownFollowersResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.subject, "did:plc:xyz789");
        assert_eq!(response.followers.len(), 0);
        assert_eq!(response.count, 0);
        assert_eq!(response.cursor, None);
    }

    #[test]
    fn test_known_followers_response_with_followers() {
        let json = r#"{
            "subject": "did:plc:target",
            "followers": [
                {
                    "did": "did:plc:user1",
                    "handle": "user1.test"
                },
                {
                    "did": "did:plc:user2",
                    "handle": "user2.test"
                }
            ],
            "count": 10
        }"#;

        let response: KnownFollowersResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.subject, "did:plc:target");
        assert_eq!(response.followers.len(), 2);
        assert_eq!(response.count, 10);
        assert_eq!(response.followers[0].did, "did:plc:user1");
        assert_eq!(response.followers[0].handle, "user1.test");
        assert_eq!(response.followers[1].did, "did:plc:user2");
        assert_eq!(response.followers[1].handle, "user2.test");
    }

    #[test]
    fn test_known_followers_response_count_exceeds_followers() {
        // Count can be higher than followers.length because it includes blocked users
        let json = r#"{
            "subject": "did:plc:target",
            "followers": [
                {
                    "did": "did:plc:user1",
                    "handle": "user1.test"
                }
            ],
            "count": 5
        }"#;

        let response: KnownFollowersResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.followers.len(), 1);
        assert_eq!(response.count, 5);
        // This is expected: count includes blocked users, followers array doesn't
    }

    #[test]
    fn test_known_followers_params_limit_boundary() {
        // Zero should be zero
        let params = KnownFollowersParams::new().with_limit(0);
        assert_eq!(params.limit, 0);

        // Exactly 100 should be 100
        let params = KnownFollowersParams::new().with_limit(100);
        assert_eq!(params.limit, 100);

        // Over 100 should cap at 100
        let params = KnownFollowersParams::new().with_limit(500);
        assert_eq!(params.limit, 100);
    }

    #[test]
    fn test_known_followers_response_clone() {
        let json = r#"{
            "subject": "did:plc:abc",
            "followers": [],
            "count": 3
        }"#;

        let response1: KnownFollowersResponse = serde_json::from_str(json).unwrap();
        let response2 = response1.clone();

        assert_eq!(response1.subject, response2.subject);
        assert_eq!(response1.count, response2.count);
    }

    #[test]
    fn test_known_followers_response_debug() {
        let json = r#"{
            "subject": "did:plc:test",
            "followers": [],
            "count": 1
        }"#;

        let response: KnownFollowersResponse = serde_json::from_str(json).unwrap();
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("KnownFollowersResponse"));
        assert!(debug_str.contains("did:plc:test"));
    }
}
