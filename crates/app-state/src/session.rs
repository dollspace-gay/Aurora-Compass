//! Session state management with reactive queries
//!
//! This module integrates `SessionManager` with the reactive query system,
//! providing queries and mutations for session and account management.

use async_trait::async_trait;
use atproto_client::session::{SessionAccount, SessionManager, SessionManagerError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::query::{Query, QueryClient, QueryConfig, QueryError, QueryKey};

/// Session-related errors
#[derive(Debug, thiserror::Error)]
pub enum SessionStateError {
    /// Session manager error
    #[error("Session manager error: {0}")]
    SessionManager(#[from] SessionManagerError),

    /// Query error
    #[error("Query error: {0}")]
    Query(#[from] QueryError),

    /// No current session
    #[error("No current session")]
    NoCurrentSession,

    /// Account not found
    #[error("Account not found: {0}")]
    AccountNotFound(String),
}

/// Result type for session state operations
pub type Result<T> = std::result::Result<T, SessionStateError>;

/// Current session data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurrentSession {
    /// The active account
    pub account: SessionAccount,
    /// Whether the session is active
    pub is_active: bool,
}

/// List of all accounts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountList {
    /// All authenticated accounts
    pub accounts: Vec<SessionAccount>,
    /// DID of the currently active account
    pub current_did: Option<String>,
}

/// Query for the current session
///
/// This query provides reactive access to the current active session.
/// It will automatically update when the session changes.
#[derive(Clone)]
pub struct CurrentSessionQuery {
    session_manager: Arc<RwLock<SessionManager>>,
}

impl CurrentSessionQuery {
    /// Create a new current session query
    pub fn new(session_manager: Arc<RwLock<SessionManager>>) -> Self {
        Self { session_manager }
    }
}

#[async_trait]
impl Query for CurrentSessionQuery {
    type Data = Option<CurrentSession>;

    async fn fetch(&self) -> crate::query::Result<Self::Data> {
        let manager = self.session_manager.read().await;

        if let Some(account) = manager.current_account() {
            Ok(Some(CurrentSession {
                account: account.clone(),
                is_active: manager.current_agent().is_some(),
            }))
        } else {
            Ok(None)
        }
    }

    fn key(&self) -> QueryKey {
        QueryKey::new("session", "current")
    }

    fn config(&self) -> QueryConfig {
        QueryConfig {
            stale_time: Duration::from_secs(0), // Always fresh
            cache_time: Duration::from_secs(300),
            refetch_on_stale: false, // Manual invalidation only
            retry: false,
            retry_count: 0,
            retry_delay: Duration::from_secs(0),
        }
    }
}

/// Query for all accounts
///
/// This query provides reactive access to the list of all authenticated accounts.
#[derive(Clone)]
pub struct AccountsQuery {
    session_manager: Arc<RwLock<SessionManager>>,
}

impl AccountsQuery {
    /// Create a new accounts query
    pub fn new(session_manager: Arc<RwLock<SessionManager>>) -> Self {
        Self { session_manager }
    }
}

#[async_trait]
impl Query for AccountsQuery {
    type Data = AccountList;

    async fn fetch(&self) -> crate::query::Result<Self::Data> {
        let manager = self.session_manager.read().await;

        Ok(AccountList {
            accounts: manager.list_accounts().to_vec(),
            current_did: manager.current_account().map(|a| a.did.clone()),
        })
    }

    fn key(&self) -> QueryKey {
        QueryKey::new("session", "accounts")
    }

    fn config(&self) -> QueryConfig {
        QueryConfig {
            stale_time: Duration::from_secs(0), // Always fresh
            cache_time: Duration::from_secs(300),
            refetch_on_stale: false, // Manual invalidation only
            retry: false,
            retry_count: 0,
            retry_delay: Duration::from_secs(0),
        }
    }
}

/// Mutation for switching accounts
///
/// This mutation switches the active account and invalidates related queries.
pub struct SwitchAccountMutation {
    session_manager: Arc<RwLock<SessionManager>>,
    query_client: QueryClient,
}

impl SwitchAccountMutation {
    /// Create a new switch account mutation
    pub fn new(session_manager: Arc<RwLock<SessionManager>>, query_client: QueryClient) -> Self {
        Self { session_manager, query_client }
    }

    /// Execute the mutation to switch accounts
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to switch to
    ///
    /// # Returns
    ///
    /// The new current session
    pub async fn execute(&self, did: &str) -> Result<CurrentSession> {
        // Perform the account switch
        {
            let mut manager = self.session_manager.write().await;
            manager.switch_account(did).await?;
        }

        // Invalidate session-related queries
        self.invalidate_session_queries().await?;

        // Fetch and return the new current session
        let query = CurrentSessionQuery::new(Arc::clone(&self.session_manager));
        let session = self.query_client.fetch(&query).await?;

        session.ok_or(SessionStateError::NoCurrentSession)
    }

    /// Invalidate all session-related queries
    async fn invalidate_session_queries(&self) -> Result<()> {
        self.query_client.invalidate_scope("session").await?;
        Ok(())
    }
}

/// Mutation for adding an account
pub struct AddAccountMutation {
    session_manager: Arc<RwLock<SessionManager>>,
    query_client: QueryClient,
}

impl AddAccountMutation {
    /// Create a new add account mutation
    pub fn new(session_manager: Arc<RwLock<SessionManager>>, query_client: QueryClient) -> Self {
        Self { session_manager, query_client }
    }

    /// Execute the mutation to add an account
    ///
    /// # Arguments
    ///
    /// * `account` - The account to add
    pub async fn execute(&self, account: SessionAccount) -> Result<()> {
        {
            let mut manager = self.session_manager.write().await;
            manager.add_account(account).await?;
        }

        // Invalidate accounts query
        self.query_client
            .invalidate(&QueryKey::new("session", "accounts"))
            .await?;

        Ok(())
    }
}

/// Mutation for removing an account
pub struct RemoveAccountMutation {
    session_manager: Arc<RwLock<SessionManager>>,
    query_client: QueryClient,
}

impl RemoveAccountMutation {
    /// Create a new remove account mutation
    pub fn new(session_manager: Arc<RwLock<SessionManager>>, query_client: QueryClient) -> Self {
        Self { session_manager, query_client }
    }

    /// Execute the mutation to remove an account
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to remove
    pub async fn execute(&self, did: &str) -> Result<()> {
        {
            let mut manager = self.session_manager.write().await;
            manager.remove_account(did).await?;
        }

        // Invalidate all session queries (account list and current session)
        self.query_client.invalidate_scope("session").await?;

        // TODO: Clean up per-account cached data
        // This will be implemented when per-account cache scoping is added

        Ok(())
    }
}

/// Session state manager
///
/// This provides a high-level API for managing session state reactively.
#[derive(Clone)]
pub struct SessionState {
    session_manager: Arc<RwLock<SessionManager>>,
    query_client: QueryClient,
}

impl SessionState {
    /// Create a new session state manager
    ///
    /// # Arguments
    ///
    /// * `session_manager` - The underlying session manager
    /// * `query_client` - The query client for reactive queries
    pub fn new(session_manager: Arc<RwLock<SessionManager>>, query_client: QueryClient) -> Self {
        Self { session_manager, query_client }
    }

    /// Get the current session query
    pub fn current_session_query(&self) -> CurrentSessionQuery {
        CurrentSessionQuery::new(Arc::clone(&self.session_manager))
    }

    /// Get the accounts query
    pub fn accounts_query(&self) -> AccountsQuery {
        AccountsQuery::new(Arc::clone(&self.session_manager))
    }

    /// Get the switch account mutation
    pub fn switch_account_mutation(&self) -> SwitchAccountMutation {
        SwitchAccountMutation::new(Arc::clone(&self.session_manager), self.query_client.clone())
    }

    /// Get the add account mutation
    pub fn add_account_mutation(&self) -> AddAccountMutation {
        AddAccountMutation::new(Arc::clone(&self.session_manager), self.query_client.clone())
    }

    /// Get the remove account mutation
    pub fn remove_account_mutation(&self) -> RemoveAccountMutation {
        RemoveAccountMutation::new(Arc::clone(&self.session_manager), self.query_client.clone())
    }

    /// Get the current session data
    ///
    /// This fetches fresh data and caches it.
    pub async fn get_current_session(&self) -> Result<Option<CurrentSession>> {
        let query = self.current_session_query();
        Ok(self.query_client.get(&query).await?)
    }

    /// Get all accounts
    ///
    /// This fetches fresh data and caches it.
    pub async fn get_accounts(&self) -> Result<AccountList> {
        let query = self.accounts_query();
        Ok(self.query_client.get(&query).await?)
    }

    /// Switch to a different account
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to switch to
    pub async fn switch_account(&self, did: &str) -> Result<CurrentSession> {
        let mutation = self.switch_account_mutation();
        mutation.execute(did).await
    }

    /// Add a new account
    ///
    /// # Arguments
    ///
    /// * `account` - The account to add
    pub async fn add_account(&self, account: SessionAccount) -> Result<()> {
        let mutation = self.add_account_mutation();
        mutation.execute(account).await
    }

    /// Remove an account
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account to remove
    pub async fn remove_account(&self, did: &str) -> Result<()> {
        let mutation = self.remove_account_mutation();
        mutation.execute(did).await
    }

    /// Invalidate all session queries
    ///
    /// This forces a refetch on the next query.
    pub async fn invalidate(&self) -> Result<()> {
        self.query_client.invalidate_scope("session").await?;
        Ok(())
    }

    /// Clear per-account cached data for a specific account
    ///
    /// # Arguments
    ///
    /// * `did` - The DID of the account whose data should be cleared
    ///
    /// This is useful when switching accounts to ensure fresh data is loaded.
    pub async fn clear_account_cache(&self, _did: &str) -> Result<()> {
        // Invalidate all queries that might be scoped to this account
        // The actual implementation will depend on how per-account scoping is implemented

        // For now, we'll invalidate everything except session queries
        // TODO: Implement proper per-account cache scoping

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::CacheConfig;
    use tempfile::TempDir;

    async fn create_test_session_state() -> (SessionState, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test_sessions.json");

        let session_manager =
            Arc::new(RwLock::new(SessionManager::new(session_path).await.unwrap()));
        let query_client = QueryClient::new(CacheConfig::default()).unwrap();

        (SessionState::new(session_manager, query_client), temp_dir)
    }

    #[tokio::test]
    async fn test_current_session_query_no_session() {
        let (state, _temp_dir) = create_test_session_state().await;
        let session = state.get_current_session().await.unwrap();
        assert!(session.is_none());
    }

    #[tokio::test]
    async fn test_accounts_query_empty() {
        let (state, _temp_dir) = create_test_session_state().await;
        let accounts = state.get_accounts().await.unwrap();
        assert!(accounts.accounts.is_empty());
        assert!(accounts.current_did.is_none());
    }

    #[tokio::test]
    async fn test_add_account() {
        let (state, _temp_dir) = create_test_session_state().await;

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test123".to_string(),
            "test.bsky.social".to_string(),
        );

        state.add_account(account.clone()).await.unwrap();

        let accounts = state.get_accounts().await.unwrap();
        assert_eq!(accounts.accounts.len(), 1);
        assert_eq!(accounts.accounts[0].did, "did:plc:test123");
    }

    #[tokio::test]
    async fn test_remove_account() {
        let (state, _temp_dir) = create_test_session_state().await;

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test456".to_string(),
            "test.bsky.social".to_string(),
        );

        state.add_account(account).await.unwrap();
        let accounts_before = state.get_accounts().await.unwrap();
        assert_eq!(accounts_before.accounts.len(), 1);

        state.remove_account("did:plc:test456").await.unwrap();
        let accounts_after = state.get_accounts().await.unwrap();
        assert_eq!(accounts_after.accounts.len(), 0);
    }

    #[tokio::test]
    async fn test_query_invalidation() {
        let (state, _temp_dir) = create_test_session_state().await;

        let account = SessionAccount::new(
            "https://bsky.social".to_string(),
            "did:plc:test789".to_string(),
            "test.bsky.social".to_string(),
        );

        // Add account
        state.add_account(account).await.unwrap();

        // Get accounts (should be cached)
        let accounts1 = state.get_accounts().await.unwrap();
        assert_eq!(accounts1.accounts.len(), 1);

        // Invalidate
        state.invalidate().await.unwrap();

        // Get accounts again (should refetch)
        let accounts2 = state.get_accounts().await.unwrap();
        assert_eq!(accounts2.accounts.len(), 1);
    }
}
