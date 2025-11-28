//! BskyAgent - Main client for Bluesky/AT Protocol
//!
//! This module provides the high-level BskyAgent client for interacting with
//! AT Protocol services. The agent manages sessions, handles authentication,
//! and provides convenient methods for common API operations.
//!
//! # Example
//!
//! ```rust,no_run
//! use atproto_client::BskyAgent;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a new agent
//!     let mut agent = BskyAgent::new("https://bsky.social")?;
//!
//!     // Login
//!     agent.login("alice.bsky.social", "password").await?;
//!
//!     // Agent is now authenticated and ready to use
//!     println!("Logged in as: {}", agent.did().unwrap());
//!
//!     Ok(())
//! }
//! ```

use crate::session::{AtpSessionData, SessionError};
use crate::xrpc::{XrpcClient, XrpcClientConfig, XrpcError};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Errors that can occur during agent operations
#[derive(Debug, Error)]
pub enum AgentError {
    /// Session error
    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    /// XRPC error
    #[error("XRPC error: {0}")]
    Xrpc(#[from] XrpcError),

    /// No active session
    #[error("No active session - please login first")]
    NoSession,

    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Service error
    #[error("Service error: {0}")]
    Service(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Result type for agent operations
pub type Result<T> = std::result::Result<T, AgentError>;

/// Login request parameters
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    /// User identifier (handle or email)
    pub identifier: String,
    /// User password
    pub password: String,
    /// Optional auth factor token for 2FA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_factor_token: Option<String>,
}

/// Login response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    /// Access JWT token
    pub access_jwt: String,
    /// Refresh JWT token
    pub refresh_jwt: String,
    /// User DID
    pub did: String,
    /// User handle
    pub handle: String,
    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Email confirmed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_confirmed: Option<bool>,
    /// Email auth factor enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_auth_factor: Option<bool>,
    /// Session active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// Account status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did_doc: Option<serde_json::Value>,
}

/// Create account request parameters
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    /// Email address
    pub email: String,
    /// Password
    pub password: String,
    /// Handle
    pub handle: String,
    /// Invite code (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite_code: Option<String>,
    /// Verification phone (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_phone: Option<String>,
    /// Verification code (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_code: Option<String>,
}

/// Create account response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountResponse {
    /// Access JWT token
    pub access_jwt: String,
    /// Refresh JWT token
    pub refresh_jwt: String,
    /// User DID
    pub did: String,
    /// User handle
    pub handle: String,
    /// DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did_doc: Option<serde_json::Value>,
}

/// Refresh session response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshSessionResponse {
    /// New access JWT token
    pub access_jwt: String,
    /// New refresh JWT token
    pub refresh_jwt: String,
    /// User DID
    pub did: String,
    /// User handle
    pub handle: String,
    /// DID document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub did_doc: Option<serde_json::Value>,
    /// Session active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// Account status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Session event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    /// Session created (login or create account)
    Create,
    /// Session updated (token refresh)
    Update,
    /// Session expired
    Expired,
    /// Network error during session operation
    NetworkError,
}

/// Callback function type for session events
pub type SessionCallback = Arc<dyn Fn(SessionEvent, &AtpSessionData) + Send + Sync>;

/// Configuration for BskyAgent
#[derive(Debug, Clone)]
pub struct BskyAgentConfig {
    /// Service URL (e.g., "https://bsky.social")
    pub service: String,
    /// Optional custom AppView URL
    pub app_view: Option<String>,
    /// Optional custom PDS URL for writes
    pub pds_url: Option<String>,
    /// XRPC client configuration
    pub xrpc_config: XrpcClientConfig,
}

impl BskyAgentConfig {
    /// Create a new agent configuration
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            app_view: None,
            pds_url: None,
            xrpc_config: XrpcClientConfig::default(),
        }
    }

    /// Set custom AppView URL
    pub fn with_app_view(mut self, app_view: impl Into<String>) -> Self {
        self.app_view = Some(app_view.into());
        self
    }

    /// Set custom PDS URL
    pub fn with_pds_url(mut self, pds_url: impl Into<String>) -> Self {
        self.pds_url = Some(pds_url.into());
        self
    }

    /// Set XRPC client configuration
    pub fn with_xrpc_config(mut self, config: XrpcClientConfig) -> Self {
        self.xrpc_config = config;
        self
    }
}

