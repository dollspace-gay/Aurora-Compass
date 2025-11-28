//! Content moderation for Aurora Compass
//!
//! This crate handles content filtering, labeling services,
//! block/mute functionality, reporting, and content warnings.
//!
//! # Modules
//!
//! - [`blocking`] - Account blocking and muting via AT Protocol
//! - [`filtering`] - Content filtering based on labels, muted words, and preferences
//! - [`labels`] - Labeling services and label management
//! - [`reporting`] - Content reporting for accounts, posts, and messages

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod blocking;
pub mod filtering;
pub mod labels;
pub mod reporting;

// Re-export commonly used types
pub use blocking::{
    BlockError, BlockService, BlockedProfileView, GetBlocksResponse, GetMutesResponse,
    MutedProfileView,
};
pub use filtering::{ContentFilter, FilterPreferences, FilterReason, FilterResult};
pub use reporting::{ReportError, ReportReason, ReportService, ReportSubject};
