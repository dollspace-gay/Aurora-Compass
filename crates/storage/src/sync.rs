//! State synchronization system
//!
//! This module provides state synchronization coordination between:
//! - Local persisted state
//! - Cache layers (memory and disk)
//! - Server state (via network)
//!
//! It handles:
//! - Event-based updates
//! - Conflict resolution
//! - Network state management
//! - Cross-instance synchronization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{broadcast, Mutex, RwLock};

/// Errors that can occur during state synchronization
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// Failed to acquire lock
    #[error("Failed to acquire lock: {0}")]
    LockError(String),

    /// Network error during sync
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Conflict detected during sync
    #[error("Conflict detected: {0}")]
    ConflictError(String),

    /// State is out of sync
    #[error("State out of sync: {0}")]
    OutOfSync(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Invalid state version
    #[error("Invalid state version: expected {expected}, got {found}")]
    VersionMismatch {
        /// Expected version
        expected: u64,
        /// Found version
        found: u64,
    },

    /// Timeout waiting for sync
    #[error("Sync timeout: {0}")]
    Timeout(String),
}

/// Result type for sync operations
pub type Result<T> = std::result::Result<T, SyncError>;

/// State update event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateEvent {
    /// A specific key was updated
    KeyUpdated {
        /// The key that was updated
        key: String,
        /// Version number for the update
        version: u64,
        /// Timestamp of the update
        timestamp: u64,
    },

    /// Full state refresh needed
    FullRefresh {
        /// Version number for the state
        version: u64,
    },

    /// Network state changed
    NetworkStateChanged {
        /// Current network state
        state: NetworkState,
    },

    /// Conflict detected
    ConflictDetected {
        /// The conflicting key
        key: String,
        /// Local version
        local_version: u64,
        /// Remote version
        remote_version: u64,
    },
}

/// Network connectivity state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkState {
    /// Connected to network
    Online,

    /// Disconnected from network
    Offline,

    /// Network state unknown
    Unknown,
}

impl fmt::Display for NetworkState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkState::Online => write!(f, "online"),
            NetworkState::Offline => write!(f, "offline"),
            NetworkState::Unknown => write!(f, "unknown"),
        }
    }
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Last write wins (based on timestamp)
    LastWriteWins,

    /// Local changes take precedence
    LocalWins,

    /// Remote changes take precedence
    RemoteWins,

    /// Manual resolution required
    Manual,
}

/// Configuration for state synchronization
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,

    /// Maximum time to wait for sync operations
    pub sync_timeout: Duration,

    /// How often to check for updates when idle
    pub poll_interval: Duration,

    /// Maximum number of pending updates to buffer
    pub buffer_size: usize,

    /// Enable automatic retry on failures
    pub auto_retry: bool,

    /// Maximum number of retry attempts
    pub max_retries: u32,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            conflict_strategy: ConflictStrategy::LastWriteWins,
            sync_timeout: Duration::from_secs(30),
            poll_interval: Duration::from_secs(60),
            buffer_size: 100,
            auto_retry: true,
            max_retries: 3,
        }
    }
}

/// State version information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateVersion {
    /// Version number
    version: u64,

    /// Timestamp of last update
    timestamp: u64,

    /// Checksum of the state
    checksum: String,
}

/// Synchronized state entry
#[derive(Debug, Clone)]
struct SyncedEntry {
    /// Version information
    version: StateVersion,

    /// Serialized value
    value: String,

    /// Whether this entry has pending changes
    dirty: bool,
}

/// State synchronization coordinator
///
/// Coordinates state updates between local storage, cache, and server.
/// Provides event-based notifications and conflict resolution.
pub struct StateSync {
    config: SyncConfig,
    network_state: Arc<RwLock<NetworkState>>,
    state_versions: Arc<RwLock<HashMap<String, StateVersion>>>,
    pending_updates: Arc<Mutex<HashMap<String, SyncedEntry>>>,
    update_tx: broadcast::Sender<UpdateEvent>,
}

impl StateSync {
    /// Create a new state synchronization coordinator
    ///
    /// # Arguments
    ///
    /// * `config` - Synchronization configuration
    ///
    /// # Returns
    ///
    /// A new `StateSync` instance
    pub fn new(config: SyncConfig) -> Self {
        let (update_tx, _update_rx) = broadcast::channel(config.buffer_size);

        Self {
            config,
            network_state: Arc::new(RwLock::new(NetworkState::Unknown)),
            state_versions: Arc::new(RwLock::new(HashMap::new())),
            pending_updates: Arc::new(Mutex::new(HashMap::new())),
            update_tx,
        }
    }

