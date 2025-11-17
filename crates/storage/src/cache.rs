//! Cache management system
//!
//! This module provides caching with LRU eviction, TTL support, and tiered storage.

use lru::LruCache;
use serde::{de::DeserializeOwned, Serialize};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use thiserror::Error;

use crate::kv::{KvError, KvStore};

/// Cache error types
#[derive(Debug, Error)]
pub enum CacheError {
    /// Key not found
    #[error("Key not found: {0}")]
    NotFound(String),

    /// Entry expired
    #[error("Entry expired: {0}")]
    Expired(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// KV store error
    #[error("KV store error: {0}")]
    Kv(#[from] KvError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for cache operations
pub type Result<T> = std::result::Result<T, CacheError>;

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
struct CacheEntry<V> {
    /// The cached value
    value: V,
    /// When the entry was created
    created_at: SystemTime,
    /// When the entry expires (None = never)
    expires_at: Option<SystemTime>,
    /// Size in bytes (approximate)
    size: usize,
}

impl<V> CacheEntry<V> {
    fn new(value: V, ttl: Option<Duration>, size: usize) -> Self {
        let created_at = SystemTime::now();
        let expires_at = ttl.map(|d| created_at + d);

        Self { value, created_at, expires_at, size }
    }

    fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in memory cache
    pub max_entries: usize,
    /// Maximum size in bytes
    pub max_size_bytes: usize,
    /// Default TTL for entries
    pub default_ttl: Option<Duration>,
    /// Enable disk cache
    pub enable_disk_cache: bool,
    /// Disk cache path
    pub disk_cache_path: Option<PathBuf>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_size_bytes: 100 * 1024 * 1024,            // 100MB
            default_ttl: Some(Duration::from_secs(3600)), // 1 hour
            enable_disk_cache: false,
            disk_cache_path: None,
        }
    }
}

impl CacheConfig {
    /// Create a new cache configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum entries
    pub fn max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Set maximum size in bytes
    pub fn max_size_bytes(mut self, bytes: usize) -> Self {
        self.max_size_bytes = bytes;
        self
    }

    /// Set default TTL
    pub fn default_ttl(mut self, ttl: Option<Duration>) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Enable disk cache with path
    pub fn with_disk_cache(mut self, path: impl Into<PathBuf>) -> Self {
        self.enable_disk_cache = true;
        self.disk_cache_path = Some(path.into());
        self
    }
}

/// In-memory LRU cache
pub struct MemoryCache<V> {
    cache: Arc<Mutex<LruCache<String, CacheEntry<V>>>>,
    current_size: Arc<Mutex<usize>>,
    config: CacheConfig,
}

impl<V: Clone> MemoryCache<V> {
    /// Create a new memory cache
    pub fn new(config: CacheConfig) -> Self {
        let capacity =
            NonZeroUsize::new(config.max_entries).unwrap_or(NonZeroUsize::new(1).unwrap());

        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            current_size: Arc::new(Mutex::new(0)),
            config,
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &str) -> Result<Option<V>> {
        let mut cache = self.cache.lock().unwrap();

        // Check if entry exists and is expired
        let is_expired = cache.peek(key).map(|e| e.is_expired()).unwrap_or(false);

        if is_expired {
            // Remove expired entry
            if let Some(entry) = cache.pop(key) {
                let mut size = self.current_size.lock().unwrap();
                *size = size.saturating_sub(entry.size);
            }
            return Err(CacheError::Expired(key.to_string()));
        }

        // Get value if it exists
        if let Some(entry) = cache.get(key) {
            return Ok(Some(entry.value.clone()));
        }

        Ok(None)
    }

