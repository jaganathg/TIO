use serde::{Deserialize, Serialize};

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