    /// Subscribe to state update events
    ///
    /// # Returns
    ///
    /// A broadcast receiver for update events
    pub fn subscribe(&self) -> broadcast::Receiver<UpdateEvent> {
        self.update_tx.subscribe()
    }

    /// Get the current network state
    ///
    /// # Returns
    ///
    /// Current network connectivity state
    pub async fn network_state(&self) -> NetworkState {
        *self.network_state.read().await
    }

    /// Set the network state
    ///
    /// # Arguments
    ///
    /// * `state` - New network state
    pub async fn set_network_state(&self, state: NetworkState) -> Result<()> {
        let mut current = self.network_state.write().await;
        if *current != state {
            *current = state;
            let _ = self
                .update_tx
                .send(UpdateEvent::NetworkStateChanged { state });
        }
        Ok(())
    }

    /// Update a state value
    ///
    /// # Arguments
    ///
    /// * `key` - State key to update
    /// * `value` - Serialized value
    ///
    /// # Returns
    ///
    /// The version number assigned to this update
    pub async fn update<T>(&self, key: impl Into<String>, value: &T) -> Result<u64>
    where
        T: Serialize,
    {
        let key = key.into();
        let serialized = serde_json::to_string(value)?;
        let checksum = format!("{:x}", md5::compute(&serialized));

        let mut versions = self.state_versions.write().await;
        let current_version = versions.get(&key).map(|v| v.version).unwrap_or(0);
        let new_version = current_version + 1;

        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let version_info = StateVersion {
            version: new_version,
            timestamp,
            checksum: checksum.clone(),
        };

        versions.insert(key.clone(), version_info.clone());
        drop(versions);

        // Store in pending updates (mark as dirty if offline)
        let network_state = self.network_state.read().await;
        let is_offline = *network_state == NetworkState::Offline;
        drop(network_state);

        let mut pending = self.pending_updates.lock().await;
        pending.insert(
            key.clone(),
            SyncedEntry {
                version: version_info,
                value: serialized,
                dirty: is_offline,
            },
        );

        // Emit update event
        let _ =
            self.update_tx
                .send(UpdateEvent::KeyUpdated { key, version: new_version, timestamp });

        Ok(new_version)
    }

