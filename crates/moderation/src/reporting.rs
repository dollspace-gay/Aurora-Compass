//! Content reporting functionality
//!
//! This module provides functionality for reporting content to moderation services
//! via the AT Protocol's `com.atproto.moderation.createReport` endpoint.
//!
//! # Overview
//!
//! Users can report various types of content:
//! - **Accounts**: Report a user for spam, impersonation, harassment, etc.
//! - **Posts**: Report a post for policy violations, sexual content, etc.
//! - **Lists/Feeds**: Report curated content for violations
//! - **Chat Messages**: Report direct messages for harassment, spam, etc.
//!
//! # Example
//!
//! ```rust,no_run
//! use moderation::reporting::{ReportService, ReportReason};
//! use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = XrpcClientConfig::new("https://bsky.social");
//!     let client = XrpcClient::new(config);
//!     let service = ReportService::new(client);
//!
//!     // Report an account for spam
//!     let result = service.report_account(
//!         "did:plc:spammer123",
//!         ReportReason::Spam,
//!         Some("Sending unsolicited promotional messages".to_string()),
//!     ).await?;
//!     println!("Report submitted with ID: {}", result.id);
//!
//!     // Report a post for misleading content
//!     let result = service.report_post(
//!         "at://did:plc:abc/app.bsky.feed.post/123",
//!         "bafyreigxyz...",
//!         ReportReason::Misleading,
//!         Some("Spreading misinformation about health".to_string()),
//!     ).await?;
//!     println!("Report #{} created at {}", result.id, result.created_at);
//!
//!     Ok(())
//! }
//! ```

use atproto_client::xrpc::{XrpcClient, XrpcRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during report operations
#[derive(Debug, Error)]
pub enum ReportError {
    /// API/XRPC error
    #[error("API error: {0}")]
    ApiError(String),

    /// Invalid report subject
    #[error("Invalid subject: {0}")]
    InvalidSubject(String),

    /// Invalid reason type
    #[error("Invalid reason: {0}")]
    InvalidReason(String),

    /// No active session
    #[error("No active session")]
    NoSession,

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Report details too long
    #[error("Report details too long: {0} characters (max {1})")]
    DetailsTooLong(usize, usize),
}

/// Result type for report operations
pub type Result<T> = std::result::Result<T, ReportError>;

/// Maximum length for report details (in characters)
const MAX_DETAILS_LENGTH: usize = 2000;

// ============================================================================
// Report Reason Types
// ============================================================================

/// Reason for submitting a report
///
/// These correspond to the AT Protocol moderation reason types defined in
/// `com.atproto.moderation.defs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportReason {
    /// Spam: excessive mentions, unwanted messages, promotional content
    Spam,
    /// Violation: illegal content, glaring violations of law or ToS
    Violation,
    /// Misleading: impersonation, misinformation, false claims
    Misleading,
    /// Sexual: unwanted sexual content, unlabeled adult material
    Sexual,
    /// Rude: harassment, anti-social behavior, trolling, intolerance
    Rude,
    /// Other: issues not covered by other categories
    Other,
    /// Appeal: appealing a moderation decision on your own account
    Appeal,
}

impl ReportReason {
    /// Get the AT Protocol reason type string
    ///
    /// Returns the fully qualified NSID for this reason type.
    pub fn as_at_uri(&self) -> &'static str {
        match self {
            Self::Spam => "com.atproto.moderation.defs#reasonSpam",
            Self::Violation => "com.atproto.moderation.defs#reasonViolation",
            Self::Misleading => "com.atproto.moderation.defs#reasonMisleading",
            Self::Sexual => "com.atproto.moderation.defs#reasonSexual",
            Self::Rude => "com.atproto.moderation.defs#reasonRude",
            Self::Other => "com.atproto.moderation.defs#reasonOther",
            Self::Appeal => "com.atproto.moderation.defs#reasonAppeal",
        }
    }

    /// Get a human-readable description of the reason
    pub fn description(&self) -> &'static str {
        match self {
            Self::Spam => "Spam or excessive unwanted content",
            Self::Violation => "Illegal content or terms of service violation",
            Self::Misleading => "Impersonation or misinformation",
            Self::Sexual => "Sexual content that should be labeled",
            Self::Rude => "Harassment or anti-social behavior",
            Self::Other => "Other issue not covered by other categories",
            Self::Appeal => "Appeal a moderation decision",
        }
    }

    /// Get all reason types suitable for reporting accounts
    pub fn for_account() -> &'static [ReportReason] {
        &[Self::Misleading, Self::Spam, Self::Violation, Self::Rude, Self::Other]
    }

    /// Get all reason types suitable for reporting posts
    pub fn for_post() -> &'static [ReportReason] {
        &[
            Self::Misleading,
            Self::Spam,
            Self::Sexual,
            Self::Violation,
            Self::Rude,
            Self::Other,
        ]
    }

    /// Get all reason types suitable for reporting lists/feeds
    pub fn for_list() -> &'static [ReportReason] {
        &[Self::Violation, Self::Rude, Self::Other]
    }

    /// Get all reason types suitable for reporting chat messages
    pub fn for_chat() -> &'static [ReportReason] {
        &[Self::Spam, Self::Sexual, Self::Violation, Self::Rude, Self::Other]
    }
}

