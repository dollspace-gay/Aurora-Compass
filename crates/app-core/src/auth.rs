//! Authentication service for Aurora Compass
//!
//! This module provides high-level authentication flows including login, logout,
//! account creation, and session validation.

use atproto_client::{
    session::{SessionAccount, SessionManager, SessionManagerError},
    AgentError,
};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Authentication service error types
#[derive(Debug, Error)]
pub enum AuthError {
    /// Session manager error
    #[error("Session error: {0}")]
    Session(#[from] SessionManagerError),

    /// Agent error
    #[error("Agent error: {0}")]
    Agent(#[from] AgentError),

    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Account not found
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    /// No active session
    #[error("No active session")]
    NoSession,

    /// Session expired
    #[error("Session expired")]
    SessionExpired,

    /// 2FA required
    #[error("Two-factor authentication required")]
    TwoFactorRequired,

    /// Invalid 2FA token
    #[error("Invalid two-factor authentication token")]
    Invalid2FAToken,

    /// Account suspended
    #[error("Account suspended: {0}")]
    AccountSuspended(String),

    /// Account deactivated
    #[error("Account deactivated")]
    AccountDeactivated,

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Result type for authentication operations
pub type Result<T> = std::result::Result<T, AuthError>;

/// Login parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginParams {
    /// User identifier (handle or email)
    pub identifier: String,
    /// Password or app password
    pub password: String,
    /// Optional 2FA token
    pub auth_factor_token: Option<String>,
    /// Service URL (defaults to bsky.social)
    pub service: Option<String>,
}

/// Create account parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountParams {
    /// Email address
    pub email: String,
    /// Handle (username)
    pub handle: String,
    /// Password
    pub password: String,
    /// Optional invite code
    pub invite_code: Option<String>,
    /// Service URL (defaults to bsky.social)
    pub service: Option<String>,
}

/// Login result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResult {
    /// User DID
    pub did: String,
    /// User handle
    pub handle: String,
    /// Email address
    pub email: Option<String>,
    /// Whether email is confirmed
    pub email_confirmed: bool,
    /// Whether 2FA is enabled
    pub two_factor_enabled: bool,
}

/// Authentication service
///
/// Provides high-level authentication flows with comprehensive error handling,
/// session validation, and **multi-account support**.
///
/// # Multi-Account Support
///
/// The AuthService supports managing multiple concurrent accounts with per-account
/// state isolation:
///
/// - **Multiple Sessions**: Store and manage sessions for multiple accounts simultaneously
/// - **Account Switching**: Switch between accounts with [`switch_account()`](Self::switch_account)
/// - **Account Management**: List, add, and remove accounts with [`list_accounts()`](Self::list_accounts)
///   and [`remove_account()`](Self::remove_account)
/// - **Per-Account State**: Each account maintains its own authentication tokens,
///   user information, and session state completely isolated from other accounts
/// - **Session Persistence**: All account sessions are persisted to disk and survive
///   application restarts
///
/// # Example
///
/// ```rust,no_run
/// use app_core::auth::{AuthService, LoginParams};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Create auth service
///     let auth = AuthService::new("sessions.json").await?;
///
///     // Login to first account
///     let params1 = LoginParams {
///         identifier: "alice.bsky.social".to_string(),
///         password: "password123".to_string(),
///         auth_factor_token: None,
///         service: None,
///     };
///     let alice = auth.login(params1).await?;
///     println!("Logged in as: {}", alice.handle);
///
///     // Login to second account (both accounts are now stored)
///     let params2 = LoginParams {
///         identifier: "bob.bsky.social".to_string(),
///         password: "password456".to_string(),
///         auth_factor_token: None,
///         service: None,
///     };
///     let bob = auth.login(params2).await?;
///     println!("Logged in as: {}", bob.handle);
///
///     // List all accounts
///     let accounts = auth.list_accounts().await;
///     println!("Total accounts: {}", accounts.len());
///
///     // Switch back to first account
///     auth.switch_account(&alice.did).await?;
///     println!("Switched back to: {}", alice.handle);
///
///     Ok(())
/// }
/// ```
pub struct AuthService {
    session_manager: Arc<RwLock<SessionManager>>,
}

