//! Storage layer for Aurora Compass
//!
//! This crate provides database abstraction, key-value storage,
//! caching, and data persistence.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod app_state;
pub mod cache;
pub mod database;
pub mod kv;
pub mod persistence;
pub mod preferences;
pub mod sync;

pub use app_state::{AppPersistedState, ColorMode, LanguagePrefs, OnboardingState};
pub use cache::{CacheConfig, CacheError, DiskCache, MemoryCache, TieredCache};
pub use database::{
    Database, DatabaseConfig, DatabaseError, DatabaseTransaction, MigrationDefinition,
    SqliteDatabase, SynchronousMode,
};
pub use kv::{AccountStore, CompareAndSwapError, DeviceStore, KvConfig, KvError, KvStore};
pub use persistence::{
    MigratableState, PersistedState, PersistenceConfig, PersistenceError, StateMigration,
};
pub use preferences::{
    MessagePreferences, MessagePrivacy, NotificationPreferences, NotificationTypeSettings,
};
pub use sync::{ConflictStrategy, NetworkState, StateSync, SyncConfig, SyncError, UpdateEvent};
