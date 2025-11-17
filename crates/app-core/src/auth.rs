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
/// session validation, and multi-account support.
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
///     // Login
///     let params = LoginParams {
///         identifier: "alice.bsky.social".to_string(),
///         password: "password123".to_string(),
///         auth_factor_token: None,
///         service: None,
///     };
///
///     let result = auth.login(params).await?;
///     println!("Logged in as: {}", result.handle);
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

                    let account = manager
                        .current_account()
                        .ok_or(AuthError::SessionExpired)?;

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
                atproto_client::session::is_jwt_expiring_soon(
                    access_jwt,
                    Duration::seconds(3600),
                )
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
}
