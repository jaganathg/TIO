use crate::config::DatabaseConfig;
use crate::errors::{DatabaseError, DatabaseResult, DatabaseType, ErrorContext};
use influxdb2::{models::DataPoint, Client};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct InfluxDBPoolConfig {
    pub url: String,
    pub token: String,
    pub org: String,
    pub bucket: String,
    pub timeout: Duration,
    pub retry_attempts: u32,
}

impl InfluxDBPoolConfig {
    pub fn builder() -> InfluxDBPoolConfigBuilder {
        InfluxDBPoolConfigBuilder::default()
    }

    pub fn from_database_config(config: &DatabaseConfig) -> Self {
        Self {
            url: config.influxdb.url.clone(),
            token: config.influxdb.token.clone(),
            org: config.influxdb.org.clone(),
            bucket: config.influxdb.bucket.clone(),
            timeout: Duration::from_secs(config.influxdb.timeout_secs),
            retry_attempts: 3,
        }
    }

    pub fn validate(&self) -> DatabaseResult<()> {
        if self.url.is_empty() {
            return Err(DatabaseError::Configuration {
                message: "InfluxDB URL cannot be empty".into(),
                database: DatabaseType::InfluxDB,
                context: ErrorContext::new("config_validation"),
            });
        }

        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(DatabaseError::Configuration {
                message: "InfluxDB URL must start with 'http://' or 'https://'".into(),
                database: DatabaseType::InfluxDB,
                context: ErrorContext::new("config_validation"),
            });
        }

        if self.bucket.is_empty() {
            return Err(DatabaseError::Configuration {
                message: "InfluxDB bucket name cannot be empty".into(),
                database: DatabaseType::InfluxDB,
                context: ErrorContext::new("config_validation"),
            });
        }

