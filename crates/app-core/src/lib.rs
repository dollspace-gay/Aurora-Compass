//! Core application logic for Aurora Compass
//!
//! This crate contains shared business logic for feeds, profiles,
//! posts, messages, and other core features.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod auth;
pub mod branding;
pub mod feeds;
pub mod messages;
pub mod notifications;
pub mod posts;
pub mod profiles;
