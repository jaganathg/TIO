use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use thiserror::Error;
use std::borrow::Cow;
use std::cell::OnceCell;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseError {
    #[error("Connection error: {message}")]
    Connection {
        message: Cow<'static, str>,
        database: DatabaseType,
        context: ErrorContext,
    },

    #[error("Pool error: {message}")]
    Pool {
        message: Cow<'static, str>,
        database: DatabaseType,
        pool_state: PoolState,
        context: ErrorContext,
    },

    #[error("Query error: {message}")]
    Query {
        message: Cow<'static, str>,
        database: DatabaseType,
        query_type: QueryType,
        context: ErrorContext,
    },

    #[error("Migration error: {message}")]
    Migration {
        message: Cow<'static, str>,
        database: DatabaseType,
        migration_version: Option<String>,
        context: ErrorContext,
    },

    #[error("Configuration error: {message}")]
    Configuration {
        message: Cow<'static, str>,
        database: DatabaseType,
        context: ErrorContext,
    },

    #[error("Timeout error: {message}")]
    Timeout {
        message: Cow<'static, str>,
        database: DatabaseType,
        operation: String,
        timeout_duration: std::time::Duration,
        context: ErrorContext,
    },

    #[error("Serialization error: {message}")]
    Serialization {
        message: Cow<'static, str>,
        database: DatabaseType,
        data_type: String,
        context: ErrorContext,
    },

    #[error("Health check failed: {message}")]
    HealthCheck {
        message: Cow<'static, str>,
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
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Default for ErrorSeverity {
    fn default() -> Self {
        ErrorSeverity::Error
    }
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
    #[serde(skip)]
    pub timestamp_cell: OnceCell<SystemTime>,

    pub operation: Cow<'static, str>,
    pub component: Cow<'static, str>,
    pub correlation_id: Option<String>,
    pub retry_count: u32,
    pub severity: ErrorSeverity,
    pub additional_info: Vec<(Cow<'static, str>, Cow<'static, str>)>,
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;
pub type PoolResult<T> = Result<T, DatabaseError>;
pub type QueryResult<T> = Result<T, DatabaseError>;
pub type MigrationResult<T> = Result<T, DatabaseError>;

impl DatabaseError {
    pub fn connection_failed(database: DatabaseType, message: impl Into<Cow<'static, str>>) -> Self {
        DatabaseError::Connection {
            message: message.into(),
            database,
            context: ErrorContext::new("connection_failed").with_severity(ErrorSeverity::Error),
        }
    }

    pub fn pool_exhausted(database: DatabaseType, message: impl Into<Cow<'static, str>>) -> Self {
        DatabaseError::Pool {
            message: message.into(),
            database,
            pool_state: PoolState::Exhausted,
            context: ErrorContext::new("pool_exhausted").with_severity(ErrorSeverity::Error),
        }
    }

    pub fn query_failed(
        database: DatabaseType,
        query_type: QueryType,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        DatabaseError::Query {
            message: message.into(),
            database,
            query_type,
            context: ErrorContext::new("query_failed").with_severity(ErrorSeverity::Error),
        }
    }

    pub fn timeout(
        database: DatabaseType,
        operation: impl Into<String>,
        duration: std::time::Duration,
    ) -> Self {
        DatabaseError::Timeout {
            message: format!("Operation timed out after {:?}", duration).into(),
            database,
            operation: operation.into(),
            timeout_duration: duration,
            context: ErrorContext::new("timeout").with_severity(ErrorSeverity::Warning),
        }
    }

    pub fn with_context(mut self, key: impl Into<Cow<'static, str>>, value: impl Into<Cow<'static, str>>) -> Self {
        match &mut self {
            DatabaseError::Connection { context, .. }
            | DatabaseError::Pool { context, .. }
            | DatabaseError::Query { context, .. }
            | DatabaseError::Migration { context, .. }
            | DatabaseError::Configuration { context, .. }
            | DatabaseError::Timeout { context, .. }
            | DatabaseError::Serialization { context, .. }
            | DatabaseError::HealthCheck { context, .. } => {
                context.add_context(key, value);
            }
        }
        self
    }

    pub fn severity(&self) -> &ErrorSeverity {
        match self {
            DatabaseError::Connection { context, .. }
            | DatabaseError::Pool { context, .. }
            | DatabaseError::Query { context, .. }
            | DatabaseError::Migration { context, .. }
            | DatabaseError::Configuration { context, .. }
            | DatabaseError::Timeout { context, .. }
            | DatabaseError::Serialization { context, .. }
            | DatabaseError::HealthCheck { context, .. } => &context.severity,
        }
    }

    pub fn should_alert(&self) -> bool {
        matches!(self.severity(), ErrorSeverity::Critical)
    }

    pub fn database_type(&self) -> &DatabaseType {
        match self {
            DatabaseError::Connection { database, .. }
            | DatabaseError::Pool { database, .. }
            | DatabaseError::Query { database, .. }
            | DatabaseError::Migration { database, .. }
            | DatabaseError::Configuration { database, .. }
            | DatabaseError::Timeout { database, .. }
            | DatabaseError::Serialization { database, .. }
            | DatabaseError::HealthCheck { database, .. } => database,
        }
    }
}

impl ErrorContext {
    pub fn new(operation: impl Into<Cow<'static, str>>) -> Self {
        Self {
            timestamp_cell: OnceCell::new(),
            operation: operation.into(),
            component: Cow::Borrowed("database"), 
            correlation_id: None,
            retry_count: 0,
            severity: ErrorSeverity::default(),
            additional_info: Vec::new(),
        }
    }

    pub fn with_component(mut self, component: impl Into<Cow<'static, str>>) -> Self {
        self.component = component.into();
        self
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn increase_retry(mut self) -> Self {
        self.retry_count += 1;
        self
    }

    pub fn timestamp(&self) -> SystemTime {
        *self.timestamp_cell.get_or_init(|| SystemTime::now())
    }

    pub fn add_context(&mut self, key: impl Into<Cow<'static, str>>, value: impl Into<Cow<'static, str>>) {
        self.additional_info.push((key.into(), value.into()));
    }

}

impl From<crate::config::ConfigError> for DatabaseError {
    fn from(value: crate::config::ConfigError) -> Self {
        DatabaseError::Configuration {
            message: value.to_string().into(),
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
                db_err.message().to_string(),
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

#[cfg(test)]
mod tests {
        use super::*;

        #[test]
        fn test_error_construction_helpers() {
            let conn_err = DatabaseError::connection_failed(DatabaseType::SQLite, "Connection refused");

            match conn_err {
                DatabaseError::Connection { message, database, ..} => {
                    assert_eq!(message, "Connection refused");
                    assert!(matches!(database, DatabaseType::SQLite))
            }
            _ => panic!("Expected Connection error"),
        }

        let pool_err = DatabaseError::pool_exhausted(DatabaseType::Redis, "Pool full");
        match pool_err {
            DatabaseError::Pool { pool_state, ..} => {
                assert!(matches!(pool_state, PoolState::Exhausted))
            }
            _ => panic!("Expected Pool error"),
        }
    }

    #[test]
    fn test_error_context_building() {
        let context = ErrorContext::new("test_operation")
            .with_component("test_component")
            .with_correlation_id("test-123")
            .increase_retry();

        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.component, "test_component");
        assert_eq!(context.correlation_id, Some("test-123".to_string()));
        assert_eq!(context.retry_count, 1);
    }

    #[test]
    fn test_with_context_chaining() {
        let error = DatabaseError::connection_failed(DatabaseType::SQLite, "Test error")
            .with_context("host", "localhost")
            .with_context("port", "5432");

        match error {
            DatabaseError::Connection { context, ..} => {
                assert!(context.additional_info.iter().any(|(k, v)| k == "host" && v == "localhost"));
                assert!(context.additional_info.iter().any(|(k, v)| k == "port" && v == "5432"));
            }
            _ => panic!("Expected Connection error"),
        }
    }

    #[test]
    fn test_sqlx_error_cconversion() {
        let sqlx_err = sqlx::Error::PoolTimedOut;
        let db_err: DatabaseError = sqlx_err.into();

        match db_err {
            DatabaseError::Pool { pool_state, database, ..} => {
                assert!(matches!(pool_state, PoolState::Exhausted));
                assert!(matches!(database, DatabaseType::SQLite));
            }
            _ => panic!("Expected Pool error"),
        }
    }

    #[test]
    fn test_error_serialization() {
        let error = DatabaseError::timeout(
            DatabaseType::Redis, 
            "Query_execution", std::time::Duration::from_secs(30)
        );

        let serialized = serde_json::to_string(&error).expect("Should serialize");
        let deserialized: DatabaseError = serde_json::from_str(&serialized).expect("Should deserialize");

        match deserialized {
            DatabaseError::Timeout { operation, timeout_duration, .. } => {
                assert_eq!(operation, "Query_execution");
                assert_eq!(timeout_duration, std::time::Duration::from_secs(30));
            }
            _ => panic!("Expected Timeout error"),
        }
    }

    #[test]
    fn test_database_type_display() {
        assert_eq!(DatabaseType::SQLite.to_string(), "SQLite");
        assert_eq!(DatabaseType::Redis.to_string(), "Redis");
        assert_eq!(DatabaseType::InfluxDB.to_string(), "InfluxDB");
        assert_eq!(DatabaseType::ChromaDB.to_string(), "ChromaDB");
    }
}
