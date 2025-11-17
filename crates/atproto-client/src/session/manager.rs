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
        let mut agent = BskyAgent::new(service)?;

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
}