    /// Get a state value with version information
    ///
    /// # Arguments
    ///
    /// * `key` - State key to retrieve
    ///
    /// # Returns
    ///
    /// The deserialized value and its version, or None if not found
    pub async fn get<T>(&self, key: &str) -> Result<Option<(T, u64)>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let pending = self.pending_updates.lock().await;
        if let Some(entry) = pending.get(key) {
            let value: T = serde_json::from_str(&entry.value)?;
            return Ok(Some((value, entry.version.version)));
        }
        Ok(None)
    }

    /// Get the version information for a key
    ///
    /// # Arguments
    ///
    /// * `key` - State key
    ///
    /// # Returns
    ///
    /// Version number and timestamp, or None if not found
    pub async fn get_version(&self, key: &str) -> Option<(u64, u64)> {
        let versions = self.state_versions.read().await;
        versions.get(key).map(|v| (v.version, v.timestamp))
    }

    /// Resolve a conflict between local and remote state
    ///
    /// # Arguments
    ///
    /// * `key` - The conflicting key
    /// * `local_value` - Local state value
    /// * `local_version` - Local version number
    /// * `remote_value` - Remote state value
    /// * `remote_version` - Remote version number
    /// * `remote_timestamp` - Remote update timestamp
    ///
    /// # Returns
    ///
    /// The resolved value and whether local state should be updated
    pub async fn resolve_conflict<T>(
        &self,
        key: &str,
        local_value: &T,
        local_version: u64,
        remote_value: &T,
        remote_version: u64,
        remote_timestamp: u64,
    ) -> Result<(String, bool)>
    where
        T: Serialize,
    {
        // Emit conflict detection event
        let _ = self.update_tx.send(UpdateEvent::ConflictDetected {
            key: key.to_string(),
            local_version,
            remote_version,
        });

        match self.config.conflict_strategy {
            ConflictStrategy::LastWriteWins => {
                let versions = self.state_versions.read().await;
                let local_timestamp = versions.get(key).map(|v| v.timestamp).unwrap_or(0);

                if remote_timestamp > local_timestamp {
                    // Remote is newer
                    Ok((serde_json::to_string(remote_value)?, true))
                } else {
                    // Local is newer or same age
                    Ok((serde_json::to_string(local_value)?, false))
                }
            }

            ConflictStrategy::LocalWins => Ok((serde_json::to_string(local_value)?, false)),

            ConflictStrategy::RemoteWins => Ok((serde_json::to_string(remote_value)?, true)),

            ConflictStrategy::Manual => Err(SyncError::ConflictError(format!(
                "Manual conflict resolution required for key: {}",
                key
            ))),
        }
    }

    /// Sync pending updates to remote
    ///
    /// # Returns
    ///
    /// Number of updates successfully synced
    pub async fn sync_pending(&self) -> Result<usize> {
        let network_state = self.network_state.read().await;
        if *network_state == NetworkState::Offline {
            return Err(SyncError::NetworkError("Cannot sync while offline".to_string()));
        }
        drop(network_state);

        let mut pending = self.pending_updates.lock().await;
        let dirty_count = pending.values().filter(|e| e.dirty).count();

        // In a real implementation, this would send updates to the server
        // For now, just mark them as clean
        for entry in pending.values_mut() {
            if entry.dirty {
                entry.dirty = false;
            }
        }

        Ok(dirty_count)
    }

    /// Clear all pending updates
    pub async fn clear_pending(&self) {
        let mut pending = self.pending_updates.lock().await;
        pending.clear();
    }

    /// Get count of pending updates
    ///
    /// # Returns
    ///
    /// Number of dirty entries waiting to sync
    pub async fn pending_count(&self) -> usize {
        let pending = self.pending_updates.lock().await;
        pending.values().filter(|e| e.dirty).count()
    }

    /// Trigger a full refresh from remote state
    ///
    /// # Arguments
    ///
    /// * `version` - The new state version after refresh
    pub async fn refresh(&self, version: u64) -> Result<()> {
        let network_state = self.network_state.read().await;
        if *network_state == NetworkState::Offline {
            return Err(SyncError::NetworkError("Cannot refresh while offline".to_string()));
        }
        drop(network_state);

        // In a real implementation, this would fetch from the server
        let _ = self.update_tx.send(UpdateEvent::FullRefresh { version });

        Ok(())
    }

    /// Wait for a specific update event
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait
    ///
    /// # Returns
    ///
    /// The next update event, or timeout error
    pub async fn wait_for_update(&self, timeout: Duration) -> Result<UpdateEvent> {
        let mut rx = self.subscribe();

        tokio::select! {
            result = rx.recv() => {
                result.map_err(|e| SyncError::Timeout(format!("Channel error: {}", e)))
            }
            _ = tokio::time::sleep(timeout) => {
                Err(SyncError::Timeout("Timeout waiting for update".to_string()))
            }
        }
    }
}

impl Clone for StateSync {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            network_state: Arc::clone(&self.network_state),
            state_versions: Arc::clone(&self.state_versions),
            pending_updates: Arc::clone(&self.pending_updates),
            update_tx: self.update_tx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_basic_update() {
        let sync = StateSync::new(SyncConfig::default());

        let version = sync.update("test_key", &"test_value").await.unwrap();
        assert_eq!(version, 1);

        let (value, ver): (String, u64) = sync.get("test_key").await.unwrap().unwrap();
        assert_eq!(value, "test_value");
        assert_eq!(ver, 1);
    }

    #[tokio::test]
    async fn test_sync_version_increment() {
        let sync = StateSync::new(SyncConfig::default());

        let v1 = sync.update("key", &"value1").await.unwrap();
        assert_eq!(v1, 1);

        let v2 = sync.update("key", &"value2").await.unwrap();
        assert_eq!(v2, 2);

        let v3 = sync.update("key", &"value3").await.unwrap();
        assert_eq!(v3, 3);
    }

    #[tokio::test]
    async fn test_sync_network_state() {
        let sync = StateSync::new(SyncConfig::default());

        assert_eq!(sync.network_state().await, NetworkState::Unknown);

        sync.set_network_state(NetworkState::Online).await.unwrap();
        assert_eq!(sync.network_state().await, NetworkState::Online);

        sync.set_network_state(NetworkState::Offline).await.unwrap();
        assert_eq!(sync.network_state().await, NetworkState::Offline);
    }

