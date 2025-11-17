//! AT Protocol Client Library
//!
//! This crate provides a complete Rust implementation of the AT Protocol client,
//! including XRPC, Lexicon schemas, session management, and the BskyAgent.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod agent;
pub mod lexicon;
pub mod session;
pub mod types;
pub mod xrpc;

pub use agent::BskyAgent;
pub use types::{AtUri, Did, Handle, StrongRef, Tid};

/// Result type for AT Protocol operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for AT Protocol operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Network error
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// API error with status code and message
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message from server
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_types() {
        let err = Error::InvalidInput("test".to_string());
        assert!(err.to_string().contains("Invalid input"));
    }
}
