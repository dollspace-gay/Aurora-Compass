//! Query management
//!
//! This module provides a reactive query system similar to TanStack Query for managing
//! server state with caching, background refetching, and stale-while-revalidate patterns.

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use storage::{CacheConfig, TieredCache};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;

/// Query errors
#[derive(Debug, Error)]
pub enum QueryError {
    /// Query fetch failed
    #[error("Query fetch failed: {0}")]
    FetchError(String),

    /// Query not found in cache
    #[error("Query not found: {0}")]
    NotFound(String),

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(#[from] storage::CacheError),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Query cancelled
    #[error("Query cancelled")]
    Cancelled,

    /// Timeout
    #[error("Query timeout: {0}")]
    Timeout(String),
}

/// Result type for query operations
pub type Result<T> = std::result::Result<T, QueryError>;

/// Query key for identifying and caching queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct QueryKey {
    /// Scope of the query (e.g., "posts", "profiles", "feeds")
    pub scope: String,

    /// Unique identifier within the scope
    pub id: String,

    /// Optional parameters
    pub params: HashMap<String, String>,
}

impl QueryKey {
    /// Create a new query key
    pub fn new(scope: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            id: id.into(),
            params: HashMap::new(),
        }
    }

    /// Add a parameter to the query key
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Convert to cache key string
    pub fn to_cache_key(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.scope.hash(&mut hasher);
        self.id.hash(&mut hasher);
        // Hash params in sorted order for consistency
        let mut params: Vec<_> = self.params.iter().collect();
        params.sort_by_key(|(k, _)| *k);
        for (k, v) in params {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }
        format!("query:{}:{}:{:x}", self.scope, self.id, hasher.finish())
    }
}

impl fmt::Display for QueryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.scope, self.id)?;
        if !self.params.is_empty() {
            write!(f, "?")?;
            let mut first = true;
            for (k, v) in &self.params {
                if !first {
                    write!(f, "&")?;
                }
                write!(f, "{}={}", k, v)?;
                first = false;
            }
        }
        Ok(())
    }
}

/// Query state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryState {
    /// Query is idle (not fetching)
    Idle,

    /// Query is fetching data
    Fetching,

    /// Query fetch succeeded
    Success,

    /// Query fetch failed
    Error,
}

/// Query metadata
#[derive(Debug, Clone)]
struct QueryMeta {
    /// Current state
    state: QueryState,

    /// When the data was last fetched
    fetched_at: Option<SystemTime>,

    /// When the data becomes stale
    stale_at: Option<SystemTime>,

    /// Number of fetch attempts
    fetch_count: u32,

    /// Last error if any
    last_error: Option<String>,
}

impl QueryMeta {
    fn new() -> Self {
        Self {
            state: QueryState::Idle,
            fetched_at: None,
            stale_at: None,
            fetch_count: 0,
            last_error: None,
        }
    }

    fn is_stale(&self) -> bool {
        if let Some(stale_at) = self.stale_at {
            SystemTime::now() >= stale_at
        } else {
            true
        }
    }
}

/// Query configuration
#[derive(Debug, Clone)]
pub struct QueryConfig {
    /// Time until data becomes stale
    pub stale_time: Duration,

    /// Time until data is garbage collected
    pub cache_time: Duration,

    /// Enable background refetching when stale
    pub refetch_on_stale: bool,

    /// Retry failed queries
    pub retry: bool,

    /// Maximum retry attempts
    pub retry_count: u32,

    /// Retry delay
    pub retry_delay: Duration,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            stale_time: Duration::from_secs(0), // Immediately stale by default
            cache_time: Duration::from_secs(300), // 5 minutes
            refetch_on_stale: true,
            retry: true,
            retry_count: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// Query trait for defining data fetching logic
#[async_trait]
pub trait Query: Send + Sync + Clone {
    /// The type of data this query returns
    type Data: Serialize + DeserializeOwned + Clone + Send + Sync;

    /// Fetch the data
    async fn fetch(&self) -> Result<Self::Data>;

    /// Get the query key
    fn key(&self) -> QueryKey;

