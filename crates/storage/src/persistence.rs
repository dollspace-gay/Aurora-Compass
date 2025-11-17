//! Data persistence layer
//!
//! This module provides serialization, versioning, and state recovery for application data.

use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

/// Persistence error types
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// State not initialized
    #[error("State not initialized")]
    NotInitialized,

    /// Corruption detected
    #[error("Corruption detected: {0}")]
    Corruption(String),

    /// Version mismatch
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        /// Expected version
        expected: u32,
        /// Found version
        found: u32,
    },

    /// Migration failed
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
}

/// Result type for persistence operations
pub type Result<T> = std::result::Result<T, PersistenceError>;

/// Versioned state container
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
struct VersionedState<T> {
    /// Version number
    version: u32,
    /// Checksum for corruption detection
    checksum: String,
    /// The actual state data
    data: T,
}

impl<T: Serialize> VersionedState<T> {
    fn new(version: u32, data: T) -> Result<Self> {
        let data_json = serde_json::to_string(&data)?;
        let checksum = format!("{:x}", md5::compute(&data_json));

        Ok(Self { version, checksum, data })
    }

    fn verify_checksum(&self) -> Result<()> {
        let data_json = serde_json::to_string(&self.data)?;
        let computed = format!("{:x}", md5::compute(&data_json));

        if computed != self.checksum {
            return Err(PersistenceError::Corruption(format!(
                "Checksum mismatch: expected {}, got {}",
                self.checksum, computed
            )));
        }

        Ok(())
    }
}

/// Persistence configuration
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Path to the persistence file
    pub path: PathBuf,
    /// Current schema version
    pub version: u32,
    /// Enable atomic writes with temp files
    pub atomic_writes: bool,
    /// Enable automatic backups
    pub auto_backup: bool,
    /// Number of backups to keep
    pub backup_count: usize,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("state.json"),
            version: 1,
            atomic_writes: true,
            auto_backup: true,
            backup_count: 3,
        }
    }
}

impl PersistenceConfig {
    /// Create a new configuration
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), ..Default::default() }
    }

    /// Set schema version
    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Enable or disable atomic writes
    pub fn atomic_writes(mut self, enabled: bool) -> Self {
        self.atomic_writes = enabled;
        self
    }

    /// Configure backups
    pub fn backups(mut self, enabled: bool, count: usize) -> Self {
        self.auto_backup = enabled;
        self.backup_count = count;
        self
    }
}

/// Persisted state manager
pub struct PersistedState<T> {
    config: PersistenceConfig,
    state: Arc<RwLock<Option<T>>>,
    _phantom: PhantomData<T>,
}

impl<T> PersistedState<T>
where
    T: Serialize + DeserializeOwned + Clone + Default,
{
    /// Create a new persisted state manager
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(None)),
            _phantom: PhantomData,
        }
    }

    /// Initialize by loading from disk
    pub async fn init(&self) -> Result<()> {
        match self.load_from_disk().await {
            Ok(data) => {
                let mut state = self.state.write().await;
                *state = Some(data);
                Ok(())
            }
            Err(PersistenceError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist yet, use default
                let mut state = self.state.write().await;
                *state = Some(T::default());
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Get the current state
    pub async fn get(&self) -> Result<T> {
        let state = self.state.read().await;
        state.clone().ok_or(PersistenceError::NotInitialized)
    }

    /// Update the state and persist to disk
    pub async fn update<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut T),
    {
        let mut state = self.state.write().await;

        if let Some(current) = state.as_mut() {
            f(current);
            self.write_to_disk(current).await?;
            Ok(())
        } else {
            Err(PersistenceError::NotInitialized)
        }
    }

    /// Set the entire state and persist
    pub async fn set(&self, new_state: T) -> Result<()> {
        let mut state = self.state.write().await;
        *state = Some(new_state.clone());
        self.write_to_disk(&new_state).await
    }

    /// Clear the persisted state
    pub async fn clear(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = Some(T::default());

        if self.config.path.exists() {
            fs::remove_file(&self.config.path).await?;
        }

        Ok(())
    }

    /// Load state from disk
    async fn load_from_disk(&self) -> Result<T> {
        let contents = fs::read_to_string(&self.config.path).await?;

        let versioned: VersionedState<T> = serde_json::from_str(&contents)?;

        // Verify checksum
        versioned.verify_checksum()?;

        // Check version
        if versioned.version != self.config.version {
            return Err(PersistenceError::VersionMismatch {
                expected: self.config.version,
                found: versioned.version,
            });
        }

        Ok(versioned.data)
    }

    /// Write state to disk
    async fn write_to_disk(&self, data: &T) -> Result<()> {
        let versioned = VersionedState::new(self.config.version, data.clone())?;
        let json = serde_json::to_string_pretty(&versioned)?;

        if self.config.atomic_writes {
            self.write_atomic(&json).await?;
        } else {
            fs::write(&self.config.path, json).await?;
        }

        // Create backup if enabled
        if self.config.auto_backup {
            let _ = self.create_backup().await;
        }

        Ok(())
    }

    /// Write atomically using temp file + rename
    async fn write_atomic(&self, contents: &str) -> Result<()> {
        let temp_path = self.config.path.with_extension("tmp");

        // Write to temp file
        let mut file = fs::File::create(&temp_path).await?;
        file.write_all(contents.as_bytes()).await?;
        file.sync_all().await?;
        drop(file);

        // Atomic rename
        fs::rename(&temp_path, &self.config.path).await?;

        Ok(())
    }

    /// Create a backup of the current state
    async fn create_backup(&self) -> Result<()> {
        if !self.config.path.exists() {
            return Ok(());
        }

        // Rotate backups
        for i in (1..self.config.backup_count).rev() {
            let from = self.backup_path(i);
            let to = self.backup_path(i + 1);

            if from.exists() {
                let _ = fs::rename(&from, &to).await;
            }
        }

        // Create new backup
        let backup_path = self.backup_path(1);
        let _ = fs::copy(&self.config.path, &backup_path).await;

        Ok(())
    }

    /// Get backup file path
    fn backup_path(&self, n: usize) -> PathBuf {
        let mut path = self.config.path.clone();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        path.set_file_name(format!("{}.backup.{}", filename, n));
        path
    }

    /// Restore from backup
    pub async fn restore_from_backup(&self, backup_number: usize) -> Result<()> {
        let backup_path = self.backup_path(backup_number);

        if !backup_path.exists() {
            return Err(PersistenceError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Backup not found",
            )));
        }

        fs::copy(&backup_path, &self.config.path).await?;
        self.init().await
    }
}