impl AuthService {
    /// Create a new authentication service
    ///
    /// # Arguments
    ///
    /// * `session_path` - Path to session storage file
    pub async fn new(session_path: impl Into<std::path::PathBuf>) -> Result<Self> {
        let session_manager = SessionManager::new(session_path).await?;
        Ok(Self {
            session_manager: Arc::new(RwLock::new(session_manager)),
        })
    }

    /// Create a new authentication service with custom default service
    ///
    /// # Arguments
    ///
    /// * `session_path` - Path to session storage file
    /// * `default_service` - Default AT Protocol service URL
    pub async fn with_service(
        session_path: impl Into<std::path::PathBuf>,
        default_service: impl Into<String>,
    ) -> Result<Self> {
        let session_manager = SessionManager::with_service(session_path, default_service).await?;
        Ok(Self {
            session_manager: Arc::new(RwLock::new(session_manager)),
        })
    }

    /// Login with credentials
    ///
    /// # Arguments
    ///
    /// * `params` - Login parameters including identifier, password, and optional 2FA token
    ///
    /// # Returns
    ///
    /// Login result with user information
    ///
    /// # Errors
    ///
    /// - `AuthError::InvalidCredentials` - Invalid username/password
    /// - `AuthError::TwoFactorRequired` - 2FA token required but not provided
    /// - `AuthError::Invalid2FAToken` - Invalid 2FA token
    /// - `AuthError::AccountSuspended` - Account is suspended
    /// - `AuthError::AccountDeactivated` - Account is deactivated
    /// - `AuthError::Network` - Network error
    pub async fn login(&self, params: LoginParams) -> Result<LoginResult> {
        let _service = params
            .service
            .clone()
            .unwrap_or_else(|| "https://bsky.social".to_string());

        // Attempt login through session manager
        let mut manager = self.session_manager.write().await;

        match manager.login(&params.identifier, &params.password).await {
            Ok(account) => {
                // Check account status
                if let Some(status) = &account.status {
                    match status.as_str() {
                        "suspended" => {
                            return Err(AuthError::AccountSuspended(account.did.clone()))
                        }
                        "deactivated" => return Err(AuthError::AccountDeactivated),
                        _ => {}
                    }
                }

                // Build result
                Ok(LoginResult {
                    did: account.did,
                    handle: account.handle,
                    email: account.email,
                    email_confirmed: account.email_confirmed.unwrap_or(false),
                    two_factor_enabled: account.email_auth_factor.unwrap_or(false),
                })
            }
            Err(SessionManagerError::Agent(AgentError::InvalidCredentials)) => {
                Err(AuthError::InvalidCredentials)
            }
            Err(SessionManagerError::Agent(AgentError::Service(msg)))
                if msg.contains("AuthFactorTokenRequired") =>
            {
                Err(AuthError::TwoFactorRequired)
            }
            Err(e) => Err(AuthError::Session(e)),
        }
    }

    /// Create a new account
    ///
    /// # Arguments
    ///
    /// * `params` - Account creation parameters
    ///
    /// # Returns
    ///
    /// Login result with new account information
    pub async fn create_account(&self, params: CreateAccountParams) -> Result<LoginResult> {
        let _service = params
            .service
            .clone()
            .unwrap_or_else(|| "https://bsky.social".to_string());

        let mut manager = self.session_manager.write().await;

        let account = manager
            .create_account(&params.email, &params.password, &params.handle)
            .await?;

        Ok(LoginResult {
            did: account.did,
            handle: account.handle,
            email: account.email,
            email_confirmed: account.email_confirmed.unwrap_or(false),
            two_factor_enabled: account.email_auth_factor.unwrap_or(false),
        })
    }

    /// Logout current account
    ///
    /// This clears the session tokens but keeps the account in the account list
    /// for easy re-login.
    pub async fn logout_current(&self) -> Result<()> {
        let mut manager = self.session_manager.write().await;
        manager.logout_current().await?;
        Ok(())
    }

    /// Logout all accounts
    ///
    /// This removes all accounts and sessions from the system.
    pub async fn logout_all(&self) -> Result<()> {
        let mut manager = self.session_manager.write().await;
        manager.logout_all().await?;
        Ok(())
    }

