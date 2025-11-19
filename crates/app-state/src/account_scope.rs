//! Per-account cache scoping utilities
//!
//! This module provides utilities for scoping query caches by account DID,
//! ensuring proper data isolation between accounts.

use crate::query::{QueryClient, QueryError, QueryKey};

/// Account scope manager
///
/// Manages cache scoping for per-account data, ensuring that cached data
/// is properly isolated between different accounts.
pub struct AccountScopeManager {
    query_client: QueryClient,
}

impl AccountScopeManager {
    /// Create a new account scope manager
    pub fn new(query_client: QueryClient) -> Self {
        Self { query_client }
    }

    /// Create a scoped query key for account-specific data
    ///
    /// # Arguments
    ///
    /// * `account_did` - The DID of the account
    /// * `scope` - The data scope (e.g., "posts", "profiles", "feeds")
    /// * `id` - The identifier within the scope
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_state::account_scope::AccountScopeManager;
    /// # use app_state::query::{QueryClient, QueryKey};
    /// # use storage::CacheConfig;
    /// # async fn example() {
    /// let query_client = QueryClient::new(CacheConfig::default()).unwrap();
    /// let scope_manager = AccountScopeManager::new(query_client);
    ///
    /// let key = scope_manager.scoped_key(
    ///     "did:plc:abc123",
    ///     "posts",
    ///     "timeline"
    /// );
    /// assert_eq!(key.scope, "account:did:plc:abc123:posts");
    /// # }
    /// ```
    pub fn scoped_key(&self, account_did: &str, scope: &str, id: &str) -> QueryKey {
        QueryKey::new(format!("account:{}:{}", account_did, scope), id)
    }

    /// Invalidate all cached data for a specific account
    ///
    /// # Arguments
    ///
    /// * `account_did` - The DID of the account whose cache should be cleared
    ///
    /// This removes all queries scoped to the account, useful when switching
    /// accounts or logging out.
    pub async fn invalidate_account(&self, account_did: &str) -> Result<(), QueryError> {
        let scope_prefix = format!("account:{}", account_did);
        self.query_client.invalidate_scope(&scope_prefix).await
    }

    /// Invalidate a specific scope for an account
    ///
    /// # Arguments
    ///
    /// * `account_did` - The DID of the account
    /// * `scope` - The scope to invalidate (e.g., "posts", "profiles")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use app_state::account_scope::AccountScopeManager;
    /// # use app_state::query::QueryClient;
    /// # use storage::CacheConfig;
    /// # async fn example() {
    /// # let query_client = QueryClient::new(CacheConfig::default()).unwrap();
    /// let scope_manager = AccountScopeManager::new(query_client);
    ///
    /// // Invalidate all posts for this account
    /// scope_manager.invalidate_account_scope("did:plc:abc123", "posts").await.unwrap();
    /// # }
    /// ```
    pub async fn invalidate_account_scope(
        &self,
        account_did: &str,
        scope: &str,
    ) -> Result<(), QueryError> {
        let full_scope = format!("account:{}:{}", account_did, scope);
        self.query_client.invalidate_scope(&full_scope).await
    }

    /// Invalidate all accounts except the current one
    ///
    /// # Arguments
    ///
    /// * `current_did` - The DID of the account to keep cached
    /// * `all_dids` - List of all account DIDs
    ///
    /// This is useful when switching accounts to clear stale data from
    /// inactive accounts while preserving the new active account's cache.
    pub async fn invalidate_except(
        &self,
        current_did: &str,
        all_dids: &[String],
    ) -> Result<(), QueryError> {
        for did in all_dids {
            if did != current_did {
                self.invalidate_account(did).await?;
            }
        }
        Ok(())
    }

    /// Clear cache for orphaned accounts
    ///
    /// # Arguments
    ///
    /// * `valid_dids` - List of DIDs for accounts that should be kept
    ///
    /// This removes cached data for accounts that no longer exist,
    /// useful for cleanup after account deletion.
    pub async fn cleanup_orphaned_caches(
        &self,
        _valid_dids: &[String],
    ) -> Result<(), QueryError> {
        // This is a placeholder implementation
        // In a real implementation, we would need to:
        // 1. Scan all cache keys
        // 2. Identify account-scoped keys
        // 3. Extract DIDs from those keys
        // 4. Remove keys for DIDs not in valid_dids

        // For now, we'll just document the intended behavior
        // TODO: Implement cache key scanning when QueryClient supports it

        Ok(())
    }
}

/// Helper trait for creating account-scoped query keys
pub trait AccountScoped {
    /// Create an account-scoped query key
    fn account_key(account_did: &str, scope: &str, id: &str) -> QueryKey {
        QueryKey::new(format!("account:{}:{}", account_did, scope), id)
    }

    /// Create an account-scoped query key with parameters
    fn account_key_with_params(
        account_did: &str,
        scope: &str,
        id: &str,
        params: Vec<(&str, &str)>,
    ) -> QueryKey {
        let mut key = Self::account_key(account_did, scope, id);
        for (k, v) in params {
            key = key.with_param(k, v);
        }
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::CacheConfig;

    #[tokio::test]
    async fn test_scoped_key_creation() {
        let query_client = QueryClient::new(CacheConfig::default()).unwrap();
        let manager = AccountScopeManager::new(query_client);

        let key = manager.scoped_key("did:plc:abc123", "posts", "timeline");

        assert_eq!(key.scope, "account:did:plc:abc123:posts");
        assert_eq!(key.id, "timeline");
    }

    #[tokio::test]
    async fn test_invalidate_account() {
        let query_client = QueryClient::new(CacheConfig::default()).unwrap();
        let manager = AccountScopeManager::new(query_client);

        // This should not error even if no data exists
        let result = manager.invalidate_account("did:plc:test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalidate_account_scope() {
        let query_client = QueryClient::new(CacheConfig::default()).unwrap();
        let manager = AccountScopeManager::new(query_client);

        // This should not error even if no data exists
        let result = manager
            .invalidate_account_scope("did:plc:test", "posts")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalidate_except() {
        let query_client = QueryClient::new(CacheConfig::default()).unwrap();
        let manager = AccountScopeManager::new(query_client);

        let all_dids = vec![
            "did:plc:alice".to_string(),
            "did:plc:bob".to_string(),
            "did:plc:charlie".to_string(),
        ];

        // Should not error
        let result = manager.invalidate_except("did:plc:alice", &all_dids).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_account_scoped_trait() {
        struct TestScoped;
        impl AccountScoped for TestScoped {}

        let key = TestScoped::account_key("did:plc:test", "feeds", "home");
        assert_eq!(key.scope, "account:did:plc:test:feeds");
        assert_eq!(key.id, "home");
    }

    #[test]
    fn test_account_scoped_with_params() {
        struct TestScoped;
        impl AccountScoped for TestScoped {}

        let key = TestScoped::account_key_with_params(
            "did:plc:test",
            "posts",
            "timeline",
            vec![("limit", "50"), ("cursor", "abc")],
        );

        assert_eq!(key.scope, "account:did:plc:test:posts");
        assert_eq!(key.id, "timeline");
        assert_eq!(key.params.get("limit"), Some(&"50".to_string()));
        assert_eq!(key.params.get("cursor"), Some(&"abc".to_string()));
    }
}