    #[tokio::test]
    async fn test_sync_offline_updates() {
        let sync = StateSync::new(SyncConfig::default());

        sync.set_network_state(NetworkState::Offline).await.unwrap();

        sync.update("key1", &"value1").await.unwrap();
        sync.update("key2", &"value2").await.unwrap();

        assert_eq!(sync.pending_count().await, 2);

        sync.set_network_state(NetworkState::Online).await.unwrap();
        let synced = sync.sync_pending().await.unwrap();
        assert_eq!(synced, 2);
        assert_eq!(sync.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_sync_conflict_last_write_wins() {
        let config = SyncConfig {
            conflict_strategy: ConflictStrategy::LastWriteWins,
            ..Default::default()
        };
        let sync = StateSync::new(config);

        // Local update (older)
        sync.update("key", &"local").await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;

        let local_timestamp = sync.get_version("key").await.unwrap().1;
        let remote_timestamp = local_timestamp + 1000;

        let (resolved, should_update) = sync
            .resolve_conflict("key", &"local", 1, &"remote", 2, remote_timestamp)
            .await
            .unwrap();

        assert!(should_update);
        assert_eq!(resolved, "\"remote\"");
    }

    #[tokio::test]
    async fn test_sync_conflict_local_wins() {
        let config = SyncConfig {
            conflict_strategy: ConflictStrategy::LocalWins,
            ..Default::default()
        };
        let sync = StateSync::new(config);

        let (resolved, should_update) = sync
            .resolve_conflict("key", &"local", 1, &"remote", 2, 0)
            .await
            .unwrap();

        assert!(!should_update);
        assert_eq!(resolved, "\"local\"");
    }

    #[tokio::test]
    async fn test_sync_conflict_remote_wins() {
        let config = SyncConfig {
            conflict_strategy: ConflictStrategy::RemoteWins,
            ..Default::default()
        };
        let sync = StateSync::new(config);

        let (resolved, should_update) = sync
            .resolve_conflict("key", &"local", 1, &"remote", 2, 0)
            .await
            .unwrap();

        assert!(should_update);
        assert_eq!(resolved, "\"remote\"");
    }

    #[tokio::test]
    async fn test_sync_events() {
        let sync = StateSync::new(SyncConfig::default());
        let mut rx = sync.subscribe();

        // Update a key
        tokio::spawn({
            let sync = sync.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                sync.update("test", &"value").await.unwrap();
            }
        });

        // Wait for event
        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();

        match event {
            UpdateEvent::KeyUpdated { key, version, .. } => {
                assert_eq!(key, "test");
                assert_eq!(version, 1);
            }
            _ => panic!("Expected KeyUpdated event"),
        }
    }

    #[tokio::test]
    async fn test_sync_network_event() {
        let sync = StateSync::new(SyncConfig::default());
        let mut rx = sync.subscribe();

        sync.set_network_state(NetworkState::Online).await.unwrap();

        let event = rx.recv().await.unwrap();
        match event {
            UpdateEvent::NetworkStateChanged { state } => {
                assert_eq!(state, NetworkState::Online);
            }
            _ => panic!("Expected NetworkStateChanged event"),
        }
    }

    #[tokio::test]
    async fn test_sync_clear_pending() {
        let sync = StateSync::new(SyncConfig::default());

        sync.set_network_state(NetworkState::Offline).await.unwrap();
        sync.update("key1", &"value1").await.unwrap();
        sync.update("key2", &"value2").await.unwrap();

        assert_eq!(sync.pending_count().await, 2);

        sync.clear_pending().await;
        assert_eq!(sync.pending_count().await, 0);
    }

    #[tokio::test]
    async fn test_sync_wait_for_update() {
        let sync = StateSync::new(SyncConfig::default());

        // Spawn a task to update after a delay
        tokio::spawn({
            let sync = sync.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                sync.update("delayed_key", &"delayed_value").await.unwrap();
            }
        });

        // Wait for the update
        let event = sync.wait_for_update(Duration::from_secs(1)).await.unwrap();

        match event {
            UpdateEvent::KeyUpdated { key, .. } => {
                assert_eq!(key, "delayed_key");
            }
            _ => panic!("Expected KeyUpdated event"),
        }
    }

    #[tokio::test]
    async fn test_sync_timeout() {
        let sync = StateSync::new(SyncConfig::default());

        // This should timeout as no update will occur
        let result = sync.wait_for_update(Duration::from_millis(100)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SyncError::Timeout(_)));
    }
}
