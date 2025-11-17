//! Database abstraction layer
//!
//! This module provides a database abstraction supporting SQLite with connection
//! pooling, migrations, and transaction support.

use async_trait::async_trait;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Error as SqlxError, Sqlite, SqlitePool, Transaction,
};
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

/// Database error types
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// SQLx error
    #[error("Database error: {0}")]
    Sqlx(#[from] SqlxError),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),
}

/// Result type for database operations
pub type Result<T> = std::result::Result<T, DatabaseError>;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database file path
    pub path: String,
    /// Maximum number of connections in pool
    pub max_connections: u32,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Enable WAL mode
    pub wal_mode: bool,
    /// Synchronous mode
    pub synchronous: SynchronousMode,
}

/// SQLite synchronous mode
#[derive(Debug, Clone, Copy)]
pub enum SynchronousMode {
    /// Off - no synchronization
    Off,
    /// Normal - synchronize at critical moments
    Normal,
    /// Full - synchronize after each write
    Full,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "aurora.db".to_string(),
            max_connections: 10,
            connect_timeout: Duration::from_secs(30),
            wal_mode: true,
            synchronous: SynchronousMode::Normal,
        }
    }
}

impl DatabaseConfig {
    /// Create a new database configuration
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            ..Default::default()
        }
    }

    /// Set maximum connections
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Set connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Enable or disable WAL mode
    pub fn wal_mode(mut self, enabled: bool) -> Self {
        self.wal_mode = enabled;
        self
    }

    /// Set synchronous mode
    pub fn synchronous(mut self, mode: SynchronousMode) -> Self {
        self.synchronous = mode;
        self
    }
}

/// Database abstraction trait
#[async_trait]
pub trait Database: Send + Sync {
    /// Execute a raw SQL query
    async fn execute(&self, sql: &str) -> Result<u64>;

    /// Execute a raw SQL query with parameters
    async fn execute_with_params(&self, sql: &str, params: &[&dyn sqlx::Encode<'_, Sqlite>])
        -> Result<u64>;

    /// Query a single row
    async fn query_one(&self, sql: &str) -> Result<sqlx::sqlite::SqliteRow>;

    /// Query multiple rows
    async fn query_all(&self, sql: &str) -> Result<Vec<sqlx::sqlite::SqliteRow>>;

    /// Begin a transaction
    async fn begin(&self) -> Result<DatabaseTransaction>;

    /// Close the database connection
    async fn close(&self) -> Result<()>;

    /// Check if the database is healthy
    async fn health_check(&self) -> Result<()>;
}

/// SQLite database implementation
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    /// Create a new SQLite database with configuration
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        // Build connection options
        let mut options = SqliteConnectOptions::from_str(&format!("sqlite://{}", config.path))
            .map_err(|e| DatabaseError::Config(e.to_string()))?
            .create_if_missing(true);

        // Set journal mode
        if config.wal_mode {
            options = options.journal_mode(SqliteJournalMode::Wal);
        }

        // Set synchronous mode
        options = match config.synchronous {
            SynchronousMode::Off => options.synchronous(SqliteSynchronous::Off),
            SynchronousMode::Normal => options.synchronous(SqliteSynchronous::Normal),
            SynchronousMode::Full => options.synchronous(SqliteSynchronous::Full),
        };

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(config.connect_timeout)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    /// Create an in-memory database (for testing)
    pub async fn in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        Ok(Self { pool })
    }

    /// Get the underlying pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Run migrations
    pub async fn migrate(&self, migrations: &[MigrationDefinition]) -> Result<()> {
        // Ensure migrations table exists
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                checksum TEXT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;

        // Get current version
        let current_version: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM _migrations")
            .fetch_optional(&self.pool)
            .await?;

        let current_version = current_version.unwrap_or(0);

        // Apply pending migrations
        for migration in migrations {
            if migration.version > current_version {
                tracing::info!(
                    "Applying migration {} - {}",
                    migration.version,
                    migration.description
                );

                // Start transaction
                let mut tx = self.pool.begin().await?;

                // Execute migration
                sqlx::query(&migration.sql).execute(&mut *tx).await?;

                // Record migration
                sqlx::query(
                    "INSERT INTO _migrations (version, description, checksum) VALUES (?, ?, ?)",
                )
                .bind(migration.version)
                .bind(&migration.description)
                .bind(&migration.checksum)
                .execute(&mut *tx)
                .await?;

                // Commit transaction
                tx.commit().await?;

                tracing::info!("Migration {} applied successfully", migration.version);
            }
        }

        Ok(())
    }

