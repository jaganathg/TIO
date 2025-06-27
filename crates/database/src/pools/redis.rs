use crate::config::DatabaseConfig;
use crate::errors::{DatabaseError, DatabaseResult, DatabaseType, ErrorContext, ErrorSeverity};
use bb8_redis::{bb8::Pool, RedisConnectionManager};
use redis::{AsyncCommands, RedisResult};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct RedisPoolConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub connection_timeout: Duration,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
    pub retry_attempts: u32,
}

impl RedisPoolConfig {
    pub fn builder() -> RedisPoolConfigBuilder {
        RedisPoolConfigBuilder::default()
    }

    pub fn from_database_config(config: &DatabaseConfig) -> Self {
        Self {
            url: config.redis.url.clone(),
            max_connections: config.redis.max_connections,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(config.redis.connection_timeout_secs),
            connection_timeout: Duration::from_secs(10),
            read_timeout: Some(Duration::from_secs(5)),
            write_timeout: Some(Duration::from_secs(5)),
            retry_attempts: 3,
        }
    }

    pub fn validate(&self) -> DatabaseResult<()> {
        if self.url.is_empty() {
            return Err(DatabaseError::Configuration {
                message: "Redis URL cannot be empty".into(),
                database: DatabaseType::Redis,
                context: ErrorContext::new("config_validation"),
            });
        }

        if !self.url.starts_with("redis://") && !self.url.starts_with("rediss://") {
            return Err(DatabaseError::Configuration {
                message: "Redis URL must start with 'redis://' or 'rediss://'".into(),
                database: DatabaseType::Redis,
                context: ErrorContext::new("config_validation"),
            });
        }

        if self.max_connections == 0 {
            return Err(DatabaseError::Configuration {
                message: "max_connection must be > 0".into(),
                database: DatabaseType::Redis,
                context: ErrorContext::new("config_validation"),
            });
        }

        if self.min_connections > self.max_connections {
            return Err(DatabaseError::Configuration {
                message: "min_connections cannot exceed max_connections".into(),
                database: DatabaseType::Redis,
                context: ErrorContext::new("config_validation"),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct RedisPoolConfigBuilder {
    url: Option<String>,
    max_connections: Option<u32>,
    min_connections: Option<u32>,
    acquire_timeout: Option<Duration>,
    connection_timeout: Option<Duration>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    retry_attempts: Option<u32>,
}

impl RedisPoolConfigBuilder {
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

    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = Some(timeout);
        self
    }

    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = Some(timeout);
        self
    }

    pub fn write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = Some(timeout);
        self
    }

    pub fn retry_attempts(mut self, attempts: u32) -> Self {
        self.retry_attempts = Some(attempts);
        self
    }

    pub fn build(self) -> RedisPoolConfig {
        RedisPoolConfig {
            url: self
                .url
                .unwrap_or_else(|| "redis://localhost:6379".to_string()),
            max_connections: self.max_connections.unwrap_or(20),
            min_connections: self.min_connections.unwrap_or(2),
            acquire_timeout: self.acquire_timeout.unwrap_or(Duration::from_secs(30)),
            connection_timeout: self.connection_timeout.unwrap_or(Duration::from_secs(10)),
            read_timeout: self.read_timeout,
            write_timeout: self.write_timeout,
            retry_attempts: self.retry_attempts.unwrap_or(3),
        }
    }
}

#[derive(Debug, Default)]
pub struct RedisMetrics {
    pub total_connections: AtomicU32,
    pub active_connections: AtomicU32,
    pub connection_errors: AtomicU64,
    pub command_count: AtomicU64,
    pub total_command_time_ms: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

impl RedisMetrics {
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

    pub fn record_command(&self, duration_ms: u64) {
        self.command_count.fetch_add(1, Ordering::Relaxed);
        self.total_command_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn average_command_time_ms(&self) -> f64 {
        let count = self.command_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            (self.total_command_time_ms.load(Ordering::Relaxed) as f64) / (count as f64)
        }
    }
    pub fn cache_hit_ratio(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }
}

pub struct RedisPool {
    pool: Pool<RedisConnectionManager>,
    config: RedisPoolConfig,
    metrics: RedisMetrics,
}

impl RedisPool {
    pub async fn new(config: RedisPoolConfig) -> DatabaseResult<Self> {
        config.validate()?;

        let manager = RedisConnectionManager::new(config.url.clone()).map_err(|e| {
            DatabaseError::connection_failed(
                DatabaseType::Redis,
                format!("Failed to create Redis connection manager: {}", e),
            )
        })?;

        let pool = Pool::builder()
            .max_size(config.max_connections)
            .min_idle(Some(config.min_connections))
            .connection_timeout(config.connection_timeout)
            .build(manager)
            .await
            .map_err(|e| {
                DatabaseError::connection_failed(
                    DatabaseType::Redis,
                    format!("Failed to create Redis pool: {}", e),
                )
                .with_context("url", config.url.clone())
                .with_context("max_connections", config.max_connections.to_string())
            })?;

        Ok(Self {
            pool,
            config,
            metrics: RedisMetrics::default(),
        })
    }

    pub async fn from_database_config(db_config: &DatabaseConfig) -> DatabaseResult<Self> {
        let config = RedisPoolConfig::from_database_config(db_config);
        Self::new(config).await
    }

    pub async fn get<K: redis::ToRedisArgs + Send + Sync>(
        &self,
        key: K,
    ) -> DatabaseResult<Option<String>> {
        let start = Instant::now();
        let mut conn = self.acquire_connection().await?;

        let result: RedisResult<Option<String>> = conn.get(key).await;
        let duration = start.elapsed();

        self.metrics.decrement_active();
        self.metrics
            .record_command(std::cmp::max(1, duration.as_micros() as u64 / 1000));

        match result {
            Ok(Some(value)) => {
                self.metrics.record_cache_hit();
                Ok(Some(value))
            }
            Ok(None) => {
                self.metrics.record_cache_miss();
                Ok(None)
            }
            Err(e) => {
                self.metrics.increment_errors();
                Err(DatabaseError::query_failed(
                    DatabaseType::Redis,
                    crate::errors::QueryType::Select,
                    format!("Redis GET failed: {}", e),
                )
                .with_context("duration_ms", duration.as_millis().to_string()))
            }
        }
    }

    pub async fn set<K: redis::ToRedisArgs + Send + Sync, V: redis::ToRedisArgs + Send + Sync>(
        &self,
        key: K,
        value: V,
        expiration_secs: Option<u64>,
    ) -> DatabaseResult<()> {
        let start = Instant::now();
        let mut conn = self.acquire_connection().await?;

        let result = match expiration_secs {
            Some(exp) => conn.set_ex(key, value, exp).await,
            None => conn.set(key, value).await,
        };
        let duration = start.elapsed();

        self.metrics.decrement_active();
        self.metrics
            .record_command(std::cmp::max(1, duration.as_micros() as u64 / 1000));

        result.map_err(|e| {
            self.metrics.increment_errors();
            DatabaseError::query_failed(
                DatabaseType::Redis,
                crate::errors::QueryType::Insert,
                format!("Redis SET failed: {}", e),
            )
            .with_context("duration_ms", duration.as_millis().to_string())
        })
    }

    pub async fn del<K: redis::ToRedisArgs + Send + Sync>(&self, key: K) -> DatabaseResult<bool> {
        let start = Instant::now();
        let mut conn = self.acquire_connection().await?;

        let result: RedisResult<i32> = conn.del(key).await;
        let duration = start.elapsed();

        self.metrics.decrement_active();
        self.metrics
            .record_command(std::cmp::max(1, duration.as_micros() as u64 / 1000));

        result.map(|deleted_count| deleted_count > 0).map_err(|e| {
            self.metrics.increment_errors();
            DatabaseError::query_failed(
                DatabaseType::Redis,
                crate::errors::QueryType::Delete,
                format!("Redis DEL failed: {}", e),
            )
            .with_context("duration_ms", duration.as_millis().to_string())
        })
    }

    pub async fn exists<K: redis::ToRedisArgs + Send + Sync>(
        &self,
        key: K,
    ) -> DatabaseResult<bool> {
        let start = Instant::now();
        let mut conn = self.acquire_connection().await?;

        let result: RedisResult<bool> = conn.exists(key).await;
        let duration = start.elapsed();

        self.metrics.decrement_active();
        self.metrics
            .record_command(std::cmp::max(1, duration.as_micros() as u64 / 1000));

        result.map_err(|e| {
            self.metrics.increment_errors();
            DatabaseError::query_failed(
                DatabaseType::Redis,
                crate::errors::QueryType::Select,
                format!("Redis EXISTS failed: {}", e),
            )
            .with_context("duration_ms", duration.as_millis().to_string())
        })
    }
    pub async fn expire<K: redis::ToRedisArgs + Send + Sync>(
        &self,
        key: K,
        expiration_secs: u64,
    ) -> DatabaseResult<bool> {
        let start = Instant::now();
        let mut conn = self.acquire_connection().await?;

        let result: RedisResult<bool> = conn.expire(key, expiration_secs as i64).await;
        let duration = start.elapsed();

        self.metrics.decrement_active();
        self.metrics
            .record_command(std::cmp::max(1, duration.as_micros() as u64 / 1000));

        result.map_err(|e| {
            self.metrics.increment_errors();
            DatabaseError::query_failed(
                DatabaseType::Redis,
                crate::errors::QueryType::Update,
                format!("Redis EXPIRE failed: {}", e),
            )
            .with_context("duration_ms", duration.as_millis().to_string())
        })
    }

    pub async fn incr<K: redis::ToRedisArgs + Send + Sync>(&self, key: K) -> DatabaseResult<i64> {
        let start = Instant::now();
        let mut conn = self.acquire_connection().await?;

        let result: RedisResult<i64> = conn.incr(key, 1).await;
        let duration = start.elapsed();

        self.metrics.decrement_active();
        self.metrics
            .record_command(std::cmp::max(1, duration.as_micros() as u64 / 1000));

        result.map_err(|e| {
            self.metrics.increment_errors();
            DatabaseError::query_failed(
                DatabaseType::Redis,
                crate::errors::QueryType::Update,
                format!("Redis INCR failed: {}", e),
            )
            .with_context("duration_ms", duration.as_millis().to_string())
        })
    }

    async fn acquire_connection(
        &self,
    ) -> DatabaseResult<bb8::PooledConnection<RedisConnectionManager>> {
        let start = Instant::now();
        self.metrics.increment_active();

        let result = timeout(self.config.acquire_timeout, self.pool.get()).await;

        match result {
            Ok(Ok(conn)) => {
                self.metrics.increment_connections();
                Ok(conn)
            }
            Ok(Err(e)) => {
                self.metrics.decrement_active();
                self.metrics.increment_errors();
                Err(DatabaseError::connection_failed(
                    DatabaseType::Redis,
                    format!("Failed to acquire Redis connection: {}", e),
                )
                .with_context(
                    "acquire_timeout",
                    format!("{:?}", self.config.acquire_timeout),
                )
                .with_context("elapsed", format!("{:?}", start.elapsed())))
            }
            Err(_) => {
                self.metrics.decrement_active();
                self.metrics.increment_errors();
                Err(DatabaseError::timeout(
                    DatabaseType::Redis,
                    "connection_acquire",
                    self.config.acquire_timeout,
                ))
            }
        }
    }

    pub async fn health_check(&self) -> DatabaseResult<RedisHealthStatus> {
        let start = Instant::now();

        let result = self.get("__health_check__").await;
        let duration = start.elapsed();

        match result {
            Ok(_) => Ok(RedisHealthStatus {
                is_healthy: true,
                response_time: duration,
                active_connections: self.metrics.active_connections.load(Ordering::Relaxed),
                total_connections: self.metrics.total_connections.load(Ordering::Relaxed),
                error_count: self.metrics.connection_errors.load(Ordering::Relaxed),
                avg_command_time_ms: self.metrics.average_command_time_ms() as u64,
                cache_hit_ratio: self.metrics.cache_hit_ratio(),
            }),
            Err(e) => {
                self.metrics.increment_errors();
                Err(DatabaseError::HealthCheck {
                    message: format!("Redis health check failed: {}", e).into(),
                    database: DatabaseType::Redis,
                    check_type: crate::errors::HealthCheckType::Connection,
                    context: ErrorContext::new("health_check")
                        .with_severity(ErrorSeverity::Warning)
                        .with_component("redis_pool"),
                })
            }
        }
    }

    pub fn config(&self) -> &RedisPoolConfig {
        &self.config
    }

    pub fn metrics(&self) -> &RedisMetrics {
        &self.metrics
    }

    pub fn is_closed(&self) -> bool {
        self.pool.state().connections == 0 && self.pool.state().idle_connections == 0
    }
}

#[derive(Debug)]
pub struct RedisHealthStatus {
    pub is_healthy: bool,
    pub response_time: Duration,
    pub active_connections: u32,
    pub total_connections: u32,
    pub error_count: u64,
    pub avg_command_time_ms: u64,
    pub cache_hit_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_pool() -> RedisPool {
        let config = RedisPoolConfig::builder()
            .url("redis://localhost:6379")
            .max_connections(5)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(1))
            .build();

        RedisPool::new(config)
            .await
            .expect("Failed to create test Redis pool")
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let pool = create_test_pool().await;
        assert!(!pool.is_closed());
        assert_eq!(pool.config().max_connections, 5);
    }

    #[tokio::test]
    async fn test_basic_operations() {
        let pool = create_test_pool().await;

        pool.set("test_key", "test_value", Some(60)).await.unwrap();

        let value = pool.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        let exists = pool.exists("test_key").await.unwrap();
        assert!(exists);

        let deleted = pool.del("test_key").await.unwrap();
        assert!(deleted);

        let value_after_del = pool.get("test_key").await.unwrap();
        assert_eq!(value_after_del, None);
    }

    #[tokio::test]
    async fn test_increment_operations() {
        let pool = create_test_pool().await;

        let value1 = pool.incr("counter").await.unwrap();
        assert_eq!(value1, 1);

        let value2 = pool.incr("counter").await.unwrap();
        assert_eq!(value2, 2);

        pool.del("counter").await.unwrap();
    }

    #[tokio::test]
    async fn test_expiration() {
        let pool = create_test_pool().await;

        pool.set("expire_test", "value", None).await.unwrap();

        let set_expire = pool.expire("expire_test", 1).await.unwrap();
        assert!(set_expire);

        let exists_before = pool.exists("exists_test").await.unwrap();
        assert!(exists_before);

        tokio::time::sleep(Duration::from_secs(2)).await;

        let exists_after = pool.exists("expire_test").await.unwrap();
        assert!(!exists_after);
    }

    #[tokio::test]
    async fn test_metircs_tracking() {
        let pool = create_test_pool().await;

        pool.set("metrics_test", "value", None).await.unwrap();
        pool.get("metrics_test").await.unwrap();
        pool.get("nonexistent_key").await.unwrap();

        let metrics = pool.metrics();
        assert!(metrics.command_count.load(Ordering::Relaxed) >= 3);
        assert!(metrics.cache_hits.load(Ordering::Relaxed) >= 1);
        assert!(metrics.cache_misses.load(Ordering::Relaxed) >= 1);

        pool.del("metrics_test").await.unwrap();
    }

    #[tokio::test]
    async fn test_health_check() {
        let pool = create_test_pool().await;
        let health = pool.health_check().await.unwrap();

        assert!(health.is_healthy);
        assert!(health.response_time.as_millis() < 1000);
        assert!(health.cache_hit_ratio >= 0.0);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let invalid_config = RedisPoolConfig {
            url: "invalid://url".to_string(),
            max_connections: 10,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
            read_timeout: None,
            write_timeout: None,
            retry_attempts: 3,
        };

        assert!(invalid_config.validate().is_err());
    }
}
