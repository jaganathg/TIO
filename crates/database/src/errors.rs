use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseError {
    #[error("Connection error: {message}")]
    Connection {
        message: String,
        database: DatabaseType,
        context: ErrorContext,
    },

    #[error("Pool error: {message}")]
    Pool {
        message: String,
        database: DatabaseType,
        pool_state: PoolState,
        context: ErrorContext,
    },

    #[error("Query error: {message}")]
    Query {
        message: String,
        database: DatabaseType,
        query_type: QueryType,
        context: ErrorContext,
    },

    #[error("Migration error: {message}")]
    Migration {
        message: String,
        database: DatabaseType,
        migration_version: Option<String>,
        context: ErrorContext,
    },

    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        database: DatabaseType,
        context: ErrorContext,
    },

    #[error("Timeout error: {message}")]
    Timeout {
        message: String,
        database: DatabaseType,
        operation: String,
        timeout_duration: std::time::Duration,
        context: ErrorContext,
    },

    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        database: DatabaseType,
        data_type: String,
        context: ErrorContext,
    },

    #[error("Health check failed: {message}")]
    HealthCheck {
        message: String,
        database: DatabaseType,
        check_type: HealthCheckType,
        context: ErrorContext,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseType {
    SQLite,
    Redis,
    InfluxDB,
    ChromaDB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
    CreateTable,
    Migration,
    HealthCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolState {
    Exhausted,
    Disconnected,
    Unhealthy,
    Initializing,
    ShuttingDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthCheckType {
    Connection,
    Query,
    Pool,
    Migration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub timestamp: SystemTime,
    pub operation: String,
    pub component: String,
    pub correlation_id: Option<String>,
    pub retry_count: u32,
    pub additional_info: std::collections::HashMap<String, String>,
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;
pub type PoolResult<T> = Result<T, DatabaseError>;
pub type QueryResult<T> = Result<T, DatabaseError>;
pub type MigrationResult<T> = Result<T, DatabaseError>;

impl DatabaseError {
    pub fn connection_failed(database: DatabaseType, message: impl Into<String>) -> Self {
        DatabaseError::Connection {
            message: message.into(),
            database,
            context: ErrorContext::new("connection_failed"),
        }
    }

    pub fn pool_exhausted(database: DatabaseType, message: impl Into<String>) -> Self {
        DatabaseError::Pool {
            message: message.into(),
            database,
            pool_state: PoolState::Exhausted,
            context: ErrorContext::new("pool_exhausted"),
        }
    }

    pub fn query_failed(
        database: DatabaseType,
        query_type: QueryType,
        message: impl Into<String>,
    ) -> Self {
        DatabaseError::Query {
            message: message.into(),
            database,
            query_type,
            context: ErrorContext::new("query_failed"),
        }
    }

    pub fn timeout(
        database: DatabaseType,
        operation: impl Into<String>,
        duration: std::time::Duration,
    ) -> Self {
        DatabaseError::Timeout {
            message: format!("Operation timed out after {:?}", duration),
            database,
            operation: operation.into(),
            timeout_duration: duration,
            context: ErrorContext::new("timeout"),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        match &mut self {
            DatabaseError::Connection { context, .. }
            | DatabaseError::Pool { context, .. }
            | DatabaseError::Query { context, .. }
            | DatabaseError::Migration { context, .. }
            | DatabaseError::Configuration { context, .. }
            | DatabaseError::Timeout { context, .. }
            | DatabaseError::Serialization { context, .. }
            | DatabaseError::HealthCheck { context, .. } => {
                context.additional_info.insert(key.into(), value.into());
            }
        }
        self
    }
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            timestamp: SystemTime::now(),
            operation: operation.into(),
            component: "database".to_string(),
            correlation_id: None,
            retry_count: 0,
            additional_info: std::collections::HashMap::new(),
        }
    }

    pub fn with_component(mut self, component: impl Into<String>) -> Self {
        self.component = component.into();
        self
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn increase_retry(mut self) -> Self {
        self.retry_count += 1;
        self
    }
}

impl From<crate::config::ConfigError> for DatabaseError {
    fn from(value: crate::config::ConfigError) -> Self {
        DatabaseError::Configuration {
            message: value.to_string(),
            database: DatabaseType::SQLite,
            context: ErrorContext::new("configuration_error"),
        }
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::PoolTimedOut => {
                DatabaseError::pool_exhausted(DatabaseType::SQLite, "Connection pool timed out")
            }
            sqlx::Error::Database(db_err) => DatabaseError::query_failed(
                DatabaseType::SQLite,
                QueryType::Select,
                db_err.message(),
            ),
            _ => DatabaseError::connection_failed(DatabaseType::SQLite, value.to_string()),
        }
    }
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseType::SQLite => write!(f, "SQLite"),
            DatabaseType::Redis => write!(f, "Redis"),
            DatabaseType::InfluxDB => write!(f, "InfluxDB"),
            DatabaseType::ChromaDB => write!(f, "ChromaDB"),
        }
    }
}
