//! Autocomplete service for mentions and hashtags
//!
//! Provides autocomplete functionality for user mentions (@) and hashtags (#).
//! Features include:
//! - Actor search for mention autocomplete
//! - Tag suggestions for hashtag autocomplete
//! - Result caching for performance
//! - Integration with RichTextEditor

use crate::editor::{AutocompleteSuggestion, SuggestionType};
use atproto_client::xrpc::XrpcClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;

/// Maximum number of autocomplete results to return
pub const MAX_AUTOCOMPLETE_RESULTS: usize = 10;

/// Cache entry expiration time (5 minutes)
const CACHE_EXPIRATION: Duration = Duration::from_secs(300);

/// Errors that can occur during autocomplete operations
#[derive(Debug, Error)]
pub enum AutocompleteError {
    /// XRPC client error
    #[error("XRPC error: {0}")]
    Xrpc(#[from] atproto_client::xrpc::XrpcError),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid query
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),
}

/// Result type for autocomplete operations
pub type Result<T> = std::result::Result<T, AutocompleteError>;

/// Actor profile for mention autocomplete
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorProfile {
    /// Actor's DID
    pub did: String,
    /// Actor's handle
    pub handle: String,
    /// Display name
    pub display_name: Option<String>,
    /// Avatar URL
    pub avatar: Option<String>,
    /// Bio/description
    pub description: Option<String>,
}

/// Hashtag suggestion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashtagSuggestion {
    /// The tag (without #)
    pub tag: String,
    /// Number of uses (if available)
    pub count: Option<u64>,
}

/// Autocomplete result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AutocompleteResult {
    /// Mention result
    Mention(ActorProfile),
    /// Hashtag result
    Hashtag(HashtagSuggestion),
}

impl AutocompleteResult {
    /// Get the display text for this result
    pub fn display_text(&self) -> String {
        match self {
            AutocompleteResult::Mention(actor) => {
                if let Some(display_name) = &actor.display_name {
                    format!("{} (@{})", display_name, actor.handle)
                } else {
                    format!("@{}", actor.handle)
                }
            }
            AutocompleteResult::Hashtag(tag) => {
                if let Some(count) = tag.count {
                    format!("#{} ({})", tag.tag, count)
                } else {
                    format!("#{}", tag.tag)
                }
            }
        }
    }

    /// Get the completion value (what gets inserted)
    pub fn completion_value(&self) -> String {
        match self {
            AutocompleteResult::Mention(actor) => actor.handle.clone(),
            AutocompleteResult::Hashtag(tag) => tag.tag.clone(),
        }
    }

    /// Get the DID for mention results
    pub fn did(&self) -> Option<&str> {
        match self {
            AutocompleteResult::Mention(actor) => Some(&actor.did),
            AutocompleteResult::Hashtag(_) => None,
        }
    }
}

/// Cache entry with expiration
#[derive(Debug, Clone)]
struct CacheEntry {
    results: Vec<AutocompleteResult>,
    timestamp: Instant,
}

impl CacheEntry {
    fn new(results: Vec<AutocompleteResult>) -> Self {
        Self {
            results,
            timestamp: Instant::now(),
        }
    }

    fn is_expired(&self) -> bool {
        self.timestamp.elapsed() > CACHE_EXPIRATION
    }
}

/// Response from actor search
#[derive(Debug, Deserialize)]
struct ActorSearchResponse {
    actors: Vec<ActorProfile>,
}

/// Autocomplete service
///
/// Provides autocomplete functionality for mentions and hashtags with caching.
///
/// # Example
///
/// ```rust,no_run
/// use app_core::autocomplete::AutocompleteService;
/// use app_core::editor::{RichTextEditor, SuggestionType, AutocompleteSuggestion};
/// use atproto_client::xrpc::XrpcClient;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Arc::new(RwLock::new(XrpcClient::new("https://bsky.social")?));
/// let service = AutocompleteService::new(client);
///
/// let suggestion = AutocompleteSuggestion {
///     suggestion_type: SuggestionType::Mention,
///     trigger_position: 0,
///     query: "ali".to_string(),
/// };
///
/// let results = service.search(&suggestion).await?;
/// for result in results {
///     println!("{}", result.display_text());
/// }
/// # Ok(())
/// # }
/// ```
pub struct AutocompleteService {
    client: Arc<RwLock<XrpcClient>>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl AutocompleteService {
    /// Create a new autocomplete service
    pub fn new(client: Arc<RwLock<XrpcClient>>) -> Self {
        Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Search for autocomplete results based on a suggestion
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::autocomplete::AutocompleteService;
    /// # use app_core::editor::{AutocompleteSuggestion, SuggestionType};
    /// # use atproto_client::xrpc::XrpcClient;
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(RwLock::new(XrpcClient::new("https://bsky.social")?));
    /// # let service = AutocompleteService::new(client);
    /// let suggestion = AutocompleteSuggestion {
    ///     suggestion_type: SuggestionType::Mention,
    ///     trigger_position: 0,
    ///     query: "alice".to_string(),
    /// };
    ///
    /// let results = service.search(&suggestion).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search(&self, suggestion: &AutocompleteSuggestion) -> Result<Vec<AutocompleteResult>> {
        // Check cache first
        let cache_key = format!("{:?}:{}", suggestion.suggestion_type, suggestion.query);

        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&cache_key) {
                if !entry.is_expired() {
                    return Ok(entry.results.clone());
                }
            }
        }

