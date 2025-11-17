//! Search functionality
//!
//! This module provides search capabilities for actors (users) and posts,
//! including typeahead/autocomplete search and paginated results.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::profiles::{ProfileView, ProfileViewBasic};
use atproto_client::xrpc::XrpcClient;

/// Errors that can occur during search operations
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    /// Network or API error
    #[error("API error: {0}")]
    ApiError(String),

    /// JSON parsing error
    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    /// Invalid query
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

/// Result type for search operations
pub type Result<T> = std::result::Result<T, SearchError>;

/// Parameters for actor search
#[derive(Debug, Clone, Default)]
pub struct ActorSearchParams {
    /// Search query (handle or display name)
    pub query: String,

    /// Pagination cursor
    pub cursor: Option<String>,

    /// Number of results to return (default 25, max 100)
    pub limit: u32,
}

/// Response from actor search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSearchResponse {
    /// Search results
    pub actors: Vec<ProfileView>,

    /// Pagination cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Parameters for actor typeahead/autocomplete search
#[derive(Debug, Clone, Default)]
pub struct ActorTypeaheadParams {
    /// Search query prefix
    pub query: String,

    /// Number of results to return (default 8, max 25)
    pub limit: u32,
}

/// Response from actor typeahead search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorTypeaheadResponse {
    /// Typeahead results (basic profile views)
    pub actors: Vec<ProfileViewBasic>,
}

/// Actor search service
pub struct ActorSearchService {
    client: Arc<RwLock<XrpcClient>>,
}

impl ActorSearchService {
    /// Create a new actor search service
    pub fn new(client: Arc<RwLock<XrpcClient>>) -> Self {
        Self { client }
    }

    /// Search for actors (users) by handle or display name
    ///
    /// This provides paginated search results with full profile information.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::search::{ActorSearchService, ActorSearchParams};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let search = ActorSearchService::new(client);
    /// let params = ActorSearchParams {
    ///     query: "alice".to_string(),
    ///     cursor: None,
    ///     limit: 25,
    /// };
    /// let results = search.search_actors(params).await?;
    /// for actor in results.actors {
    ///     println!("@{}: {}", actor.handle, actor.display_name.unwrap_or_default());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_actors(&self, params: ActorSearchParams) -> Result<ActorSearchResponse> {
        if params.query.trim().is_empty() {
            return Err(SearchError::InvalidQuery(
                "Query cannot be empty".to_string(),
            ));
        }

        let client = self.client.read().await;

        let mut request = atproto_client::XrpcRequest::query("app.bsky.actor.searchActors")
            .param("q", params.query.trim())
            .param("limit", params.limit.to_string());

        if let Some(cursor) = params.cursor {
            request = request.param("cursor", cursor);
        }

        let response = client
            .query(request)
            .await
            .map_err(|e| SearchError::ApiError(e.to_string()))?;

        let search_response: ActorSearchResponse = serde_json::from_value(response.data)
            .map_err(SearchError::ParseError)?;

        Ok(search_response)
    }

    /// Fast typeahead search for actor autocomplete
    ///
    /// This provides quick results for autocomplete/typeahead UIs,
    /// returning basic profile information.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use app_core::search::{ActorSearchService, ActorTypeaheadParams};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::default();
    /// # let client = Arc::new(RwLock::new(XrpcClient::new(config)));
    /// let search = ActorSearchService::new(client);
    /// let params = ActorTypeaheadParams {
    ///     query: "ali".to_string(),
    ///     limit: 8,
    /// };
    /// let results = search.search_actors_typeahead(params).await?;
    /// for actor in results.actors {
    ///     println!("@{}", actor.handle);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_actors_typeahead(
        &self,
        params: ActorTypeaheadParams,
    ) -> Result<ActorTypeaheadResponse> {
        let mut query = params.query.trim().to_lowercase();

        // Remove trailing dot for better UX (going from "foo" to "foo." shouldn't clear results)
        if query.ends_with('.') {
            query = query.trim_end_matches('.').to_string();
        }

        if query.is_empty() {
            return Ok(ActorTypeaheadResponse { actors: vec![] });
        }

        let client = self.client.read().await;

        let request =
            atproto_client::XrpcRequest::query("app.bsky.actor.searchActorsTypeahead")
                .param("q", &query)
                .param("limit", params.limit.to_string());

        let response = client
            .query(request)
            .await
            .map_err(|e| SearchError::ApiError(e.to_string()))?;

        let search_response: ActorTypeaheadResponse = serde_json::from_value(response.data)
            .map_err(SearchError::ParseError)?;

        Ok(search_response)
    }
}

