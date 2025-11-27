//! Application state management for Aurora Compass
//!
//! This crate provides reactive state management with query/mutation patterns,
//! state synchronization, and optimistic updates.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod account_scope;
pub mod cache;
pub mod mutation;
pub mod query;
pub mod session;
pub mod sync;
pub mod unread;