        if self.token.is_empty() {
            return Err(DatabaseError::Configuration {
                message: "InfluxDB token cannot be emtpy".into(),
                database: DatabaseType::InfluxDB,
                context: ErrorContext::new("config_validation"),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct InfluxDBPoolConfigBuilder {
    url: Option<String>,
    token: Option<String>,
    org: Option<String>,
    bucket: Option<String>,
    timeout: Option<Duration>,
    retry_attempts: Option<u32>,
}

impl InfluxDBPoolConfigBuilder {
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn org(mut self, org: impl Into<String>) -> Self {
        self.org = Some(org.into());
        self
    }

    pub fn bucket(mut self, bucket: impl Into<String>) -> Self {
        self.bucket = Some(bucket.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn retry_attempts(mut self, attempts: u32) -> Self {
        self.retry_attempts = Some(attempts);
        self
    }

    pub fn build(self) -> InfluxDBPoolConfig {
        InfluxDBPoolConfig {
            url: self
                .url
                .unwrap_or_else(|| "http://localhost:8086".to_string()),
            token: self.token.unwrap_or_else(|| "my-token".to_string()),
            org: self.org.unwrap_or_else(|| "my-org".to_string()),
            bucket: self.bucket.unwrap_or_else(|| "my-bucket".to_string()),
            timeout: self.timeout.unwrap_or(Duration::from_secs(10)),
            retry_attempts: self.retry_attempts.unwrap_or(3),
        }
    }
}

#[derive(Debug, Default)]
pub struct InfluxDBMetrics {
    pub write_count: AtomicU64,
    pub query_count: AtomicU64,
    pub total_write_time_ms: AtomicU64,
    pub total_query_time_ms: AtomicU64,
    pub connection_errors: AtomicU64,
    pub bytes_written: AtomicU64,
}

impl InfluxDBMetrics {
    pub fn record_write(&self, duration_ms: u64, bytes: u64) {
        self.write_count.fetch_add(1, Ordering::Relaxed);
        self.total_write_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_query(&self, duration_ms: u64) {
        self.query_count.fetch_add(1, Ordering::Relaxed);
        self.total_query_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn average_write_time_ms(&self) -> f64 {
        let count = self.write_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            (self.total_write_time_ms.load(Ordering::Relaxed) as f64) / (count as f64)
        }
    }
    pub fn average_query_time_ms(&self) -> f64 {
        let count = self.query_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            (self.total_query_time_ms.load(Ordering::Relaxed) as f64) / (count as f64)
        }
    }

    pub fn writes_per_second(&self) -> f64 {
        let total_time_secs = self.total_write_time_ms.load(Ordering::Relaxed) as f64 / 1000.0;
        let count = self.write_count.load(Ordering::Relaxed) as f64;

        if total_time_secs == 0.0 {
            0.0
        } else {
            count / total_time_secs
        }
    }

    pub fn bytes_per_second(&self) -> f64 {
        let total_time_secs = self.total_write_time_ms.load(Ordering::Relaxed) as f64 / 1000.0;
        let bytes = self.bytes_written.load(Ordering::Relaxed) as f64;

        if total_time_secs == 0.0 {
            0.0
        } else {
            bytes / total_time_secs
        }
    }
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

pub struct InfluxDBPool {
    client: Client,
    config: InfluxDBPoolConfig,
    metrics: InfluxDBMetrics,
}

impl InfluxDBPool {
    pub async fn new(config: InfluxDBPoolConfig) -> DatabaseResult<Self> {
        config.validate()?;

        let client = Client::new(&config.url, &config.org, &config.token);

        Ok(Self {
            client,
            config,
            metrics: InfluxDBMetrics::default(),
        })
    }

    pub async fn from_database_config(db_config: &DatabaseConfig) -> DatabaseResult<Self> {
        let config = InfluxDBPoolConfig::from_database_config(db_config);
        Self::new(config).await
    }

    pub async fn write_point(
        &self,
        measurement: &str,
        tags: Vec<(&str, &str)>,
        fields: Vec<(&str, FieldValue)>,
        timestamp: Option<i64>,
    ) -> DatabaseResult<()> {
        let start = Instant::now();

        let mut point = DataPoint::builder(measurement);

        if let Some(ts) = timestamp {
            point = point.timestamp(ts);
        }

        for (key, value) in tags {
            point = point.tag(key, value);
        }

        for (key, value) in fields {
            point = match value {
                FieldValue::String(s) => point.field(key, s.clone()),
                FieldValue::Integer(i) => point.field(key, i),
                FieldValue::Float(f) => point.field(key, f),
                FieldValue::Boolean(b) => point.field(key, b),
            };
        }

        let data_point = point.build().map_err(|e| DatabaseError::Configuration {
            message: format!("Failed to build DataPoint: {}", e).into(),
            database: DatabaseType::InfluxDB,
            context: ErrorContext::new("build_datapoint"),
        })?;

        let estimated_bytes = self.estimate_datapoint_size(&data_point);
        let result = timeout(
            self.config.timeout,
            self.client
                .write(&self.config.bucket, futures::stream::iter(vec![data_point])),
        )
        .await;

        let duration = start.elapsed();

        match result {
            Ok(Ok(_)) => {
                self.metrics.record_write(
                    std::cmp::max(1, duration.as_micros() as u64 / 1000),
                    estimated_bytes,
                );
                Ok(())
            }
            Ok(Err(e)) => {
                self.metrics.increment_errors();
                Err(DatabaseError::query_failed(
                    DatabaseType::InfluxDB,
                    crate::errors::QueryType::Insert,
                    format!("InfluxDB write failed: {}", e),
                )
                .with_context("duration_ms", duration.as_millis().to_string())
                .with_context("measurement", measurement.to_string()))
            }
            Err(_) => {
                self.metrics.increment_errors();
                Err(DatabaseError::timeout(
                    DatabaseType::InfluxDB,
                    "write_point",
                    self.config.timeout,
                ))
            }
        }
    }

    pub async fn write_points(&self, points: Vec<DataPoint>) -> DatabaseResult<()> {
        let start = Instant::now();

        let estimated_bytes: u64 = points.iter().map(|p| self.estimate_datapoint_size(p)).sum();

        let points_count = points.len();
        let result = timeout(
            self.config.timeout,
            self.client
                .write(&self.config.bucket, futures::stream::iter(points)),
        )
        .await;
        let duration = start.elapsed();

        match result {
            Ok(Ok(_)) => {
                self.metrics.record_write(
                    std::cmp::max(1, duration.as_micros() as u64 / 1000),
                    estimated_bytes,
                );
                Ok(())
            }
            Ok(Err(e)) => {
                self.metrics.increment_errors();
                Err(DatabaseError::query_failed(
                    DatabaseType::InfluxDB,
                    crate::errors::QueryType::Insert,
                    format!("InfluxDB batch write failed: {}", e),
                )
                .with_context("duration_ms", duration.as_millis().to_string())
                .with_context("points_count", points_count.to_string()))
            }
            Err(_) => {
                self.metrics.increment_errors();
                Err(DatabaseError::timeout(
                    DatabaseType::InfluxDB,
                    "write_points",
                    self.config.timeout,
                ))
            }
        }
    }

    pub async fn query<T>(&self, _query: &str) -> DatabaseResult<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        Err(DatabaseError::Configuration {
            message: "Generic queries require custom structs implementing FromDataPoint trait (not yet implemented)".into(),
            database: DatabaseType::InfluxDB,
            context: ErrorContext::new("generic_query_not_implemented"),
        })
    }

    pub async fn query_raw(&self, _query: &str) -> DatabaseResult<String> {
        Err(DatabaseError::Configuration {
            message: "Raw queries require proper Flux implementation (not yet implemented)".into(),
            database: DatabaseType::InfluxDB,
            context: ErrorContext::new("raw_query_not_implemented"),
        })
    }

    pub async fn create_bucket(&self, _bucket: &str) -> DatabaseResult<()> {
        Err(DatabaseError::Configuration {
            message: "Bucket management requires InfluxDB Management API (not yet implemented)"
                .into(),
            database: DatabaseType::InfluxDB,
            context: ErrorContext::new("bucket_management_not_implemented"),
        })
    }

    pub async fn bucket_exists(&self, _bucket: &str) -> DatabaseResult<bool> {
        Ok(true)
    }

    pub async fn drop_bucket(&self, _bucket: &str) -> DatabaseResult<()> {
        Err(DatabaseError::Configuration {
            message: "Bucket management requires InfluxDB Management API (not yet implemented)"
                .into(),
            database: DatabaseType::InfluxDB,
            context: ErrorContext::new("bucket_management_not_implemented"),
        })
    }

    pub async fn health_check(&self) -> DatabaseResult<InfluxDBHealthStatus> {
        let start = Instant::now();

        let bucket_exists_result = self.bucket_exists(&self.config.bucket).await;
        let bucket_exists = bucket_exists_result.is_ok() && bucket_exists_result.unwrap();

        let write_test = self
            .write_point(
                "__health_check__",
                vec![("test", "true")],
                vec![("value", FieldValue::Integer(1))],
                None,
            )
            .await
            .is_ok();

        let query_test = self.query_raw("bucket() > limit(n:1)").await.is_ok();

        if write_test {
            let _ = self
                .query_raw("drop(measurement: \"__health_check__\")")
                .await;
        }

        let duration = start.elapsed();
        let is_healthy = bucket_exists && write_test && query_test;

        Ok(InfluxDBHealthStatus {
            is_healthy,
            response_time: duration,
            bucket_exists,
            write_test_success: write_test,
            query_test_success: query_test,
            error_count: self.metrics.connection_errors.load(Ordering::Relaxed),
            avg_write_time_ms: self.metrics.average_write_time_ms(),
            avg_query_time_ms: self.metrics.average_query_time_ms(),
        })
    }

    fn estimate_datapoint_size(&self, _data_point: &DataPoint) -> u64 {
        128
    }

    pub fn config(&self) -> &InfluxDBPoolConfig {
        &self.config
    }

    pub fn metrics(&self) -> &InfluxDBMetrics {
        &self.metrics
    }
}

#[derive(Debug)]
pub struct InfluxDBHealthStatus {
    pub is_healthy: bool,
    pub response_time: Duration,
    pub bucket_exists: bool,
    pub write_test_success: bool,
    pub query_test_success: bool,
    pub error_count: u64,
    pub avg_write_time_ms: f64,
    pub avg_query_time_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_pool() -> InfluxDBPool {
        let config = InfluxDBPoolConfig::builder()
            .url("http://localhost:8086")
            .token("my-token")
            .org("my-org")
            .bucket("test_bucket")
            .timeout(Duration::from_secs(5))
            .build();

        InfluxDBPool::new(config)
            .await
            .expect("Failed to create test InfluxDB pool")
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let pool = create_test_pool().await;
        assert_eq!(pool.config().bucket, "test_bucket");
        assert_eq!(pool.config().timeout, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_write_operations() {
        let pool = create_test_pool().await;

        let result = pool
            .write_point(
                "test_measurement",
                vec![("tag1", "value1")],
                vec![("field1", FieldValue::Integer(42))],
                None,
            )
            .await;

        println!("Write result: {:?}", result);
    }

    #[tokio::test]
    async fn test_config_validation() {
        let invalid_config = InfluxDBPoolConfig {
            url: "invalid://url".to_string(),
            token: "".to_string(),
            org: "test".to_string(),
            bucket: "test".to_string(),
            timeout: Duration::from_secs(10),
            retry_attempts: 3,
        };

        assert!(invalid_config.validate().is_err());
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let pool = create_test_pool().await;
        let metrics = pool.metrics();

        assert_eq!(metrics.write_count.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.query_count.load(Ordering::Relaxed), 0);

        metrics.record_write(100, 256);
        metrics.record_query(50);

        assert_eq!(metrics.write_count.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.query_count.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.average_write_time_ms(), 100.0);
        assert_eq!(metrics.average_query_time_ms(), 50.0);
    }
}
