//! AT Protocol Session Management
//!
//! This module implements session management for AT Protocol, including:
//! - JWT parsing and validation
//! - Session state management
//! - Token refresh flows
//! - Session persistence
//! - Multi-account support
//!
//! # Example
//!
//! ```rust
//! use atproto_client::session::{SessionAccount, AtpSessionData, is_session_expired};
//!
//! // Create a session account
//! let account = SessionAccount {
//!     service: "https://bsky.social".to_string(),
//!     did: "did:plc:abc123".to_string(),
//!     handle: "alice.bsky.social".to_string(),
//!     email: Some("alice@example.com".to_string()),
//!     email_confirmed: Some(true),
//!     email_auth_factor: Some(false),
//!     refresh_jwt: Some("refresh_token".to_string()),
//!     access_jwt: Some("access_token".to_string()),
//!     signup_queued: Some(false),
//!     active: Some(true),
//!     status: None,
//!     pds_url: None,
//!     is_self_hosted: Some(false),
//!     app_view_url: Some("https://api.bsky.app".to_string()),
//! };
//!
//! // Check if session is expired
//! let expired = is_session_expired(&account);
//! ```

mod manager;

pub use manager::{SessionManager, SessionManagerError, SessionStorage};

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during session operations
#[derive(Debug, Error)]
pub enum SessionError {
    /// JWT parsing error
    #[error("JWT parsing error: {0}")]
    JwtParseError(String),

    /// JWT validation error
    #[error("JWT validation error: {0}")]
    JwtValidationError(#[from] jsonwebtoken::errors::Error),

    /// Token expired
    #[error("Token expired at {0}")]
    TokenExpired(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid session state
    #[error("Invalid session state: {0}")]
    InvalidState(String),
}

/// Result type for session operations
pub type Result<T> = std::result::Result<T, SessionError>;

/// A persisted account with authentication tokens
///
/// This structure matches the TypeScript `PersistedAccount` type from the
/// original Bluesky app and contains all the information needed to restore
/// a user's session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionAccount {
    /// The PDS service URL (e.g., "https://bsky.social")
    pub service: String,

    /// The user's DID (Decentralized Identifier)
    pub did: String,

    /// The user's handle (e.g., "alice.bsky.social")
    pub handle: String,

    /// The user's email address (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Whether the email has been confirmed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_confirmed: Option<bool>,

    /// Whether email is used as an auth factor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_auth_factor: Option<bool>,

    /// Refresh JWT token (can expire)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_jwt: Option<String>,

    /// Access JWT token (can expire)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_jwt: Option<String>,

    /// Whether the account signup is queued
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signup_queued: Option<bool>,

    /// Whether the session is active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,

    /// Account status (e.g., "takendown", "suspended", "deactivated")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Custom PDS URL if different from service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pds_url: Option<String>,

    /// Whether this is a self-hosted PDS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_self_hosted: Option<bool>,

    /// Custom AppView URL for read operations (if different from service)
    /// This is a key differentiator allowing users to choose their AppView provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_view_url: Option<String>,
}

impl SessionAccount {
    /// Create a new session account with required fields
    pub fn new(service: String, did: String, handle: String) -> Self {
        Self {
            service,
            did,
            handle,
            email: None,
            email_confirmed: None,
            email_auth_factor: None,
            refresh_jwt: None,
            access_jwt: None,
            signup_queued: None,
            active: Some(true),
            status: None,
            pds_url: None,
            is_self_hosted: None,
            app_view_url: None,
        }
    }

    /// Convert to ATP session data
    pub fn to_session_data(&self) -> Result<AtpSessionData> {
        Ok(AtpSessionData {
            access_jwt: self.access_jwt.clone().unwrap_or_default(),
            did: self.did.clone(),
            email: self.email.clone(),
            email_auth_factor: self.email_auth_factor,
            email_confirmed: self.email_confirmed,
            handle: self.handle.clone(),
            refresh_jwt: self.refresh_jwt.clone().unwrap_or_default(),
            active: self.active.unwrap_or(true),
            status: self.status.clone(),
        })
    }

    /// Check if this session has valid tokens
    pub fn has_tokens(&self) -> bool {
        self.access_jwt.is_some() && self.refresh_jwt.is_some()
    }
}

/// Active session data used by the BskyAgent
///
/// This structure matches the TypeScript `AtpSessionData` type and contains
/// the active session information needed for making authenticated requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtpSessionData {
    /// Access JWT token for authenticated requests
    pub access_jwt: String,

    /// The user's DID
    pub did: String,