/// Main agent for interacting with AT Protocol services
///
/// BskyAgent provides a high-level interface for authenticating and making
/// requests to AT Protocol services. It manages sessions, handles token
/// refresh, and provides convenient methods for common operations.
///
/// # Example
///
/// ```rust,no_run
/// use atproto_client::BskyAgent;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut agent = BskyAgent::new("https://bsky.social")?;
///
///     // Login
///     agent.login("alice.bsky.social", "password").await?;
///
///     // Now you can make authenticated requests
///     // ...
///
///     Ok(())
/// }
/// ```
pub struct BskyAgent {
    /// Service URL (PDS)
    service: String,
    /// Optional custom AppView URL (for reads)
    app_view: Option<String>,
    /// Optional custom PDS URL (for writes)
    pds_url: Option<String>,
    /// XRPC client for reads (uses AppView if configured)
    read_client: XrpcClient,
    /// XRPC client for writes (uses PDS)
    write_client: XrpcClient,
    /// Current session data
    session: Arc<RwLock<Option<AtpSessionData>>>,
    /// Session event callback
    session_callback: Option<SessionCallback>,
}

impl BskyAgent {
    /// Create a new BskyAgent with default configuration
    ///
    /// # Arguments
    ///
    /// * `service` - The PDS service URL (e.g., "https://bsky.social")
    ///
    /// # Example
    ///
    /// ```rust
    /// use atproto_client::BskyAgent;
    ///
    /// let agent = BskyAgent::new("https://bsky.social").unwrap();
    /// ```
    pub fn new(service: impl Into<String>) -> Result<Self> {
        let service = service.into();
        let config = BskyAgentConfig::new(service.clone());
        Self::with_config(config)
    }

    /// Create a new BskyAgent with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Agent configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use atproto_client::{BskyAgent, BskyAgentConfig};
    ///
    /// let config = BskyAgentConfig::new("https://bsky.social")
    ///     .with_app_view("https://api.bsky.app");
    /// let agent = BskyAgent::with_config(config).unwrap();
    /// ```
    pub fn with_config(config: BskyAgentConfig) -> Result<Self> {
        // Use AppView for reads if configured, otherwise use service
        let read_url = config.app_view.as_ref().unwrap_or(&config.service);
        let mut read_xrpc_config = config.xrpc_config.clone();
        read_xrpc_config.service_url = read_url.clone();
        let read_client = XrpcClient::new(read_xrpc_config);

        // Always use the original service (PDS) for writes
        let write_url = config.pds_url.as_ref().unwrap_or(&config.service);
        let mut write_xrpc_config = config.xrpc_config;
        write_xrpc_config.service_url = write_url.clone();
        let write_client = XrpcClient::new(write_xrpc_config);

        Ok(Self {
            service: config.service,
            app_view: config.app_view,
            pds_url: config.pds_url,
            read_client,
            write_client,
            session: Arc::new(RwLock::new(None)),
            session_callback: None,
        })
    }

    /// Set a callback for session events
    ///
    /// The callback will be invoked when session events occur (create, update, expire, etc.)
    ///
    /// # Arguments
    ///
    /// * `callback` - Function to call on session events
    pub fn set_session_callback<F>(&mut self, callback: F)
    where
        F: Fn(SessionEvent, &AtpSessionData) + Send + Sync + 'static,
    {
        self.session_callback = Some(Arc::new(callback));
    }

    /// Login to the service
    ///
    /// # Arguments
    ///
    /// * `identifier` - User handle or email
    /// * `password` - User password
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::BskyAgent;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut agent = BskyAgent::new("https://bsky.social")?;
    /// agent.login("alice.bsky.social", "password").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn login(
        &mut self,
        identifier: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<AtpSessionData> {
        self.login_with_token(identifier, password, None).await
    }

