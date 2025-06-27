use crate::config::DatabaseConfig;
use crate::errors::{
    DatabaseError, DatabaseResult, DatabaseType, ErrorContext, ErrorSeverity, QueryType,
};
use sqlx::{sqlite::SqlitePoolOptions, Sqlite, SqlitePool as SqlxSqlitePool};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct SqlitePoolConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
    pub enable_wal: bool,
    pub enable_foreign_keys: bool,
}

impl SqlitePoolConfig {
    pub fn builder() -> SqlitePoolConfigBuilder {
        SqlitePoolConfigBuilder::default()
    }

    pub fn from_database_config(config: &DatabaseConfig) -> Self {
        Self {
            url: config.sqlite.url.clone(),
            max_connections: config.sqlite.max_connections,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(config.sqlite.connection_timeout_secs),
            idle_timeout: None,
            max_lifetime: None,
            enable_wal: config.sqlite.enable_wal,
            enable_foreign_keys: true,
        }
    }

    pub fn validate(&self) -> DatabaseResult<()> {
        if !self.url.starts_with("sqlite:") {
            return Err(DatabaseError::Configuration {
                message: "SQLite URL must start with 'sqlite:'".into(),
                database: DatabaseType::SQLite,
                context: ErrorContext::new("config_validation"),
            });
        }
        if self.max_connections == 0 {
            return Err(DatabaseError::Configuration {
                message: "max_connections must be > 0".into(),
                database: DatabaseType::SQLite,
                context: ErrorContext::new("config_validation"),
            });
        }

        if self.min_connections > self.max_connections {
            return Err(DatabaseError::Configuration {
                message: "min_connections cannot exceed max_connections".into(),
                database: DatabaseType::SQLite,
                context: ErrorContext::new("config_validation"),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SqlitePoolConfigBuilder {
    url: Option<String>,
    max_connections: Option<u32>,
    min_connections: Option<u32>,
    acquire_timeout: Option<Duration>,
    idle_timeout: Option<Duration>,
    max_lifetime: Option<Duration>,
    enable_wal: Option<bool>,
    enable_foreign_keys: Option<bool>,
}

impl SqlitePoolConfigBuilder {
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = Some(max);
        self
    }

    pub fn min_connections(mut self, min: u32) -> Self {
        self.min_connections = Some(min);
        self
    }
    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = Some(timeout);
        self
    }
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.max_lifetime = Some(lifetime);
        self
    }
    pub fn enable_wal(mut self, enable: bool) -> Self {
        self.enable_wal = Some(enable);
        self
    }

    pub fn enable_foreign_keys(mut self, enable: bool) -> Self {
        self.enable_foreign_keys = Some(enable);
        self
    }

    pub fn build(self) -> SqlitePoolConfig {
        SqlitePoolConfig {
            url: self.url.unwrap_or_else(|| "sqlite::memory:".to_string()),
            max_connections: self.max_connections.unwrap_or(10),
            min_connections: self.min_connections.unwrap_or(1),
            acquire_timeout: self.acquire_timeout.unwrap_or(Duration::from_secs(30)),
            idle_timeout: self.idle_timeout,
            max_lifetime: self.max_lifetime,
            enable_wal: self.enable_wal.unwrap_or(true),
            enable_foreign_keys: self.enable_foreign_keys.unwrap_or(true),
        }
    }
}

#[derive(Debug, Default)]
pub struct PoolMetrics {
    pub total_connections: AtomicU32,
    pub active_connections: AtomicU32,
    pub connection_errors: AtomicU64,
    pub query_count: AtomicU64,
    pub total_query_time_ms: AtomicU64,
}

impl PoolMetrics {
    pub fn increment_connections(&self) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_active(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_active(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_query(&self, duration_ms: u64) {
        self.query_count.fetch_add(1, Ordering::Relaxed);
        self.total_query_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    pub fn average_query_time_ms(&self) -> f64 {
        let count = self.query_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            (self.total_query_time_ms.load(Ordering::Relaxed) as f64) / (count as f64)
        }
    }
}

pub struct SqlitePool {
    pool: SqlxSqlitePool,
    config: SqlitePoolConfig,
    metrics: PoolMetrics,
}

impl SqlitePool {
    pub async fn new(config: SqlitePoolConfig) -> DatabaseResult<Self> {
        config.validate()?;

        let mut options = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(config.acquire_timeout);

        if let Some(idle_timeout) = config.idle_timeout {
            options = options.idle_timeout(idle_timeout);
        }
        if let Some(max_lifetime) = config.max_lifetime {
            options = options.max_lifetime(max_lifetime);
        }
        let pool = options.connect(&config.url).await.map_err(|e| {
            DatabaseError::connection_failed(
                DatabaseType::SQLite,
                format!("Failed to create connection pool: {}", e),
            )
            .with_context("url", config.url.clone())
            .with_context("max_connections", config.max_connections.to_string())
        })?;

        let sqlite_pool = Self {
            pool,
            config,
            metrics: PoolMetrics::default(),
        };

        sqlite_pool.configure_sqlite().await?;

        Ok(sqlite_pool)
    }

    pub async fn from_database_config(db_config: &DatabaseConfig) -> DatabaseResult<Self> {
        let config = SqlitePoolConfig::from_database_config(db_config);
        Self::new(config).await
    }

    async fn configure_sqlite(&self) -> DatabaseResult<()> {
        let mut conn = self.acquire_connection().await?;

        if self.config.enable_wal {
            sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&mut *conn)
                .await
                .map_err(|e| {
                    DatabaseError::query_failed(
                        DatabaseType::SQLite,
                        QueryType::Select,
                        format!("Failed to enable WAL mode: {}", e),
                    )
                })?;
        }

        if self.config.enable_foreign_keys {
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&mut *conn)
                .await
                .map_err(|e| {
                    DatabaseError::query_failed(
                        DatabaseType::SQLite,
                        QueryType::Select,
                        format!("Failed to enable foreign keys: {}", e),
                    )
                })?;
        }
        Ok(())
    }

    pub async fn acquire_connection(&self) -> DatabaseResult<sqlx::pool::PoolConnection<Sqlite>> {
        let start = Instant::now();
        self.metrics.increment_active();

        let result = timeout(self.config.acquire_timeout, self.pool.acquire()).await;

        match result {
            std::result::Result::Ok(conn_result) => match conn_result {
                std::result::Result::Ok(conn) => {
                    self.metrics.increment_connections();
                    std::result::Result::Ok(conn)
                }
                Err(e) => {
                    self.metrics.decrement_active();
                    self.metrics.increment_errors();
                    Err(DatabaseError::connection_failed(
                        DatabaseType::SQLite,
                        format!("Failed to acquire connection: {}", e),
                    )
                    .with_context(
                        "acquire_timeout",
                        format!("{:?}", self.config.acquire_timeout),
                    )
                    .with_context("elapsed", format!("{:?}", start.elapsed())))
                }
            },
            Err(_) => {
                self.metrics.decrement_active();
                self.metrics.increment_errors();
                Err(DatabaseError::timeout(
                    DatabaseType::SQLite,
                    "connection_acquire",
                    self.config.acquire_timeout,
                ))
            }
        }
    }

    pub async fn execute(&self, sql: &str) -> DatabaseResult<sqlx::sqlite::SqliteQueryResult> {
        let start = Instant::now();
        let mut conn = self.acquire_connection().await?;

        let result = sqlx::query(sql).execute(&mut *conn).await;
        let duration = start.elapsed();

        self.metrics.decrement_active();
        self.metrics
            .record_query(std::cmp::max(1, duration.as_micros() as u64 / 1000));

        result.map_err(|e| {
            DatabaseError::query_failed(
                DatabaseType::SQLite,
                QueryType::Select,
                format!("Query execution failed: {}", e),
            )
            .with_context("duration_ms", duration.as_millis().to_string())
        })
    }

    pub async fn fetch_all<R>(&self, sql: &str) -> DatabaseResult<Vec<R>>
    where
        R: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let start = Instant::now();
        let mut conn = self.acquire_connection().await?;

        let result = sqlx::query_as::<_, R>(sql).fetch_all(&mut *conn).await;
        let duration = start.elapsed();

        self.metrics.decrement_active();
        self.metrics
            .record_query(std::cmp::max(1, duration.as_micros() as u64 / 1000));

        result.map_err(|e| {
            DatabaseError::query_failed(
                DatabaseType::SQLite,
                QueryType::Select,
                format!("Query fetch failed: {}", e),
            )
            .with_context("duration_ms", duration.as_millis().to_string())
            .with_context("sql", sql.to_string())
        })
    }

    pub async fn fetch_one<R>(&self, sql: &str) -> DatabaseResult<R>
    where
        R: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let start = Instant::now();
        let mut conn = self.acquire_connection().await?;

        let result = sqlx::query_as::<_, R>(sql).fetch_one(&mut *conn).await;
        let duration = start.elapsed();

        self.metrics.decrement_active();
        self.metrics
            .record_query(std::cmp::max(1, duration.as_micros() as u64 / 1000));

        result.map_err(|e| {
            DatabaseError::query_failed(
                DatabaseType::SQLite,
                QueryType::Select,
                format!("Query fetch one failed: {}", e),
            )
            .with_context("duration_ms", duration.as_millis().to_string())
        })
    }