    /// The user's email address (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Whether email is used as an auth factor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_auth_factor: Option<bool>,

    /// Whether the email has been confirmed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_confirmed: Option<bool>,

    /// The user's handle
    pub handle: String,

    /// Refresh JWT token for getting new access tokens
    pub refresh_jwt: String,

    /// Whether the session is active
    #[serde(default = "default_active")]
    pub active: bool,

    /// Account status (e.g., "takendown", "suspended", "deactivated")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

fn default_active() -> bool {
    true
}

impl AtpSessionData {
    /// Convert to session account
    pub fn to_session_account(&self, service: String) -> SessionAccount {
        SessionAccount {
            service,
            did: self.did.clone(),
            handle: self.handle.clone(),
            email: self.email.clone(),
            email_confirmed: self.email_confirmed,
            email_auth_factor: self.email_auth_factor,
            refresh_jwt: Some(self.refresh_jwt.clone()),
            access_jwt: Some(self.access_jwt.clone()),
            signup_queued: None,
            active: Some(self.active),
            status: self.status.clone(),
            pds_url: None,
            is_self_hosted: None,
            app_view_url: None,
        }
    }
}

/// JWT claims structure
///
/// This represents the decoded payload of a JWT token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (DID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,

    /// Issued at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<i64>,

    /// Expiration timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,

    /// Scope (e.g., "com.atproto.access" for access tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Additional claims
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Parse JWT claims without validation
///
/// This is useful for extracting expiration time and other claims from a JWT
/// without verifying the signature. Should only be used for informational purposes.
///
/// # Arguments
///
/// * `token` - The JWT token string
///
/// # Example
///
/// ```rust
/// use atproto_client::session::parse_jwt_claims;
///
/// let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJkaWQ6cGxjOmFiYzEyMyIsImV4cCI6MTcwMDAwMDAwMH0.signature";
/// match parse_jwt_claims(token) {
///     Ok(claims) => println!("DID: {:?}", claims.sub),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn parse_jwt_claims(token: &str) -> Result<JwtClaims> {
    // Parse header to get algorithm
    let header = decode_header(token)?;

    // Create a validation that doesn't check signature
    let mut validation = Validation::new(header.alg);
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    validation.validate_nbf = false;

    // Decode without verification
    let token_data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(&[]), // Dummy key since we're not validating
        &validation,
    )?;

    Ok(token_data.claims)
}

/// Get the expiration time from a JWT token
///
/// Returns None if the token doesn't have an expiration claim or if parsing fails.
///
/// # Arguments
///
/// * `token` - The JWT token string
///
/// # Example
///
/// ```rust
/// use atproto_client::session::get_jwt_expiration;
///
/// let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
/// if let Some(exp_time) = get_jwt_expiration(token) {
///     println!("Token expires at: {}", exp_time);
/// }
/// ```
pub fn get_jwt_expiration(token: &str) -> Option<DateTime<Utc>> {
    let claims = parse_jwt_claims(token).ok()?;
    claims.exp.and_then(|exp| DateTime::from_timestamp(exp, 0))
}

/// Check if a JWT token is expired
///
/// A token is considered expired if:
/// - It doesn't have an expiration claim (returns true for safety)
/// - The expiration time is in the past
///
/// # Arguments
///
/// * `token` - The JWT token string
///
/// # Example
///
/// ```rust
/// use atproto_client::session::is_jwt_expired;
///
/// let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
/// if is_jwt_expired(token) {
///     println!("Token is expired, need to refresh");
/// }
/// ```
pub fn is_jwt_expired(token: &str) -> bool {
    match get_jwt_expiration(token) {
        Some(exp_time) => exp_time <= Utc::now(),
        None => true, // If we can't get expiration, consider it expired for safety
    }
}

/// Check if a JWT token will expire soon (within the given duration)
///
/// # Arguments
///
/// * `token` - The JWT token string
/// * `threshold` - Duration before expiration to consider "soon"
///
/// # Example
///
/// ```rust
/// use atproto_client::session::is_jwt_expiring_soon;
/// use chrono::Duration;
///
/// let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
/// // Check if token expires within the next 5 minutes
/// if is_jwt_expiring_soon(token, Duration::minutes(5)) {
///     println!("Token expires soon, should refresh proactively");
/// }
/// ```
pub fn is_jwt_expiring_soon(token: &str, threshold: Duration) -> bool {
    match get_jwt_expiration(token) {
        Some(exp_time) => exp_time <= Utc::now() + threshold,
        None => true,
    }
}

