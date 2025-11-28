//! Content moderation for Aurora Compass
//!
//! This crate handles content filtering, labeling services,
//! block/mute functionality, and content warnings.
//!
//! # Modules
//!
//! - [`blocking`] - Account blocking and muting via AT Protocol
//! - [`filtering`] - Content filtering based on labels, muted words, and preferences
//! - [`labels`] - Labeling services and label management

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod blocking;
pub mod filtering;
pub mod labels;

// Re-export commonly used types
pub use blocking::{BlockError, BlockService, BlockedProfileView, GetBlocksResponse, GetMutesResponse, MutedProfileView};
pub use filtering::{ContentFilter, FilterPreferences, FilterReason, FilterResult};
