//! Phase 1 Integration Tests
//!
//! Comprehensive end-to-end tests for the Foundation & Infrastructure phase.

use atproto_client::session::{SessionAccount, SessionManager};
use storage::{
    AppPersistedState, ColorMode, DatabaseConfig, KvConfig, KvStore, LanguagePrefs,
    OnboardingState, PersistedState, PersistenceConfig, SqliteDatabase,
};
use tempfile::TempDir;

/// Test full session lifecycle with persistence
#[tokio::test]
async fn test_session_lifecycle_with_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let session_path = temp_dir.path().join("sessions.json");

    // Phase 1: Create session manager and add account
    {
        let mut manager = SessionManager::new(&session_path).await.unwrap();

        let account = SessionAccount {
            service: "https://bsky.social".to_string(),
            did: "did:plc:test123".to_string(),
            handle: "alice.bsky.social".to_string(),
            email: Some("alice@example.com".to_string()),
            email_confirmed: Some(true),
            email_auth_factor: Some(false),
            access_jwt: Some("access_token_123".to_string()),
            refresh_jwt: Some("refresh_token_123".to_string()),
            active: Some(true),
            status: None,
            pds_url: Some("https://pds.example.com".to_string()),
            signup_queued: Some(false),
            is_self_hosted: Some(false),
        };

        manager.add_account(account).await.unwrap();
        assert_eq!(manager.list_accounts().len(), 1);
    }

    // Phase 2: Restart and verify persistence
    {
        let manager = SessionManager::new(&session_path).await.unwrap();
        assert_eq!(manager.list_accounts().len(), 1);

        let account = manager.get_account("did:plc:test123").unwrap();
        assert_eq!(account.handle, "alice.bsky.social");
        assert_eq!(account.email, Some("alice@example.com".to_string()));
    }
}

/// Test multi-account switching and isolation
#[tokio::test]
async fn test_multi_account_isolation() {
    let temp_dir = TempDir::new().unwrap();
    let session_path = temp_dir.path().join("multi_sessions.json");

    let mut manager = SessionManager::new(&session_path).await.unwrap();

    // Add two accounts
    let account1 = SessionAccount {
        service: "https://bsky.social".to_string(),
        did: "did:plc:alice".to_string(),
        handle: "alice.bsky.social".to_string(),
        email: Some("alice@example.com".to_string()),
        email_confirmed: Some(true),
        email_auth_factor: Some(false),
        access_jwt: Some("alice_access".to_string()),
        refresh_jwt: Some("alice_refresh".to_string()),
        active: Some(true),
        status: None,
        pds_url: None,
        signup_queued: Some(false),
        is_self_hosted: Some(false),
    };

    let account2 = SessionAccount {
        service: "https://bsky.social".to_string(),
        did: "did:plc:bob".to_string(),
        handle: "bob.bsky.social".to_string(),
        email: Some("bob@example.com".to_string()),
        email_confirmed: Some(true),
        email_auth_factor: Some(false),
        access_jwt: Some("bob_access".to_string()),
        refresh_jwt: Some("bob_refresh".to_string()),
        active: Some(true),
        status: None,
        pds_url: None,
        signup_queued: Some(false),
        is_self_hosted: Some(false),
    };

    manager.add_account(account1).await.unwrap();
    manager.add_account(account2).await.unwrap();

    assert_eq!(manager.list_accounts().len(), 2);

    // Verify account isolation
    let alice = manager.get_account("did:plc:alice").unwrap();
    let bob = manager.get_account("did:plc:bob").unwrap();

    assert_eq!(alice.handle, "alice.bsky.social");
    assert_eq!(bob.handle, "bob.bsky.social");
    assert_eq!(alice.email, Some("alice@example.com".to_string()));
    assert_eq!(bob.email, Some("bob@example.com".to_string()));
}

/// Test app state persistence and migration
#[tokio::test]
async fn test_app_state_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("app_state.json");

    // Phase 1: Create and save state
    {
        let config = PersistenceConfig::new(&state_path)
            .version(1)
            .atomic_writes(true)
            .backups(true, 3);

        let storage = PersistedState::<AppPersistedState>::new(config);
        storage.init().await.unwrap();

        let state = AppPersistedState {
            color_mode: ColorMode::Dark,
            language_prefs: LanguagePrefs {
                primary_language: "es".to_string(),
                ..Default::default()
            },
            onboarding: OnboardingState {
                current_step: Some("profile_setup".to_string()),
                completed: false,
                ..Default::default()
            },
            ..Default::default()
        };

        storage.set(state.clone()).await.unwrap();
    }

    // Phase 2: Load and verify state
    {
        let config = PersistenceConfig::new(&state_path).version(1);
        let storage = PersistedState::<AppPersistedState>::new(config);
        storage.init().await.unwrap();

        let state: AppPersistedState = storage.get().await.unwrap();
        assert_eq!(state.color_mode, ColorMode::Dark);
        assert_eq!(state.language_prefs.primary_language, "es");
        assert_eq!(state.onboarding.current_step, Some("profile_setup".to_string()));
        assert!(!state.onboarding.completed);
    }
}