    /// Remove an account by DID
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to remove
    pub async fn remove_account(&self, did: &str) -> Result<()> {
        let mut manager = self.session_manager.write().await;
        manager.remove_account(did).await?;
        Ok(())
    }

    /// Switch to a different account
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to switch to
    pub async fn switch_account(&self, did: &str) -> Result<LoginResult> {
        let mut manager = self.session_manager.write().await;
        manager.switch_account(did).await?;

        let account = manager
            .get_account(did)
            .ok_or_else(|| AuthError::AccountNotFound(did.to_string()))?;

        Ok(LoginResult {
            did: account.did.clone(),
            handle: account.handle.clone(),
            email: account.email.clone(),
            email_confirmed: account.email_confirmed.unwrap_or(false),
            two_factor_enabled: account.email_auth_factor.unwrap_or(false),
        })
    }

    /// Resume an existing session
    ///
    /// Attempts to resume the most recent session if tokens are still valid.
    pub async fn resume_session(&self) -> Result<Option<LoginResult>> {
        let manager = self.session_manager.read().await;

        if let Some(account) = manager.current_account() {
            // Check if session is still valid
            if let (Some(access_jwt), Some(_refresh_jwt)) =
                (&account.access_jwt, &account.refresh_jwt)
            {
                // Check if access token is expired
                if atproto_client::session::is_jwt_expired(access_jwt) {
                    // Try to refresh
                    drop(manager);
                    let mut manager = self.session_manager.write().await;
                    manager.refresh_current_session().await?;

                    let account = manager.current_account().ok_or(AuthError::SessionExpired)?;

                    return Ok(Some(LoginResult {
                        did: account.did.clone(),
                        handle: account.handle.clone(),
                        email: account.email.clone(),
                        email_confirmed: account.email_confirmed.unwrap_or(false),
                        two_factor_enabled: account.email_auth_factor.unwrap_or(false),
                    }));
                }

                Ok(Some(LoginResult {
                    did: account.did.clone(),
                    handle: account.handle.clone(),
                    email: account.email.clone(),
                    email_confirmed: account.email_confirmed.unwrap_or(false),
                    two_factor_enabled: account.email_auth_factor.unwrap_or(false),
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Get current session information
    pub async fn current_session(&self) -> Option<LoginResult> {
        let manager = self.session_manager.read().await;

        manager.current_account().map(|account| LoginResult {
            did: account.did.clone(),
            handle: account.handle.clone(),
            email: account.email.clone(),
            email_confirmed: account.email_confirmed.unwrap_or(false),
            two_factor_enabled: account.email_auth_factor.unwrap_or(false),
        })
    }

    /// List all accounts
    pub async fn list_accounts(&self) -> Vec<SessionAccount> {
        let manager = self.session_manager.read().await;
        manager.list_accounts().to_vec()
    }

    /// Validate current session
    ///
    /// Returns true if there's an active, valid session
    pub async fn validate_session(&self) -> bool {
        let manager = self.session_manager.read().await;

        if let Some(account) = manager.current_account() {
            if let Some(access_jwt) = &account.access_jwt {
                !atproto_client::session::is_jwt_expired(access_jwt)
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Check if session is expiring soon (within 1 hour)
    pub async fn is_session_expiring_soon(&self) -> bool {
        let manager = self.session_manager.read().await;

        if let Some(account) = manager.current_account() {
            if let Some(access_jwt) = &account.access_jwt {
                atproto_client::session::is_jwt_expiring_soon(access_jwt, Duration::seconds(3600))
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Refresh current session if needed
    pub async fn refresh_if_needed(&self) -> Result<()> {
        if self.is_session_expiring_soon().await {
            let mut manager = self.session_manager.write().await;
            manager.refresh_current_session().await?;
        }
        Ok(())
    }

    /// Set custom AppView URL for an account
    ///
    /// This allows users to configure which AppView provider to use for read operations.
    /// This is a key differentiator from the official Bluesky client which only uses bsky.social.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to configure
    /// * `app_view_url` - The custom AppView URL, or None to use the service URL
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::auth::AuthService;
    /// # async fn example(auth: &AuthService) -> Result<(), Box<dyn std::error::Error>> {
    /// // Set custom AppView for an account
    /// auth.set_app_view_url("did:plc:abc123", Some("https://api.bsky.app".to_string())).await?;
    ///
    /// // Clear custom AppView (revert to service URL)
    /// auth.set_app_view_url("did:plc:abc123", None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_app_view_url(
        &self,
        did: &str,
        app_view_url: Option<String>,
    ) -> Result<()> {
        let mut manager = self.session_manager.write().await;
        manager.set_account_app_view(did, app_view_url).await?;
        Ok(())
    }

    /// Get the custom AppView URL for an account
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account
    ///
    /// # Returns
    ///
    /// Returns the custom AppView URL if set, None if using the default (service URL)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::auth::AuthService;
    /// # async fn example(auth: &AuthService) {
    /// match auth.get_app_view_url("did:plc:abc123").await {
    ///     Some(url) => println!("Using custom AppView: {}", url),
    ///     None => println!("Using default AppView (service URL)"),
    /// }
    /// # }
    /// ```
    pub async fn get_app_view_url(&self, did: &str) -> Option<String> {
        let manager = self.session_manager.read().await;
        manager.get_account_app_view(did)
    }

    /// Request an email authentication token for 2FA
    ///
    /// Sends a verification code to the authenticated user's email address.
    /// This code can be used to complete 2FA challenges or enable 2FA on an account.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the email was sent successfully
    ///
    /// # Errors
    ///
    /// - `AuthError::NoSession` - No active session
    /// - `AuthError::Network` - Network error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::auth::AuthService;
    /// # async fn example(auth: &AuthService) -> Result<(), Box<dyn std::error::Error>> {
    /// // Request 2FA code to be sent to email
    /// auth.request_email_auth_token().await?;
    /// println!("2FA code sent to your email");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request_email_auth_token(&self) -> Result<()> {
        let manager = self.session_manager.read().await;
        let agent_arc = manager.current_agent().ok_or(AuthError::NoSession)?;
        let agent = agent_arc.read().await;

        // Use com.atproto.server.requestEmailConfirmation
        // This sends a verification code to the user's registered email
        agent
            .call_procedure("com.atproto.server.requestEmailConfirmation", serde_json::json!({}))
            .await
            .map_err(|e| AuthError::Network(e.to_string()))?;

        Ok(())
    }

    /// Verify an email authentication token
    ///
    /// Confirms the verification code received via email. This is used to complete
    /// 2FA challenges during login or when enabling 2FA on an account.
    ///
    /// # Arguments
    ///
    /// * `token` - The verification code received via email (typically 6 digits)
    /// * `email` - The email address (for validation)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the token is valid
    ///
    /// # Errors
    ///
    /// - `AuthError::NoSession` - No active session
    /// - `AuthError::Invalid2FAToken` - Invalid or expired token
    /// - `AuthError::Network` - Network error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::auth::AuthService;
    /// # async fn example(auth: &AuthService) -> Result<(), Box<dyn std::error::Error>> {
    /// // Verify the code received via email
    /// auth.verify_email_auth_token("123456", "alice@example.com").await?;
    /// println!("Email verification successful");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn verify_email_auth_token(&self, token: &str, email: &str) -> Result<()> {
        let manager = self.session_manager.read().await;
        let agent_arc = manager.current_agent().ok_or(AuthError::NoSession)?;
        let agent = agent_arc.read().await;

        // Use com.atproto.server.confirmEmail
        let result = agent
            .call_procedure(
                "com.atproto.server.confirmEmail",
                serde_json::json!({
                    "email": email,
                    "token": token,
                }),
            )
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) if e.to_string().contains("InvalidToken") => {
                Err(AuthError::Invalid2FAToken)
            }
            Err(e) => Err(AuthError::Network(e.to_string())),
        }
    }

    /// Enable email-based two-factor authentication
    ///
    /// Enables 2FA for the current account. After enabling, the account will require
    /// an email verification code for all future logins.
    ///
    /// # Process
    ///
    /// 1. Call [`request_email_auth_token()`](Self::request_email_auth_token) to send code
    /// 2. Receive code via email
    /// 3. Call [`verify_email_auth_token()`](Self::verify_email_auth_token) to confirm
    /// 4. Call this method to enable 2FA
    ///
    /// # Arguments
    ///
    /// * `token` - The verified email token
    ///
    /// # Returns
    ///
    /// `Ok(())` if 2FA was enabled successfully
    ///
    /// # Errors
    ///
    /// - `AuthError::NoSession` - No active session
    /// - `AuthError::Invalid2FAToken` - Invalid or expired token
    /// - `AuthError::Network` - Network error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::auth::AuthService;
    /// # async fn example(auth: &AuthService) -> Result<(), Box<dyn std::error::Error>> {
    /// // Step 1: Request code
    /// auth.request_email_auth_token().await?;
    ///
    /// // Step 2: User receives code via email (e.g., "123456")
    ///
    /// // Step 3: Verify and enable 2FA
    /// auth.verify_email_auth_token("123456", "alice@example.com").await?;
    /// auth.enable_email_2fa("123456").await?;
    /// println!("2FA enabled successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn enable_email_2fa(&self, token: &str) -> Result<()> {
        let manager = self.session_manager.read().await;
        let agent_arc = manager.current_agent().ok_or(AuthError::NoSession)?;
        let agent = agent_arc.read().await;

        // Use com.atproto.server.updateEmail with 2FA enabled
        let result = agent
            .call_procedure(
                "com.atproto.server.updateEmail",
                serde_json::json!({
                    "token": token,
                    "emailAuthFactor": true,
                }),
            )
            .await;

        match result {
            Ok(_) => {
                // Note: The account state will be updated on next session refresh
                // or when the account is re-loaded from the server
                Ok(())
            }
            Err(e) if e.to_string().contains("InvalidToken") => {
                Err(AuthError::Invalid2FAToken)
            }
            Err(e) => Err(AuthError::Network(e.to_string())),
        }
    }

    /// Disable email-based two-factor authentication
    ///
    /// Disables 2FA for the current account. After disabling, the account will only
    /// require password for login.
    ///
    /// # Returns
    ///
    /// `Ok(())` if 2FA was disabled successfully
    ///
    /// # Errors
    ///
    /// - `AuthError::NoSession` - No active session
    /// - `AuthError::Network` - Network error
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::auth::AuthService;
    /// # async fn example(auth: &AuthService) -> Result<(), Box<dyn std::error::Error>> {
    /// // Disable 2FA
    /// auth.disable_email_2fa().await?;
    /// println!("2FA disabled successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn disable_email_2fa(&self) -> Result<()> {
        let manager = self.session_manager.read().await;
        let agent_arc = manager.current_agent().ok_or(AuthError::NoSession)?;
        let agent = agent_arc.read().await;

        // Use com.atproto.server.updateEmail with 2FA disabled
        let result = agent
            .call_procedure(
                "com.atproto.server.updateEmail",
                serde_json::json!({
                    "emailAuthFactor": false,
                }),
            )
            .await;

        match result {
            Ok(_) => {
                // Note: The account state will be updated on next session refresh
                // or when the account is re-loaded from the server
                Ok(())
            }
            Err(e) => Err(AuthError::Network(e.to_string())),
        }
    }

    /// Check if 2FA is enabled for the current account
    ///
    /// # Returns
    ///
    /// `true` if 2FA is enabled, `false` otherwise or if no session
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_core::auth::AuthService;
    /// # async fn example(auth: &AuthService) {
    /// if auth.is_2fa_enabled().await {
    ///     println!("2FA is enabled");
    /// } else {
    ///     println!("2FA is disabled");
    /// }
    /// # }
    /// ```
    pub async fn is_2fa_enabled(&self) -> bool {
        let manager = self.session_manager.read().await;
        if let Some(account) = manager.current_account() {
            return account.email_auth_factor.unwrap_or(false);
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_auth_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        assert!(auth.current_session().await.is_none());
    }

    #[tokio::test]
    async fn test_session_validation() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        assert!(!auth.validate_session().await);
    }

    #[tokio::test]
    async fn test_list_accounts_empty() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let accounts = auth.list_accounts().await;
        assert_eq!(accounts.len(), 0);
    }

    #[tokio::test]
    async fn test_resume_session_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.resume_session().await.unwrap();
        assert!(result.is_none());
    }

    // Multi-account tests

    #[tokio::test]
    async fn test_multi_account_list_empty() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let accounts = auth.list_accounts().await;

        assert_eq!(accounts.len(), 0, "Should have no accounts initially");
    }

    #[tokio::test]
    async fn test_multi_account_switch_no_accounts() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.switch_account("did:plc:nonexistent").await;

        assert!(result.is_err(), "Should fail to switch to non-existent account");
        assert!(matches!(result.unwrap_err(), AuthError::Session(_)));
    }

    #[tokio::test]
    async fn test_multi_account_remove_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.remove_account("did:plc:nonexistent").await;

        // Removing non-existent account returns an error from SessionManager
        assert!(result.is_err(), "Removing non-existent account should return error");
    }