/// Migration trait for upgrading state between versions
pub trait StateMigration {
    /// Migrate from one version to another
    fn migrate(&self, from_version: u32, data: serde_json::Value) -> Result<serde_json::Value>;
}

/// Persisted state with migration support
pub struct MigratableState<T> {
    inner: PersistedState<T>,
    migrations: Vec<Box<dyn StateMigration + Send + Sync>>,
}

impl<T> MigratableState<T>
where
    T: Serialize + DeserializeOwned + Clone + Default,
{
    /// Create a new migratable state
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            inner: PersistedState::new(config),
            migrations: Vec::new(),
        }
    }

    /// Add a migration
    pub fn add_migration(mut self, migration: impl StateMigration + Send + Sync + 'static) -> Self {
        self.migrations.push(Box::new(migration));
        self
    }

    /// Initialize with migration support
    pub async fn init(&self) -> Result<()> {
        // Try to load and migrate if needed
        match self.load_and_migrate().await {
            Ok(data) => {
                self.inner.set(data).await?;
                Ok(())
            }
            Err(PersistenceError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                // No existing state, use default
                self.inner.init().await
            }
            Err(e) => Err(e),
        }
    }

    /// Load and apply migrations
    async fn load_and_migrate(&self) -> Result<T> {
        let contents = fs::read_to_string(&self.inner.config.path).await?;
        let raw: serde_json::Value = serde_json::from_str(&contents)?;

        let version = raw
            .get("version")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| PersistenceError::Corruption("Missing version".to_string()))?
            as u32;

        if version == self.inner.config.version {
            // No migration needed
            let versioned: VersionedState<T> = serde_json::from_value(raw)?;
            versioned.verify_checksum()?;
            return Ok(versioned.data);
        }

        // Apply migrations
        let mut current_data = raw
            .get("data")
            .cloned()
            .ok_or_else(|| PersistenceError::Corruption("Missing data field".to_string()))?;

        for migration in &self.migrations {
            current_data = migration.migrate(version, current_data)?;
        }

        // Deserialize migrated data
        let migrated: T = serde_json::from_value(current_data)?;
        Ok(migrated)
    }

    /// Delegate to inner PersistedState
    pub async fn get(&self) -> Result<T> {
        self.inner.get().await
    }

    /// Delegate to inner PersistedState
    pub async fn update<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce(&mut T),
    {
        self.inner.update(f).await
    }

    /// Delegate to inner PersistedState
    pub async fn set(&self, new_state: T) -> Result<()> {
        self.inner.set(new_state).await
    }

    /// Delegate to inner PersistedState
    pub async fn clear(&self) -> Result<()> {
        self.inner.clear().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
    struct TestState {
        counter: i32,
        name: String,
    }

    #[tokio::test]
    async fn test_persisted_state_init() {
        let config = PersistenceConfig::new("test_init.json");
        let state: PersistedState<TestState> = PersistedState::new(config);

        state.init().await.unwrap();

        let current = state.get().await.unwrap();
        assert_eq!(current, TestState::default());

        // Cleanup
        let _ = state.clear().await;
    }

    #[tokio::test]
    async fn test_persisted_state_update() {
        let config = PersistenceConfig::new("test_update.json");
        let state: PersistedState<TestState> = PersistedState::new(config);

        state.init().await.unwrap();

        state
            .update(|s| {
                s.counter = 42;
                s.name = "test".to_string();
            })
            .await
            .unwrap();

        let current = state.get().await.unwrap();
        assert_eq!(current.counter, 42);
        assert_eq!(current.name, "test");

        // Cleanup
        let _ = state.clear().await;
    }

    #[tokio::test]
    async fn test_persisted_state_persistence() {
        let config = PersistenceConfig::new("test_persistence.json");

        {
            let state: PersistedState<TestState> = PersistedState::new(config.clone());
            state.init().await.unwrap();

            state
                .update(|s| {
                    s.counter = 99;
                    s.name = "persisted".to_string();
                })
                .await
                .unwrap();
        }

        // Create new instance and load
        {
            let state: PersistedState<TestState> = PersistedState::new(config.clone());
            state.init().await.unwrap();

            let current = state.get().await.unwrap();
            assert_eq!(current.counter, 99);
            assert_eq!(current.name, "persisted");

            // Cleanup
            let _ = state.clear().await;
        }
    }

    #[tokio::test]
    async fn test_persisted_state_corruption_detection() {
        let config = PersistenceConfig::new("test_corruption.json");
        let state: PersistedState<TestState> = PersistedState::new(config.clone());

        state.init().await.unwrap();
        state
            .update(|s| {
                s.counter = 42;
            })
            .await
            .unwrap();

        // Manually corrupt the file
        let mut contents = fs::read_to_string(&config.path).await.unwrap();
        contents = contents.replace("42", "99");
        fs::write(&config.path, contents).await.unwrap();

        // Try to load - should fail with corruption error
        let state2: PersistedState<TestState> = PersistedState::new(config.clone());
        let result = state2.init().await;
        assert!(matches!(result, Err(PersistenceError::Corruption(_))));

        // Cleanup
        let _ = fs::remove_file(&config.path).await;
    }

    #[tokio::test]
    async fn test_persisted_state_backup() {
        let config = PersistenceConfig::new("test_backup.json").backups(true, 2);

        let state: PersistedState<TestState> = PersistedState::new(config.clone());
        state.init().await.unwrap();

        // Make several updates to create backups
        for i in 1..=3 {
            state
                .update(|s| {
                    s.counter = i;
                })
                .await
                .unwrap();
        }

        // Restore from backup.2 (which should have counter=2)
        state.restore_from_backup(2).await.unwrap();

        let current = state.get().await.unwrap();
        assert_eq!(current.counter, 2); // Two updates ago

        // Cleanup
        let _ = state.clear().await;
        let _ = fs::remove_file(&state.backup_path(1)).await;
        let _ = fs::remove_file(&state.backup_path(2)).await;
    }

    #[tokio::test]
    async fn test_versioned_state() {
        let state = TestState { counter: 42, name: "test".to_string() };

        let versioned = VersionedState::new(1, state).unwrap();
        assert_eq!(versioned.version, 1);

        // Verify checksum passes
        versioned.verify_checksum().unwrap();
    }

    #[tokio::test]
    async fn test_atomic_writes() {
        let config = PersistenceConfig::new("test_atomic.json").atomic_writes(true);

        let state: PersistedState<TestState> = PersistedState::new(config.clone());
        state.init().await.unwrap();

        state
            .update(|s| {
                s.counter = 123;
            })
            .await
            .unwrap();

        // Verify temp file was cleaned up
        let temp_path = config.path.with_extension("tmp");
        assert!(!temp_path.exists());

        // Cleanup
        let _ = state.clear().await;
    }

    struct TestMigration;

    impl StateMigration for TestMigration {
        fn migrate(
            &self,
            _from_version: u32,
            mut data: serde_json::Value,
        ) -> Result<serde_json::Value> {
            // Example migration: add a default field
            if let Some(obj) = data.as_object_mut() {
                obj.insert("name".to_string(), serde_json::Value::String("migrated".to_string()));
            }
            Ok(data)
        }
    }

    #[tokio::test]
    async fn test_migratable_state() {
        let config = PersistenceConfig::new("test_migration.json").version(2);

        let state = MigratableState::<TestState>::new(config).add_migration(TestMigration);

        state.init().await.unwrap();

        state
            .update(|s| {
                s.counter = 100;
            })
            .await
            .unwrap();

        let current = state.get().await.unwrap();
        assert_eq!(current.counter, 100);

        // Cleanup
        let _ = state.clear().await;
    }
}