        // Perform search based on suggestion type
        let results = match suggestion.suggestion_type {
            SuggestionType::Mention => self.search_actors(&suggestion.query).await?,
            SuggestionType::Hashtag => self.search_hashtags(&suggestion.query).await?,
        };

        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, CacheEntry::new(results.clone()));

        Ok(results)
    }

    /// Search for actors (users) for mention autocomplete
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::autocomplete::AutocompleteService;
    /// # use atproto_client::xrpc::XrpcClient;
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(RwLock::new(XrpcClient::new("https://bsky.social")?));
    /// # let service = AutocompleteService::new(client);
    /// let results = service.search_actors("alice").await?;
    /// for result in results {
    ///     println!("{}", result.display_text());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_actors(&self, query: &str) -> Result<Vec<AutocompleteResult>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }

        // Validate query length (AT Protocol limit)
        if query.len() > 100 {
            return Err(AutocompleteError::InvalidQuery(
                "Query too long".to_string(),
            ));
        }

        let client = self.client.read().await;

        // Use app.bsky.actor.searchActors
        let request = atproto_client::xrpc::XrpcRequest::query("app.bsky.actor.searchActors")
            .param("q", query)
            .param("limit", MAX_AUTOCOMPLETE_RESULTS.to_string());

        let response = client.query::<ActorSearchResponse>(request).await?;

        Ok(response
            .data
            .actors
            .into_iter()
            .map(AutocompleteResult::Mention)
            .collect())
    }

    /// Search for hashtags for hashtag autocomplete
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::autocomplete::AutocompleteService;
    /// # use atproto_client::xrpc::XrpcClient;
    /// # use std::sync::Arc;
    /// # use tokio::sync::RwLock;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client = Arc::new(RwLock::new(XrpcClient::new("https://bsky.social")?));
    /// # let service = AutocompleteService::new(client);
    /// let results = service.search_hashtags("rust").await?;
    /// for result in results {
    ///     println!("{}", result.display_text());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_hashtags(&self, query: &str) -> Result<Vec<AutocompleteResult>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }

        // Validate query length
        if query.len() > 100 {
            return Err(AutocompleteError::InvalidQuery(
                "Query too long".to_string(),
            ));
        }

        let client = self.client.read().await;

        // Use app.bsky.unspecced.getTaggedSuggestions for hashtag search
        let request = atproto_client::xrpc::XrpcRequest::query("app.bsky.unspecced.getTaggedSuggestions")
            .param("q", query)
            .param("limit", MAX_AUTOCOMPLETE_RESULTS.to_string());

        // Define a simple response type for hashtag suggestions
        #[derive(Deserialize)]
        struct TagSuggestionsResponse {
            suggestions: Vec<HashtagSuggestion>,
        }

        let response = client.query::<TagSuggestionsResponse>(request).await;

        // If the endpoint doesn't exist or fails, return local suggestions
        match response {
            Ok(data) => {
                Ok(data
                    .data
                    .suggestions
                    .into_iter()
                    .map(AutocompleteResult::Hashtag)
                    .collect())
            }
            Err(_) => {
                // Fallback to local suggestions
                Ok(self.get_local_hashtag_suggestions(query))
            }
        }
    }

    /// Get local hashtag suggestions (fallback when API is unavailable)
    fn get_local_hashtag_suggestions(&self, query: &str) -> Vec<AutocompleteResult> {
        // Common hashtags that match the query
        let common_tags = [
            "rust", "rustlang", "programming", "coding", "developer",
            "software", "tech", "technology", "opensource", "github",
            "bluesky", "atproto", "web3", "decentralized", "social",
            "art", "photography", "music", "news", "politics",
        ];

        common_tags
            .iter()
            .filter(|tag| tag.starts_with(query))
            .take(MAX_AUTOCOMPLETE_RESULTS)
            .map(|tag| {
                AutocompleteResult::Hashtag(HashtagSuggestion {
                    tag: tag.to_string(),
                    count: None,
                })
            })
            .collect()
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Remove expired entries from the cache
    pub async fn cleanup_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_client() -> Arc<RwLock<XrpcClient>> {
        let config = atproto_client::xrpc::XrpcClientConfig::new("https://bsky.social");
        Arc::new(RwLock::new(XrpcClient::new(config)))
    }

    #[test]
    fn test_actor_profile() {
        let profile = ActorProfile {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
            description: Some("Test user".to_string()),
        };

        assert_eq!(profile.did, "did:plc:test123");
        assert_eq!(profile.handle, "alice.bsky.social");
    }

    #[test]
    fn test_hashtag_suggestion() {
        let tag = HashtagSuggestion {
            tag: "rust".to_string(),
            count: Some(12345),
        };

        assert_eq!(tag.tag, "rust");
        assert_eq!(tag.count, Some(12345));
    }

    #[test]
    fn test_autocomplete_result_display_text_mention() {
        let result = AutocompleteResult::Mention(ActorProfile {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
            description: None,
        });

        assert_eq!(result.display_text(), "Alice (@alice.bsky.social)");
    }

    #[test]
    fn test_autocomplete_result_display_text_mention_no_display_name() {
        let result = AutocompleteResult::Mention(ActorProfile {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: None,
            avatar: None,
            description: None,
        });

        assert_eq!(result.display_text(), "@alice.bsky.social");
    }

    #[test]
    fn test_autocomplete_result_display_text_hashtag() {
        let result = AutocompleteResult::Hashtag(HashtagSuggestion {
            tag: "rust".to_string(),
            count: Some(12345),
        });

        assert_eq!(result.display_text(), "#rust (12345)");
    }

    #[test]
    fn test_autocomplete_result_display_text_hashtag_no_count() {
        let result = AutocompleteResult::Hashtag(HashtagSuggestion {
            tag: "rust".to_string(),
            count: None,
        });

        assert_eq!(result.display_text(), "#rust");
    }

    #[test]
    fn test_autocomplete_result_completion_value_mention() {
        let result = AutocompleteResult::Mention(ActorProfile {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: Some("Alice".to_string()),
            avatar: None,
            description: None,
        });

        assert_eq!(result.completion_value(), "alice.bsky.social");
    }

    #[test]
    fn test_autocomplete_result_completion_value_hashtag() {
        let result = AutocompleteResult::Hashtag(HashtagSuggestion {
            tag: "rust".to_string(),
            count: Some(12345),
        });

        assert_eq!(result.completion_value(), "rust");
    }

    #[test]
    fn test_autocomplete_result_did() {
        let mention = AutocompleteResult::Mention(ActorProfile {
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            display_name: None,
            avatar: None,
            description: None,
        });

        assert_eq!(mention.did(), Some("did:plc:test123"));

        let hashtag = AutocompleteResult::Hashtag(HashtagSuggestion {
            tag: "rust".to_string(),
            count: None,
        });

        assert_eq!(hashtag.did(), None);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new(vec![]);
        assert!(!entry.is_expired());
    }

    #[tokio::test]
    async fn test_autocomplete_service_creation() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        // Service should be created successfully
        assert_eq!(service.cache.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_search_actors_empty_query() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        let results = service.search_actors("").await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_actors_too_long() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        let long_query = "a".repeat(101);
        let result = service.search_actors(&long_query).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_hashtags_empty_query() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        let results = service.search_hashtags("").await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_hashtags_too_long() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        let long_query = "a".repeat(101);
        let result = service.search_hashtags(&long_query).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_local_hashtag_suggestions() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        let results = service.get_local_hashtag_suggestions("ru");
        assert!(!results.is_empty());

        // Should include "rust" and "rustlang"
        let tags: Vec<String> = results
            .iter()
            .map(|r| r.completion_value())
            .collect();

        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"rustlang".to_string()));
    }

    #[tokio::test]
    async fn test_local_hashtag_suggestions_no_match() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        let results = service.get_local_hashtag_suggestions("zzz");
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        // Add something to cache
        {
            let mut cache = service.cache.write().await;
            cache.insert(
                "test".to_string(),
                CacheEntry::new(vec![]),
            );
        }

        assert_eq!(service.cache.read().await.len(), 1);

        service.clear_cache().await;
        assert_eq!(service.cache.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_cache() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        // Add fresh entry
        {
            let mut cache = service.cache.write().await;
            cache.insert(
                "fresh".to_string(),
                CacheEntry::new(vec![]),
            );
        }

        service.cleanup_cache().await;

        // Fresh entry should still be there
        assert_eq!(service.cache.read().await.len(), 1);
    }

    #[test]
    fn test_autocomplete_error_display() {
        let error = AutocompleteError::InvalidQuery("test".to_string());
        assert_eq!(error.to_string(), "Invalid query: test");
    }

    #[tokio::test]
    async fn test_search_with_mention_suggestion() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        let suggestion = AutocompleteSuggestion {
            suggestion_type: SuggestionType::Mention,
            trigger_position: 0,
            query: "".to_string(), // Empty query
        };

        let results = service.search(&suggestion).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_search_with_hashtag_suggestion() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        let suggestion = AutocompleteSuggestion {
            suggestion_type: SuggestionType::Hashtag,
            trigger_position: 0,
            query: "rust".to_string(),
        };

        let results = service.search(&suggestion).await.unwrap();
        // Should use local fallback
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let client = create_test_client();
        let service = AutocompleteService::new(client);

        let suggestion = AutocompleteSuggestion {
            suggestion_type: SuggestionType::Hashtag,
            trigger_position: 0,
            query: "rust".to_string(),
        };

        // First call - populates cache
        let results1 = service.search(&suggestion).await.unwrap();

        // Second call - should hit cache
        let results2 = service.search(&suggestion).await.unwrap();

        assert_eq!(results1, results2);
    }
}
