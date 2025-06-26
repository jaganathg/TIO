use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file error: {0}")]
    File(#[from] config::ConfigError),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

pub struct DatabaseConfigBuilder {
    sqlite: Option<SqliteConfig>,
    redis: Option<RedisConfig>,
    influxdb: Option<InfluxDbConfig>,
    chromadb: Option<ChromaDbConfig>,
}

impl DatabaseConfigBuilder {
    pub fn new() -> Self {
        Self {
            sqlite: None,
            redis: None,
            influxdb: None,
            chromadb: None,
        }
    }

    pub fn with_sqlite(mut self, sqlite: SqliteConfig) -> Self {
        self.sqlite = Some(sqlite);
        self
    }

    pub fn with_redis(mut self, redis: RedisConfig) -> Self {
        self.redis = Some(redis);
        self
    }

    pub fn with_influxdb(mut self, influxdb: InfluxDbConfig) -> Self {
        self.influxdb = Some(influxdb);
        self
    }

    pub fn with_chromadb(mut self, chromadb: ChromaDbConfig) -> Self {
        self.chromadb = Some(chromadb);
        self
    }

    pub fn build(self) -> Result<DatabaseConfig, ConfigError> {
        let config = DatabaseConfig {
            sqlite: self
                .sqlite
                .ok_or(ConfigError::MissingField("sqlite".into()))?,
            redis: self
                .redis
                .ok_or(ConfigError::MissingField("redis".into()))?,
            influxdb: self
                .influxdb
                .ok_or(ConfigError::MissingField("influxdb".into()))?,
            chromadb: self
                .chromadb
                .ok_or(ConfigError::MissingField("chromadb".into()))?,
        };

        config.validate()?;
        Ok(config)
    }
}

/// Database configuration for all supported databases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// SQLite database configuration
    pub sqlite: SqliteConfig,

    /// Redis configuration  
    pub redis: RedisConfig,

    /// InfluxDB configuration
    pub influxdb: InfluxDbConfig,

    /// ChromaDB configuration
    pub chromadb: ChromaDbConfig,
}

/// SQLite-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// Database file path (e.g., "sqlite:./data/app.db")
    pub url: String,

    /// Maximum number of connections in the pool
    #[serde(default = "default_sqlite_max_connections")]
    pub max_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// SQLite-specific settings
    #[serde(default)]
    pub enable_wal: bool,

    #[serde(default = "default_busy_timeout")]
    pub busy_timeout_ms: u32,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL (e.g., "redis://:password@localhost:6379")
    pub url: String,

    /// Database number to use (0-15)
    #[serde(default)]
    pub database: u8,

    /// Maximum number of connections in the pool
    #[serde(default = "default_redis_max_connections")]
    pub max_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,

    /// Idle connection timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
}

/// InfluxDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluxDbConfig {
    /// InfluxDB server URL (e.g., "http://localhost:8086")
    pub url: String,

    /// Authentication token
    pub token: String,

    /// Organization name
    pub org: String,

    /// Default bucket for market data
    pub bucket: String,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub timeout_secs: u64,
}

/// ChromaDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaDbConfig {
    /// ChromaDB server URL (e.g., "http://localhost:8000")
    pub url: String,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub timeout_secs: u64,

    /// Maximum retries for failed requests
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

// Default value functions
fn default_sqlite_max_connections() -> u32 {
    10
}
fn default_redis_max_connections() -> u32 {
    20
}
fn default_connection_timeout() -> u64 {
    30
}
fn default_idle_timeout() -> u64 {
    300
}
fn default_busy_timeout() -> u32 {
    30_000
}
fn default_request_timeout() -> u64 {
    30
}
fn default_max_retries() -> u32 {
    3
}