    /// Get the query configuration
    fn config(&self) -> QueryConfig {
        QueryConfig::default()
    }
}

/// Query client for managing queries
pub struct QueryClient {
    cache: Arc<TieredCache<String>>,
    meta: Arc<RwLock<HashMap<String, QueryMeta>>>,
    background_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl QueryClient {
    /// Create a new query client
    pub fn new(cache_config: CacheConfig) -> Result<Self> {
        let cache = TieredCache::new(cache_config).map_err(QueryError::CacheError)?;

        Ok(Self {
            cache: Arc::new(cache),
            meta: Arc::new(RwLock::new(HashMap::new())),
            background_tasks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get query data, using cache if available
    pub async fn get<Q: Query + 'static>(&self, query: &Q) -> Result<Q::Data> {
        let key = query.key();
        let cache_key = key.to_cache_key();
        let config = query.config();

        // Check if we have cached data
        if let Ok(Some(cached)) = self.cache.get(&cache_key) {
            let data: Q::Data = serde_json::from_str(&cached)?;

            // Check if stale
            let meta = self.meta.read().await;
            if let Some(query_meta) = meta.get(&cache_key) {
                if !query_meta.is_stale() {
                    // Fresh data, return immediately
                    return Ok(data);
                }

                // Stale data - trigger background refetch if enabled
                if config.refetch_on_stale {
                    drop(meta);
                    self.spawn_background_refetch(query.clone(), cache_key.clone())
                        .await;
                } else {
                    drop(meta);
                }

                return Ok(data);
            }

            // No metadata but have cache - return it
            return Ok(data);
        }

        // No cache, fetch fresh data
        self.fetch(query).await
    }

    /// Spawn a background refetch task for stale data
    async fn spawn_background_refetch<Q: Query + 'static>(&self, query: Q, cache_key: String) {
        // Check if there's already a background fetch in progress for this key
        {
            let tasks = self.background_tasks.lock().await;
            if tasks.contains_key(&cache_key) {
                // Already fetching in background, skip
                return;
            }
        }

        // Clone Arc references for the background task
        let cache = Arc::clone(&self.cache);
        let meta = Arc::clone(&self.meta);
        let background_tasks = Arc::clone(&self.background_tasks);
        let task_cache_key = cache_key.clone();

        // Spawn background task
        let handle = tokio::spawn(async move {
            let config = query.config();

            // Update state to fetching
            {
                let mut meta_guard = meta.write().await;
                let query_meta = meta_guard
                    .entry(task_cache_key.clone())
                    .or_insert_with(QueryMeta::new);
                query_meta.state = QueryState::Fetching;
                query_meta.fetch_count += 1;
            }

            // Fetch with retry logic
            let mut last_error = None;
            let max_attempts = if config.retry { config.retry_count } else { 1 };

            for attempt in 0..max_attempts {
                if attempt > 0 {
                    tokio::time::sleep(config.retry_delay).await;
                }

                match query.fetch().await {
                    Ok(data) => {
                        // Store in cache
                        if let Ok(serialized) = serde_json::to_string(&data) {
                            let ttl = Some(config.cache_time);
                            let _ = cache.put(task_cache_key.clone(), serialized, ttl);
                        }

                        // Update metadata
                        let now = SystemTime::now();
                        let mut meta_guard = meta.write().await;
                        if let Some(query_meta) = meta_guard.get_mut(&task_cache_key) {
                            query_meta.state = QueryState::Success;
                            query_meta.fetched_at = Some(now);
                            query_meta.stale_at = Some(now + config.stale_time);
                            query_meta.last_error = None;
                        }

                        break;
                    }
                    Err(e) => {
                        last_error = Some(e);
                    }
                }
            }

            // If all attempts failed, update error state
            if let Some(error) = last_error {
                let mut meta_guard = meta.write().await;
                if let Some(query_meta) = meta_guard.get_mut(&task_cache_key) {
                    query_meta.state = QueryState::Error;
                    query_meta.last_error = Some(error.to_string());
                }
            }

            // Remove task from background_tasks map
            let mut tasks = background_tasks.lock().await;
            tasks.remove(&task_cache_key);
        });

        // Store the task handle
        let mut tasks = self.background_tasks.lock().await;
        tasks.insert(cache_key, handle);
    }

    /// Fetch query data (always fetches, ignoring cache)
    pub async fn fetch<Q: Query>(&self, query: &Q) -> Result<Q::Data> {
        let key = query.key();
        let cache_key = key.to_cache_key();
        let config = query.config();

        // Update state to fetching
        {
            let mut meta = self.meta.write().await;
            let query_meta = meta.entry(cache_key.clone()).or_insert_with(QueryMeta::new);
            query_meta.state = QueryState::Fetching;
            query_meta.fetch_count += 1;
        }

        // Fetch with retry logic
        let mut last_error = None;
        let max_attempts = if config.retry { config.retry_count } else { 1 };

        for attempt in 0..max_attempts {
            if attempt > 0 {
                tokio::time::sleep(config.retry_delay).await;
            }

            match query.fetch().await {
                Ok(data) => {
                    // Store in cache
                    let serialized = serde_json::to_string(&data)?;
                    let ttl = Some(config.cache_time);
                    self.cache.put(cache_key.clone(), serialized, ttl)?;

                    // Update metadata
                    let now = SystemTime::now();
                    let mut meta = self.meta.write().await;
                    if let Some(query_meta) = meta.get_mut(&cache_key) {
                        query_meta.state = QueryState::Success;
                        query_meta.fetched_at = Some(now);
                        query_meta.stale_at = Some(now + config.stale_time);
                        query_meta.last_error = None;
                    }

                    return Ok(data);
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        // All retries failed
        let error = last_error.unwrap();
        let error_msg = error.to_string();

        {
            let mut meta = self.meta.write().await;
            if let Some(query_meta) = meta.get_mut(&cache_key) {
                query_meta.state = QueryState::Error;
                query_meta.last_error = Some(error_msg.clone());
            }
        }

        Err(error)
    }

    /// Prefetch query data in the background
    pub async fn prefetch<Q: Query + 'static>(&self, query: Q) {
        let client = self.clone();
        tokio::spawn(async move {
            let _ = client.fetch(&query).await;
        });
    }

    /// Invalidate cached query data
    pub async fn invalidate(&self, key: &QueryKey) -> Result<()> {
        let cache_key = key.to_cache_key();

        // Remove from cache
        self.cache.remove(&cache_key)?;

        // Remove metadata
        let mut meta = self.meta.write().await;
        meta.remove(&cache_key);

        Ok(())
    }

    /// Invalidate all queries matching a scope
    pub async fn invalidate_scope(&self, scope: &str) -> Result<()> {
        let mut meta = self.meta.write().await;
        let keys_to_remove: Vec<String> = meta
            .keys()
            .filter(|k| k.starts_with(&format!("query:{}:", scope)))
            .cloned()
            .collect();

        for cache_key in keys_to_remove {
            self.cache.remove(&cache_key)?;
            meta.remove(&cache_key);
        }

        Ok(())
    }

    /// Get query state
    pub async fn state(&self, key: &QueryKey) -> QueryState {
        let cache_key = key.to_cache_key();
        let meta = self.meta.read().await;
        meta.get(&cache_key)
            .map(|m| m.state)
            .unwrap_or(QueryState::Idle)
    }

    /// Clear all cached queries
    pub async fn clear(&self) -> Result<()> {
        self.cache.clear()?;
        let mut meta = self.meta.write().await;
        meta.clear();
        Ok(())
    }
}

impl Clone for QueryClient {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
            meta: Arc::clone(&self.meta),
            background_tasks: Arc::clone(&self.background_tasks),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
    struct TestData {
        value: String,
    }

    #[derive(Clone)]
    struct TestQuery {
        key: QueryKey,
        data: TestData,
        should_fail: bool,
    }

    #[async_trait]
    impl Query for TestQuery {
        type Data = TestData;

        async fn fetch(&self) -> Result<Self::Data> {
            if self.should_fail {
                Err(QueryError::FetchError("simulated failure".to_string()))
            } else {
                Ok(self.data.clone())
            }
        }

        fn key(&self) -> QueryKey {
            self.key.clone()
        }
    }

    #[tokio::test]
    async fn test_query_key_creation() {
        let key = QueryKey::new("posts", "123")
            .with_param("sort", "recent")
            .with_param("limit", "20");

        assert_eq!(key.scope, "posts");
        assert_eq!(key.id, "123");
        assert_eq!(key.params.get("sort"), Some(&"recent".to_string()));
        assert_eq!(key.params.get("limit"), Some(&"20".to_string()));
    }

    #[tokio::test]
    async fn test_query_key_to_cache_key() {
        let key = QueryKey::new("posts", "123");
        let cache_key = key.to_cache_key();

        assert!(cache_key.starts_with("query:posts:123:"));
    }

    #[tokio::test]
    async fn test_query_client_fetch() {
        let client = QueryClient::new(CacheConfig::default()).unwrap();
        let query = TestQuery {
            key: QueryKey::new("test", "1"),
            data: TestData { value: "test value".to_string() },
            should_fail: false,
        };

        let result = client.fetch(&query).await.unwrap();
        assert_eq!(result.value, "test value");
    }

    #[tokio::test]
    async fn test_query_client_cache() {
        let client = QueryClient::new(CacheConfig::default()).unwrap();
        let query = TestQuery {
            key: QueryKey::new("test", "2"),
            data: TestData { value: "cached value".to_string() },
            should_fail: false,
        };

        // First fetch
        let result1 = client.get(&query).await.unwrap();
        assert_eq!(result1.value, "cached value");

        // Second fetch should use cache
        let result2 = client.get(&query).await.unwrap();
        assert_eq!(result2.value, "cached value");
    }

    #[tokio::test]
    async fn test_query_invalidation() {
        let client = QueryClient::new(CacheConfig::default()).unwrap();
        let key = QueryKey::new("test", "3");
        let query = TestQuery {
            key: key.clone(),
            data: TestData { value: "invalidate test".to_string() },
            should_fail: false,
        };

        // Fetch and cache
        client.fetch(&query).await.unwrap();

        // Invalidate
        client.invalidate(&key).await.unwrap();

        // State should be idle after invalidation
        assert_eq!(client.state(&key).await, QueryState::Idle);
    }

    #[tokio::test]
    async fn test_query_scope_invalidation() {
        let client = QueryClient::new(CacheConfig::default()).unwrap();

        let query1 = TestQuery {
            key: QueryKey::new("posts", "1"),
            data: TestData { value: "post 1".to_string() },
            should_fail: false,
        };

        let query2 = TestQuery {
            key: QueryKey::new("posts", "2"),
            data: TestData { value: "post 2".to_string() },
            should_fail: false,
        };

        // Fetch both
        client.fetch(&query1).await.unwrap();
        client.fetch(&query2).await.unwrap();

        // Invalidate all posts
        client.invalidate_scope("posts").await.unwrap();

        // Both should be idle
        assert_eq!(client.state(&query1.key()).await, QueryState::Idle);
        assert_eq!(client.state(&query2.key()).await, QueryState::Idle);
    }

    #[tokio::test]
    async fn test_query_retry() {
        let mut config = CacheConfig::default();
        config.max_entries = 1000;

        let client = QueryClient::new(config).unwrap();
        let query = TestQuery {
            key: QueryKey::new("test", "fail"),
            data: TestData { value: "will fail".to_string() },
            should_fail: true,
        };

        let result = client.fetch(&query).await;
        assert!(result.is_err());
        assert_eq!(client.state(&query.key()).await, QueryState::Error);
    }

    #[tokio::test]
    async fn test_query_clear() {
        let client = QueryClient::new(CacheConfig::default()).unwrap();
        let query = TestQuery {
            key: QueryKey::new("test", "clear"),
            data: TestData { value: "to be cleared".to_string() },
            should_fail: false,
        };

        client.fetch(&query).await.unwrap();
        client.clear().await.unwrap();

        assert_eq!(client.state(&query.key()).await, QueryState::Idle);
    }
}