/// Search result ranking utilities
pub struct SearchRanking;

impl SearchRanking {
    /// Deduplicate search results by handle
    ///
    /// Removes duplicate profiles based on handle, keeping the first occurrence.
    pub fn deduplicate_by_handle(profiles: Vec<ProfileViewBasic>) -> Vec<ProfileViewBasic> {
        let mut seen = std::collections::HashSet::new();
        profiles
            .into_iter()
            .filter(|profile| {
                if seen.contains(&profile.handle) {
                    false
                } else {
                    seen.insert(profile.handle.clone());
                    true
                }
            })
            .collect()
    }

    /// Rank search results by relevance
    ///
    /// Ranks results based on:
    /// - Exact matches (handle exactly matches query)
    /// - Prefix matches (handle starts with query)
    /// - Contains matches (handle contains query)
    /// - Display name matches
    pub fn rank_by_relevance(
        profiles: Vec<ProfileViewBasic>,
        query: &str,
    ) -> Vec<ProfileViewBasic> {
        let query_lower = query.to_lowercase();
        let mut results = profiles;

        results.sort_by(|a, b| {
            let a_score = Self::calculate_score(&a.handle, a.display_name.as_deref(), &query_lower);
            let b_score = Self::calculate_score(&b.handle, b.display_name.as_deref(), &query_lower);
            b_score.cmp(&a_score) // Higher scores first
        });

        results
    }

    fn calculate_score(handle: &str, display_name: Option<&str>, query: &str) -> u32 {
        let handle_lower = handle.to_lowercase();
        let mut score = 0u32;

        // Exact handle match
        if handle_lower == query {
            score += 1000;
        }
        // Handle starts with query
        else if handle_lower.starts_with(query) {
            score += 500;
        }
        // Handle contains query
        else if handle_lower.contains(query) {
            score += 100;
        }

        // Display name matches
        if let Some(name) = display_name {
            let name_lower = name.to_lowercase();
            if name_lower == query {
                score += 800;
            } else if name_lower.starts_with(query) {
                score += 400;
            } else if name_lower.contains(query) {
                score += 50;
            }
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_profile(handle: &str, display_name: Option<&str>) -> ProfileViewBasic {
        ProfileViewBasic {
            did: format!("did:plc:{}", handle),
            handle: handle.to_string(),
            display_name: display_name.map(|s| s.to_string()),
            avatar: None,
            associated: None,
            viewer: None,
            labels: None,
            created_at: None,
        }
    }

    #[test]
    fn test_actor_search_params_default() {
        let params = ActorSearchParams::default();
        assert_eq!(params.query, "");
        assert_eq!(params.cursor, None);
        assert_eq!(params.limit, 0);
    }

    #[test]
    fn test_actor_typeahead_params_default() {
        let params = ActorTypeaheadParams::default();
        assert_eq!(params.query, "");
        assert_eq!(params.limit, 0);
    }

    #[test]
    fn test_deduplicate_by_handle() {
        let profiles = vec![
            create_test_profile("alice.bsky.social", Some("Alice")),
            create_test_profile("bob.bsky.social", Some("Bob")),
            create_test_profile("alice.bsky.social", Some("Alice Duplicate")),
        ];

        let deduped = SearchRanking::deduplicate_by_handle(profiles);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].handle, "alice.bsky.social");
        assert_eq!(deduped[1].handle, "bob.bsky.social");
        // First occurrence is kept
        assert_eq!(deduped[0].display_name, Some("Alice".to_string()));
    }