/// Check if a session account is expired
///
/// A session is considered expired if:
/// - It doesn't have an access token
/// - The access token is expired
/// - The refresh token is expired (if access token is also expired)
///
/// # Arguments
///
/// * `account` - The session account to check
///
/// # Example
///
/// ```rust
/// use atproto_client::session::{SessionAccount, is_session_expired};
///
/// let account = SessionAccount::new(
///     "https://bsky.social".to_string(),
///     "did:plc:abc123".to_string(),
///     "alice.bsky.social".to_string(),
/// );
///
/// if is_session_expired(&account) {
///     println!("Session expired, user needs to log in again");
/// }
/// ```
pub fn is_session_expired(account: &SessionAccount) -> bool {
    // No access token means expired
    let Some(ref access_token) = account.access_jwt else {
        return true;
    };

    // If access token is not expired, session is valid
    if !is_jwt_expired(access_token) {
        return false;
    }

    // Access token is expired, check refresh token
    match &account.refresh_jwt {
        Some(refresh_token) => is_jwt_expired(refresh_token),
        None => true, // No refresh token means we can't refresh
    }
}

/// Check if an account signup is queued
///
/// This checks if the access JWT has the signup queue scope, which indicates
/// the account is in a signup queue and hasn't been fully activated yet.
///
/// # Arguments
///
/// * `access_jwt` - The access JWT token
///
/// # Example
///
/// ```rust
/// use atproto_client::session::is_signup_queued;
///
/// let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
/// if is_signup_queued(token) {
///     println!("Account is in signup queue");
/// }
/// ```
pub fn is_signup_queued(access_jwt: &str) -> bool {
    parse_jwt_claims(access_jwt)
        .ok()
        .and_then(|claims| claims.scope)
        .map(|scope| scope.contains("signup_queue"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_account_new() {
        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        assert_eq!(account.service, "https://bsky.social");
        assert_eq!(account.did, "did:plc:abc123");
        assert_eq!(account.handle, "alice.bsky.social");
        assert_eq!(account.active, Some(true));
        assert!(account.access_jwt.is_none());
        assert!(account.refresh_jwt.is_none());
    }

    #[test]
    fn test_session_account_has_tokens() {
        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        assert!(!account.has_tokens());

        account.access_jwt = Some("access".to_string());
        assert!(!account.has_tokens());

        account.refresh_jwt = Some("refresh".to_string());
        assert!(account.has_tokens());
    }

    #[test]
    fn test_session_account_to_session_data() {
        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );
        account.access_jwt = Some("access_token".to_string());
        account.refresh_jwt = Some("refresh_token".to_string());
        account.email = Some("alice@example.com".to_string());

        let session_data = account.to_session_data().unwrap();

        assert_eq!(session_data.access_jwt, "access_token");
        assert_eq!(session_data.refresh_jwt, "refresh_token");
        assert_eq!(session_data.did, "did:plc:abc123");
        assert_eq!(session_data.handle, "alice.bsky.social");
        assert_eq!(session_data.email, Some("alice@example.com".to_string()));
        assert!(session_data.active);
    }

    #[test]
    fn test_atp_session_data_to_session_account() {
        let session_data = AtpSessionData {
            access_jwt: "access_token".to_string(),
            did: "did:plc:abc123".to_string(),
            email: Some("alice@example.com".to_string()),
            email_auth_factor: Some(false),
            email_confirmed: Some(true),
            handle: "alice.bsky.social".to_string(),
            refresh_jwt: "refresh_token".to_string(),
            active: true,
            status: None,
        };

        let account = session_data.to_session_account("https://bsky.social".to_string());

        assert_eq!(account.service, "https://bsky.social");
        assert_eq!(account.access_jwt, Some("access_token".to_string()));
        assert_eq!(account.refresh_jwt, Some("refresh_token".to_string()));
        assert_eq!(account.did, "did:plc:abc123");
        assert_eq!(account.handle, "alice.bsky.social");
        assert_eq!(account.email, Some("alice@example.com".to_string()));
    }

    #[test]
    fn test_session_account_serialization() {
        let account = SessionAccount {
            service: "https://bsky.social".to_string(),
            did: "did:plc:abc123".to_string(),
            handle: "alice.bsky.social".to_string(),
            email: Some("alice@example.com".to_string()),
            email_confirmed: Some(true),
            email_auth_factor: Some(false),
            refresh_jwt: Some("refresh".to_string()),
            access_jwt: Some("access".to_string()),
            signup_queued: Some(false),
            active: Some(true),
            status: None,
            pds_url: None,
            is_self_hosted: Some(false),
            app_view_url: None,
        };

        let json = serde_json::to_string(&account).unwrap();
        let deserialized: SessionAccount = serde_json::from_str(&json).unwrap();

        assert_eq!(account, deserialized);
    }

    #[test]
    fn test_is_session_expired_no_tokens() {
        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        assert!(is_session_expired(&account));
    }

    #[test]
    fn test_parse_jwt_claims() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        // Create a test JWT token
        let claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::hours(1)).timestamp()),
            scope: Some("com.atproto.access".to_string()),
            extra: serde_json::json!({}),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        // Parse the claims
        let parsed = parse_jwt_claims(&token).unwrap();

        assert_eq!(parsed.sub, Some("did:plc:test123".to_string()));
        assert_eq!(parsed.scope, Some("com.atproto.access".to_string()));
        assert!(parsed.exp.is_some());
        assert!(parsed.iat.is_some());
    }

    #[test]
    fn test_get_jwt_expiration() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let exp_time = Utc::now() + Duration::hours(2);
        let claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some(exp_time.timestamp()),
            scope: None,
            extra: serde_json::json!({}),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        let parsed_exp = get_jwt_expiration(&token).unwrap();

        // Allow 1 second difference for test execution time
        let diff = (parsed_exp.timestamp() - exp_time.timestamp()).abs();
        assert!(diff <= 1, "Expiration time should match within 1 second");
    }

    #[test]
    fn test_is_jwt_expired_with_valid_token() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        // Create a token that expires in 1 hour
        let claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::hours(1)).timestamp()),
            scope: None,
            extra: serde_json::json!({}),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        // Should not be expired
        assert!(!is_jwt_expired(&token));
    }

    #[test]
    fn test_is_jwt_expired_with_expired_token() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        // Create a token that expired 1 hour ago
        let claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some((Utc::now() - Duration::hours(2)).timestamp()),
            exp: Some((Utc::now() - Duration::hours(1)).timestamp()),
            scope: None,
            extra: serde_json::json!({}),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        // Should be expired
        assert!(is_jwt_expired(&token));
    }

    #[test]
    fn test_is_jwt_expiring_soon() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        // Create a token that expires in 3 minutes
        let claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::minutes(3)).timestamp()),
            scope: None,
            extra: serde_json::json!({}),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        // Should be expiring soon (within 5 minutes)
        assert!(is_jwt_expiring_soon(&token, Duration::minutes(5)));

        // Should not be expiring soon (within 2 minutes)
        assert!(!is_jwt_expiring_soon(&token, Duration::minutes(2)));
    }

    #[test]
    fn test_is_signup_queued_true() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::hours(1)).timestamp()),
            scope: Some("signup_queue".to_string()),
            extra: serde_json::json!({}),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        assert!(is_signup_queued(&token));
    }

    #[test]
    fn test_is_signup_queued_false() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::hours(1)).timestamp()),
            scope: Some("com.atproto.access".to_string()),
            extra: serde_json::json!({}),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        assert!(!is_signup_queued(&token));
    }

    #[test]
    fn test_is_session_expired_with_valid_tokens() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let access_claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::hours(1)).timestamp()),
            scope: Some("com.atproto.access".to_string()),
            extra: serde_json::json!({}),
        };

        let refresh_claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::days(30)).timestamp()),
            scope: Some("com.atproto.refresh".to_string()),
            extra: serde_json::json!({}),
        };

        let access_token = encode(
            &Header::new(Algorithm::HS256),
            &access_claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        let refresh_token = encode(
            &Header::new(Algorithm::HS256),
            &refresh_claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "alice.bsky.social".to_string(),
        );
        account.access_jwt = Some(access_token);
        account.refresh_jwt = Some(refresh_token);

        assert!(!is_session_expired(&account));
    }

    #[test]
    fn test_is_session_expired_with_expired_access_valid_refresh() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        // Expired access token
        let access_claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some((Utc::now() - Duration::hours(2)).timestamp()),
            exp: Some((Utc::now() - Duration::hours(1)).timestamp()),
            scope: Some("com.atproto.access".to_string()),
            extra: serde_json::json!({}),
        };

        // Valid refresh token
        let refresh_claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::days(30)).timestamp()),
            scope: Some("com.atproto.refresh".to_string()),
            extra: serde_json::json!({}),
        };

        let access_token = encode(
            &Header::new(Algorithm::HS256),
            &access_claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        let refresh_token = encode(
            &Header::new(Algorithm::HS256),
            &refresh_claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "alice.bsky.social".to_string(),
        );
        account.access_jwt = Some(access_token);
        account.refresh_jwt = Some(refresh_token);

        // Session should not be expired because refresh token is still valid
        assert!(!is_session_expired(&account));
    }

    #[test]
    fn test_is_session_expired_with_both_tokens_expired() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        // Expired access token
        let access_claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some((Utc::now() - Duration::hours(2)).timestamp()),
            exp: Some((Utc::now() - Duration::hours(1)).timestamp()),
            scope: Some("com.atproto.access".to_string()),
            extra: serde_json::json!({}),
        };

        // Expired refresh token
        let refresh_claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some((Utc::now() - Duration::days(31)).timestamp()),
            exp: Some((Utc::now() - Duration::days(1)).timestamp()),
            scope: Some("com.atproto.refresh".to_string()),
            extra: serde_json::json!({}),
        };

        let access_token = encode(
            &Header::new(Algorithm::HS256),
            &access_claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        let refresh_token = encode(
            &Header::new(Algorithm::HS256),
            &refresh_claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "alice.bsky.social".to_string(),
        );
        account.access_jwt = Some(access_token);
        account.refresh_jwt = Some(refresh_token);

        // Session should be expired
        assert!(is_session_expired(&account));
    }

    #[test]
    fn test_round_trip_conversion() {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

        let access_claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::hours(1)).timestamp()),
            scope: Some("com.atproto.access".to_string()),
            extra: serde_json::json!({}),
        };

        let refresh_claims = JwtClaims {
            sub: Some("did:plc:test123".to_string()),
            iat: Some(Utc::now().timestamp()),
            exp: Some((Utc::now() + Duration::days(30)).timestamp()),
            scope: Some("com.atproto.refresh".to_string()),
            extra: serde_json::json!({}),
        };

        let access_token = encode(
            &Header::new(Algorithm::HS256),
            &access_claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        let refresh_token = encode(
            &Header::new(Algorithm::HS256),
            &refresh_claims,
            &EncodingKey::from_secret(b"test_secret"),
        )
        .unwrap();

        // Create session account
        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "alice.bsky.social".to_string(),
        );
        account.access_jwt = Some(access_token.clone());
        account.refresh_jwt = Some(refresh_token.clone());
        account.email = Some("test@example.com".to_string());

        // Convert to session data
        let session_data = account.to_session_data().unwrap();

        // Convert back to session account
        let round_trip_account = session_data.to_session_account("https://bsky.social".to_string());

        // Verify key fields match
        assert_eq!(account.did, round_trip_account.did);
        assert_eq!(account.handle, round_trip_account.handle);
        assert_eq!(account.email, round_trip_account.email);
        assert_eq!(account.access_jwt, round_trip_account.access_jwt);
        assert_eq!(account.refresh_jwt, round_trip_account.refresh_jwt);
    }

    #[test]
    fn test_session_account_with_app_view_url() {
        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        // Initially no AppView URL
        assert_eq!(account.app_view_url, None);

        // Set custom AppView URL
        account.app_view_url = Some("https://api.bsky.app".to_string());
        assert_eq!(
            account.app_view_url,
            Some("https://api.bsky.app".to_string())
        );
    }

    #[test]
    fn test_session_account_serialization_with_app_view() {
        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );
        account.app_view_url = Some("https://custom.appview.social".to_string());
        account.access_jwt = Some("access_token".to_string());
        account.refresh_jwt = Some("refresh_token".to_string());

        // Serialize
        let json = serde_json::to_string(&account).unwrap();

        // Verify app_view_url is in JSON
        assert!(json.contains("custom.appview.social"));
        assert!(json.contains("appViewUrl"));

        // Deserialize
        let deserialized: SessionAccount = serde_json::from_str(&json).unwrap();

        // Verify fields match
        assert_eq!(
            deserialized.app_view_url,
            Some("https://custom.appview.social".to_string())
        );
        assert_eq!(account, deserialized);
    }

    #[test]
    fn test_session_account_serialization_without_app_view() {
        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        // Serialize
        let json = serde_json::to_string(&account).unwrap();

        // Verify appViewUrl is NOT in JSON when None (skip_serializing_if)
        assert!(!json.contains("appViewUrl"));

        // Deserialize
        let deserialized: SessionAccount = serde_json::from_str(&json).unwrap();

        // Verify fields match
        assert_eq!(deserialized.app_view_url, None);
        assert_eq!(account, deserialized);
    }
}