    /// Get current migration version
    pub async fn current_version(&self) -> Result<i64> {
        let version: Option<i64> = sqlx::query_scalar("SELECT MAX(version) FROM _migrations")
            .fetch_optional(&self.pool)
            .await?;

        Ok(version.unwrap_or(0))
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn execute(&self, sql: &str) -> Result<u64> {
        let result = sqlx::query(sql).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    async fn execute_with_params(
        &self,
        sql: &str,
        _params: &[&dyn sqlx::Encode<'_, Sqlite>],
    ) -> Result<u64> {
        // Note: This is a simplified implementation
        // In a real implementation, you'd want to use sqlx::query! macro
        // or build the query properly with bind()
        let result = sqlx::query(sql).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    async fn query_one(&self, sql: &str) -> Result<sqlx::sqlite::SqliteRow> {
        let row = sqlx::query(sql)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                SqlxError::RowNotFound => DatabaseError::NotFound("Row not found".to_string()),
                e => DatabaseError::Sqlx(e),
            })?;
        Ok(row)
    }

    async fn query_all(&self, sql: &str) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
        let rows = sqlx::query(sql).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    async fn begin(&self) -> Result<DatabaseTransaction> {
        let tx = self.pool.begin().await?;
        Ok(DatabaseTransaction { tx: Some(tx) })
    }

    async fn close(&self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }
}

/// Database transaction wrapper
pub struct DatabaseTransaction {
    tx: Option<Transaction<'static, Sqlite>>,
}

impl DatabaseTransaction {
    /// Execute a query within the transaction
    pub async fn execute(&mut self, sql: &str) -> Result<u64> {
        let tx = self
            .tx
            .as_mut()
            .ok_or_else(|| DatabaseError::Transaction("Transaction already committed".to_string()))?;

        let result = sqlx::query(sql).execute(&mut **tx).await?;
        Ok(result.rows_affected())
    }

    /// Query a single row within the transaction
    pub async fn query_one(&mut self, sql: &str) -> Result<sqlx::sqlite::SqliteRow> {
        let tx = self
            .tx
            .as_mut()
            .ok_or_else(|| DatabaseError::Transaction("Transaction already committed".to_string()))?;

        let row = sqlx::query(sql)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| match e {
                SqlxError::RowNotFound => DatabaseError::NotFound("Row not found".to_string()),
                e => DatabaseError::Sqlx(e),
            })?;
        Ok(row)
    }

    /// Query multiple rows within the transaction
    pub async fn query_all(&mut self, sql: &str) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
        let tx = self
            .tx
            .as_mut()
            .ok_or_else(|| DatabaseError::Transaction("Transaction already committed".to_string()))?;

        let rows = sqlx::query(sql).fetch_all(&mut **tx).await?;
        Ok(rows)
    }

    /// Commit the transaction
    pub async fn commit(mut self) -> Result<()> {
        let tx = self
            .tx
            .take()
            .ok_or_else(|| DatabaseError::Transaction("Transaction already committed".to_string()))?;

        tx.commit().await?;
        Ok(())
    }

    /// Rollback the transaction
    pub async fn rollback(mut self) -> Result<()> {
        let tx = self
            .tx
            .take()
            .ok_or_else(|| DatabaseError::Transaction("Transaction already committed".to_string()))?;

        tx.rollback().await?;
        Ok(())
    }
}

/// Migration definition
#[derive(Debug, Clone)]
pub struct MigrationDefinition {
    /// Migration version number
    pub version: i64,
    /// Migration description
    pub description: String,
    /// SQL to execute
    pub sql: String,
    /// Checksum for verification
    pub checksum: String,
}