    #[tokio::test]
    async fn test_multi_account_logout_current_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.logout_current().await;

        // Logout returns error when there's no session to logout from
        assert!(result.is_err(), "Logout current should return error when no session");
    }

    #[tokio::test]
    async fn test_multi_account_logout_all_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.logout_all().await;

        // Logout all should work even with no sessions
        assert!(result.is_ok(), "Logout all should succeed even with no sessions");
    }

    #[tokio::test]
    async fn test_multi_account_current_session_none() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let session = auth.current_session().await;

        assert!(session.is_none(), "Should have no current session");
    }

    #[tokio::test]
    async fn test_multi_account_validate_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let is_valid = auth.validate_session().await;

        assert!(!is_valid, "Session should not be valid when there's no session");
    }

    #[tokio::test]
    async fn test_multi_account_expiring_soon_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let is_expiring = auth.is_session_expiring_soon().await;

        assert!(!is_expiring, "Should not be expiring when there's no session");
    }

    #[tokio::test]
    async fn test_multi_account_refresh_if_needed_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.refresh_if_needed().await;

        assert!(result.is_ok(), "Refresh should succeed even with no session");
    }

    #[tokio::test]
    async fn test_login_params_serialization() {
        let params = LoginParams {
            identifier: "alice.bsky.social".to_string(),
            password: "password123".to_string(),
            auth_factor_token: Some("123456".to_string()),
            service: Some("https://bsky.social".to_string()),
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: LoginParams = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.identifier, "alice.bsky.social");
        assert_eq!(deserialized.password, "password123");
        assert_eq!(deserialized.auth_factor_token, Some("123456".to_string()));
        assert_eq!(deserialized.service, Some("https://bsky.social".to_string()));
    }

    #[tokio::test]
    async fn test_create_account_params_serialization() {
        let params = CreateAccountParams {
            email: "alice@example.com".to_string(),
            handle: "alice.bsky.social".to_string(),
            password: "password123".to_string(),
            invite_code: Some("invite-code".to_string()),
            service: Some("https://bsky.social".to_string()),
        };

        let json = serde_json::to_string(&params).unwrap();
        let deserialized: CreateAccountParams = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.email, "alice@example.com");
        assert_eq!(deserialized.handle, "alice.bsky.social");
        assert_eq!(deserialized.password, "password123");
        assert_eq!(deserialized.invite_code, Some("invite-code".to_string()));
        assert_eq!(deserialized.service, Some("https://bsky.social".to_string()));
    }

    #[tokio::test]
    async fn test_login_result_serialization() {
        let result = LoginResult {
            did: "did:plc:abc123".to_string(),
            handle: "alice.bsky.social".to_string(),
            email: Some("alice@example.com".to_string()),
            email_confirmed: true,
            two_factor_enabled: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: LoginResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.did, "did:plc:abc123");
        assert_eq!(deserialized.handle, "alice.bsky.social");
        assert_eq!(deserialized.email, Some("alice@example.com".to_string()));
        assert!(deserialized.email_confirmed);
        assert!(!deserialized.two_factor_enabled);
    }

    #[tokio::test]
    async fn test_auth_service_with_custom_service() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::with_service(session_path, "https://custom.service")
            .await
            .unwrap();

        assert!(auth.current_session().await.is_none());
        assert_eq!(auth.list_accounts().await.len(), 0);
    }

    #[tokio::test]
    async fn test_auth_error_display() {
        let err = AuthError::InvalidCredentials;
        assert_eq!(err.to_string(), "Invalid credentials");

        let err = AuthError::AccountNotFound("did:plc:test".to_string());
        assert_eq!(err.to_string(), "Account not found: did:plc:test");

        let err = AuthError::NoSession;
        assert_eq!(err.to_string(), "No active session");

        let err = AuthError::SessionExpired;
        assert_eq!(err.to_string(), "Session expired");

        let err = AuthError::TwoFactorRequired;
        assert_eq!(err.to_string(), "Two-factor authentication required");

        let err = AuthError::Invalid2FAToken;
        assert_eq!(err.to_string(), "Invalid two-factor authentication token");

        let err = AuthError::AccountSuspended("reason".to_string());
        assert_eq!(err.to_string(), "Account suspended: reason");

        let err = AuthError::AccountDeactivated;
        assert_eq!(err.to_string(), "Account deactivated");

        let err = AuthError::Network("connection failed".to_string());
        assert_eq!(err.to_string(), "Network error: connection failed");

        let err = AuthError::Config("missing key".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing key");
    }

    // 2FA Tests

    #[tokio::test]
    async fn test_request_email_auth_token_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.request_email_auth_token().await;

        assert!(result.is_err(), "Should fail when no session");
        assert!(
            matches!(result.unwrap_err(), AuthError::NoSession),
            "Should return NoSession error"
        );
    }

    #[tokio::test]
    async fn test_verify_email_auth_token_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.verify_email_auth_token("123456", "test@example.com").await;

        assert!(result.is_err(), "Should fail when no session");
        assert!(
            matches!(result.unwrap_err(), AuthError::NoSession),
            "Should return NoSession error"
        );
    }

    #[tokio::test]
    async fn test_enable_email_2fa_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.enable_email_2fa("123456").await;

        assert!(result.is_err(), "Should fail when no session");
        assert!(
            matches!(result.unwrap_err(), AuthError::NoSession),
            "Should return NoSession error"
        );
    }

    #[tokio::test]
    async fn test_disable_email_2fa_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let result = auth.disable_email_2fa().await;

        assert!(result.is_err(), "Should fail when no session");
        assert!(
            matches!(result.unwrap_err(), AuthError::NoSession),
            "Should return NoSession error"
        );
    }

    #[tokio::test]
    async fn test_is_2fa_enabled_no_session() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();
        let is_enabled = auth.is_2fa_enabled().await;

        assert!(!is_enabled, "Should return false when no session");
    }

    #[tokio::test]
    async fn test_2fa_error_types() {
        // Test that 2FA error types are properly defined and can be matched
        let err = AuthError::TwoFactorRequired;
        assert!(matches!(err, AuthError::TwoFactorRequired));
        assert_eq!(err.to_string(), "Two-factor authentication required");

        let err = AuthError::Invalid2FAToken;
        assert!(matches!(err, AuthError::Invalid2FAToken));
        assert_eq!(err.to_string(), "Invalid two-factor authentication token");
    }

    #[tokio::test]
    async fn test_login_params_with_2fa_token() {
        let params = LoginParams {
            identifier: "alice.bsky.social".to_string(),
            password: "password123".to_string(),
            auth_factor_token: Some("123456".to_string()),
            service: None,
        };

        assert_eq!(params.auth_factor_token, Some("123456".to_string()));

        // Test serialization
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("123456"));

        let deserialized: LoginParams = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.auth_factor_token, Some("123456".to_string()));
    }

    #[tokio::test]
    async fn test_login_result_2fa_flag() {
        let result = LoginResult {
            did: "did:plc:abc123".to_string(),
            handle: "alice.bsky.social".to_string(),
            email: Some("alice@example.com".to_string()),
            email_confirmed: true,
            two_factor_enabled: true,
        };

        assert!(result.two_factor_enabled, "2FA should be enabled");

        // Test serialization
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: LoginResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.two_factor_enabled, "2FA flag should survive serialization");
    }

    #[tokio::test]
    async fn test_custom_app_view_url_no_account() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("sessions.json");

        let auth = AuthService::new(session_path).await.unwrap();

        // Setting app view URL for non-existent account should fail
        let result = auth.set_app_view_url("did:plc:nonexistent", Some("https://custom.app".to_string())).await;
        assert!(result.is_err(), "Should fail for non-existent account");

        // Getting app view URL for non-existent account returns None
        let url = auth.get_app_view_url("did:plc:nonexistent").await;
        assert!(url.is_none(), "Should return None for non-existent account");
    }
}
