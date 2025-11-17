//! Test utilities and fixtures for AT Protocol client testing
//!
//! This module provides common test helpers, mock data generators,
//! and fixtures for testing AT Protocol functionality.

#![allow(dead_code)] // Test utilities may not all be used yet

use crate::types::{AtUri, Did, Handle, StrongRef, Tid};
use std::time::{Duration, SystemTime};

/// Test DIDs for use in tests
pub mod dids {
    use super::*;

    /// Alice's DID (PLC method)
    pub fn alice() -> Did {
        Did::new("did:plc:alice123456789abc").unwrap()
    }

    /// Bob's DID (PLC method)
    pub fn bob() -> Did {
        Did::new("did:plc:bob123456789defg").unwrap()
    }

    /// Carol's DID (Web method)
    pub fn carol() -> Did {
        Did::new("did:web:carol.example.com").unwrap()
    }

    /// Test PDS service DID
    pub fn pds() -> Did {
        Did::new("did:web:pds.test.example.com").unwrap()
    }

    /// Generate a random test DID with PLC method
    pub fn random_plc() -> Did {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Did::new(format!("did:plc:test{:x}", timestamp)).unwrap()
    }
}

/// Test handles for use in tests
pub mod handles {
    use super::*;

    /// Alice's handle
    pub fn alice() -> Handle {
        Handle::new("alice.bsky.social").unwrap()
    }

    /// Bob's handle
    pub fn bob() -> Handle {
        Handle::new("bob.bsky.social").unwrap()
    }

    /// Carol's handle (custom domain)
    pub fn carol() -> Handle {
        Handle::new("carol.example.com").unwrap()
    }

    /// Test subdomain handle
    pub fn subdomain() -> Handle {
        Handle::new("user.subdomain.bsky.social").unwrap()
    }

    /// Generate a random test handle
    pub fn random() -> Handle {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Handle::new(format!("user{:x}.test.example.com", timestamp)).unwrap()
    }
}

/// Test AT URIs for use in tests
pub mod uris {
    use super::*;

    /// Post URI for Alice
    pub fn alice_post() -> AtUri {
        AtUri::new("at://did:plc:alice123456789abc/app.bsky.feed.post/abc123").unwrap()
    }

    /// Profile URI for Bob
    pub fn bob_profile() -> AtUri {
        AtUri::new("at://did:plc:bob123456789defg/app.bsky.actor.profile/self").unwrap()
    }

    /// Like record URI
    pub fn like_record() -> AtUri {
        AtUri::new("at://did:plc:alice123456789abc/app.bsky.feed.like/xyz789").unwrap()
    }

    /// Repost record URI
    pub fn repost_record() -> AtUri {
        AtUri::new("at://did:plc:bob123456789defg/app.bsky.feed.repost/def456").unwrap()
    }

    /// Follow record URI
    pub fn follow_record() -> AtUri {
        AtUri::new("at://did:plc:alice123456789abc/app.bsky.graph.follow/follow123").unwrap()
    }

    /// Generate a random post URI
    pub fn random_post(did: &Did) -> AtUri {
        let tid = Tid::now();
        AtUri::new(format!("at://{}/app.bsky.feed.post/{}", did.as_str(), tid.as_str())).unwrap()
    }

    /// Create a URI from components
    pub fn from_parts(authority: &str, collection: &str, rkey: &str) -> AtUri {
        AtUri::new(format!("at://{}/{}/{}", authority, collection, rkey)).unwrap()
    }
}

/// Test TIDs for use in tests
pub mod tids {
    use super::*;

    /// Fixed TID for reproducible tests (2024-01-01 00:00:00 UTC)
    pub fn fixed() -> Tid {
        let timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(1704067200);
        Tid::from_timestamp(timestamp)
    }

    /// Old TID (2023-01-01)
    pub fn old() -> Tid {
        let timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(1672531200);
        Tid::from_timestamp(timestamp)
    }

    /// Recent TID (approximately now)
    pub fn recent() -> Tid {
        Tid::now()
    }

    /// Create a sequence of TIDs spaced 1 second apart
    pub fn sequence(count: usize) -> Vec<Tid> {
        let base = SystemTime::UNIX_EPOCH + Duration::from_secs(1704067200);
        (0..count)
            .map(|i| Tid::from_timestamp(base + Duration::from_secs(i as u64)))
            .collect()
    }
}

/// Test CIDs (Content Identifiers) for use in tests
pub mod cids {
    /// Valid CID for a post
    pub fn post() -> String {
        "bafyreigq4zsipbk5w3uqkbmh2w2633c4tcwudryvoqkfrq3mqfs3d5e3wq".to_string()
    }

    /// Valid CID for an image
    pub fn image() -> String {
        "bafkreiabf3z4vjwcky4q52m5tkdvi5yvy5ykhfhqjexgc2zqe6mxqhvkqe".to_string()
    }

    /// Valid CID for a video
    pub fn video() -> String {
        "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi".to_string()
    }

    /// Generate a test CID with a specific prefix
    pub fn with_prefix(prefix: &str) -> String {
        format!("{}abcdefghijklmnopqrstuvwxyz234567", prefix)
    }
}