    /// Put a value in the cache
    pub fn put(&self, key: impl Into<String>, value: V, ttl: Option<Duration>) -> Result<()> {
        let key = key.into();
        let size = std::mem::size_of_val(&value);
        let entry = CacheEntry::new(value, ttl.or(self.config.default_ttl), size);

        let mut cache = self.cache.lock().unwrap();
        let mut current_size = self.current_size.lock().unwrap();

        // Evict entries if we exceed size limit
        while *current_size + size > self.config.max_size_bytes && !cache.is_empty() {
            if let Some((_, evicted)) = cache.pop_lru() {
                *current_size = current_size.saturating_sub(evicted.size);
            }
        }

        // Update size if replacing existing entry
        if let Some(old_entry) = cache.put(key, entry) {
            *current_size = current_size.saturating_sub(old_entry.size);
        }

        *current_size += size;

        Ok(())
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &str) -> Result<bool> {
        let mut cache = self.cache.lock().unwrap();

        if let Some(entry) = cache.pop(key) {
            let mut size = self.current_size.lock().unwrap();
            *size = size.saturating_sub(entry.size);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Clear all entries
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();

        let mut size = self.current_size.lock().unwrap();
        *size = 0;
    }

    /// Check if key exists and is not expired
    pub fn contains(&self, key: &str) -> bool {
        let mut cache = self.cache.lock().unwrap();

        if let Some(entry) = cache.peek(key) {
            if entry.is_expired() {
                cache.pop(key);
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.cache.lock().unwrap().len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.lock().unwrap().is_empty()
    }

    /// Get current size in bytes
    pub fn size_bytes(&self) -> usize {
        *self.current_size.lock().unwrap()
    }

    /// Remove expired entries
    pub fn evict_expired(&self) -> usize {
        let mut cache = self.cache.lock().unwrap();
        let mut size = self.current_size.lock().unwrap();
        let mut count = 0;

        // Collect expired keys
        let expired_keys: Vec<String> = cache
            .iter()
            .filter_map(|(k, v)| {
                if v.is_expired() {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();

        // Remove expired entries
        for key in expired_keys {
            if let Some(entry) = cache.pop(&key) {
                *size = size.saturating_sub(entry.size);
                count += 1;
            }
        }

        count
    }
}

/// Disk cache using key-value store
pub struct DiskCache<V> {
    store: Arc<KvStore>,
    config: CacheConfig,
    _phantom: std::marker::PhantomData<V>,
}

impl<V> DiskCache<V> {
    /// Create a new disk cache
    pub fn new(config: CacheConfig) -> Result<Self> {
        let path = config
            .disk_cache_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("cache.db"));

        let kv_config = crate::kv::KvConfig::new(path.to_string_lossy().to_string())
            .cache_capacity(64 * 1024 * 1024) // 64MB cache
            .use_compression(true);

        let store = KvStore::new(kv_config)?;

        Ok(Self {
            store: Arc::new(store),
            config,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Remove a value from disk cache
    pub fn remove(&self, key: &str) -> Result<bool> {
        Ok(self.store.remove(key)?)
    }

    /// Clear all entries
    pub fn clear(&self) -> Result<()> {
        self.store.clear()?;
        Ok(())
    }

    /// Check if key exists
    pub fn contains(&self, key: &str) -> Result<bool> {
        Ok(self.store.contains(key)?)
    }
}

impl<V: Serialize + DeserializeOwned> DiskCache<V> {
    /// Get a value from disk cache
    pub fn get(&self, key: &str) -> Result<Option<V>> {
        let entry: Option<CacheEntry<V>> = self.store.get(key)?;

        if let Some(entry) = entry {
            if entry.is_expired() {
                self.store.remove(key)?;
                return Err(CacheError::Expired(key.to_string()));
            }
            return Ok(Some(entry.value));
        }

        Ok(None)
    }

    /// Put a value in disk cache
    pub fn put(&self, key: impl Into<String>, value: V, ttl: Option<Duration>) -> Result<()> {
        let key = key.into();
        let size = 0; // Size calculation for disk can be more complex
        let entry = CacheEntry::new(value, ttl.or(self.config.default_ttl), size);

        self.store.set(&key, &entry)?;
        Ok(())
    }
}

/// Tiered cache with memory and disk layers
pub struct TieredCache<V> {
    memory: MemoryCache<V>,
    disk: Option<DiskCache<V>>,
}

impl<V: Clone + Serialize + DeserializeOwned> TieredCache<V> {
    /// Create a new tiered cache
    pub fn new(config: CacheConfig) -> Result<Self> {
        let memory = MemoryCache::new(config.clone());

        let disk = if config.enable_disk_cache {
            Some(DiskCache::new(config)?)
        } else {
            None
        };

        Ok(Self { memory, disk })
    }

    /// Get a value from cache (checks memory first, then disk)
    pub fn get(&self, key: &str) -> Result<Option<V>> {
        // Try memory first
        if let Some(value) = self.memory.get(key)? {
            return Ok(Some(value));
        }

        // Try disk if available
        if let Some(disk) = &self.disk {
            if let Some(value) = disk.get(key)? {
                // Promote to memory cache
                self.memory.put(key, value.clone(), None)?;
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    /// Put a value in cache (writes to both memory and disk)
    pub fn put(&self, key: impl Into<String>, value: V, ttl: Option<Duration>) -> Result<()> {
        let key = key.into();

        // Write to memory
        self.memory.put(&key, value.clone(), ttl)?;

        // Write to disk if available
        if let Some(disk) = &self.disk {
            disk.put(&key, value, ttl)?;
        }

        Ok(())
    }

    /// Remove from all cache layers
    pub fn remove(&self, key: &str) -> Result<bool> {
        let mem_removed = self.memory.remove(key)?;

        let disk_removed = if let Some(disk) = &self.disk {
            disk.remove(key)?
        } else {
            false
        };

        Ok(mem_removed || disk_removed)
    }

    /// Clear all cache layers
    pub fn clear(&self) -> Result<()> {
        self.memory.clear();

        if let Some(disk) = &self.disk {
            disk.clear()?;
        }

        Ok(())
    }

    /// Invalidate entries matching a predicate
    pub fn invalidate<F>(&self, _predicate: F) -> Result<usize>
    where
        F: Fn(&str) -> bool,
    {
        // For now, this is a placeholder
        // Full implementation would iterate through keys
        Ok(0)
    }

    /// Evict expired entries from memory
    pub fn evict_expired(&self) -> usize {
        self.memory.evict_expired()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_cache_basic() {
        let config = CacheConfig::new().max_entries(10);
        let cache: MemoryCache<String> = MemoryCache::new(config);

        cache.put("key1", "value1".to_string(), None).unwrap();

        let value = cache.get("key1").unwrap();
        assert_eq!(value, Some("value1".to_string()));
    }

    #[test]
    fn test_memory_cache_ttl() {
        let config = CacheConfig::new().default_ttl(Some(Duration::from_millis(100)));
        let cache: MemoryCache<i32> = MemoryCache::new(config);

        cache.put("key1", 42, None).unwrap();

        // Should be available immediately
        assert_eq!(cache.get("key1").unwrap(), Some(42));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Should be expired
        let result = cache.get("key1");
        assert!(matches!(result, Err(CacheError::Expired(_))));
    }

    #[test]
    fn test_memory_cache_lru_eviction() {
        let config = CacheConfig::new().max_entries(3);
        let cache: MemoryCache<i32> = MemoryCache::new(config);

        cache.put("key1", 1, None).unwrap();
        cache.put("key2", 2, None).unwrap();
        cache.put("key3", 3, None).unwrap();

        // Cache is full
        assert_eq!(cache.len(), 3);

        // Add one more - should evict LRU (key1)
        cache.put("key4", 4, None).unwrap();

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get("key1").unwrap(), None); // Evicted
        assert_eq!(cache.get("key2").unwrap(), Some(2));
        assert_eq!(cache.get("key3").unwrap(), Some(3));
        assert_eq!(cache.get("key4").unwrap(), Some(4));
    }

    #[test]
    fn test_memory_cache_remove() {
        let config = CacheConfig::new();
        let cache: MemoryCache<String> = MemoryCache::new(config);

        cache.put("key1", "value1".to_string(), None).unwrap();
        assert!(cache.contains("key1"));

        let removed = cache.remove("key1").unwrap();
        assert!(removed);
        assert!(!cache.contains("key1"));

        let removed_again = cache.remove("key1").unwrap();
        assert!(!removed_again);
    }

    #[test]
    fn test_memory_cache_clear() {
        let config = CacheConfig::new();
        let cache: MemoryCache<i32> = MemoryCache::new(config);

        cache.put("key1", 1, None).unwrap();
        cache.put("key2", 2, None).unwrap();
        cache.put("key3", 3, None).unwrap();

        assert_eq!(cache.len(), 3);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_memory_cache_size_limit() {
        let config = CacheConfig::new().max_entries(100).max_size_bytes(1000);

        let cache: MemoryCache<String> = MemoryCache::new(config);

        // Add entries - should respect size limit
        for i in 0..50 {
            cache
                .put(format!("key{}", i), "x".repeat(100), None)
                .unwrap();
        }

        // Size should be under limit due to eviction
        assert!(cache.size_bytes() <= 1000);
    }

    #[test]
    fn test_disk_cache() {
        let config = CacheConfig::new().with_disk_cache("test_disk_cache.db");

        let cache = DiskCache::<String>::new(config).unwrap();

        cache.put("key1", "value1".to_string(), None).unwrap();

        let value = cache.get("key1").unwrap();
        assert_eq!(value, Some("value1".to_string()));

        cache.clear().unwrap();
    }

    #[test]
    fn test_tiered_cache() {
        let config = CacheConfig::new()
            .max_entries(5)
            .with_disk_cache("test_tiered_cache.db");

        let cache: TieredCache<i32> = TieredCache::new(config).unwrap();

        cache.put("key1", 42, None).unwrap();

        // Should be in memory
        assert_eq!(cache.get("key1").unwrap(), Some(42));

        // Clear memory only
        cache.memory.clear();

        // Should still be available from disk and promoted to memory
        assert_eq!(cache.get("key1").unwrap(), Some(42));

        cache.clear().unwrap();
    }

    #[test]
    fn test_tiered_cache_eviction() {
        let config = CacheConfig::new()
            .max_entries(2)
            .with_disk_cache("test_tiered_eviction.db");

        let cache: TieredCache<i32> = TieredCache::new(config).unwrap();

        cache.put("key1", 1, None).unwrap();
        cache.put("key2", 2, None).unwrap();
        cache.put("key3", 3, None).unwrap(); // Evicts key1 from memory

        // key1 should still be on disk
        assert_eq!(cache.get("key1").unwrap(), Some(1)); // Promoted from disk

        cache.clear().unwrap();
    }

    #[test]
    fn test_config_builder() {
        let config = CacheConfig::new()
            .max_entries(500)
            .max_size_bytes(50 * 1024 * 1024)
            .default_ttl(Some(Duration::from_secs(1800)))
            .with_disk_cache("/tmp/cache");

        assert_eq!(config.max_entries, 500);
        assert_eq!(config.max_size_bytes, 50 * 1024 * 1024);
        assert_eq!(config.default_ttl, Some(Duration::from_secs(1800)));
        assert!(config.enable_disk_cache);
    }
}