    pub async fn begin_transaction(&self) -> DatabaseResult<sqlx::Transaction<'_, Sqlite>> {
        self.pool.begin().await.map_err(|e| {
            self.metrics.increment_errors();
            DatabaseError::query_failed(
                DatabaseType::SQLite,
                QueryType::Select,
                format!("Failed to begin transaction: {}", e),
            )
        })
    }

    pub async fn health_check(&self) -> DatabaseResult<HealthStatus> {
        let start = Instant::now();

        let result = sqlx::query("SELECT 1").fetch_one(&self.pool).await;

        let duration = start.elapsed();

        match result {
            Ok(_) => Ok(HealthStatus {
                is_healthy: true,
                response_time: duration,
                active_connections: self.metrics.active_connections.load(Ordering::Relaxed),
                total_connections: self.metrics.total_connections.load(Ordering::Relaxed),
                error_count: self.metrics.connection_errors.load(Ordering::Relaxed),
                avg_query_time_ms: self.metrics.average_query_time_ms() as u64,
            }),
            Err(e) => {
                self.metrics.increment_errors();
                Err(DatabaseError::HealthCheck {
                    message: format!("Health check failed: {}", e).into(),
                    database: DatabaseType::SQLite,
                    check_type: crate::errors::HealthCheckType::Query,
                    context: ErrorContext::new("health_check")
                        .with_severity(ErrorSeverity::Warning)
                        .with_component("sqlite_pool"),
                })
            }
        }
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    pub fn metrics(&self) -> &PoolMetrics {
        &self.metrics
    }

    pub fn config(&self) -> &SqlitePoolConfig {
        &self.config
    }
    pub fn is_closed(&self) -> bool {
        self.pool.is_closed()
    }
}

