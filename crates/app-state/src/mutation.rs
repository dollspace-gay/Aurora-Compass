//! Mutation management
//!
//! This module provides mutation handling with optimistic updates, rollback on failure,
//! and automatic cache invalidation.

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

use crate::query::{QueryClient, QueryKey};

/// Mutation errors
#[derive(Debug, Error)]
pub enum MutationError {
    /// Mutation execution failed
    #[error("Mutation failed: {0}")]
    ExecutionError(String),

    /// Optimistic update failed
    #[error("Optimistic update failed: {0}")]
    OptimisticError(String),

    /// Rollback failed
    #[error("Rollback failed: {0}")]
    RollbackError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Mutation cancelled
    #[error("Mutation cancelled")]
    Cancelled,
}

/// Result type for mutation operations
pub type Result<T> = std::result::Result<T, MutationError>;

/// Mutation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationState {
    /// Mutation is idle
    Idle,

    /// Mutation is pending (optimistic update applied)
    Pending,

    /// Mutation succeeded
    Success,

    /// Mutation failed (rolled back)
    Error,
}

/// Optimistic update for rollback
#[derive(Debug, Clone)]
struct OptimisticUpdate {
    /// The query key that was updated
    query_key: QueryKey,

    /// Previous value (for rollback)
    #[allow(dead_code)]
    previous_value: Option<String>,

    /// When the update was applied
    #[allow(dead_code)]
    applied_at: SystemTime,
}

/// Mutation context for tracking optimistic updates
#[derive(Debug, Clone)]
pub struct MutationContext {
    updates: Vec<OptimisticUpdate>,
}

impl MutationContext {
    /// Create a new mutation context
    pub fn new() -> Self {
        Self { updates: Vec::new() }
    }

    /// Record an optimistic update
    pub fn record_update(&mut self, key: QueryKey, previous_value: Option<String>) {
        self.updates.push(OptimisticUpdate {
            query_key: key,
            previous_value,
            applied_at: SystemTime::now(),
        });
    }
}

impl Default for MutationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Mutation configuration
#[derive(Debug, Clone, Default)]
pub struct MutationConfig {
    /// Retry failed mutations
    pub retry: bool,

    /// Maximum retry attempts
    pub retry_count: u32,

    /// Invalidation rules (scopes to invalidate after success)
    pub invalidate_scopes: Vec<String>,

    /// Invalidation keys (specific queries to invalidate)
    pub invalidate_keys: Vec<QueryKey>,
}

/// Mutation trait for defining data modification logic
#[async_trait]
pub trait Mutation: Send + Sync {
    /// Input type for the mutation
    type Input: Send + Sync;

    /// Output type returned by the mutation
    type Output: Serialize + DeserializeOwned + Clone + Send + Sync;

    /// Execute the mutation
    async fn mutate(&self, input: Self::Input) -> Result<Self::Output>;

    /// Apply optimistic update before mutation completes
    ///
    /// This is called before the mutation executes, allowing the UI to update immediately.
    /// Return the query keys and their new values.
    async fn optimistic_update(
        &self,
        _input: &Self::Input,
        _ctx: &mut MutationContext,
    ) -> Result<()> {
        Ok(())
    }

    /// Get mutation configuration
    fn config(&self) -> MutationConfig {
        MutationConfig::default()
    }
}

/// Mutation client for managing mutations
pub struct MutationClient {
    query_client: Arc<QueryClient>,
    state: Arc<RwLock<HashMap<String, MutationState>>>,
    pending_contexts: Arc<Mutex<HashMap<String, MutationContext>>>,
}

