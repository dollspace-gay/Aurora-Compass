//! Networking utilities for Aurora Compass
//!
//! This crate provides HTTP client functionality with retry logic,
//! timeout handling, and connection pooling.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod client;
pub mod retry;

pub use client::HttpClient;