    /// Login with 2FA auth token
    ///
    /// # Arguments
    ///
    /// * `identifier` - User handle or email
    /// * `password` - User password
    /// * `auth_token` - 2FA authentication token
    pub async fn login_with_token(
        &mut self,
        identifier: impl Into<String>,
        password: impl Into<String>,
        auth_token: Option<String>,
    ) -> Result<AtpSessionData> {
        let request = LoginRequest {
            identifier: identifier.into(),
            password: password.into(),
            auth_factor_token: auth_token,
        };

        use crate::xrpc::XrpcRequest;

        let xrpc_request = XrpcRequest::procedure("com.atproto.server.createSession")
            .json_body(&request)
            .map_err(|e| AgentError::Service(e.to_string()))?;

        let response: LoginResponse = self
            .write_client
            .procedure(xrpc_request)
            .await
            .map(|r| r.data)?;

        let session_data = AtpSessionData {
            access_jwt: response.access_jwt,
            refresh_jwt: response.refresh_jwt,
            did: response.did,
            handle: response.handle,
            email: response.email,
            email_confirmed: response.email_confirmed,
            email_auth_factor: response.email_auth_factor,
            active: response.active.unwrap_or(true),
            status: response.status,
        };

        // Update session
        {
            let mut session = self.session.write().unwrap();
            *session = Some(session_data.clone());
        }

        // Update client auth headers
        self.update_auth_headers(&session_data.access_jwt);

        // Fire session callback
        if let Some(ref callback) = self.session_callback {
            callback(SessionEvent::Create, &session_data);
        }

        Ok(session_data)
    }