#[derive(Debug)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub response_time: Duration,
    pub active_connections: u32,
    pub total_connections: u32,
    pub error_count: u64,
    pub avg_query_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_pool() -> SqlitePool {
        let config = SqlitePoolConfig::builder()
            .url("sqlite::memory:")
            .max_connections(5)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(1))
            .build();

        SqlitePool::new(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let pool = create_test_pool().await;
        assert!(!pool.is_closed());
        assert_eq!(pool.config().max_connections, 5);
    }

    #[tokio::test]
    async fn test_health_check() {
        let pool = create_test_pool().await;
        let health = pool.health_check().await.unwrap();
        assert!(health.is_healthy);
        assert!(health.response_time.as_millis() < 1000);
    }

    #[tokio::test]
    async fn test_execute_query() {
        let pool = create_test_pool().await;

        let result = pool
            .execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        assert_eq!(result.rows_affected(), 0);
    }

    #[tokio::test]
    async fn test_transaction() {
        let pool = create_test_pool().await;

        pool.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .await
            .unwrap();

        let mut tx = pool.begin_transaction().await.unwrap();

        sqlx::query("INSERT INTO test (name) VALUES ('test')")
            .execute(&mut *tx)
            .await
            .unwrap();

        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let pool = create_test_pool().await;

        pool.execute("CREATE TABLE test_metrics (id INTEGER PRIMARY KEY, data TEXT)")
            .await
            .unwrap();

        let metrics = pool.metrics();
        assert!(metrics.query_count.load(Ordering::Relaxed) > 0);
        assert!(metrics.total_query_time_ms.load(Ordering::Relaxed) > 0);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let invalid_config = SqlitePoolConfig {
            url: "invalid://url".to_string(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: None,
            max_lifetime: None,
            enable_wal: true,
            enable_foreign_keys: true,
        };

        assert!(invalid_config.validate().is_err());
    }
}