impl MutationClient {
    /// Create a new mutation client
    pub fn new(query_client: Arc<QueryClient>) -> Self {
        Self {
            query_client,
            state: Arc::new(RwLock::new(HashMap::new())),
            pending_contexts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Execute a mutation with optimistic updates
    pub async fn mutate<M: Mutation>(
        &self,
        mutation: &M,
        input: M::Input,
        mutation_id: impl Into<String>,
    ) -> Result<M::Output> {
        let id = mutation_id.into();
        let config = mutation.config();

        // Set state to pending
        {
            let mut state = self.state.write().await;
            state.insert(id.clone(), MutationState::Pending);
        }

        // Create mutation context for tracking optimistic updates
        let mut ctx = MutationContext::new();

        // Apply optimistic update
        if let Err(e) = mutation.optimistic_update(&input, &mut ctx).await {
            // Optimistic update failed, revert to idle
            let mut state = self.state.write().await;
            state.insert(id.clone(), MutationState::Idle);
            return Err(e);
        }

        // Store context for potential rollback
        {
            let mut contexts = self.pending_contexts.lock().await;
            contexts.insert(id.clone(), ctx.clone());
        }

        // Execute the mutation
        let result = mutation.mutate(input).await;

        match result {
            Ok(output) => {
                // Success - update state
                {
                    let mut state = self.state.write().await;
                    state.insert(id.clone(), MutationState::Success);
                }

                // Remove pending context
                {
                    let mut contexts = self.pending_contexts.lock().await;
                    contexts.remove(&id);
                }

                // Invalidate caches as specified
                for scope in &config.invalidate_scopes {
                    let _ = self.query_client.invalidate_scope(scope).await;
                }

                for key in &config.invalidate_keys {
                    let _ = self.query_client.invalidate(key).await;
                }

                Ok(output)
            }
            Err(e) => {
                // Failure - rollback optimistic updates
                {
                    let mut state = self.state.write().await;
                    state.insert(id.clone(), MutationState::Error);
                }

                // Rollback
                if let Err(rollback_err) = self.rollback(&id).await {
                    tracing::error!("Rollback failed: {}", rollback_err);
                }

                Err(e)
            }
        }
    }

    /// Rollback optimistic updates for a mutation
    async fn rollback(&self, mutation_id: &str) -> Result<()> {
        let mut contexts = self.pending_contexts.lock().await;

        if let Some(ctx) = contexts.remove(mutation_id) {
            // Rollback in reverse order
            for update in ctx.updates.iter().rev() {
                // Re-invalidate the query to force refetch
                let _ = self.query_client.invalidate(&update.query_key).await;
            }
        }

        Ok(())
    }

    /// Get mutation state
    pub async fn state(&self, mutation_id: &str) -> MutationState {
        let state = self.state.read().await;
        state
            .get(mutation_id)
            .copied()
            .unwrap_or(MutationState::Idle)
    }

    /// Reset mutation state
    pub async fn reset(&self, mutation_id: &str) {
        let mut state = self.state.write().await;
        state.remove(mutation_id);

        let mut contexts = self.pending_contexts.lock().await;
        contexts.remove(mutation_id);
    }

    /// Clear all mutation states
    pub async fn clear(&self) {
        let mut state = self.state.write().await;
        state.clear();

        let mut contexts = self.pending_contexts.lock().await;
        contexts.clear();
    }
}

impl Clone for MutationClient {
    fn clone(&self) -> Self {
        Self {
            query_client: Arc::clone(&self.query_client),
            state: Arc::clone(&self.state),
            pending_contexts: Arc::clone(&self.pending_contexts),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{Query, QueryClient, QueryConfig, QueryKey};
    use storage::CacheConfig;

    #[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
    struct TestData {
        value: String,
    }

    #[derive(Clone)]
    #[allow(dead_code)]
    struct TestQuery {
        key: QueryKey,
        data: TestData,
    }

    #[async_trait]
    impl Query for TestQuery {
        type Data = TestData;

        async fn fetch(&self) -> crate::query::Result<Self::Data> {
            Ok(self.data.clone())
        }

        fn key(&self) -> QueryKey {
            self.key.clone()
        }

        fn config(&self) -> QueryConfig {
            QueryConfig {
                stale_time: std::time::Duration::from_secs(60),
                ..Default::default()
            }
        }
    }

    struct TestMutation {
        should_fail: bool,
    }

    #[async_trait]
    impl Mutation for TestMutation {
        type Input = String;
        type Output = TestData;

        async fn mutate(&self, input: Self::Input) -> Result<Self::Output> {
            if self.should_fail {
                Err(MutationError::ExecutionError("simulated failure".to_string()))
            } else {
                Ok(TestData { value: input })
            }
        }
    }

    #[tokio::test]
    async fn test_mutation_success() {
        let query_client = Arc::new(QueryClient::new(CacheConfig::default()).unwrap());
        let mutation_client = MutationClient::new(query_client);

        let mutation = TestMutation { should_fail: false };
        let result = mutation_client
            .mutate(&mutation, "test value".to_string(), "test_mutation")
            .await
            .unwrap();

        assert_eq!(result.value, "test value");
        assert_eq!(mutation_client.state("test_mutation").await, MutationState::Success);
    }

    #[tokio::test]
    async fn test_mutation_failure() {
        let query_client = Arc::new(QueryClient::new(CacheConfig::default()).unwrap());
        let mutation_client = MutationClient::new(query_client);

        let mutation = TestMutation { should_fail: true };
        let result = mutation_client
            .mutate(&mutation, "test value".to_string(), "test_mutation_fail")
            .await;

        assert!(result.is_err());
        assert_eq!(mutation_client.state("test_mutation_fail").await, MutationState::Error);
    }

    #[tokio::test]
    async fn test_mutation_reset() {
        let query_client = Arc::new(QueryClient::new(CacheConfig::default()).unwrap());
        let mutation_client = MutationClient::new(query_client);

        let mutation = TestMutation { should_fail: false };
        mutation_client
            .mutate(&mutation, "test".to_string(), "reset_test")
            .await
            .unwrap();

        mutation_client.reset("reset_test").await;
        assert_eq!(mutation_client.state("reset_test").await, MutationState::Idle);
    }

    #[tokio::test]
    async fn test_mutation_clear() {
        let query_client = Arc::new(QueryClient::new(CacheConfig::default()).unwrap());
        let mutation_client = MutationClient::new(query_client);

        let mutation = TestMutation { should_fail: false };
        mutation_client
            .mutate(&mutation, "test1".to_string(), "clear_test1")
            .await
            .unwrap();
        mutation_client
            .mutate(&mutation, "test2".to_string(), "clear_test2")
            .await
            .unwrap();

        mutation_client.clear().await;
        assert_eq!(mutation_client.state("clear_test1").await, MutationState::Idle);
        assert_eq!(mutation_client.state("clear_test2").await, MutationState::Idle);
    }
}
