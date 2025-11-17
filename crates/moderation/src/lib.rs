//! Content moderation for Aurora Compass
//!
//! This crate handles content filtering, labeling services,
//! block/mute functionality, and content warnings.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod blocking;
pub mod filtering;
pub mod labels;
