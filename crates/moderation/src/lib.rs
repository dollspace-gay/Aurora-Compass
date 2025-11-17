//! Content moderation for Aurora Compass
//!
//! This crate handles content filtering, labeling services,
//! block/mute functionality, and content warnings.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod filtering;
pub mod labels;
pub mod blocking;