    #[test]
    fn test_rank_by_relevance_exact_match() {
        let profiles = vec![
            create_test_profile("alice.bsky.social", Some("Alice")),
            create_test_profile("alice", Some("Alice Original")),
            create_test_profile("bob.alice", Some("Bob")),
        ];

        let ranked = SearchRanking::rank_by_relevance(profiles, "alice");

        // Exact handle match should be first
        assert_eq!(ranked[0].handle, "alice");
    }

    #[test]
    fn test_rank_by_relevance_prefix_match() {
        let profiles = vec![
            create_test_profile("bob.bsky.social", Some("Bob")),
            create_test_profile("alice.bsky.social", Some("Alice")),
            create_test_profile("alice123.bsky.social", Some("Alice 123")),
        ];

        let ranked = SearchRanking::rank_by_relevance(profiles, "alice");

        // Profiles starting with "alice" should come before others
        assert!(ranked[0].handle.starts_with("alice"));
        assert!(ranked[1].handle.starts_with("alice"));
    }

    #[test]
    fn test_rank_by_relevance_display_name() {
        let profiles = vec![
            create_test_profile("user1.bsky.social", Some("Alice")),
            create_test_profile("user2.bsky.social", Some("Bob")),
            create_test_profile("alice.bsky.social", Some("Someone")),
        ];

        let ranked = SearchRanking::rank_by_relevance(profiles, "alice");

        // Exact display name match (800) should beat prefix handle match (500)
        assert_eq!(ranked[0].display_name, Some("Alice".to_string()));

        // Prefix handle match should come second
        assert_eq!(ranked[1].handle, "alice.bsky.social");

        // No match should come last
        assert_eq!(ranked[2].handle, "user2.bsky.social");
    }

    #[test]
    fn test_rank_by_relevance_contains() {
        let profiles = vec![
            create_test_profile("bob.bsky.social", Some("Bob")),
            create_test_profile("realalice.bsky.social", Some("Real Alice")),
            create_test_profile("alice.bsky.social", Some("Alice")),
        ];

        let ranked = SearchRanking::rank_by_relevance(profiles, "alice");

        // Exact or prefix match should beat contains match
        assert_eq!(ranked[0].handle, "alice.bsky.social");
    }

    #[test]
    fn test_calculate_score_exact_handle() {
        let score = SearchRanking::calculate_score("alice", None, "alice");
        assert_eq!(score, 1000);
    }

    #[test]
    fn test_calculate_score_prefix_handle() {
        let score = SearchRanking::calculate_score("alice123", None, "alice");
        assert_eq!(score, 500);
    }

    #[test]
    fn test_calculate_score_contains_handle() {
        let score = SearchRanking::calculate_score("realalice", None, "alice");
        assert_eq!(score, 100);
    }

    #[test]
    fn test_calculate_score_display_name() {
        let score = SearchRanking::calculate_score("user123", Some("Alice"), "alice");
        assert_eq!(score, 800);
    }

    #[test]
    fn test_calculate_score_combined() {
        // Both handle and display name match
        let score = SearchRanking::calculate_score("alice", Some("Alice"), "alice");
        assert_eq!(score, 1800); // 1000 (exact handle) + 800 (exact display name)
    }

    #[test]
    fn test_actor_search_response_serialization() {
        let response = ActorSearchResponse {
            actors: vec![],
            cursor: Some("next-page".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ActorSearchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cursor, Some("next-page".to_string()));
    }

    #[test]
    fn test_actor_typeahead_response_serialization() {
        let response = ActorTypeaheadResponse { actors: vec![] };

        let json = serde_json::to_string(&response).unwrap();
        let _deserialized: ActorTypeaheadResponse = serde_json::from_str(&json).unwrap();
    }
}
