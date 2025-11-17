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
