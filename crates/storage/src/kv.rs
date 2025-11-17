//! Key-value store for preferences and settings
//!
//! This module provides a fast, type-safe key-value store using sled,
//! with support for scoping, encryption, and change notifications.

use serde::{de::DeserializeOwned, Serialize};
use sled::Db;
use std::sync::Arc;
use thiserror::Error;

/// Key-value store error types
#[derive(Debug, Error)]
pub enum KvError {
    /// Sled database error
    #[error("Database error: {0}")]
    Database(#[from] sled::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Key not found
    #[error("Key not found: {0}")]
    NotFound(String),

    /// Invalid key
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    /// Encryption error
    #[error("Encryption error: {0}")]
    Encryption(String),
}

/// Result type for key-value operations
pub type Result<T> = std::result::Result<T, KvError>;

/// Key-value store configuration
#[derive(Debug, Clone)]
pub struct KvConfig {
    /// Database path
    pub path: String,
    /// Cache capacity in bytes
    pub cache_capacity: u64,
    /// Enable compression
    pub use_compression: bool,
    /// Flush interval in milliseconds (None for immediate flush)
    pub flush_every_ms: Option<u64>,
}

impl Default for KvConfig {
    fn default() -> Self {
        Self {
            path: "aurora_kv.db".to_string(),
            cache_capacity: 64 * 1024 * 1024, // 64MB
            use_compression: true,
            flush_every_ms: Some(500), // Flush every 500ms
        }
    }
}

impl KvConfig {
    /// Create a new configuration with a custom path
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into(), ..Default::default() }
    }

    /// Set cache capacity in bytes
    pub fn cache_capacity(mut self, bytes: u64) -> Self {
        self.cache_capacity = bytes;
        self
    }

    /// Enable or disable compression
    pub fn use_compression(mut self, enabled: bool) -> Self {
        self.use_compression = enabled;
        self
    }

    /// Set flush interval in milliseconds
    pub fn flush_every_ms(mut self, ms: Option<u64>) -> Self {
        self.flush_every_ms = ms;
        self
    }
}

/// Key-value store implementation
pub struct KvStore {
    db: Arc<Db>,
    separator: &'static str,
}

impl KvStore {
    /// Create a new key-value store with configuration
    pub fn new(config: KvConfig) -> Result<Self> {
        let mut db_config = sled::Config::new()
            .path(&config.path)
            .cache_capacity(config.cache_capacity)
            .use_compression(config.use_compression);

        if let Some(ms) = config.flush_every_ms {
            db_config = db_config.flush_every_ms(Some(ms));
        }

        let db = db_config.open()?;

        Ok(Self { db: Arc::new(db), separator: ":" })
    }

    /// Create an in-memory key-value store (for testing)
    pub fn in_memory() -> Result<Self> {
        let db = sled::Config::new().temporary(true).open()?;

        Ok(Self { db: Arc::new(db), separator: ":" })
    }