impl std::fmt::Display for ReportReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

// ============================================================================
// Report Subject Types
// ============================================================================

/// Subject of a report (what is being reported)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ReportSubject {
    /// Report an account/user
    Account(AccountSubject),
    /// Report a record (post, list, feed, starter pack)
    Record(RecordSubject),
    /// Report a chat message
    ChatMessage(ChatMessageSubject),
}

/// Subject for reporting an account
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSubject {
    /// Type marker for the AT Protocol
    #[serde(rename = "$type")]
    type_: String,
    /// DID of the account being reported
    pub did: String,
}

impl AccountSubject {
    /// Create a new account subject
    ///
    /// # Arguments
    ///
    /// * `did` - DID of the account to report
    pub fn new(did: impl Into<String>) -> Self {
        Self {
            type_: "com.atproto.admin.defs#repoRef".to_string(),
            did: did.into(),
        }
    }

    /// Get the DID of the account
    pub fn did(&self) -> &str {
        &self.did
    }
}

/// Subject for reporting a record (post, list, feed, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSubject {
    /// Type marker for the AT Protocol
    #[serde(rename = "$type")]
    type_: String,
    /// AT URI of the record
    pub uri: String,
    /// CID of the record
    pub cid: String,
}

impl RecordSubject {
    /// Create a new record subject
    ///
    /// # Arguments
    ///
    /// * `uri` - AT URI of the record (e.g., `at://did:plc:abc/app.bsky.feed.post/123`)
    /// * `cid` - CID of the record
    pub fn new(uri: impl Into<String>, cid: impl Into<String>) -> Self {
        Self {
            type_: "com.atproto.repo.strongRef".to_string(),
            uri: uri.into(),
            cid: cid.into(),
        }
    }

    /// Get the AT URI
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Get the CID
    pub fn cid(&self) -> &str {
        &self.cid
    }
}

/// Subject for reporting a chat message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageSubject {
    /// Type marker for the AT Protocol
    #[serde(rename = "$type")]
    type_: String,
    /// ID of the message
    pub message_id: String,
    /// ID of the conversation
    pub convo_id: String,
    /// DID of the message sender
    pub did: String,
}