impl DatabaseConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.sqlite.validate()?;
        self.redis.validate()?;
        self.influxdb.validate()?;
        self.chromadb.validate()?;
        Ok(())
    }

    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .build()?;

        settings.try_deserialize()
    }

    /// Load configuration from environment variables and defaults
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::Environment::with_prefix("DATABASE"))
            .build()?;

        settings.try_deserialize()
    }

    pub fn load_with_validation(path: &str) -> Result<Self, ConfigError> {
        let config = Self::from_file(path).map_err(ConfigError::File)?;
        config.validate()?;
        Ok(config)
    }

    pub fn from_env_with_profile(profile: &str) -> Result<Self, ConfigError> {
        let prefix = format!("DATABASE_{}", profile.to_uppercase());
        let settings = config::Config::builder()
            .add_source(config::Environment::with_prefix(&prefix))
            .build()
            .map_err(ConfigError::File)?;

        let config: Self = settings.try_deserialize().map_err(ConfigError::File)?;

        config.validate()?;
        Ok(config)
    }

    pub fn from_env_with_validation() -> Result<Self, ConfigError> {
        let config = Self::from_env().map_err(ConfigError::File)?;
        config.validate()?;
        Ok(config)
    }

    /// Create a default development configuration
    pub fn development() -> Self {
        DatabaseConfig {
            sqlite: SqliteConfig {
                url: "sqlite:./data/app.db".to_string(),
                max_connections: 10,
                connection_timeout_secs: 30,
                enable_wal: true,
                busy_timeout_ms: 30_000,
            },
            redis: RedisConfig {
                url: "redis://:redispassword@localhost:6379".to_string(),
                database: 0,
                max_connections: 20,
                connection_timeout_secs: 30,
                idle_timeout_secs: 300,
            },
            influxdb: InfluxDbConfig {
                url: "http://localhost:8086".to_string(),
                token: "my-super-secret-auth-token".to_string(),
                org: "trading-org".to_string(),
                bucket: "market-data".to_string(),
                timeout_secs: 30,
            },
            chromadb: ChromaDbConfig {
                url: "http://localhost:8000".to_string(),
                timeout_secs: 30,
                max_retries: 3,
            },
        }
    }

    pub fn production() -> Self {
        DatabaseConfig {
            sqlite: SqliteConfig {
                url: "sqlite:./data/production.db".to_string(),
                max_connections: 50,
                connection_timeout_secs: 60,
                enable_wal: true,
                busy_timeout_ms: 60_000,
            },
            redis: RedisConfig {
                url: "redis://:redispassword@localhost:6379".to_string(),
                database: 0,
                max_connections: 100,
                connection_timeout_secs: 60,
                idle_timeout_secs: 600,
            },
            influxdb: InfluxDbConfig {
                url: "http://localhost:8086".to_string(),
                token: "my-super-secret-auth-token".to_string(),
                org: "trading-org".to_string(),
                bucket: "market-data".to_string(),
                timeout_secs: 30,
            },
            chromadb: ChromaDbConfig {
                url: "http://localhost:8000".to_string(),
                timeout_secs: 30,
                max_retries: 3,
            },
        }
    }

    pub fn testing() -> Self {
        DatabaseConfig {
            sqlite: SqliteConfig {
                url: "sqlite::memory:".to_string(),
                max_connections: 1,
                connection_timeout_secs: 5,
                enable_wal: false,
                busy_timeout_ms: 1000,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                database: 15,
                max_connections: 5,
                connection_timeout_secs: 5,
                idle_timeout_secs: 30,
            },
            influxdb: InfluxDbConfig {
                url: "http://localhost:8086".to_string(),
                token: "test-token".to_string(),
                org: "test-org".to_string(),
                bucket: "test-data".to_string(),
                timeout_secs: 5,
            },
            chromadb: ChromaDbConfig {
                url: "http://localhost:8000".to_string(),
                timeout_secs: 5,
                max_retries: 1,
            },
        }
    }
}

impl SqliteConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.url.is_empty() {
            return Err(ConfigError::InvalidValue(
                "SQLite URL cannot be empty".into(),
            ));
        }
        if !self.url.starts_with("sqlite:") {
            return Err(ConfigError::InvalidUrl(
                "SQLite URL must start with 'sqlite:'".into(),
            ));
        }
        if self.max_connections == 0 {
            return Err(ConfigError::InvalidValue(
                "SQLite max_connections must be > 0".into(),
            ));
        }
        Ok(())
    }
}

impl RedisConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.url.is_empty() {
            return Err(ConfigError::InvalidValue(
                "Redis URL cannot be empty".into(),
            ));
        }
        if !self.url.starts_with("redis:") {
            return Err(ConfigError::InvalidUrl(
                "Redis URL must start with 'redis:'".into(),
            ));
        }
        if self.max_connections == 0 {
            return Err(ConfigError::InvalidValue(
                "Redis max_connections must be > 0".into(),
            ));
        }
        Ok(())
    }
}

impl InfluxDbConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.url.is_empty() {
            return Err(ConfigError::InvalidValue(
                "InfluxDB URL cannot be empty".into(),
            ));
        }
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(ConfigError::InvalidUrl(
                "InfluxDB URL must start with 'http://' or 'https://'".into(),
            ));
        }
        Ok(())
    }
}

impl ChromaDbConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.url.is_empty() {
            return Err(ConfigError::InvalidValue(
                "ChromaDB URL cannot be empty".into(),
            ));
        }
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(ConfigError::InvalidUrl(
                "ChromaDB URL must start with 'http://' or 'https://'".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_development_config() {
        let config = DatabaseConfig::development();

        assert_eq!(config.sqlite.url, "sqlite:./data/app.db");
        assert_eq!(config.redis.database, 0);
        assert_eq!(config.influxdb.org, "trading-org");
        assert_eq!(config.chromadb.max_retries, 3);
    }

    #[test]
    fn test_config_serialization() {
        let config = DatabaseConfig::development();

        // Test serialization
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("sqlite"));
        assert!(toml_str.contains("redis"));

        // Test deserialization
        let parsed: DatabaseConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.sqlite.url, config.sqlite.url);
    }
}