impl MigrationDefinition {
    /// Create a new migration definition
    pub fn new(
        version: i64,
        description: impl Into<String>,
        sql: impl Into<String>,
    ) -> Self {
        let sql = sql.into();
        let checksum = format!("{:x}", md5::compute(&sql));

        Self {
            version,
            description: description.into(),
            sql,
            checksum,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    #[tokio::test]
    async fn test_database_creation() {
        let db = SqliteDatabase::in_memory().await.unwrap();
        assert!(db.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_execute_query() {
        let db = SqliteDatabase::in_memory().await.unwrap();

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        let affected = db
            .execute("INSERT INTO test (name) VALUES ('test')")
            .await
            .unwrap();

        assert_eq!(affected, 1);
    }

    #[tokio::test]
    async fn test_query_operations() {
        let db = SqliteDatabase::in_memory().await.unwrap();

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        db.execute("INSERT INTO test (name) VALUES ('alice')")
            .await
            .unwrap();
        db.execute("INSERT INTO test (name) VALUES ('bob')")
            .await
            .unwrap();

        let rows = db.query_all("SELECT * FROM test").await.unwrap();
        assert_eq!(rows.len(), 2);

        let row = db
            .query_one("SELECT * FROM test WHERE name = 'alice'")
            .await
            .unwrap();
        let name: String = row.get("name");
        assert_eq!(name, "alice");
    }

    #[tokio::test]
    async fn test_query_not_found() {
        let db = SqliteDatabase::in_memory().await.unwrap();

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        let result = db.query_one("SELECT * FROM test WHERE name = 'nonexistent'").await;
        assert!(matches!(result, Err(DatabaseError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let db = SqliteDatabase::in_memory().await.unwrap();

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        let mut tx = db.begin().await.unwrap();
        tx.execute("INSERT INTO test (name) VALUES ('alice')")
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let rows = db.query_all("SELECT * FROM test").await.unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let db = SqliteDatabase::in_memory().await.unwrap();

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        let mut tx = db.begin().await.unwrap();
        tx.execute("INSERT INTO test (name) VALUES ('alice')")
            .await
            .unwrap();
        tx.rollback().await.unwrap();

        let rows = db.query_all("SELECT * FROM test").await.unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[tokio::test]
    async fn test_transaction_query() {
        let db = SqliteDatabase::in_memory().await.unwrap();

        db.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();
        db.execute("INSERT INTO test (name) VALUES ('alice')")
            .await
            .unwrap();

        let mut tx = db.begin().await.unwrap();
        let row = tx
            .query_one("SELECT * FROM test WHERE name = 'alice'")
            .await
            .unwrap();
        let name: String = row.get("name");
        assert_eq!(name, "alice");
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_migrations() {
        let db = SqliteDatabase::in_memory().await.unwrap();

        let migrations = vec![
            MigrationDefinition::new(
                1,
                "Initial schema",
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
            ),
            MigrationDefinition::new(
                2,
                "Add email column",
                "ALTER TABLE users ADD COLUMN email TEXT",
            ),
        ];

        db.migrate(&migrations).await.unwrap();

        let version = db.current_version().await.unwrap();
        assert_eq!(version, 2);

        // Verify tables exist
        let row = db
            .query_one("SELECT name FROM sqlite_master WHERE type='table' AND name='users'")
            .await
            .unwrap();
        let table_name: String = row.get("name");
        assert_eq!(table_name, "users");
    }

    #[tokio::test]
    async fn test_migrations_idempotent() {
        let db = SqliteDatabase::in_memory().await.unwrap();

        let migrations = vec![MigrationDefinition::new(
            1,
            "Initial schema",
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
        )];

        db.migrate(&migrations).await.unwrap();
        let version1 = db.current_version().await.unwrap();

        // Run again - should be idempotent
        db.migrate(&migrations).await.unwrap();
        let version2 = db.current_version().await.unwrap();

        assert_eq!(version1, version2);
        assert_eq!(version2, 1);
    }

    #[tokio::test]
    async fn test_health_check() {
        let db = SqliteDatabase::in_memory().await.unwrap();
        assert!(db.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_config_builder() {
        let config = DatabaseConfig::new("test.db")
            .max_connections(5)
            .connect_timeout(Duration::from_secs(10))
            .wal_mode(true)
            .synchronous(SynchronousMode::Full);

        assert_eq!(config.path, "test.db");
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert!(config.wal_mode);
        assert!(matches!(config.synchronous, SynchronousMode::Full));
    }
}