/// Test strong references for use in tests
pub mod strong_refs {
    use super::*;

    /// Strong ref to Alice's post
    pub fn alice_post() -> StrongRef {
        StrongRef { uri: uris::alice_post(), cid: cids::post() }
    }

    /// Strong ref to Bob's profile
    pub fn bob_profile() -> StrongRef {
        StrongRef {
            uri: uris::bob_profile(),
            cid: cids::with_prefix("bafyrei"),
        }
    }

    /// Create a strong ref from URI and CID
    pub fn from_parts(uri: AtUri, cid: String) -> StrongRef {
        StrongRef { uri, cid }
    }
}

/// XRPC test data structures and helpers
pub mod xrpc {
    use serde::{Deserialize, Serialize};

    /// Test query input
    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    pub struct TestQuery {
        pub name: String,
        pub value: i32,
    }

    impl TestQuery {
        pub fn new(name: impl Into<String>, value: i32) -> Self {
            Self { name: name.into(), value }
        }
    }

    /// Test procedure input
    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    pub struct TestInput {
        pub text: String,
        pub count: u32,
    }

    impl TestInput {
        pub fn new(text: impl Into<String>, count: u32) -> Self {
            Self { text: text.into(), count }
        }
    }

    /// Test procedure output
    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    pub struct TestOutput {
        pub uri: String,
        pub cid: String,
    }

    impl TestOutput {
        pub fn new(uri: impl Into<String>, cid: impl Into<String>) -> Self {
            Self { uri: uri.into(), cid: cid.into() }
        }
    }
}

/// Assertion helpers for AT Protocol types
pub mod assertions {
    use super::*;

    /// Assert that a DID is valid and has the expected method
    pub fn assert_did_method(did: &Did, expected_method: &str) {
        assert_eq!(
            did.method(),
            expected_method,
            "DID {} should have method {}",
            did.as_str(),
            expected_method
        );
    }

    /// Assert that a handle is valid and normalized
    pub fn assert_handle_normalized(handle: &Handle) {
        assert_eq!(
            handle.as_str(),
            handle.as_str().to_lowercase(),
            "Handle should be normalized to lowercase"
        );
    }

    /// Assert that an AT URI has the expected collection
    pub fn assert_uri_collection(uri: &AtUri, expected_collection: &str) {
        assert_eq!(
            uri.collection(),
            Some(expected_collection),
            "URI {} should have collection {}",
            uri.as_str(),
            expected_collection
        );
    }

    /// Assert that two TIDs are ordered correctly
    pub fn assert_tid_ordering(older: &Tid, newer: &Tid) {
        assert!(older < newer, "TID {} should be less than {}", older.as_str(), newer.as_str());
    }

    /// Assert that a strong ref has matching URI and CID
    pub fn assert_strong_ref_valid(strong_ref: &StrongRef) {
        assert!(!strong_ref.cid.is_empty(), "CID should not be empty");
        assert!(
            strong_ref.cid.starts_with("bafy") || strong_ref.cid.starts_with("bafk"),
            "CID should start with valid prefix"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_dids() {
        let alice = dids::alice();
        assert_eq!(alice.method(), "plc");

        let carol = dids::carol();
        assert_eq!(carol.method(), "web");
    }

    #[test]
    fn test_fixture_handles() {
        let alice = handles::alice();
        assert_eq!(alice.as_str(), "alice.bsky.social");

        let carol = handles::carol();
        assert_eq!(carol.as_str(), "carol.example.com");
    }

    #[test]
    fn test_fixture_uris() {
        let post_uri = uris::alice_post();
        assert_eq!(post_uri.collection(), Some("app.bsky.feed.post"));
        assert_eq!(post_uri.rkey(), Some("abc123"));
    }

    #[test]
    fn test_fixture_tids() {
        let seq = tids::sequence(5);
        assert_eq!(seq.len(), 5);

        // Verify they're ordered
        for i in 0..seq.len() - 1 {
            assert!(seq[i] < seq[i + 1]);
        }
    }

    #[test]
    fn test_fixture_strong_refs() {
        let strong_ref = strong_refs::alice_post();
        assertions::assert_strong_ref_valid(&strong_ref);
    }

    #[test]
    fn test_random_generators() {
        let did1 = dids::random_plc();
        let did2 = dids::random_plc();
        // Should generate different DIDs
        assert_ne!(did1.as_str(), did2.as_str());

        let handle1 = handles::random();
        let handle2 = handles::random();
        // Should generate different handles
        assert_ne!(handle1.as_str(), handle2.as_str());
    }

    #[test]
    fn test_assertion_helpers() {
        let alice = dids::alice();
        assertions::assert_did_method(&alice, "plc");

        let handle = handles::alice();
        assertions::assert_handle_normalized(&handle);

        let uri = uris::alice_post();
        assertions::assert_uri_collection(&uri, "app.bsky.feed.post");

        let tids = tids::sequence(2);
        assertions::assert_tid_ordering(&tids[0], &tids[1]);
    }
}