    /// Create a new account
    ///
    /// # Arguments
    ///
    /// * `email` - Email address
    /// * `password` - Password
    /// * `handle` - Desired handle
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::BskyAgent;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut agent = BskyAgent::new("https://bsky.social")?;
    /// agent.create_account("alice@example.com", "password", "alice.bsky.social").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_account(
        &mut self,
        email: impl Into<String>,
        password: impl Into<String>,
        handle: impl Into<String>,
    ) -> Result<AtpSessionData> {
        let request = CreateAccountRequest {
            email: email.into(),
            password: password.into(),
            handle: handle.into(),
            invite_code: None,
            verification_phone: None,
            verification_code: None,
        };

        self.create_account_with_request(request).await
    }

    /// Create a new account with full request parameters
    ///
    /// # Arguments
    ///
    /// * `request` - Create account request with all parameters
    pub async fn create_account_with_request(
        &mut self,
        request: CreateAccountRequest,
    ) -> Result<AtpSessionData> {
        use crate::xrpc::XrpcRequest;

        let xrpc_request = XrpcRequest::procedure("com.atproto.server.createAccount")
            .json_body(&request)
            .map_err(|e| AgentError::Service(e.to_string()))?;

        let response: CreateAccountResponse = self
            .write_client
            .procedure(xrpc_request)
            .await
            .map(|r| r.data)?;

        let session_data = AtpSessionData {
            access_jwt: response.access_jwt,
            refresh_jwt: response.refresh_jwt,
            did: response.did,
            handle: response.handle,
            email: None,
            email_confirmed: None,
            email_auth_factor: None,
            active: true,
            status: None,
        };

        // Update session
        {
            let mut session = self.session.write().unwrap();
            *session = Some(session_data.clone());
        }

        // Update client auth headers
        self.update_auth_headers(&session_data.access_jwt);

        // Fire session callback
        if let Some(ref callback) = self.session_callback {
            callback(SessionEvent::Create, &session_data);
        }

        Ok(session_data)
    }

    /// Resume a session from stored session data
    ///
    /// # Arguments
    ///
    /// * `session_data` - Previously stored session data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::{BskyAgent, AtpSessionData};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let session_data = AtpSessionData {
    /// #     access_jwt: "token".to_string(),
    /// #     refresh_jwt: "refresh".to_string(),
    /// #     did: "did:plc:123".to_string(),
    /// #     handle: "alice.bsky.social".to_string(),
    /// #     email: None,
    /// #     email_confirmed: None,
    /// #     email_auth_factor: None,
    /// #     active: true,
    /// #     status: None,
    /// # };
    /// let mut agent = BskyAgent::new("https://bsky.social")?;
    /// agent.resume_session(session_data).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resume_session(&mut self, session_data: AtpSessionData) -> Result<()> {
        // Check if tokens are expired and refresh if needed
        let needs_refresh = crate::session::is_jwt_expired(&session_data.access_jwt);

        if needs_refresh {
            self.refresh_session_internal(session_data).await?;
        } else {
            // Update session
            {
                let mut session = self.session.write().unwrap();
                *session = Some(session_data.clone());
            }

            // Update client auth headers
            self.update_auth_headers(&session_data.access_jwt);
        }

        Ok(())
    }

    /// Refresh the current session
    ///
    /// Uses the refresh token to get new access and refresh tokens
    pub async fn refresh_session(&mut self) -> Result<()> {
        let current_session = {
            let session = self.session.read().unwrap();
            session.clone().ok_or(AgentError::NoSession)?
        };

        self.refresh_session_internal(current_session).await
    }

    /// Internal session refresh implementation
    async fn refresh_session_internal(&mut self, session_data: AtpSessionData) -> Result<()> {
        use crate::xrpc::XrpcRequest;

        // Temporarily set the refresh token as auth
        self.write_client
            .set_auth_header(Some(format!("Bearer {}", session_data.refresh_jwt)));

        let xrpc_request = XrpcRequest::procedure("com.atproto.server.refreshSession");

        let response: RefreshSessionResponse = self
            .write_client
            .procedure(xrpc_request)
            .await
            .map(|r| r.data)?;

        let new_session = AtpSessionData {
            access_jwt: response.access_jwt,
            refresh_jwt: response.refresh_jwt,
            did: response.did,
            handle: response.handle,
            email: session_data.email, // Preserve email from old session
            email_confirmed: session_data.email_confirmed,
            email_auth_factor: session_data.email_auth_factor,
            active: response.active.unwrap_or(true),
            status: response.status,
        };

        // Update session
        {
            let mut session = self.session.write().unwrap();
            *session = Some(new_session.clone());
        }

        // Update client auth headers with new access token
        self.update_auth_headers(&new_session.access_jwt);

        // Fire session callback
        if let Some(ref callback) = self.session_callback {
            callback(SessionEvent::Update, &new_session);
        }

        Ok(())
    }

    /// Logout and clear the session
    pub fn logout(&mut self) {
        let mut session = self.session.write().unwrap();
        *session = None;

        // Clear auth headers
        self.read_client.set_auth_header(None);
        self.write_client.set_auth_header(None);
    }

    /// Get the current session data
    pub fn session(&self) -> Option<AtpSessionData> {
        let session = self.session.read().unwrap();
        session.clone()
    }

    /// Check if there's an active session
    pub fn has_session(&self) -> bool {
        let session = self.session.read().unwrap();
        session.is_some()
    }

    /// Get the current user's DID
    pub fn did(&self) -> Option<String> {
        let session = self.session.read().unwrap();
        session.as_ref().map(|s| s.did.clone())
    }

    /// Get the current user's handle
    pub fn handle(&self) -> Option<String> {
        let session = self.session.read().unwrap();
        session.as_ref().map(|s| s.handle.clone())
    }

    /// Get the service URL
    pub fn service(&self) -> &str {
        &self.service
    }

    /// Get the AppView URL (if configured)
    pub fn app_view(&self) -> Option<&str> {
        self.app_view.as_deref()
    }

    /// Get the PDS URL (if configured)
    pub fn pds_url(&self) -> Option<&str> {
        self.pds_url.as_deref()
    }

    /// Get a reference to the read client
    ///
    /// The read client uses the AppView URL if configured, otherwise the service URL
    pub fn read_client(&self) -> &XrpcClient {
        &self.read_client
    }

    /// Get a reference to the write client
    ///
    /// The write client always uses the PDS URL
    pub fn write_client(&self) -> &XrpcClient {
        &self.write_client
    }

    /// Upload a blob to the PDS
    ///
    /// # Arguments
    ///
    /// * `data` - The binary data to upload
    /// * `mime_type` - MIME type of the blob (e.g., "image/jpeg")
    ///
    /// # Returns
    ///
    /// Returns a `BlobRef` containing the CID and metadata for the uploaded blob
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use atproto_client::BskyAgent;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let mut agent = BskyAgent::new("https://bsky.social")?;
    ///     agent.login("alice.bsky.social", "password").await?;
    ///
    ///     let image_data = std::fs::read("photo.jpg")?;
    ///     let blob_ref = agent.upload_blob(&image_data, "image/jpeg").await?;
    ///     println!("Uploaded blob CID: {}", blob_ref.ref_link.cid);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn upload_blob(
        &self,
        data: &[u8],
        mime_type: impl Into<String>,
    ) -> Result<crate::lexicon::BlobRef> {
        // Verify we have an active session
        if !self.has_session() {
            return Err(AgentError::NoSession);
        }

        let mime_type = mime_type.into();

        // Create the upload request
        let request = crate::xrpc::XrpcRequest::procedure("com.atproto.repo.uploadBlob")
            .body(data.to_vec())
            .encoding(mime_type.clone());

        // Upload the blob
        let response: crate::xrpc::XrpcResponse<serde_json::Value> =
            self.write_client.procedure(request).await?;

        // Parse the blob reference from response
        #[derive(Deserialize)]
        struct UploadBlobResponse {
            blob: crate::lexicon::BlobRef,
        }

        let upload_response: UploadBlobResponse = serde_json::from_value(response.data)
            .map_err(|e| AgentError::Service(format!("Failed to parse upload response: {}", e)))?;

        Ok(upload_response.blob)
    }

    /// Call a custom XRPC procedure
    ///
    /// Makes an authenticated XRPC procedure call to the PDS. This is useful for calling
    /// procedures that don't have dedicated methods in the SDK.
    ///
    /// # Arguments
    ///
    /// * `nsid` - The procedure NSID (e.g., "com.atproto.server.confirmEmail")
    /// * `params` - JSON parameters for the procedure
    ///
    /// # Returns
    ///
    /// The response data as a JSON value
    ///
    /// # Errors
    ///
    /// Returns `AgentError` if the call fails
    pub async fn call_procedure(
        &self,
        nsid: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        use crate::xrpc::XrpcRequest;

        let request = XrpcRequest::procedure(nsid)
            .json_body(&params)
            .map_err(|e| AgentError::Service(e.to_string()))?;

        let response = self.write_client.procedure(request).await?;
        Ok(response.data)
    }

    /// Update auth headers on both clients
    fn update_auth_headers(&mut self, access_token: &str) {
        let auth = format!("Bearer {}", access_token);
        self.read_client.set_auth_header(Some(auth.clone()));
        self.write_client.set_auth_header(Some(auth));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_new() {
        let agent = BskyAgent::new("https://bsky.social").unwrap();
        assert_eq!(agent.service(), "https://bsky.social");
        assert!(!agent.has_session());
        assert!(agent.did().is_none());
        assert!(agent.handle().is_none());
    }

    #[test]
    fn test_agent_config() {
        let config = BskyAgentConfig::new("https://bsky.social")
            .with_app_view("https://api.bsky.app")
            .with_pds_url("https://pds.example.com");

        assert_eq!(config.service, "https://bsky.social");
        assert_eq!(config.app_view, Some("https://api.bsky.app".to_string()));
        assert_eq!(config.pds_url, Some("https://pds.example.com".to_string()));
    }

    #[test]
    fn test_agent_with_app_view() {
        let config =
            BskyAgentConfig::new("https://bsky.social").with_app_view("https://api.bsky.app");
        let agent = BskyAgent::with_config(config).unwrap();

        assert_eq!(agent.service(), "https://bsky.social");
        assert_eq!(agent.app_view(), Some("https://api.bsky.app"));
    }

    #[test]
    fn test_agent_session_management() {
        let mut agent = BskyAgent::new("https://bsky.social").unwrap();

        // No session initially
        assert!(!agent.has_session());
        assert!(agent.session().is_none());

        // Create a session
        let session_data = AtpSessionData {
            access_jwt: "access_token".to_string(),
            refresh_jwt: "refresh_token".to_string(),
            did: "did:plc:abc123".to_string(),
            handle: "alice.bsky.social".to_string(),
            email: Some("alice@example.com".to_string()),
            email_confirmed: Some(true),
            email_auth_factor: Some(false),
            active: true,
            status: None,
        };

        // Manually set session for testing
        {
            let mut session = agent.session.write().unwrap();
            *session = Some(session_data.clone());
        }

        // Verify session is set
        assert!(agent.has_session());
        assert_eq!(agent.did(), Some("did:plc:abc123".to_string()));
        assert_eq!(agent.handle(), Some("alice.bsky.social".to_string()));
        assert_eq!(agent.session(), Some(session_data));

        // Logout
        agent.logout();
        assert!(!agent.has_session());
        assert!(agent.session().is_none());
    }

    #[test]
    fn test_login_request_serialization() {
        let request = LoginRequest {
            identifier: "alice.bsky.social".to_string(),
            password: "password".to_string(),
            auth_factor_token: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("alice.bsky.social"));
        assert!(json.contains("password"));
    }

    #[test]
    fn test_create_account_request_serialization() {
        let request = CreateAccountRequest {
            email: "alice@example.com".to_string(),
            password: "password".to_string(),
            handle: "alice.bsky.social".to_string(),
            invite_code: Some("invite123".to_string()),
            verification_phone: None,
            verification_code: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("alice@example.com"));
        assert!(json.contains("alice.bsky.social"));
        assert!(json.contains("invite123"));
    }

    #[test]
    fn test_session_event_types() {
        assert_eq!(SessionEvent::Create, SessionEvent::Create);
        assert_ne!(SessionEvent::Create, SessionEvent::Update);
    }
}
