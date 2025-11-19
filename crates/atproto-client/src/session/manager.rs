//! Session Manager for Multi-Account Support
//!
//! This module implements a session manager that handles multiple authenticated accounts,
//! matching the functionality of the original Bluesky TypeScript app's SessionStore.
//!
//! # Features
//!
//! - Store and manage multiple authenticated accounts
//! - Switch between accounts with single active agent
//! - Atomic persistence of account data
//! - Session event callbacks
//! - Account isolation with proper cleanup
//!
//! # Example
//!
//! ```rust,no_run
//! use atproto_client::session::SessionManager;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a new session manager
//!     let mut manager = SessionManager::new("sessions.json").await?;
//!
//!     // Login as first account
//!     let account1 = manager.login("alice.bsky.social", "password").await?;
//!     println!("Logged in as: {}", account1.handle);
//!
//!     // Login as second account
//!     let account2 = manager.login("bob.bsky.social", "password").await?;
//!     println!("Logged in as: {}", account2.handle);
//!
//!     // Switch back to first account
//!     manager.switch_account(&account1.did).await?;
//!     println!("Switched to: {}", manager.current_account().unwrap().handle);
//!
//!     Ok(())
//! }
//! ```

use crate::agent::{AgentError, BskyAgent, SessionCallback, SessionEvent};
use crate::session::{AtpSessionData, SessionAccount, SessionError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use storage::persistence::{PersistedState, PersistenceConfig, PersistenceError};
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors that can occur during session manager operations
#[derive(Debug, Error)]
pub enum SessionManagerError {
    /// Session error
    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    /// Agent error
    #[error("Agent error: {0}")]
    Agent(#[from] AgentError),

    /// Persistence error
    #[error("Persistence error: {0}")]
    Persistence(#[from] PersistenceError),

    /// Account not found
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    /// Account already exists
    #[error("Account already exists: {0}")]
    AccountAlreadyExists(String),

    /// No current account
    #[error("No current account selected")]
    NoCurrentAccount,

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Result type for session manager operations
pub type Result<T> = std::result::Result<T, SessionManagerError>;

/// Storage structure for persisted session data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionStorage {
    /// All authenticated accounts
    pub accounts: Vec<SessionAccount>,
    /// DID of the currently active account
    pub current_account_did: Option<String>,
}

/// Account export data structure for backup and portability
///
/// This structure contains all data needed to export and import an account,
/// including session credentials, preferences, and cached data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountExport {
    /// Schema version for backward compatibility
    pub version: u32,

    /// Timestamp of export (ISO 8601 format)
    pub exported_at: String,

    /// Account session data (including tokens if not redacted)
    pub account: SessionAccount,

    /// KV store data for this account (preferences, settings, cached data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_data: Option<storage::kv::AccountDataExport>,

    /// Whether access/refresh tokens are included in the export
    /// If false, tokens have been redacted for security
    pub tokens_included: bool,

    /// Whether sensitive data is encrypted
    /// Note: Encryption is not yet implemented; this field is reserved for future use
    pub encrypted: bool,
}

/// Session manager for multi-account support
///
/// The SessionManager handles multiple authenticated accounts and manages the active
/// BskyAgent. Only one account can be active at a time, but users can switch between
/// accounts without re-authenticating (if tokens are still valid).
///
/// # Architecture
///
/// - Stores multiple `SessionAccount` instances
/// - Maintains one active `BskyAgent` at a time
/// - Disposes of the previous agent when switching accounts
/// - Persists account data atomically to prevent data loss
/// - Supports session event callbacks for token refresh
pub struct SessionManager {
    /// All authenticated accounts
    accounts: Vec<SessionAccount>,

    /// DID of the currently active account
    current_did: Option<String>,

    /// Active agent (only one at a time)
    current_agent: Option<Arc<RwLock<BskyAgent>>>,

    /// Storage backend for persistence
    storage: Arc<PersistedState<SessionStorage>>,

    /// Session event callbacks
    callbacks: Vec<SessionCallback>,

    /// Default service URL for new agents
    default_service: String,
}

impl SessionManager {
    /// Create a new session manager with the specified storage path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the persistence file
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use atproto_client::session::SessionManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let manager = SessionManager::new("sessions.json").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(path: impl Into<PathBuf>) -> Result<Self> {
        Self::with_service(path, "https://bsky.social").await
    }

    /// Create a new session manager with custom default service URL
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the persistence file
    /// * `service` - Default service URL for new agents
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use atproto_client::session::SessionManager;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let manager = SessionManager::with_service(
    ///         "sessions.json",
    ///         "https://custom.pds.example.com"
    ///     ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn with_service(
        path: impl Into<PathBuf>,
        default_service: impl Into<String>,
    ) -> Result<Self> {
        let config = PersistenceConfig::new(path)
            .version(1)
            .atomic_writes(true)
            .backups(true, 3);

        let storage = PersistedState::new(config);
        storage.init().await?;

        // Load existing accounts if available
        let session_storage: SessionStorage = storage.get().await?;

        Ok(Self {
            accounts: session_storage.accounts,
            current_did: session_storage.current_account_did,
            current_agent: None,
            storage: Arc::new(storage),
            callbacks: Vec::new(),
            default_service: default_service.into(),
        })
    }

    /// Create a new session manager for testing (in-memory)
    ///
    /// This creates a session manager that doesn't persist to disk,
    /// useful for unit tests.
    #[cfg(test)]
    pub async fn new_in_memory() -> Result<Self> {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().map_err(PersistenceError::Io)?;
        let path = temp_dir.path().join("test_sessions.json");

        // Keep temp_dir alive by leaking it (for tests only)
        std::mem::forget(temp_dir);

        Self::new(path).await
    }

    /// Persist current state to storage
    async fn persist(&self) -> Result<()> {
        let storage_data = SessionStorage {
            accounts: self.accounts.clone(),
            current_account_did: self.current_did.clone(),
        };

        self.storage.set(storage_data).await?;
        Ok(())
    }

    /// Get the currently active account
    ///
    /// # Returns
    ///
    /// Returns `Some(&SessionAccount)` if an account is active, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &SessionManager) {
    /// if let Some(account) = manager.current_account() {
    ///     println!("Current user: {}", account.handle);
    /// }
    /// # }
    /// ```
    pub fn current_account(&self) -> Option<&SessionAccount> {
        self.current_did
            .as_ref()
            .and_then(|did| self.accounts.iter().find(|a| &a.did == did))
    }

    /// Get a mutable reference to the currently active account
    #[allow(dead_code)]
    fn current_account_mut(&mut self) -> Option<&mut SessionAccount> {
        let current_did = self.current_did.clone();
        current_did
            .as_ref()
            .and_then(|did| self.accounts.iter_mut().find(|a| &a.did == did))
    }

    /// Get the currently active agent
    ///
    /// # Returns
    ///
    /// Returns `Some(Arc<RwLock<BskyAgent>>)` if an agent is active, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &SessionManager) {
    /// if let Some(agent) = manager.current_agent() {
    ///     let agent_lock = agent.read().await;
    ///     println!("Agent service: {}", agent_lock.service());
    /// }
    /// # }
    /// ```
    pub fn current_agent(&self) -> Option<Arc<RwLock<BskyAgent>>> {
        self.current_agent.clone()
    }

    /// Get a list of all accounts
    ///
    /// # Returns
    ///
    /// Returns a slice of all authenticated accounts.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &SessionManager) {
    /// for account in manager.list_accounts() {
    ///     println!("Account: {} ({})", account.handle, account.did);
    /// }
    /// # }
    /// ```
    pub fn list_accounts(&self) -> &[SessionAccount] {
        &self.accounts
    }

    /// Get an account by DID
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to find
    ///
    /// # Returns
    ///
    /// Returns `Some(&SessionAccount)` if found, `None` otherwise.
    pub fn get_account(&self, did: &str) -> Option<&SessionAccount> {
        self.accounts.iter().find(|a| a.did == did)
    }

    /// Get a mutable reference to an account by DID
    fn get_account_mut(&mut self, did: &str) -> Option<&mut SessionAccount> {
        self.accounts.iter_mut().find(|a| a.did == did)
    }

    /// Add a new account to the manager
    ///
    /// If an account with the same DID already exists, it will be updated.
    ///
    /// # Arguments
    ///
    /// * `account` - The account to add
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::{SessionManager, SessionAccount};
    /// # async fn example(manager: &mut SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// let account = SessionAccount::new(
    ///     "https://bsky.social".to_string(),
    ///     "did:plc:abc123".to_string(),
    ///     "alice.bsky.social".to_string(),
    /// );
    /// manager.add_account(account).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_account(&mut self, account: SessionAccount) -> Result<()> {
        // Check if account already exists
        if let Some(existing) = self.get_account_mut(&account.did) {
            // Update existing account
            *existing = account;
        } else {
            // Add new account
            self.accounts.push(account);
        }

        self.persist().await
    }

    /// Remove an account from the manager
    ///
    /// If the account is currently active, the current agent will be disposed.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to remove
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &mut SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// manager.remove_account("did:plc:abc123").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_account(&mut self, did: &str) -> Result<()> {
        // Check if account exists
        let account_index = self
            .accounts
            .iter()
            .position(|a| a.did == did)
            .ok_or_else(|| SessionManagerError::AccountNotFound(did.to_string()))?;

        // If this is the current account, dispose the agent
        if self.current_did.as_ref() == Some(&did.to_string()) {
            self.dispose_current_agent();
            self.current_did = None;
        }

        // Remove the account
        self.accounts.remove(account_index);

        self.persist().await
    }

    /// Dispose of the current agent
    fn dispose_current_agent(&mut self) {
        if let Some(agent_arc) = self.current_agent.take() {
            // Try to logout gracefully
            if let Ok(mut agent) = agent_arc.try_write() {
                agent.logout();
            }
        }
    }

    /// Switch to a different account
    ///
    /// This will dispose of the current agent and create a new one for the specified account.
    /// If the account has valid tokens, it will attempt to resume the session.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to switch to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The account doesn't exist
    /// - The account has no valid tokens
    /// - Session resumption fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &mut SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// manager.switch_account("did:plc:abc123").await?;
    /// println!("Switched to: {}", manager.current_account().unwrap().handle);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn switch_account(&mut self, did: &str) -> Result<()> {
        // Verify account exists
        let account = self
            .get_account(did)
            .ok_or_else(|| SessionManagerError::AccountNotFound(did.to_string()))?
            .clone();

        // Check if account has tokens
        if !account.has_tokens() {
            return Err(SessionManagerError::InvalidOperation(
                "Account has no valid tokens - please login first".to_string(),
            ));
        }

        // Dispose current agent
        self.dispose_current_agent();

        // Create new agent for the account
        let service = account.pds_url.as_ref().unwrap_or(&account.service);

        // Configure agent with custom AppView if specified
        let mut agent = if let Some(ref app_view) = account.app_view_url {
            let config = crate::agent::BskyAgentConfig::new(service)
                .with_app_view(app_view);
            BskyAgent::with_config(config)?
        } else {
            BskyAgent::new(service)?
        };

        // Setup session callback to update stored tokens
        let did_clone = did.to_string();
        let storage = self.storage.clone();
        let accounts_clone = self.accounts.clone();

        agent.set_session_callback(move |event, session_data| {
            // Update account tokens on session events
            if matches!(event, SessionEvent::Create | SessionEvent::Update) {
                let mut accounts = accounts_clone.clone();
                if let Some(account) = accounts.iter_mut().find(|a| a.did == did_clone) {
                    account.access_jwt = Some(session_data.access_jwt.clone());
                    account.refresh_jwt = Some(session_data.refresh_jwt.clone());
                    account.active = Some(session_data.active);
                    account.status = session_data.status.clone();

                    // Persist updated accounts
                    let storage_data = SessionStorage {
                        accounts: accounts.clone(),
                        current_account_did: Some(did_clone.clone()),
                    };

                    let storage_clone = storage.clone();
                    tokio::spawn(async move {
                        let _ = storage_clone.set(storage_data).await;
                    });
                }
            }
        });

        // Resume the session
        let session_data = account.to_session_data()?;
        agent.resume_session(session_data).await?;

        // Update current state
        self.current_agent = Some(Arc::new(RwLock::new(agent)));
        self.current_did = Some(did.to_string());

        self.persist().await
    }

    /// Login with credentials and add the account
    ///
    /// This will create a new agent, login, and add the account to the manager.
    /// The newly logged in account will become the current account.
    ///
    /// # Arguments
    ///
    /// * `identifier` - User handle or email
    /// * `password` - User password
    ///
    /// # Returns
    ///
    /// Returns the newly created `SessionAccount`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &mut SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// let account = manager.login("alice.bsky.social", "password").await?;
    /// println!("Logged in as: {}", account.handle);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn login(&mut self, identifier: &str, password: &str) -> Result<SessionAccount> {
        self.login_with_service(identifier, password, &self.default_service.clone())
            .await
    }

    /// Login with credentials to a specific service
    ///
    /// # Arguments
    ///
    /// * `identifier` - User handle or email
    /// * `password` - User password
    /// * `service` - Service URL to login to
    ///
    /// # Returns
    ///
    /// Returns the newly created `SessionAccount`.
    pub async fn login_with_service(
        &mut self,
        identifier: &str,
        password: &str,
        service: &str,
    ) -> Result<SessionAccount> {
        // Dispose current agent before creating a new one
        self.dispose_current_agent();

        // Create new agent
        let mut agent = BskyAgent::new(service)?;

        // Login
        agent.login(identifier, password).await?;

        // Get session data
        let session_data = agent
            .session()
            .ok_or(SessionManagerError::NoCurrentAccount)?;

        // Convert to session account
        let account = session_data.to_session_account(service.to_string());

        // Check if account already exists
        if self.get_account(&account.did).is_some() {
            // Update existing account
            let existing = self.get_account_mut(&account.did).unwrap();
            *existing = account.clone();
        } else {
            // Add new account
            self.accounts.push(account.clone());
        }

        // Set as current account
        self.current_did = Some(account.did.clone());
        self.current_agent = Some(Arc::new(RwLock::new(agent)));

        self.persist().await?;

        Ok(account)
    }

    /// Create a new account and add it to the manager
    ///
    /// # Arguments
    ///
    /// * `email` - Email address
    /// * `password` - Password
    /// * `handle` - Desired handle
    ///
    /// # Returns
    ///
    /// Returns the newly created `SessionAccount`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &mut SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// let account = manager.create_account(
    ///     "alice@example.com",
    ///     "password",
    ///     "alice.bsky.social"
    /// ).await?;
    /// println!("Created account: {}", account.handle);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_account(
        &mut self,
        email: &str,
        password: &str,
        handle: &str,
    ) -> Result<SessionAccount> {
        self.create_account_with_service(email, password, handle, &self.default_service.clone())
            .await
    }

    /// Create a new account on a specific service
    ///
    /// # Arguments
    ///
    /// * `email` - Email address
    /// * `password` - Password
    /// * `handle` - Desired handle
    /// * `service` - Service URL to create account on
    ///
    /// # Returns
    ///
    /// Returns the newly created `SessionAccount`.
    pub async fn create_account_with_service(
        &mut self,
        email: &str,
        password: &str,
        handle: &str,
        service: &str,
    ) -> Result<SessionAccount> {
        // Dispose current agent before creating a new one
        self.dispose_current_agent();

        // Create new agent
        let mut agent = BskyAgent::new(service)?;

        // Create account
        agent.create_account(email, password, handle).await?;

        // Get session data
        let session_data = agent
            .session()
            .ok_or(SessionManagerError::NoCurrentAccount)?;

        // Convert to session account
        let account = session_data.to_session_account(service.to_string());

        // Add account
        self.accounts.push(account.clone());

        // Set as current account
        self.current_did = Some(account.did.clone());
        self.current_agent = Some(Arc::new(RwLock::new(agent)));

        self.persist().await?;

        Ok(account)
    }

    /// Resume an existing session for an account
    ///
    /// This is similar to `switch_account` but is more explicit about resuming a session.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to resume
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The account doesn't exist
    /// - The account has no valid tokens
    /// - Session resumption fails
    pub async fn resume_session(&mut self, did: &str) -> Result<()> {
        self.switch_account(did).await
    }

    /// Logout the current account
    ///
    /// This will dispose of the current agent and clear tokens for the current account.
    /// The account will remain in the list but with cleared tokens.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &mut SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// manager.logout_current().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn logout_current(&mut self) -> Result<()> {
        let current_did = self
            .current_did
            .clone()
            .ok_or(SessionManagerError::NoCurrentAccount)?;

        // Dispose agent
        self.dispose_current_agent();

        // Clear tokens for current account but keep it in the list
        if let Some(account) = self.get_account_mut(&current_did) {
            account.access_jwt = None;
            account.refresh_jwt = None;
            account.active = Some(false);
        }

        self.current_did = None;

        self.persist().await
    }

    /// Logout all accounts
    ///
    /// This will dispose of the current agent and clear all account data.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &mut SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// manager.logout_all().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn logout_all(&mut self) -> Result<()> {
        // Dispose agent
        self.dispose_current_agent();

        // Clear all accounts
        self.accounts.clear();
        self.current_did = None;

        self.persist().await
    }

    /// Refresh the current session's tokens
    ///
    /// This will use the refresh token to get new access and refresh tokens.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No account is currently active
    /// - The refresh token is invalid or expired
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &mut SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// manager.refresh_current_session().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refresh_current_session(&mut self) -> Result<()> {
        let current_did = self
            .current_did
            .clone()
            .ok_or(SessionManagerError::NoCurrentAccount)?;

        // Get current agent
        let agent_arc = self
            .current_agent
            .as_ref()
            .ok_or(SessionManagerError::NoCurrentAccount)?
            .clone();

        // Refresh session
        {
            let mut agent = agent_arc.write().await;
            agent.refresh_session().await?;

            // Get updated session data
            if let Some(session_data) = agent.session() {
                // Update stored account
                if let Some(account) = self.get_account_mut(&current_did) {
                    account.access_jwt = Some(session_data.access_jwt.clone());
                    account.refresh_jwt = Some(session_data.refresh_jwt.clone());
                    account.active = Some(session_data.active);
                    account.status = session_data.status.clone();
                }
            }
        }

        self.persist().await
    }

    /// Register a session event callback
    ///
    /// The callback will be invoked when session events occur (create, update, expire, etc.)
    ///
    /// # Arguments
    ///
    /// * `callback` - Function to call on session events
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # use atproto_client::agent::SessionEvent;
    /// # async fn example(manager: &mut SessionManager) {
    /// manager.on_session_event(|event, session_data| {
    ///     match event {
    ///         SessionEvent::Create => println!("Session created for {}", session_data.handle),
    ///         SessionEvent::Update => println!("Session updated for {}", session_data.handle),
    ///         SessionEvent::Expired => println!("Session expired for {}", session_data.handle),
    ///         SessionEvent::NetworkError => println!("Network error for {}", session_data.handle),
    ///     }
    /// });
    /// # }
    /// ```
    pub fn on_session_event<F>(&mut self, callback: F)
    where
        F: Fn(SessionEvent, &AtpSessionData) + Send + Sync + 'static,
    {
        self.callbacks.push(Arc::new(callback));
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
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &mut SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// // Set custom AppView
    /// manager.set_account_app_view("did:plc:abc123", Some("https://api.bsky.app".to_string())).await?;
    ///
    /// // Clear custom AppView (use service URL)
    /// manager.set_account_app_view("did:plc:abc123", None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_account_app_view(
        &mut self,
        did: &str,
        app_view_url: Option<String>,
    ) -> Result<()> {
        // Check if account exists
        if self.get_account(did).is_none() {
            return Err(SessionManagerError::AccountNotFound(did.to_string()));
        }

        // Check if this is the current account
        let is_current_account = self.current_did.as_ref() == Some(&did.to_string());

        // Update the AppView URL
        let account = self.get_account_mut(did).unwrap();
        account.app_view_url = app_view_url.clone();

        // If this is the current account, recreate the agent with new AppView
        if is_current_account {
            // Get account data before disposing agent
            let account_data = self.get_account(did).unwrap().clone();

            // Dispose current agent
            self.dispose_current_agent();

            // Recreate agent with new configuration if account has tokens
            if account_data.has_tokens() {
                let service = account_data.pds_url.as_ref().unwrap_or(&account_data.service);

                let mut agent = if let Some(ref app_view) = account_data.app_view_url {
                    let config = crate::agent::BskyAgentConfig::new(service)
                        .with_app_view(app_view);
                    BskyAgent::with_config(config)?
                } else {
                    BskyAgent::new(service)?
                };

                // Resume session
                let session_data = account_data.to_session_data()?;
                agent.resume_session(session_data).await?;

                self.current_agent = Some(Arc::new(RwLock::new(agent)));
            }
        }

        // Persist changes
        self.persist().await
    }

    /// Get the custom AppView URL for an account
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account
    ///
    /// # Returns
    ///
    /// Returns the custom AppView URL if set, None otherwise
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &SessionManager) {
    /// if let Some(account) = manager.get_account("did:plc:abc123") {
    ///     match manager.get_account_app_view("did:plc:abc123") {
    ///         Some(url) => println!("Using custom AppView: {}", url),
    ///         None => println!("Using default AppView (service URL)"),
    ///     }
    /// }
    /// # }
    /// ```
    pub fn get_account_app_view(&self, did: &str) -> Option<String> {
        self.get_account(did)
            .and_then(|account| account.app_view_url.clone())
    }

    /// Export an account for backup or transfer
    ///
    /// Creates an `AccountExport` containing all account data including session credentials,
    /// preferences, and optionally cached data from the KV store.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to export
    /// * `include_tokens` - Whether to include access/refresh tokens (security risk if unencrypted)
    /// * `include_kv_data` - Whether to include KV store data (preferences, cache)
    /// * `kv_store` - Optional reference to KV store for exporting account data
    ///
    /// # Security Considerations
    ///
    /// **WARNING**: Exported files containing tokens should be treated as highly sensitive.
    /// Anyone with access to the tokens can impersonate the user. Consider:
    /// - Only including tokens when absolutely necessary
    /// - Encrypting exports before storage (encryption support planned for future)
    /// - Storing exports in secure locations only
    /// - Deleting exports after use
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// // Export without tokens (safer for long-term storage)
    /// let export = manager.export_account("did:plc:abc123", false, true, None).await?;
    ///
    /// // Export with tokens (use with caution!)
    /// let export_with_tokens = manager.export_account(
    ///     "did:plc:abc123",
    ///     true,
    ///     true,
    ///     None
    /// ).await?;
    ///
    /// // Serialize to JSON
    /// let json = serde_json::to_string_pretty(&export)?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn export_account(
        &self,
        did: &str,
        include_tokens: bool,
        include_kv_data: bool,
        kv_store: Option<&storage::kv::AccountStore>,
    ) -> Result<AccountExport> {
        // Get the account
        let account = self
            .get_account(did)
            .ok_or_else(|| SessionManagerError::AccountNotFound(did.to_string()))?
            .clone();

        // Optionally redact tokens for security
        let mut export_account = account.clone();
        if !include_tokens {
            export_account.access_jwt = None;
            export_account.refresh_jwt = None;
        }

        // Optionally include KV store data
        let account_data = if include_kv_data {
            if let Some(kv) = kv_store {
                Some(kv.export_account_data(did).map_err(|e| {
                    SessionManagerError::InvalidOperation(format!(
                        "Failed to export KV data: {}",
                        e
                    ))
                })?)
            } else {
                None
            }
        } else {
            None
        };

        // Get current timestamp in ISO 8601 format
        let exported_at = chrono::Utc::now().to_rfc3339();

        Ok(AccountExport {
            version: 1,
            exported_at,
            account: export_account,
            account_data,
            tokens_included: include_tokens,
            encrypted: false, // Reserved for future encryption support
        })
    }

    /// Export all accounts for backup
    ///
    /// Creates a vector of `AccountExport` for all accounts in the manager.
    ///
    /// # Arguments
    ///
    /// * `include_tokens` - Whether to include access/refresh tokens
    /// * `include_kv_data` - Whether to include KV store data
    /// * `kv_store` - Optional reference to KV store for exporting account data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::SessionManager;
    /// # async fn example(manager: &SessionManager) -> Result<(), Box<dyn std::error::Error>> {
    /// let exports = manager.export_all_accounts(false, true, None).await?;
    /// println!("Exported {} accounts", exports.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn export_all_accounts(
        &self,
        include_tokens: bool,
        include_kv_data: bool,
        kv_store: Option<&storage::kv::AccountStore>,
    ) -> Result<Vec<AccountExport>> {
        let mut exports = Vec::new();

        for account in &self.accounts {
            let export = self
                .export_account(&account.did, include_tokens, include_kv_data, kv_store)
                .await?;
            exports.push(export);
        }

        Ok(exports)
    }

    /// Import an account from an export
    ///
    /// Imports an account from an `AccountExport`, optionally merging or replacing
    /// existing account data.
    ///
    /// # Arguments
    ///
    /// * `export` - The account export to import
    /// * `merge` - If true, merge with existing data; if false, replace existing account
    /// * `kv_store` - Optional reference to KV store for importing account data
    ///
    /// # Behavior
    ///
    /// - If `merge` is true and account exists: Updates session data, merges KV data
    /// - If `merge` is false and account exists: Replaces all account data
    /// - If account doesn't exist: Creates new account regardless of merge setting
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use atproto_client::session::{SessionManager, AccountExport};
    /// # async fn example(manager: &mut SessionManager, export: AccountExport) -> Result<(), Box<dyn std::error::Error>> {
    /// // Import and merge with existing data
    /// manager.import_account(export.clone(), true, None).await?;
    ///
    /// // Import and replace existing data
    /// manager.import_account(export, false, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn import_account(
        &mut self,
        export: AccountExport,
        merge: bool,
        kv_store: Option<&storage::kv::AccountStore>,
    ) -> Result<()> {
        // Validate schema version
        if export.version > 1 {
            return Err(SessionManagerError::InvalidOperation(format!(
                "Unsupported export version: {}. This version of the app only supports version 1.",
                export.version
            )));
        }

        // Check if account already exists
        let account_exists = self.get_account(&export.account.did).is_some();

        if account_exists && !merge {
            // Replace mode: remove existing account data first
            if let Some(kv) = kv_store {
                kv.remove_account(&export.account.did).map_err(|e| {
                    SessionManagerError::InvalidOperation(format!(
                        "Failed to remove existing KV data: {}",
                        e
                    ))
                })?;
            }
        }

        // Add or update the account in session manager
        self.add_account(export.account.clone()).await?;

        // Import KV store data if provided
        if let Some(account_data) = export.account_data {
            if let Some(kv) = kv_store {
                kv.import_account_data(&account_data).map_err(|e| {
                    SessionManagerError::InvalidOperation(format!("Failed to import KV data: {}", e))
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_creation() {
        let manager = SessionManager::new_in_memory().await.unwrap();
        assert_eq!(manager.list_accounts().len(), 0);
        assert!(manager.current_account().is_none());
        assert!(manager.current_agent().is_none());
    }

    #[tokio::test]
    async fn test_add_account() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        manager.add_account(account.clone()).await.unwrap();

        assert_eq!(manager.list_accounts().len(), 1);
        assert_eq!(manager.get_account("did:plc:abc123").unwrap().handle, "alice.bsky.social");
    }

    #[tokio::test]
    async fn test_add_multiple_accounts() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account1 = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        let account2 = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:def456".to_string(),
            "bob.bsky.social".to_string(),
        );

        manager.add_account(account1).await.unwrap();
        manager.add_account(account2).await.unwrap();

        assert_eq!(manager.list_accounts().len(), 2);
    }

    #[tokio::test]
    async fn test_update_existing_account() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        manager.add_account(account.clone()).await.unwrap();

        // Update the account
        account.email = Some("alice@example.com".to_string());
        manager.add_account(account.clone()).await.unwrap();

        // Should still have only one account
        assert_eq!(manager.list_accounts().len(), 1);
        assert_eq!(
            manager.get_account("did:plc:abc123").unwrap().email,
            Some("alice@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn test_remove_account() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        manager.add_account(account).await.unwrap();
        assert_eq!(manager.list_accounts().len(), 1);

        manager.remove_account("did:plc:abc123").await.unwrap();
        assert_eq!(manager.list_accounts().len(), 0);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_account() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let result = manager.remove_account("did:plc:nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionManagerError::AccountNotFound(_)));
    }

    #[tokio::test]
    async fn test_get_account() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        manager.add_account(account).await.unwrap();

        let retrieved = manager.get_account("did:plc:abc123").unwrap();
        assert_eq!(retrieved.handle, "alice.bsky.social");

        assert!(manager.get_account("did:plc:nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_persistence_survives_restart() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test_sessions.json");

        // First instance
        {
            let mut manager = SessionManager::new(&path).await.unwrap();
            let account = SessionAccount::new(
                "https://bsky.social".to_string(),
                "did:plc:abc123".to_string(),
                "alice.bsky.social".to_string(),
            );
            manager.add_account(account).await.unwrap();
        }

        // Second instance (simulating restart)
        {
            let manager = SessionManager::new(&path).await.unwrap();
            assert_eq!(manager.list_accounts().len(), 1);
            assert_eq!(manager.get_account("did:plc:abc123").unwrap().handle, "alice.bsky.social");
        }
    }

    #[tokio::test]
    async fn test_logout_current_preserves_account() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );
        account.access_jwt = Some("access".to_string());
        account.refresh_jwt = Some("refresh".to_string());

        manager.add_account(account).await.unwrap();
        manager.current_did = Some("did:plc:abc123".to_string());

        manager.logout_current().await.unwrap();

        // Account should still exist but with cleared tokens
        assert_eq!(manager.list_accounts().len(), 1);
        let account = manager.get_account("did:plc:abc123").unwrap();
        assert!(account.access_jwt.is_none());
        assert!(account.refresh_jwt.is_none());
        assert_eq!(account.active, Some(false));
        assert!(manager.current_account().is_none());
    }

    #[tokio::test]
    async fn test_logout_all() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account1 = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        let account2 = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:def456".to_string(),
            "bob.bsky.social".to_string(),
        );

        manager.add_account(account1).await.unwrap();
        manager.add_account(account2).await.unwrap();

        manager.logout_all().await.unwrap();

        assert_eq!(manager.list_accounts().len(), 0);
        assert!(manager.current_account().is_none());
    }

    #[tokio::test]
    async fn test_switch_account_without_tokens() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );

        manager.add_account(account).await.unwrap();

        let result = manager.switch_account("did:plc:abc123").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionManagerError::InvalidOperation(_)));
    }

    #[tokio::test]
    async fn test_storage_serialization() {
        let storage = SessionStorage {
            accounts: vec![
                SessionAccount::new(
                    "https://bsky.social".to_string(),
                    "did:plc:abc123".to_string(),
                    "alice.bsky.social".to_string(),
                ),
                SessionAccount::new(
                    "https://bsky.social".to_string(),
                    "did:plc:def456".to_string(),
                    "bob.bsky.social".to_string(),
                ),
            ],
            current_account_did: Some("did:plc:abc123".to_string()),
        };

        let json = serde_json::to_string(&storage).unwrap();
        let deserialized: SessionStorage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.accounts.len(), 2);
        assert_eq!(deserialized.current_account_did, Some("did:plc:abc123".to_string()));
    }

    #[tokio::test]
    async fn test_set_account_app_view() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );
        account.access_jwt = Some("access_token".to_string());
        account.refresh_jwt = Some("refresh_token".to_string());

        manager.add_account(account).await.unwrap();

        // Set custom AppView
        manager
            .set_account_app_view("did:plc:abc123", Some("https://api.bsky.app".to_string()))
            .await
            .unwrap();

        // Verify it was set
        let app_view = manager.get_account_app_view("did:plc:abc123");
        assert_eq!(app_view, Some("https://api.bsky.app".to_string()));

        // Verify it's persisted
        let account = manager.get_account("did:plc:abc123").unwrap();
        assert_eq!(
            account.app_view_url,
            Some("https://api.bsky.app".to_string())
        );
    }

    #[tokio::test]
    async fn test_clear_account_app_view() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:abc123".to_string(),
            "alice.bsky.social".to_string(),
        );
        account.access_jwt = Some("access_token".to_string());
        account.refresh_jwt = Some("refresh_token".to_string());
        account.app_view_url = Some("https://api.bsky.app".to_string());

        manager.add_account(account).await.unwrap();

        // Clear custom AppView
        manager
            .set_account_app_view("did:plc:abc123", None)
            .await
            .unwrap();

        // Verify it was cleared
        let app_view = manager.get_account_app_view("did:plc:abc123");
        assert_eq!(app_view, None);
    }

    #[tokio::test]
    async fn test_set_app_view_nonexistent_account() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let result = manager
            .set_account_app_view(
                "did:plc:nonexistent",
                Some("https://api.bsky.app".to_string()),
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SessionManagerError::AccountNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_get_app_view_nonexistent_account() {
        let manager = SessionManager::new_in_memory().await.unwrap();

        let app_view = manager.get_account_app_view("did:plc:nonexistent");
        assert_eq!(app_view, None);
    }

    #[tokio::test]
    async fn test_app_view_persistence() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test_app_view_sessions.json");

        // First instance - set AppView
        {
            let mut manager = SessionManager::new(&path).await.unwrap();

            let mut account = SessionAccount::new(
                "https://bsky.social".to_string(),
                "did:plc:abc123".to_string(),
                "alice.bsky.social".to_string(),
            );
            account.access_jwt = Some("access_token".to_string());
            account.refresh_jwt = Some("refresh_token".to_string());

            manager.add_account(account).await.unwrap();
            manager
                .set_account_app_view("did:plc:abc123", Some("https://custom.appview".to_string()))
                .await
                .unwrap();
        }

        // Second instance - verify AppView persisted
        {
            let manager = SessionManager::new(&path).await.unwrap();

            let app_view = manager.get_account_app_view("did:plc:abc123");
            assert_eq!(app_view, Some("https://custom.appview".to_string()));

            let account = manager.get_account("did:plc:abc123").unwrap();
            assert_eq!(
                account.app_view_url,
                Some("https://custom.appview".to_string())
            );
        }
    }

    // Export/Import Tests

    #[tokio::test]
    async fn test_export_account_without_tokens() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "test.bsky.social".to_string(),
        );
        account.access_jwt = Some("secret_access_token".to_string());
        account.refresh_jwt = Some("secret_refresh_token".to_string());
        account.email = Some("test@example.com".to_string());

        manager.add_account(account).await.unwrap();

        // Export without tokens
        let export = manager
            .export_account("did:plc:test123", false, false, None)
            .await
            .unwrap();

        // Verify tokens were redacted
        assert_eq!(export.version, 1);
        assert_eq!(export.account.did, "did:plc:test123");
        assert_eq!(export.account.handle, "test.bsky.social");
        assert_eq!(export.account.email, Some("test@example.com".to_string()));
        assert!(export.account.access_jwt.is_none());
        assert!(export.account.refresh_jwt.is_none());
        assert!(!export.tokens_included);
        assert!(!export.encrypted);
        assert!(export.account_data.is_none());
    }

    #[tokio::test]
    async fn test_export_account_with_tokens() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "test.bsky.social".to_string(),
        );
        account.access_jwt = Some("secret_access_token".to_string());
        account.refresh_jwt = Some("secret_refresh_token".to_string());

        manager.add_account(account).await.unwrap();

        // Export with tokens
        let export = manager
            .export_account("did:plc:test123", true, false, None)
            .await
            .unwrap();

        // Verify tokens were included
        assert_eq!(
            export.account.access_jwt,
            Some("secret_access_token".to_string())
        );
        assert_eq!(
            export.account.refresh_jwt,
            Some("secret_refresh_token".to_string())
        );
        assert!(export.tokens_included);
    }

    #[tokio::test]
    async fn test_export_account_with_kv_data() {
        use storage::kv::{AccountStore, KvStore};
        use std::sync::Arc;

        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "test.bsky.social".to_string(),
        );
        manager.add_account(account).await.unwrap();

        // Create KV store and add some data
        let kv = Arc::new(KvStore::in_memory().unwrap());
        let account_store = AccountStore::new(kv);
        account_store
            .set("did:plc:test123", "preference1", &"value1".to_string())
            .unwrap();
        account_store
            .set("did:plc:test123", "preference2", &42i32)
            .unwrap();

        // Export with KV data
        let export = manager
            .export_account("did:plc:test123", false, true, Some(&account_store))
            .await
            .unwrap();

        // Verify KV data was included
        assert!(export.account_data.is_some());
        let kv_data = export.account_data.unwrap();
        assert_eq!(kv_data.account_id, "did:plc:test123");
        assert_eq!(kv_data.key_count, 2);
        assert!(kv_data.size_bytes > 0);
    }

    #[tokio::test]
    async fn test_export_all_accounts() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account1 = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:alice".to_string(),
            "alice.bsky.social".to_string(),
        );
        let account2 = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:bob".to_string(),
            "bob.bsky.social".to_string(),
        );

        manager.add_account(account1).await.unwrap();
        manager.add_account(account2).await.unwrap();

        // Export all accounts
        let exports = manager
            .export_all_accounts(false, false, None)
            .await
            .unwrap();

        assert_eq!(exports.len(), 2);
        assert!(exports.iter().any(|e| e.account.did == "did:plc:alice"));
        assert!(exports.iter().any(|e| e.account.did == "did:plc:bob"));
    }

    #[tokio::test]
    async fn test_import_account_new() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:imported".to_string(),
            "imported.bsky.social".to_string(),
        );
        account.email = Some("imported@example.com".to_string());

        let export = AccountExport {
            version: 1,
            exported_at: chrono::Utc::now().to_rfc3339(),
            account: account.clone(),
            account_data: None,
            tokens_included: false,
            encrypted: false,
        };

        // Import new account
        manager.import_account(export, false, None).await.unwrap();

        // Verify account was imported
        assert_eq!(manager.list_accounts().len(), 1);
        let imported = manager.get_account("did:plc:imported").unwrap();
        assert_eq!(imported.handle, "imported.bsky.social");
        assert_eq!(imported.email, Some("imported@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_import_account_merge() {
        use storage::kv::{AccountStore, KvStore};
        use std::sync::Arc;

        let mut manager = SessionManager::new_in_memory().await.unwrap();

        // Create existing account
        let mut existing = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "test.bsky.social".to_string(),
        );
        existing.email = Some("old@example.com".to_string());
        manager.add_account(existing).await.unwrap();

        // Create KV store with existing data
        let kv = Arc::new(KvStore::in_memory().unwrap());
        let account_store = AccountStore::new(kv);
        account_store
            .set("did:plc:test123", "old_key", &"old_value".to_string())
            .unwrap();

        // Create import with updated email
        let mut updated = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "test.bsky.social".to_string(),
        );
        updated.email = Some("new@example.com".to_string());

        let export = AccountExport {
            version: 1,
            exported_at: chrono::Utc::now().to_rfc3339(),
            account: updated,
            account_data: None,
            tokens_included: false,
            encrypted: false,
        };

        // Import with merge
        manager
            .import_account(export, true, Some(&account_store))
            .await
            .unwrap();

        // Verify account was updated
        let imported = manager.get_account("did:plc:test123").unwrap();
        assert_eq!(imported.email, Some("new@example.com".to_string()));

        // Verify old KV data still exists (merge mode)
        let old_value: Option<String> = account_store.get("did:plc:test123", "old_key").unwrap();
        assert_eq!(old_value, Some("old_value".to_string()));
    }

    #[tokio::test]
    async fn test_import_account_replace() {
        use storage::kv::{AccountStore, KvStore, AccountDataExport};
        use std::sync::Arc;

        let mut manager = SessionManager::new_in_memory().await.unwrap();

        // Create existing account
        let existing = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "test.bsky.social".to_string(),
        );
        manager.add_account(existing).await.unwrap();

        // Create KV store with existing data
        let kv = Arc::new(KvStore::in_memory().unwrap());
        let account_store = AccountStore::new(kv);
        account_store
            .set("did:plc:test123", "old_key", &"old_value".to_string())
            .unwrap();

        // Create import with new KV data
        let updated = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "test.bsky.social".to_string(),
        );

        let kv_export = AccountDataExport {
            account_id: "did:plc:test123".to_string(),
            data: vec![("new_key".to_string(), serde_json::json!("new_value"))],
            size_bytes: 100,
            key_count: 1,
        };

        let export = AccountExport {
            version: 1,
            exported_at: chrono::Utc::now().to_rfc3339(),
            account: updated,
            account_data: Some(kv_export),
            tokens_included: false,
            encrypted: false,
        };

        // Import with replace
        manager
            .import_account(export, false, Some(&account_store))
            .await
            .unwrap();

        // Verify old KV data was removed (replace mode)
        let old_value: Option<String> = account_store.get("did:plc:test123", "old_key").unwrap();
        assert!(old_value.is_none());

        // Verify new KV data was added
        let new_value: Option<String> = account_store.get("did:plc:test123", "new_key").unwrap();
        assert_eq!(new_value, Some("new_value".to_string()));
    }

    #[tokio::test]
    async fn test_import_export_roundtrip() {
        use storage::kv::{AccountStore, KvStore};
        use std::sync::Arc;

        let mut manager1 = SessionManager::new_in_memory().await.unwrap();

        // Create account with data
        let mut account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:roundtrip".to_string(),
            "roundtrip.bsky.social".to_string(),
        );
        account.email = Some("roundtrip@example.com".to_string());
        account.access_jwt = Some("access_token".to_string());
        account.refresh_jwt = Some("refresh_token".to_string());

        manager1.add_account(account).await.unwrap();

        // Create KV store with data
        let kv = Arc::new(KvStore::in_memory().unwrap());
        let account_store = AccountStore::new(kv.clone());
        account_store
            .set("did:plc:roundtrip", "test_key", &"test_value".to_string())
            .unwrap();

        // Export
        let export = manager1
            .export_account("did:plc:roundtrip", true, true, Some(&account_store))
            .await
            .unwrap();

        // Import to new manager
        let mut manager2 = SessionManager::new_in_memory().await.unwrap();
        let kv2 = Arc::new(KvStore::in_memory().unwrap());
        let account_store2 = AccountStore::new(kv2);

        manager2
            .import_account(export, false, Some(&account_store2))
            .await
            .unwrap();

        // Verify everything was transferred
        let imported = manager2.get_account("did:plc:roundtrip").unwrap();
        assert_eq!(imported.handle, "roundtrip.bsky.social");
        assert_eq!(imported.email, Some("roundtrip@example.com".to_string()));
        assert_eq!(imported.access_jwt, Some("access_token".to_string()));
        assert_eq!(imported.refresh_jwt, Some("refresh_token".to_string()));

        let kv_value: Option<String> = account_store2
            .get("did:plc:roundtrip", "test_key")
            .unwrap();
        assert_eq!(kv_value, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_import_unsupported_version() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test".to_string(),
            "test.bsky.social".to_string(),
        );

        let export = AccountExport {
            version: 99, // Future version
            exported_at: chrono::Utc::now().to_rfc3339(),
            account,
            account_data: None,
            tokens_included: false,
            encrypted: false,
        };

        // Should fail with unsupported version error
        let result = manager.import_account(export, false, None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SessionManagerError::InvalidOperation(_)
        ));
    }

    #[tokio::test]
    async fn test_export_nonexistent_account() {
        let manager = SessionManager::new_in_memory().await.unwrap();

        let result = manager
            .export_account("did:plc:nonexistent", false, false, None)
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SessionManagerError::AccountNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_export_serialization() {
        let mut manager = SessionManager::new_in_memory().await.unwrap();

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test".to_string(),
            "test.bsky.social".to_string(),
        );
        manager.add_account(account).await.unwrap();

        let export = manager
            .export_account("did:plc:test", false, false, None)
            .await
            .unwrap();

        // Verify JSON serialization
        let json = serde_json::to_string(&export).unwrap();
        assert!(json.contains("did:plc:test"));
        assert!(json.contains("test.bsky.social"));

        // Verify deserialization
        let deserialized: AccountExport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.account.did, "did:plc:test");
        assert_eq!(deserialized.version, 1);
    }
}