/// Test storage layer integration (Database + KV store)
#[tokio::test]
async fn test_storage_layer_integration() {
    use storage::Database;

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let kv_path = temp_dir.path().join("kv_store");

    // Test database operations
    let db_config = DatabaseConfig::new(db_path.to_str().unwrap());
    let db = SqliteDatabase::new(db_config).await.unwrap();

    db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
        .await
        .unwrap();
    db.execute("INSERT INTO test (value) VALUES ('hello')")
        .await
        .unwrap();

    let rows = db
        .query_all("SELECT value FROM test WHERE id = 1")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);

    // Test KV store operations
    let kv_config = KvConfig::new(kv_path.to_str().unwrap());
    let kv = KvStore::new(kv_config).unwrap();

    kv.set("test_key", &"test_value").unwrap();
    let value: String = kv.get("test_key").unwrap().unwrap();
    assert_eq!(value, "test_value");
}

/// Test complete user session scenario
#[tokio::test]
async fn test_complete_user_scenario() {
    let temp_dir = TempDir::new().unwrap();

    // Setup storage
    let session_path = temp_dir.path().join("sessions.json");
    let app_state_path = temp_dir.path().join("app_state.json");
    let kv_path = temp_dir.path().join("kv");

    // Initialize session manager
    let mut session_manager = SessionManager::new(&session_path).await.unwrap();

    // Initialize app state
    let app_config = PersistenceConfig::new(&app_state_path)
        .version(1)
        .atomic_writes(true);
    let app_storage = PersistedState::<AppPersistedState>::new(app_config);
    app_storage.init().await.unwrap();

    // Initialize KV store for additional user data
    let kv_config = KvConfig::new(kv_path.to_str().unwrap());
    let kv = KvStore::new(kv_config).unwrap();

    // Scenario: User logs in
    let user_account = SessionAccount {
        service: "https://bsky.social".to_string(),
        did: "did:plc:user123".to_string(),
        handle: "user.bsky.social".to_string(),
        email: Some("user@example.com".to_string()),
        email_confirmed: Some(true),
        email_auth_factor: Some(true),
        access_jwt: Some("user_access_jwt".to_string()),
        refresh_jwt: Some("user_refresh_jwt".to_string()),
        active: Some(true),
        status: None,
        pds_url: Some("https://bsky.social".to_string()),
        signup_queued: Some(false),
        is_self_hosted: Some(false),
    };

    session_manager.add_account(user_account).await.unwrap();

    // User sets preferences
    let app_state = AppPersistedState {
        color_mode: ColorMode::Dark,
        language_prefs: LanguagePrefs {
            primary_language: "en".to_string(),
            additional_languages: vec!["es".to_string()],
            ..Default::default()
        },
        onboarding: OnboardingState {
            current_step: Some("completed".to_string()),
            completed: true,
            seen_welcome: true,
        },
        ..Default::default()
    };

    app_storage.set(app_state.clone()).await.unwrap();

    // Store additional user preferences in KV
    kv.set("user_preference_1", &"value1").unwrap();
    kv.set("user_preference_2", &"value2").unwrap();

    // Simulate app restart - verify all data persists
    drop(session_manager);
    drop(app_storage);
    drop(kv);

    // Reload everything
    let session_manager = SessionManager::new(&session_path).await.unwrap();
    let app_config = PersistenceConfig::new(&app_state_path).version(1);
    let app_storage = PersistedState::<AppPersistedState>::new(app_config);
    app_storage.init().await.unwrap();
    let kv_config = KvConfig::new(kv_path.to_str().unwrap());
    let kv = KvStore::new(kv_config).unwrap();

    // Verify session
    assert_eq!(session_manager.list_accounts().len(), 1);
    let account = session_manager.get_account("did:plc:user123").unwrap();
    assert_eq!(account.handle, "user.bsky.social");

    // Verify app state
    let loaded_state: AppPersistedState = app_storage.get().await.unwrap();
    assert_eq!(loaded_state.color_mode, ColorMode::Dark);
    assert_eq!(loaded_state.language_prefs.primary_language, "en");
    assert!(loaded_state.onboarding.completed);

    // Verify KV data
    let pref1: String = kv.get("user_preference_1").unwrap().unwrap();
    let pref2: String = kv.get("user_preference_2").unwrap().unwrap();
    assert_eq!(pref1, "value1");
    assert_eq!(pref2, "value2");
}

/// Test error handling and recovery
#[tokio::test]
async fn test_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let session_path = temp_dir.path().join("error_test.json");

    let mut manager = SessionManager::new(&session_path).await.unwrap();

    // Test removing non-existent account
    let result = manager.remove_account("did:plc:nonexistent").await;
    assert!(result.is_err());

    // Test getting non-existent account
    assert!(manager.get_account("did:plc:nonexistent").is_none());

    // Test adding duplicate account (should update)
    let account = SessionAccount {
        service: "https://bsky.social".to_string(),
        did: "did:plc:test".to_string(),
        handle: "test.bsky.social".to_string(),
        email: None,
        email_confirmed: None,
        email_auth_factor: None,
        access_jwt: Some("token1".to_string()),
        refresh_jwt: Some("refresh1".to_string()),
        active: Some(true),
        status: None,
        pds_url: None,
        signup_queued: Some(false),
        is_self_hosted: Some(false),
    };

    manager.add_account(account.clone()).await.unwrap();
    assert_eq!(manager.list_accounts().len(), 1);

    // Update with new token
    let mut updated_account = account;
    updated_account.access_jwt = Some("token2".to_string());
    manager.add_account(updated_account).await.unwrap();

    // Should still have only one account with updated token
    assert_eq!(manager.list_accounts().len(), 1);
    let stored = manager.get_account("did:plc:test").unwrap();
    assert_eq!(stored.access_jwt, Some("token2".to_string()));
}