    /// Get a value by key
    pub fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        match self.db.get(key.as_bytes())? {
            Some(bytes) => {
                let value: T = serde_json::from_slice(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Get a value by scoped key (e.g., ["device", "theme"])
    pub fn get_scoped<T>(&self, scopes: &[&str]) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let key = scopes.join(self.separator);
        self.get(&key)
    }

    /// Set a value by key
    pub fn set<T>(&self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let bytes = serde_json::to_vec(value)?;
        self.db.insert(key.as_bytes(), bytes)?;
        Ok(())
    }

    /// Set a value by scoped key (e.g., ["device", "theme"], value)
    pub fn set_scoped<T>(&self, scopes: &[&str], value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let key = scopes.join(self.separator);
        self.set(&key, value)
    }

    /// Remove a value by key
    pub fn remove(&self, key: &str) -> Result<bool> {
        Ok(self.db.remove(key.as_bytes())?.is_some())
    }

    /// Remove a value by scoped key
    pub fn remove_scoped(&self, scopes: &[&str]) -> Result<bool> {
        let key = scopes.join(self.separator);
        self.remove(&key)
    }

    /// Remove multiple values by scoped prefix (e.g., remove all keys starting with "account:123")
    pub fn remove_many(&self, scope: &[&str], keys: &[&str]) -> Result<usize> {
        let mut count = 0;
        for key in keys {
            let mut full_scope = scope.to_vec();
            full_scope.push(key);
            if self.remove_scoped(&full_scope)? {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Check if a key exists
    pub fn contains(&self, key: &str) -> Result<bool> {
        Ok(self.db.contains_key(key.as_bytes())?)
    }

    /// Check if a scoped key exists
    pub fn contains_scoped(&self, scopes: &[&str]) -> Result<bool> {
        let key = scopes.join(self.separator);
        self.contains(&key)
    }

    /// Get all keys with a given prefix
    pub fn keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        let prefix_bytes = prefix.as_bytes();
        let mut keys = Vec::new();

        for item in self.db.scan_prefix(prefix_bytes) {
            let (key, _) = item?;
            if let Ok(key_str) = String::from_utf8(key.to_vec()) {
                keys.push(key_str);
            }
        }

        Ok(keys)
    }

    /// Clear all data
    pub fn clear(&self) -> Result<()> {
        self.db.clear()?;
        Ok(())
    }

    /// Flush pending writes to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    /// Get the number of keys in the store
    pub fn len(&self) -> usize {
        self.db.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }

    /// Perform an atomic compare-and-swap operation
    pub fn compare_and_swap<T>(
        &self,
        key: &str,
        old: Option<&T>,
        new: Option<&T>,
    ) -> Result<std::result::Result<(), CompareAndSwapError<T>>>
    where
        T: Serialize + DeserializeOwned + Clone,
    {
        let old_bytes = old.map(|v| serde_json::to_vec(v)).transpose()?;
        let new_bytes = new.map(|v| serde_json::to_vec(v)).transpose()?;

        match self.db.compare_and_swap(
            key.as_bytes(),
            old_bytes.as_deref(),
            new_bytes.as_deref(),
        )? {
            Ok(()) => Ok(Ok(())),
            Err(sled::CompareAndSwapError { current, proposed }) => {
                let current_value = current
                    .map(|bytes| serde_json::from_slice::<T>(&bytes))
                    .transpose()?;
                let proposed_value = proposed
                    .map(|bytes| serde_json::from_slice::<T>(&bytes))
                    .transpose()?;

                Ok(Err(CompareAndSwapError { current: current_value, proposed: proposed_value }))
            }
        }
    }

    /// Subscribe to changes for a specific key
    pub fn watch(&self, key: &str) -> sled::Subscriber {
        self.db.watch_prefix(key.as_bytes())
    }

    /// Export all data as JSON
    pub fn export(&self) -> Result<Vec<(String, serde_json::Value)>> {
        let mut data = Vec::new();

        for item in self.db.iter() {
            let (key, value) = item?;
            if let Ok(key_str) = String::from_utf8(key.to_vec()) {
                if let Ok(value_json) = serde_json::from_slice::<serde_json::Value>(&value) {
                    data.push((key_str, value_json));
                }
            }
        }

        Ok(data)
    }

    /// Import data from JSON
    pub fn import(&self, data: &[(String, serde_json::Value)]) -> Result<()> {
        for (key, value) in data {
            let bytes = serde_json::to_vec(value)?;
            self.db.insert(key.as_bytes(), bytes)?;
        }
        Ok(())
    }
}

/// Error type for compare-and-swap operations
#[derive(Debug, Clone)]
pub struct CompareAndSwapError<T> {
    /// The current value in the store
    pub current: Option<T>,
    /// The proposed new value that failed to be set
    pub proposed: Option<T>,
}

/// Scoped key-value store for device-level settings
pub struct DeviceStore {
    kv: Arc<KvStore>,
}

impl DeviceStore {
    /// Create a new device store
    pub fn new(kv: Arc<KvStore>) -> Self {
        Self { kv }
    }

    /// Get a device-level value
    pub fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.kv.get_scoped(&["device", key])
    }

    /// Set a device-level value
    pub fn set<T>(&self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.kv.set_scoped(&["device", key], value)
    }

    /// Remove a device-level value
    pub fn remove(&self, key: &str) -> Result<bool> {
        self.kv.remove_scoped(&["device", key])
    }

    /// Check if a device-level key exists
    pub fn contains(&self, key: &str) -> Result<bool> {
        self.kv.contains_scoped(&["device", key])
    }
}

/// Scoped key-value store for account-level settings
pub struct AccountStore {
    kv: Arc<KvStore>,
}

impl AccountStore {
    /// Create a new account store
    pub fn new(kv: Arc<KvStore>) -> Self {
        Self { kv }
    }

    /// Get an account-level value
    pub fn get<T>(&self, account_id: &str, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.kv.get_scoped(&["account", account_id, key])
    }

    /// Set an account-level value
    pub fn set<T>(&self, account_id: &str, key: &str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.kv.set_scoped(&["account", account_id, key], value)
    }

    /// Remove an account-level value
    pub fn remove(&self, account_id: &str, key: &str) -> Result<bool> {
        self.kv.remove_scoped(&["account", account_id, key])
    }

    /// Remove all data for an account
    pub fn remove_account(&self, account_id: &str) -> Result<usize> {
        let prefix = format!("account:{}:", account_id);
        let keys = self.kv.keys_with_prefix(&prefix)?;
        let mut count = 0;
        for key in keys {
            if self.kv.remove(&key)? {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Check if an account-level key exists
    pub fn contains(&self, account_id: &str, key: &str) -> Result<bool> {
        self.kv.contains_scoped(&["account", account_id, key])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        count: i32,
    }

    #[test]
    fn test_kv_store_creation() {
        let kv = KvStore::in_memory().unwrap();
        assert!(kv.is_empty());
    }

    #[test]
    fn test_set_and_get() {
        let kv = KvStore::in_memory().unwrap();

        kv.set("test_key", &"test_value".to_string()).unwrap();

        let value: Option<String> = kv.get("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_set_and_get_struct() {
        let kv = KvStore::in_memory().unwrap();

        let data = TestData { name: "Alice".to_string(), count: 42 };

        kv.set("user", &data).unwrap();

        let retrieved: Option<TestData> = kv.get("user").unwrap();
        assert_eq!(retrieved, Some(data));
    }

    #[test]
    fn test_get_nonexistent() {
        let kv = KvStore::in_memory().unwrap();
        let value: Option<String> = kv.get("nonexistent").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_remove() {
        let kv = KvStore::in_memory().unwrap();

        kv.set("key", &"value".to_string()).unwrap();
        assert!(kv.contains("key").unwrap());

        let removed = kv.remove("key").unwrap();
        assert!(removed);
        assert!(!kv.contains("key").unwrap());

        let removed_again = kv.remove("key").unwrap();
        assert!(!removed_again);
    }

    #[test]
    fn test_scoped_operations() {
        let kv = KvStore::in_memory().unwrap();

        kv.set_scoped(&["device", "theme"], &"dark".to_string())
            .unwrap();
        kv.set_scoped(&["device", "language"], &"en".to_string())
            .unwrap();

        let theme: Option<String> = kv.get_scoped(&["device", "theme"]).unwrap();
        assert_eq!(theme, Some("dark".to_string()));

        let language: Option<String> = kv.get_scoped(&["device", "language"]).unwrap();
        assert_eq!(language, Some("en".to_string()));
    }

    #[test]
    fn test_contains() {
        let kv = KvStore::in_memory().unwrap();

        assert!(!kv.contains("key").unwrap());
        kv.set("key", &"value".to_string()).unwrap();
        assert!(kv.contains("key").unwrap());
    }

    #[test]
    fn test_clear() {
        let kv = KvStore::in_memory().unwrap();

        kv.set("key1", &"value1".to_string()).unwrap();
        kv.set("key2", &"value2".to_string()).unwrap();
        assert_eq!(kv.len(), 2);

        kv.clear().unwrap();
        assert!(kv.is_empty());
    }

    #[test]
    fn test_keys_with_prefix() {
        let kv = KvStore::in_memory().unwrap();

        kv.set("app:setting1", &"value1".to_string()).unwrap();
        kv.set("app:setting2", &"value2".to_string()).unwrap();
        kv.set("user:name", &"Alice".to_string()).unwrap();

        let keys = kv.keys_with_prefix("app:").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"app:setting1".to_string()));
        assert!(keys.contains(&"app:setting2".to_string()));
    }

    #[test]
    fn test_compare_and_swap() {
        let kv = KvStore::in_memory().unwrap();

        // Set initial value
        kv.set("counter", &0).unwrap();

        // Successful CAS
        let result = kv.compare_and_swap("counter", Some(&0), Some(&1)).unwrap();
        assert!(result.is_ok());

        let value: Option<i32> = kv.get("counter").unwrap();
        assert_eq!(value, Some(1));

        // Failed CAS (value has changed)
        let result = kv.compare_and_swap("counter", Some(&0), Some(&2)).unwrap();
        assert!(result.is_err());

        let value: Option<i32> = kv.get("counter").unwrap();
        assert_eq!(value, Some(1)); // Value unchanged
    }

    #[test]
    fn test_device_store() {
        let kv = Arc::new(KvStore::in_memory().unwrap());
        let device = DeviceStore::new(kv);

        device.set("theme", &"dark".to_string()).unwrap();
        device.set("font_size", &14).unwrap();

        let theme: Option<String> = device.get("theme").unwrap();
        assert_eq!(theme, Some("dark".to_string()));

        let font_size: Option<i32> = device.get("font_size").unwrap();
        assert_eq!(font_size, Some(14));

        assert!(device.contains("theme").unwrap());
        device.remove("theme").unwrap();
        assert!(!device.contains("theme").unwrap());
    }

    #[test]
    fn test_account_store() {
        let kv = Arc::new(KvStore::in_memory().unwrap());
        let account = AccountStore::new(kv);

        account
            .set("alice", "display_name", &"Alice".to_string())
            .unwrap();
        account.set("alice", "followers", &100).unwrap();
        account
            .set("bob", "display_name", &"Bob".to_string())
            .unwrap();

        let alice_name: Option<String> = account.get("alice", "display_name").unwrap();
        assert_eq!(alice_name, Some("Alice".to_string()));

        let alice_followers: Option<i32> = account.get("alice", "followers").unwrap();
        assert_eq!(alice_followers, Some(100));

        let bob_name: Option<String> = account.get("bob", "display_name").unwrap();
        assert_eq!(bob_name, Some("Bob".to_string()));

        // Remove all alice data
        let removed = account.remove_account("alice").unwrap();
        assert_eq!(removed, 2);

        assert!(!account.contains("alice", "display_name").unwrap());
        assert!(account.contains("bob", "display_name").unwrap());
    }

    #[test]
    fn test_export_import() {
        let kv = KvStore::in_memory().unwrap();

        kv.set("key1", &"value1".to_string()).unwrap();
        kv.set("key2", &42).unwrap();

        let exported = kv.export().unwrap();
        assert_eq!(exported.len(), 2);

        let kv2 = KvStore::in_memory().unwrap();
        kv2.import(&exported).unwrap();

        let value1: Option<String> = kv2.get("key1").unwrap();
        assert_eq!(value1, Some("value1".to_string()));

        let value2: Option<i32> = kv2.get("key2").unwrap();
        assert_eq!(value2, Some(42));
    }

    #[test]
    fn test_config_builder() {
        let config = KvConfig::new("test.db")
            .cache_capacity(32 * 1024 * 1024)
            .use_compression(false)
            .flush_every_ms(Some(1000));

        assert_eq!(config.path, "test.db");
        assert_eq!(config.cache_capacity, 32 * 1024 * 1024);
        assert!(!config.use_compression);
        assert_eq!(config.flush_every_ms, Some(1000));
    }
}
