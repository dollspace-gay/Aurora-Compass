//! Storage layer for Aurora Compass
//!
//! This crate provides database abstraction, key-value storage,
//! caching, and data persistence.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod cache;
pub mod database;
pub mod kv;
pub mod persistence;

pub use database::{
    Database, DatabaseConfig, DatabaseError, DatabaseTransaction, MigrationDefinition,
    SqliteDatabase, SynchronousMode,
};
pub use kv::{
    AccountStore, CompareAndSwapError, DeviceStore, KvConfig, KvError, KvStore,
};
pub use cache::{CacheConfig, CacheError, DiskCache, MemoryCache, TieredCache};