impl ChatMessageSubject {
    /// Create a new chat message subject
    ///
    /// # Arguments
    ///
    /// * `message_id` - ID of the message to report
    /// * `convo_id` - ID of the conversation containing the message
    /// * `sender_did` - DID of the message sender
    pub fn new(
        message_id: impl Into<String>,
        convo_id: impl Into<String>,
        sender_did: impl Into<String>,
    ) -> Self {
        Self {
            type_: "chat.bsky.convo.defs#messageRef".to_string(),
            message_id: message_id.into(),
            convo_id: convo_id.into(),
            did: sender_did.into(),
        }
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request body for creating a report
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateReportRequest {
    /// The reason type (AT Protocol NSID)
    reason_type: String,
    /// The subject being reported
    subject: ReportSubject,
    /// Optional additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// Response from creating a report
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReportResponse {
    /// Report ID assigned by the moderation service
    pub id: u64,
    /// Reason type that was submitted
    pub reason_type: String,
    /// Subject that was reported
    pub subject: serde_json::Value,
    /// Reporter's DID
    pub reported_by: String,
    /// Timestamp when the report was created
    pub created_at: String,
}

/// Simplified report result returned to the caller
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportResult {
    /// Report ID
    pub id: u64,
    /// Timestamp when created
    pub created_at: String,
}

// ============================================================================
// Report Service
// ============================================================================

/// Service for submitting content reports
///
/// Provides methods for reporting accounts, posts, lists, and chat messages
/// to the AT Protocol moderation system.
///
/// # Example
///
/// ```rust,no_run
/// use moderation::reporting::{ReportService, ReportReason};
/// use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = XrpcClientConfig::new("https://bsky.social");
///     let client = XrpcClient::new(config);
///     let service = ReportService::new(client);
///
///     // Report spam account
///     let result = service.report_account(
///         "did:plc:spammer",
///         ReportReason::Spam,
///         Some("Sending spam DMs".to_string()),
///     ).await?;
///
///     println!("Report #{} created at {}", result.id, result.created_at);
///     Ok(())
/// }
/// ```
pub struct ReportService {
    /// XRPC client
    client: Arc<RwLock<XrpcClient>>,
}

impl ReportService {
    /// Create a new report service
    ///
    /// # Arguments
    ///
    /// * `client` - XRPC client for making API calls
    pub fn new(client: XrpcClient) -> Self {
        Self { client: Arc::new(RwLock::new(client)) }
    }

    /// Create a new report service with a shared client
    ///
    /// # Arguments
    ///
    /// * `client` - Shared XRPC client
    pub fn with_shared_client(client: Arc<RwLock<XrpcClient>>) -> Self {
        Self { client }
    }

    // =========================================================================
    // Core Report Method
    // =========================================================================

    /// Submit a report to the moderation service
    ///
    /// This is the core method that all specific report methods use.
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for the report
    /// * `subject` - What is being reported
    /// * `details` - Optional additional details about the report
    ///
    /// # Returns
    ///
    /// Report result with ID and timestamp
    ///
    /// # Errors
    ///
    /// - `ReportError::DetailsTooLong` - Details exceed max length
    /// - `ReportError::ApiError` - API error
    pub async fn create_report(
        &self,
        reason: ReportReason,
        subject: ReportSubject,
        details: Option<String>,
    ) -> Result<ReportResult> {
        // Validate details length
        if let Some(ref d) = details {
            if d.chars().count() > MAX_DETAILS_LENGTH {
                return Err(ReportError::DetailsTooLong(d.chars().count(), MAX_DETAILS_LENGTH));
            }
        }

        let request_body = CreateReportRequest {
            reason_type: reason.as_at_uri().to_string(),
            subject,
            reason: details,
        };

        let request = XrpcRequest::procedure("com.atproto.moderation.createReport")
            .json_body(&request_body)
            .map_err(|e| ReportError::ApiError(e.to_string()))?;

        let client = self.client.read().await;
        let response = client
            .procedure(request)
            .await
            .map_err(|e| ReportError::ApiError(e.to_string()))?;

        let report_response: CreateReportResponse =
            serde_json::from_value(response.data).map_err(ReportError::Serialization)?;

        Ok(ReportResult {
            id: report_response.id,
            created_at: report_response.created_at,
        })
    }

    // =========================================================================
    // Convenience Methods
    // =========================================================================

    /// Report an account
    ///
    /// # Arguments
    ///
    /// * `did` - DID of the account to report
    /// * `reason` - Reason for the report
    /// * `details` - Optional additional details
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::reporting::{ReportService, ReportReason};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = ReportService::new(client);
    /// let result = service.report_account(
    ///     "did:plc:impersonator123",
    ///     ReportReason::Misleading,
    ///     Some("Impersonating a public figure".to_string()),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn report_account(
        &self,
        did: &str,
        reason: ReportReason,
        details: Option<String>,
    ) -> Result<ReportResult> {
        self.validate_did(did)?;
        let subject = ReportSubject::Account(AccountSubject::new(did));
        self.create_report(reason, subject, details).await
    }

    /// Report a post
    ///
    /// # Arguments
    ///
    /// * `uri` - AT URI of the post
    /// * `cid` - CID of the post
    /// * `reason` - Reason for the report
    /// * `details` - Optional additional details
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::reporting::{ReportService, ReportReason};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = ReportService::new(client);
    /// let result = service.report_post(
    ///     "at://did:plc:abc/app.bsky.feed.post/xyz",
    ///     "bafyreigxyz...",
    ///     ReportReason::Sexual,
    ///     Some("Contains unlabeled adult content".to_string()),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn report_post(
        &self,
        uri: &str,
        cid: &str,
        reason: ReportReason,
        details: Option<String>,
    ) -> Result<ReportResult> {
        self.validate_uri(uri)?;
        self.validate_cid(cid)?;
        let subject = ReportSubject::Record(RecordSubject::new(uri, cid));
        self.create_report(reason, subject, details).await
    }

    /// Report a list or feed
    ///
    /// # Arguments
    ///
    /// * `uri` - AT URI of the list or feed
    /// * `cid` - CID of the list or feed
    /// * `reason` - Reason for the report
    /// * `details` - Optional additional details
    pub async fn report_list(
        &self,
        uri: &str,
        cid: &str,
        reason: ReportReason,
        details: Option<String>,
    ) -> Result<ReportResult> {
        self.validate_uri(uri)?;
        self.validate_cid(cid)?;
        let subject = ReportSubject::Record(RecordSubject::new(uri, cid));
        self.create_report(reason, subject, details).await
    }

    /// Report a chat message
    ///
    /// # Arguments
    ///
    /// * `message_id` - ID of the message
    /// * `convo_id` - ID of the conversation
    /// * `sender_did` - DID of the message sender
    /// * `reason` - Reason for the report
    /// * `details` - Optional additional details
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::reporting::{ReportService, ReportReason};
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = ReportService::new(client);
    /// let result = service.report_chat_message(
    ///     "msg123",
    ///     "convo456",
    ///     "did:plc:harasser",
    ///     ReportReason::Rude,
    ///     Some("Sending threatening messages".to_string()),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn report_chat_message(
        &self,
        message_id: &str,
        convo_id: &str,
        sender_did: &str,
        reason: ReportReason,
        details: Option<String>,
    ) -> Result<ReportResult> {
        self.validate_message_id(message_id)?;
        self.validate_convo_id(convo_id)?;
        self.validate_did(sender_did)?;

        let subject =
            ReportSubject::ChatMessage(ChatMessageSubject::new(message_id, convo_id, sender_did));
        self.create_report(reason, subject, details).await
    }

    /// Submit an appeal for a moderation decision
    ///
    /// Use this when appealing a moderation action taken against your account.
    ///
    /// # Arguments
    ///
    /// * `did` - Your DID (the account that was moderated)
    /// * `details` - Explanation of why the decision should be reconsidered
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use moderation::reporting::ReportService;
    /// # use atproto_client::xrpc::{XrpcClient, XrpcClientConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = XrpcClientConfig::new("https://bsky.social");
    /// # let client = XrpcClient::new(config);
    /// # let service = ReportService::new(client);
    /// let result = service.appeal(
    ///     "did:plc:myaccount",
    ///     "I believe my account was suspended in error because...".to_string(),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn appeal(&self, did: &str, details: String) -> Result<ReportResult> {
        self.validate_did(did)?;

        if details.trim().is_empty() {
            return Err(ReportError::InvalidReason(
                "Appeal must include details explaining why the decision should be reconsidered"
                    .to_string(),
            ));
        }

        let subject = ReportSubject::Account(AccountSubject::new(did));
        self.create_report(ReportReason::Appeal, subject, Some(details))
            .await
    }

    // =========================================================================
    // Validation Helpers
    // =========================================================================

    /// Validate a DID format
    fn validate_did(&self, did: &str) -> Result<()> {
        if did.is_empty() {
            return Err(ReportError::InvalidSubject("DID cannot be empty".to_string()));
        }
        if !did.starts_with("did:") {
            return Err(ReportError::InvalidSubject(format!(
                "DID must start with 'did:': {}",
                did
            )));
        }
        Ok(())
    }

    /// Validate an AT URI format
    fn validate_uri(&self, uri: &str) -> Result<()> {
        if uri.is_empty() {
            return Err(ReportError::InvalidSubject("URI cannot be empty".to_string()));
        }
        if !uri.starts_with("at://") {
            return Err(ReportError::InvalidSubject(format!(
                "URI must start with 'at://': {}",
                uri
            )));
        }
        Ok(())
    }

    /// Validate a CID format
    fn validate_cid(&self, cid: &str) -> Result<()> {
        if cid.is_empty() {
            return Err(ReportError::InvalidSubject("CID cannot be empty".to_string()));
        }
        Ok(())
    }

    /// Validate a message ID
    fn validate_message_id(&self, id: &str) -> Result<()> {
        if id.is_empty() {
            return Err(ReportError::InvalidSubject("Message ID cannot be empty".to_string()));
        }
        Ok(())
    }

    /// Validate a conversation ID
    fn validate_convo_id(&self, id: &str) -> Result<()> {
        if id.is_empty() {
            return Err(ReportError::InvalidSubject("Conversation ID cannot be empty".to_string()));
        }
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ReportReason Tests
    // =========================================================================

    #[test]
    fn test_report_reason_as_at_uri() {
        assert_eq!(ReportReason::Spam.as_at_uri(), "com.atproto.moderation.defs#reasonSpam");
        assert_eq!(
            ReportReason::Violation.as_at_uri(),
            "com.atproto.moderation.defs#reasonViolation"
        );
        assert_eq!(
            ReportReason::Misleading.as_at_uri(),
            "com.atproto.moderation.defs#reasonMisleading"
        );
        assert_eq!(ReportReason::Sexual.as_at_uri(), "com.atproto.moderation.defs#reasonSexual");
        assert_eq!(ReportReason::Rude.as_at_uri(), "com.atproto.moderation.defs#reasonRude");
        assert_eq!(ReportReason::Other.as_at_uri(), "com.atproto.moderation.defs#reasonOther");
        assert_eq!(ReportReason::Appeal.as_at_uri(), "com.atproto.moderation.defs#reasonAppeal");
    }

    #[test]
    fn test_report_reason_description() {
        assert!(!ReportReason::Spam.description().is_empty());
        assert!(!ReportReason::Violation.description().is_empty());
        assert!(!ReportReason::Misleading.description().is_empty());
        assert!(!ReportReason::Sexual.description().is_empty());
        assert!(!ReportReason::Rude.description().is_empty());
        assert!(!ReportReason::Other.description().is_empty());
        assert!(!ReportReason::Appeal.description().is_empty());
    }

    #[test]
    fn test_report_reason_display() {
        let reason = ReportReason::Spam;
        let display = format!("{}", reason);
        assert!(display.contains("Spam"));
    }

    #[test]
    fn test_report_reason_for_account() {
        let reasons = ReportReason::for_account();
        assert!(reasons.contains(&ReportReason::Spam));
        assert!(reasons.contains(&ReportReason::Misleading));
        assert!(reasons.contains(&ReportReason::Rude));
        assert!(!reasons.contains(&ReportReason::Sexual));
        assert!(!reasons.contains(&ReportReason::Appeal));
    }

    #[test]
    fn test_report_reason_for_post() {
        let reasons = ReportReason::for_post();
        assert!(reasons.contains(&ReportReason::Spam));
        assert!(reasons.contains(&ReportReason::Sexual));
        assert!(reasons.contains(&ReportReason::Misleading));
        assert!(!reasons.contains(&ReportReason::Appeal));
    }

    #[test]
    fn test_report_reason_for_list() {
        let reasons = ReportReason::for_list();
        assert!(reasons.contains(&ReportReason::Violation));
        assert!(reasons.contains(&ReportReason::Rude));
        assert!(reasons.contains(&ReportReason::Other));
        assert!(!reasons.contains(&ReportReason::Sexual));
        assert!(!reasons.contains(&ReportReason::Spam));
    }

    #[test]
    fn test_report_reason_for_chat() {
        let reasons = ReportReason::for_chat();
        assert!(reasons.contains(&ReportReason::Spam));
        assert!(reasons.contains(&ReportReason::Sexual));
        assert!(reasons.contains(&ReportReason::Rude));
        assert!(!reasons.contains(&ReportReason::Misleading));
    }

    #[test]
    fn test_report_reason_serialization() {
        let reason = ReportReason::Spam;
        let json = serde_json::to_string(&reason).unwrap();
        assert!(json.contains("spam"));

        let deserialized: ReportReason = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ReportReason::Spam);
    }

    #[test]
    fn test_report_reason_clone_copy() {
        let reason = ReportReason::Violation;
        let cloned = reason;
        assert_eq!(reason, cloned);
    }

    // =========================================================================
    // Subject Tests
    // =========================================================================

    #[test]
    fn test_account_subject_new() {
        let subject = AccountSubject::new("did:plc:test123");
        assert_eq!(subject.did(), "did:plc:test123");
        assert_eq!(subject.type_, "com.atproto.admin.defs#repoRef");
    }

    #[test]
    fn test_account_subject_serialization() {
        let subject = AccountSubject::new("did:plc:test123");
        let json = serde_json::to_string(&subject).unwrap();

        assert!(json.contains("\"$type\""));
        assert!(json.contains("com.atproto.admin.defs#repoRef"));
        assert!(json.contains("did:plc:test123"));

        let deserialized: AccountSubject = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.did, "did:plc:test123");
    }

    #[test]
    fn test_record_subject_new() {
        let subject = RecordSubject::new("at://did:plc:abc/app.bsky.feed.post/xyz", "bafyreigtest");
        assert_eq!(subject.uri(), "at://did:plc:abc/app.bsky.feed.post/xyz");
        assert_eq!(subject.cid(), "bafyreigtest");
        assert_eq!(subject.type_, "com.atproto.repo.strongRef");
    }

    #[test]
    fn test_record_subject_serialization() {
        let subject = RecordSubject::new("at://did:plc:abc/post/123", "bafycid");
        let json = serde_json::to_string(&subject).unwrap();

        assert!(json.contains("com.atproto.repo.strongRef"));
        assert!(json.contains("at://did:plc:abc/post/123"));
        assert!(json.contains("bafycid"));
    }

    #[test]
    fn test_chat_message_subject_new() {
        let subject = ChatMessageSubject::new("msg123", "convo456", "did:plc:sender");
        assert_eq!(subject.message_id, "msg123");
        assert_eq!(subject.convo_id, "convo456");
        assert_eq!(subject.did, "did:plc:sender");
        assert_eq!(subject.type_, "chat.bsky.convo.defs#messageRef");
    }

    #[test]
    fn test_chat_message_subject_serialization() {
        let subject = ChatMessageSubject::new("msg1", "convo1", "did:plc:sender1");
        let json = serde_json::to_string(&subject).unwrap();

        assert!(json.contains("chat.bsky.convo.defs#messageRef"));
        assert!(json.contains("messageId"));
        assert!(json.contains("convoId"));
    }

    #[test]
    fn test_report_subject_account_variant() {
        let subject = ReportSubject::Account(AccountSubject::new("did:plc:test"));
        let json = serde_json::to_string(&subject).unwrap();
        assert!(json.contains("com.atproto.admin.defs#repoRef"));
    }

    #[test]
    fn test_report_subject_record_variant() {
        let subject = ReportSubject::Record(RecordSubject::new("at://test", "cid"));
        let json = serde_json::to_string(&subject).unwrap();
        assert!(json.contains("com.atproto.repo.strongRef"));
    }

    #[test]
    fn test_report_subject_chat_variant() {
        let subject =
            ReportSubject::ChatMessage(ChatMessageSubject::new("msg", "convo", "did:plc:x"));
        let json = serde_json::to_string(&subject).unwrap();
        assert!(json.contains("chat.bsky.convo.defs#messageRef"));
    }

    // =========================================================================
    // Response Tests
    // =========================================================================

    #[test]
    fn test_create_report_response_deserialization() {
        let json = r#"{
            "id": 12345,
            "reasonType": "com.atproto.moderation.defs#reasonSpam",
            "subject": {"$type": "com.atproto.admin.defs#repoRef", "did": "did:plc:test"},
            "reportedBy": "did:plc:reporter",
            "createdAt": "2024-01-01T00:00:00Z"
        }"#;

        let response: CreateReportResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, 12345);
        assert_eq!(response.reason_type, "com.atproto.moderation.defs#reasonSpam");
        assert_eq!(response.reported_by, "did:plc:reporter");
        assert_eq!(response.created_at, "2024-01-01T00:00:00Z");
    }

    #[test]
    fn test_report_result_clone() {
        let result = ReportResult {
            id: 100,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    // =========================================================================
    // Error Tests
    // =========================================================================

    #[test]
    fn test_report_error_display() {
        let err = ReportError::InvalidSubject("bad did".to_string());
        assert!(err.to_string().contains("Invalid subject"));

        let err = ReportError::InvalidReason("bad reason".to_string());
        assert!(err.to_string().contains("Invalid reason"));

        let err = ReportError::NoSession;
        assert!(err.to_string().contains("session"));

        let err = ReportError::ApiError("network failed".to_string());
        assert!(err.to_string().contains("API error"));

        let err = ReportError::DetailsTooLong(3000, 2000);
        assert!(err.to_string().contains("3000"));
        assert!(err.to_string().contains("2000"));
    }

    #[test]
    fn test_report_error_debug() {
        let err = ReportError::InvalidSubject("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvalidSubject"));
    }

    #[test]
    fn test_report_error_from_serde() {
        let json_err = serde_json::from_str::<AccountSubject>("not json").unwrap_err();
        let report_err: ReportError = json_err.into();
        assert!(matches!(report_err, ReportError::Serialization(_)));
    }

    // =========================================================================
    // Service Validation Tests
    // =========================================================================

    #[test]
    fn test_service_validate_did_empty() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = ReportService::new(client);

        let result = service.validate_did("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_service_validate_did_invalid_prefix() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = ReportService::new(client);

        let result = service.validate_did("not-a-did");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("did:"));
    }

    #[test]
    fn test_service_validate_did_valid() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = ReportService::new(client);

        assert!(service.validate_did("did:plc:test123").is_ok());
        assert!(service.validate_did("did:web:example.com").is_ok());
    }

    #[test]
    fn test_service_validate_uri_empty() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = ReportService::new(client);

        let result = service.validate_uri("");
        assert!(result.is_err());
    }

    #[test]
    fn test_service_validate_uri_invalid_prefix() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = ReportService::new(client);

        let result = service.validate_uri("https://example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at://"));
    }

    #[test]
    fn test_service_validate_uri_valid() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = ReportService::new(client);

        assert!(service
            .validate_uri("at://did:plc:abc/app.bsky.feed.post/xyz")
            .is_ok());
    }

    #[test]
    fn test_service_validate_cid_empty() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = ReportService::new(client);

        assert!(service.validate_cid("").is_err());
        assert!(service.validate_cid("bafytest").is_ok());
    }

    #[test]
    fn test_service_validate_message_id_empty() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = ReportService::new(client);

        assert!(service.validate_message_id("").is_err());
        assert!(service.validate_message_id("msg123").is_ok());
    }

    #[test]
    fn test_service_validate_convo_id_empty() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let service = ReportService::new(client);

        assert!(service.validate_convo_id("").is_err());
        assert!(service.validate_convo_id("convo123").is_ok());
    }

    // =========================================================================
    // Service Creation Tests
    // =========================================================================

    #[test]
    fn test_report_service_new() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let _service = ReportService::new(client);
    }

    #[test]
    fn test_report_service_with_shared_client() {
        use atproto_client::xrpc::XrpcClientConfig;

        let config = XrpcClientConfig::new("https://bsky.social");
        let client = XrpcClient::new(config);
        let shared = Arc::new(RwLock::new(client));
        let _service = ReportService::with_shared_client(shared);
    }

    // =========================================================================
    // Request Serialization Tests
    // =========================================================================

    #[test]
    fn test_create_report_request_serialization() {
        let request = CreateReportRequest {
            reason_type: ReportReason::Spam.as_at_uri().to_string(),
            subject: ReportSubject::Account(AccountSubject::new("did:plc:test")),
            reason: Some("Test details".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("reasonType"));
        assert!(json.contains("com.atproto.moderation.defs#reasonSpam"));
        assert!(json.contains("subject"));
        assert!(json.contains("reason"));
        assert!(json.contains("Test details"));
    }

    #[test]
    fn test_create_report_request_without_details() {
        let request = CreateReportRequest {
            reason_type: ReportReason::Violation.as_at_uri().to_string(),
            subject: ReportSubject::Record(RecordSubject::new("at://test", "cid")),
            reason: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        // reason should be omitted when None
        assert!(!json.contains("\"reason\""));
    }

    // =========================================================================
    // Constant Tests
    // =========================================================================

    #[test]
    fn test_max_details_length() {
        assert_eq!(MAX_DETAILS_LENGTH, 2000);
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_subject_equality() {
        let a = AccountSubject::new("did:plc:test");
        let b = AccountSubject::new("did:plc:test");
        assert_eq!(a, b);

        let c = AccountSubject::new("did:plc:other");
        assert_ne!(a, c);
    }

    #[test]
    fn test_record_subject_equality() {
        let a = RecordSubject::new("at://test", "cid1");
        let b = RecordSubject::new("at://test", "cid1");
        assert_eq!(a, b);

        let c = RecordSubject::new("at://test", "cid2");
        assert_ne!(a, c);
    }

    #[test]
    fn test_report_reason_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ReportReason::Spam);
        set.insert(ReportReason::Violation);
        set.insert(ReportReason::Spam); // duplicate

        assert_eq!(set.len(), 2);
    }
}
