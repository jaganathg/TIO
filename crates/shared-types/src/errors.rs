use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Core Error System
// ============================================================================

/// Comprehensive error type that encapsulates all possible errors in the system
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradingError {
    /// Unique error ID for tracking and correlation
    pub error_id: Uuid,

    /// Error code for programmatic handling
    pub error_code: ErrorCode,

    /// The specific error category and details
    pub error_type: ErrorType,

    /// User-friendly error message
    pub user_message: String,

    /// Developer-oriented error message with technical details
    pub developer_message: String,

    /// Additional context and metadata
    pub context: ErrorContext,

    /// Chain of underlying errors that led to this error
    pub error_chain: Vec<ChainedError>,

    /// When this error occurred
    pub timestamp: DateTime<Utc>,

    /// Severity level of the error
    pub severity: ErrorSeverity,

    /// Whether this error is recoverable
    pub recoverable: bool,

    /// Suggested retry strategy
    pub retry_strategy: Option<RetryStrategy>,
}

/// Standardized error codes for consistent handling across services
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCode {
    // Market Data Errors (1000-1999)
    #[serde(rename = "MD_001")]
    SymbolNotFound,
    #[serde(rename = "MD_002")]
    NoDataAvailable,
    #[serde(rename = "MD_003")]
    InvalidTimeRange,
    #[serde(rename = "MD_004")]
    DataProviderUnavailable,
    #[serde(rename = "MD_005")]
    InvalidSymbolFormat,
    #[serde(rename = "MD_006")]
    MarketClosed,
    #[serde(rename = "MD_007")]
    DataStale,
    #[serde(rename = "MD_008")]
    RateLimitExceeded,

    // Trading Errors (2000-2999)
    #[serde(rename = "TR_001")]
    InsufficientFunds,
    #[serde(rename = "TR_002")]
    InvalidOrderSize,
    #[serde(rename = "TR_003")]
    InvalidOrderType,
    #[serde(rename = "TR_004")]
    OrderRejected,
    #[serde(rename = "TR_005")]
    PositionNotFound,
    #[serde(rename = "TR_006")]
    PortfolioNotFound,
    #[serde(rename = "TR_007")]
    RiskLimitExceeded,
    #[serde(rename = "TR_008")]
    TradingHalted,

    // Analysis Errors (3000-3999)
    #[serde(rename = "AN_001")]
    InsufficientDataForAnalysis,
    #[serde(rename = "AN_002")]
    IndicatorCalculationFailed,
    #[serde(rename = "AN_003")]
    PatternRecognitionFailed,
    #[serde(rename = "AN_004")]
    AIServiceUnavailable,
    #[serde(rename = "AN_005")]
    InvalidAnalysisParameters,
    #[serde(rename = "AN_006")]
    ModelLoadingFailed,
    #[serde(rename = "AN_007")]
    AnalysisTimeout,

    // Database Errors (4000-4999)
    #[serde(rename = "DB_001")]
    ConnectionFailed,
    #[serde(rename = "DB_002")]
    QueryFailed,
    #[serde(rename = "DB_003")]
    TransactionFailed,
    #[serde(rename = "DB_004")]
    ConstraintViolation,
    #[serde(rename = "DB_005")]
    MigrationFailed,
    #[serde(rename = "DB_006")]
    DatabaseUnavailable,
    #[serde(rename = "DB_007")]
    DataCorruption,

    // Network Errors (5000-5999)
    #[serde(rename = "NW_001")]
    ConnectionTimeout,
    #[serde(rename = "NW_002")]
    DNSResolutionFailed,
    #[serde(rename = "NW_003")]
    TLSHandshakeFailed,
    #[serde(rename = "NW_004")]
    HTTPClientError,
    #[serde(rename = "NW_005")]
    HTTPServerError,
    #[serde(rename = "NW_006")]
    WebSocketConnectionFailed,
    #[serde(rename = "NW_007")]
    NetworkUnreachable,

    // Authentication/Authorization Errors (6000-6999)
    #[serde(rename = "AU_001")]
    InvalidCredentials,
    #[serde(rename = "AU_002")]
    TokenExpired,
    #[serde(rename = "AU_003")]
    TokenInvalid,
    #[serde(rename = "AU_004")]
    InsufficientPermissions,
    #[serde(rename = "AU_005")]
    AccountLocked,
    #[serde(rename = "AU_006")]
    SessionExpired,
    #[serde(rename = "AU_007")]
    TwoFactorRequired,

    // Validation Errors (7000-7999)
    #[serde(rename = "VL_001")]
    RequiredFieldMissing,
    #[serde(rename = "VL_002")]
    InvalidFieldValue,
    #[serde(rename = "VL_003")]
    FieldTooLong,
    #[serde(rename = "VL_004")]
    FieldTooShort,
    #[serde(rename = "VL_005")]
    InvalidFormat,
    #[serde(rename = "VL_006")]
    ValueOutOfRange,
    #[serde(rename = "VL_007")]
    InvalidEnumValue,

    // System Errors (8000-8999)
    #[serde(rename = "SY_001")]
    ConfigurationError,
    #[serde(rename = "SY_002")]
    ResourceExhausted,
    #[serde(rename = "SY_003")]
    ServiceUnavailable,
    #[serde(rename = "SY_004")]
    InternalError,
    #[serde(rename = "SY_005")]
    FeatureNotImplemented,
    #[serde(rename = "SY_006")]
    MaintenanceMode,
    #[serde(rename = "SY_007")]
    VersionMismatch,

    // External Service Errors (9000-9999)
    #[serde(rename = "EX_001")]
    ThirdPartyServiceDown,
    #[serde(rename = "EX_002")]
    APIKeyInvalid,
    #[serde(rename = "EX_003")]
    QuotaExceeded,
    #[serde(rename = "EX_004")]
    ServiceDegraded,
    #[serde(rename = "EX_005")]
    UnexpectedResponse,
}

/// Specific error type with detailed information
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorType {
    #[error("Market data error: {details}")]
    MarketData {
        details: MarketDataError,
        symbol: Option<String>,
        timeframe: Option<String>,
    },

    #[error("Trading error: {details}")]
    Trading {
        details: Box<TradingErrorDetails>,
        order_id: Option<String>,
        portfolio_id: Option<String>,
    },

    #[error("Analysis error: {details}")]
    Analysis {
        details: AnalysisError,
        analysis_type: Option<String>,
        parameters: HashMap<String, String>,
    },

    #[error("Database error: {details}")]
    Database {
        details: DatabaseError,
        operation: Option<String>,
        table: Option<String>,
    },

    #[error("Network error: {details}")]
    Network {
        details: NetworkError,
        url: Option<String>,
        status_code: Option<u16>,
    },

    #[error("Authentication error: {details}")]
    Authentication {
        details: AuthenticationError,
        user_id: Option<String>,
        resource: Option<String>,
    },

    #[error("Validation error: {details}")]
    Validation {
        details: ValidationError,
        field: Option<String>,
        value: Option<String>,
    },

    #[error("System error: {details}")]
    System {
        details: SystemError,
        component: Option<String>,
        configuration: Option<String>,
    },

    #[error("External service error: {details}")]
    ExternalService {
        details: ExternalServiceError,
        service_name: String,
        endpoint: Option<String>,
    },
}

// ============================================================================
// Domain-Specific Error Types
// ============================================================================

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarketDataError {
    #[error("Symbol '{symbol}' not found")]
    SymbolNotFound { symbol: String },

    #[error("No data available for the requested time range")]
    NoDataAvailable,

    #[error("Invalid time range: start {start} is after end {end}")]
    InvalidTimeRange { start: String, end: String },

    #[error("Market data provider '{provider}' is unavailable")]
    DataProviderUnavailable { provider: String },

    #[error("Invalid symbol format: '{symbol}'")]
    InvalidSymbolFormat { symbol: String },

    #[error("Market is closed for symbol '{symbol}'")]
    MarketClosed { symbol: String },

    #[error("Data is stale, last updated: {last_updated}")]
    DataStale { last_updated: String },

    #[error("Rate limit exceeded for provider '{provider}', retry after: {retry_after}")]
    RateLimitExceeded {
        provider: String,
        retry_after: String,
    },
}

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TradingErrorDetails {
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: String, available: String },

    #[error("Invalid order size: {size}, minimum: {min}, maximum: {max}")]
    InvalidOrderSize {
        size: String,
        min: String,
        max: String,
    },

    #[error("Invalid order type: '{order_type}' not supported for symbol '{symbol}'")]
    InvalidOrderType { order_type: String, symbol: String },

    #[error("Order rejected: {reason}")]
    OrderRejected { reason: String },

    #[error("Position '{position_id}' not found")]
    PositionNotFound { position_id: String },

    #[error("Portfolio '{portfolio_id}' not found")]
    PortfolioNotFound { portfolio_id: String },

    #[error("Risk limit exceeded: {risk_type}, current: {current}, limit: {limit}")]
    RiskLimitExceeded {
        risk_type: String,
        current: String,
        limit: String,
    },

    #[error("Trading halted for symbol '{symbol}': {reason}")]
    TradingHalted { symbol: String, reason: String },
}

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalysisError {
    #[error("Insufficient data for analysis: need {required}, have {available}")]
    InsufficientDataForAnalysis { required: u32, available: u32 },

    #[error("Indicator calculation failed: {indicator}, reason: {reason}")]
    IndicatorCalculationFailed { indicator: String, reason: String },

    #[error("Pattern recognition failed: {pattern_type}, reason: {reason}")]
    PatternRecognitionFailed {
        pattern_type: String,
        reason: String,
    },

    #[error("AI service '{service}' is unavailable")]
    AIServiceUnavailable { service: String },

    #[error("Invalid analysis parameters: {parameter} = {value}")]
    InvalidAnalysisParameters { parameter: String, value: String },

    #[error("Model loading failed: {model_name}, error: {error}")]
    ModelLoadingFailed { model_name: String, error: String },

    #[error("Analysis timeout after {timeout_seconds} seconds")]
    AnalysisTimeout { timeout_seconds: u64 },
}

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DatabaseError {
    #[error("Database connection failed: {database}, error: {error}")]
    ConnectionFailed { database: String, error: String },

    #[error("Query failed: {query}, error: {error}")]
    QueryFailed { query: String, error: String },

    #[error("Transaction failed: {operation}, error: {error}")]
    TransactionFailed { operation: String, error: String },

    #[error("Constraint violation: {constraint}, value: {value}")]
    ConstraintViolation { constraint: String, value: String },

    #[error("Migration failed: {migration}, error: {error}")]
    MigrationFailed { migration: String, error: String },

    #[error("Database '{database}' is unavailable")]
    DatabaseUnavailable { database: String },

    #[error("Data corruption detected in table '{table}', row: {row_id}")]
    DataCorruption { table: String, row_id: String },
}

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetworkError {
    #[error("Connection timeout after {timeout_seconds} seconds")]
    ConnectionTimeout { timeout_seconds: u64 },

    #[error("DNS resolution failed for host '{host}'")]
    DNSResolutionFailed { host: String },

    #[error("TLS handshake failed with '{host}': {error}")]
    TLSHandshakeFailed { host: String, error: String },

    #[error("HTTP client error {status_code}: {message}")]
    HTTPClientError { status_code: u16, message: String },

    #[error("HTTP server error {status_code}: {message}")]
    HTTPServerError { status_code: u16, message: String },

    #[error("WebSocket connection failed: {reason}")]
    WebSocketConnectionFailed { reason: String },

    #[error("Network unreachable: {destination}")]
    NetworkUnreachable { destination: String },
}

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthenticationError {
    #[error("Invalid credentials for user '{user_id}'")]
    InvalidCredentials { user_id: String },

    #[error("Token expired at {expiry_time}")]
    TokenExpired { expiry_time: String },

    #[error("Token is invalid: {reason}")]
    TokenInvalid { reason: String },

    #[error("Insufficient permissions for action '{action}' on resource '{resource}'")]
    InsufficientPermissions { action: String, resource: String },

    #[error("Account '{account_id}' is locked: {reason}")]
    AccountLocked { account_id: String, reason: String },

    #[error("Session expired at {expiry_time}")]
    SessionExpired { expiry_time: String },

    #[error("Two-factor authentication required")]
    TwoFactorRequired,
}

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationError {
    #[error("Required field '{field}' is missing")]
    RequiredFieldMissing { field: String },

    #[error("Invalid value for field '{field}': {value}")]
    InvalidFieldValue { field: String, value: String },

    #[error("Field '{field}' is too long: {length}, max: {max_length}")]
    FieldTooLong {
        field: String,
        length: usize,
        max_length: usize,
    },

    #[error("Field '{field}' is too short: {length}, min: {min_length}")]
    FieldTooShort {
        field: String,
        length: usize,
        min_length: usize,
    },

    #[error("Invalid format for field '{field}': expected {expected_format}")]
    InvalidFormat {
        field: String,
        expected_format: String,
    },

    #[error("Value {value} is out of range for field '{field}': min {min}, max {max}")]
    ValueOutOfRange {
        field: String,
        value: String,
        min: String,
        max: String,
    },

    #[error("Invalid enum value '{value}' for field '{field}', valid values: {valid_values:?}")]
    InvalidEnumValue {
        field: String,
        value: String,
        valid_values: Vec<String>,
    },
}

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SystemError {
    #[error("Configuration error: {config_key} = {config_value}")]
    ConfigurationError {
        config_key: String,
        config_value: String,
    },

    #[error("Resource exhausted: {resource_type}, used: {used}, limit: {limit}")]
    ResourceExhausted {
        resource_type: String,
        used: String,
        limit: String,
    },

    #[error("Service '{service}' is unavailable")]
    ServiceUnavailable { service: String },

    #[error("Internal error: {component}, error: {error}")]
    InternalError { component: String, error: String },

    #[error("Feature '{feature}' is not implemented")]
    FeatureNotImplemented { feature: String },

    #[error("System is in maintenance mode until {end_time}")]
    MaintenanceMode { end_time: String },

    #[error("Version mismatch: client {client_version}, server {server_version}")]
    VersionMismatch {
        client_version: String,
        server_version: String,
    },
}

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExternalServiceError {
    #[error("Third-party service '{service}' is down")]
    ThirdPartyServiceDown { service: String },

    #[error("API key is invalid for service '{service}'")]
    APIKeyInvalid { service: String },

    #[error("Quota exceeded for service '{service}': {usage}/{quota}")]
    QuotaExceeded {
        service: String,
        usage: String,
        quota: String,
    },

    #[error("Service '{service}' is degraded: {performance_impact}")]
    ServiceDegraded {
        service: String,
        performance_impact: String,
    },

    #[error("Unexpected response from service '{service}': {response}")]
    UnexpectedResponse { service: String, response: String },
}

// ============================================================================
// Supporting Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Correlation ID for tracing across services
    pub correlation_id: Option<Uuid>,

    /// User ID associated with the error
    pub user_id: Option<String>,

    /// Request ID that caused the error
    pub request_id: Option<Uuid>,

    /// Service name where the error occurred
    pub service_name: String,

    /// Component within the service
    pub component: Option<String>,

    /// Function or method where error occurred
    pub function: Option<String>,

    /// File and line number (for debugging)
    pub location: Option<String>,

    /// Environment where error occurred
    pub environment: String,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainedError {
    /// Error message
    pub message: String,

    /// Error type/category
    pub error_type: String,

    /// When this error occurred in the chain
    pub timestamp: DateTime<Utc>,

    /// Source location of this error
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    #[serde(rename = "trace")]
    Trace,
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "critical")]
    Critical,
    #[serde(rename = "fatal")]
    Fatal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryStrategy {
    /// Whether retry is recommended
    pub should_retry: bool,

    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Delay between retries (in seconds)
    pub delay_seconds: u64,

    /// Backoff strategy
    pub backoff_strategy: BackoffStrategy,

    /// Conditions under which to retry
    pub retry_conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackoffStrategy {
    #[serde(rename = "fixed")]
    Fixed,
    #[serde(rename = "exponential")]
    Exponential,
    #[serde(rename = "linear")]
    Linear,
    #[serde(rename = "jittered")]
    Jittered,
}

// ============================================================================
// Error Builder and Helper Methods
// ============================================================================

impl TradingError {
    /// Create a new error with minimal information
    pub fn new(error_code: ErrorCode, error_type: ErrorType) -> Self {
        Self {
            error_id: Uuid::new_v4(),
            error_code,
            user_message: error_type.to_string(),
            developer_message: error_type.to_string(),
            error_type,
            context: ErrorContext::default(),
            error_chain: Vec::new(),
            timestamp: Utc::now(),
            severity: ErrorSeverity::Error,
            recoverable: false,
            retry_strategy: None,
        }
    }

    /// Builder pattern for creating errors
    pub fn builder(error_code: ErrorCode, error_type: ErrorType) -> TradingErrorBuilder {
        TradingErrorBuilder::new(error_code, error_type)
    }

    /// Add an error to the chain
    pub fn chain_error(mut self, message: String, error_type: String) -> Self {
        self.error_chain.push(ChainedError {
            message,
            error_type,
            timestamp: Utc::now(),
            source: None,
        });
        self
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        self.recoverable
    }

    /// Check if retry is recommended
    pub fn should_retry(&self) -> bool {
        self.retry_strategy
            .as_ref()
            .map_or(false, |s| s.should_retry)
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> &str {
        &self.user_message
    }

    /// Get technical error message for developers
    pub fn developer_message(&self) -> &str {
        &self.developer_message
    }
}

impl fmt::Display for TradingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message)
    }
}

pub struct TradingErrorBuilder {
    error: TradingError,
}

impl TradingErrorBuilder {
    pub fn new(error_code: ErrorCode, error_type: ErrorType) -> Self {
        Self {
            error: TradingError::new(error_code, error_type),
        }
    }

    pub fn user_message(mut self, message: impl Into<String>) -> Self {
        self.error.user_message = message.into();
        self
    }

    pub fn developer_message(mut self, message: impl Into<String>) -> Self {
        self.error.developer_message = message.into();
        self
    }

    pub fn severity(mut self, severity: ErrorSeverity) -> Self {
        self.error.severity = severity;
        self
    }

    pub fn recoverable(mut self, recoverable: bool) -> Self {
        self.error.recoverable = recoverable;
        self
    }

    pub fn correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.error.context.correlation_id = Some(correlation_id);
        self
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.error.context.user_id = Some(user_id.into());
        self
    }

    pub fn service_name(mut self, service_name: impl Into<String>) -> Self {
        self.error.context.service_name = service_name.into();
        self
    }

    pub fn component(mut self, component: impl Into<String>) -> Self {
        self.error.context.component = Some(component.into());
        self
    }

    pub fn retry_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.error.retry_strategy = Some(strategy);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.error.context.metadata.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> TradingError {
        self.error
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            correlation_id: None,
            user_id: None,
            request_id: None,
            service_name: "unknown".to_string(),
            component: None,
            function: None,
            location: None,
            environment: "development".to_string(),
            metadata: HashMap::new(),
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code_str = match serde_json::to_string(self) {
            Ok(s) => s.trim_matches('"').to_string(),
            Err(_) => format!("{:?}", self),
        };
        write!(f, "{}", code_str)
    }
}

// ============================================================================
// Convenience Macros for Error Creation
// ============================================================================

#[macro_export]
macro_rules! market_data_error {
    ($variant:ident, $($field:ident : $value:expr),*) => {
        TradingError::new(
            ErrorCode::SymbolNotFound, // This should map to appropriate code
            ErrorType::MarketData {
                details: MarketDataError::$variant { $($field: $value),* },
                symbol: None,
                timeframe: None,
            }
        )
    };
}

#[macro_export]
macro_rules! trading_error {
    ($variant:ident, $($field:ident : $value:expr),*) => {
        TradingError::new(
            ErrorCode::InsufficientFunds, // This should map to appropriate code
            ErrorType::Trading {
                details: TradingError::$variant { $($field: $value),* },
                order_id: None,
                portfolio_id: None,
            }
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = TradingError::new(
            ErrorCode::SymbolNotFound,
            ErrorType::MarketData {
                details: MarketDataError::SymbolNotFound {
                    symbol: "INVALID".to_string(),
                },
                symbol: Some("INVALID".to_string()),
                timeframe: None,
            },
        );

        assert_eq!(error.error_code, ErrorCode::SymbolNotFound);
        assert!(!error.is_recoverable());
        assert!(!error.should_retry());
    }

    #[test]
    fn test_error_builder() {
        let correlation_id = Uuid::new_v4();
        let error = TradingError::builder(
            ErrorCode::DatabaseUnavailable,
            ErrorType::Database {
                details: DatabaseError::ConnectionFailed {
                    database: "trading_db".to_string(),
                    error: "timeout".to_string(),
                },
                operation: Some("select".to_string()),
                table: Some("positions".to_string()),
            },
        )
        .user_message("Database is temporarily unavailable")
        .developer_message("Connection timeout to trading_db after 30s")
        .severity(ErrorSeverity::Critical)
        .recoverable(true)
        .correlation_id(correlation_id)
        .service_name("api-gateway")
        .component("database-pool")
        .metadata("timeout_seconds", "30")
        .build();

        assert_eq!(error.severity, ErrorSeverity::Critical);
        assert!(error.is_recoverable());
        assert_eq!(error.context.correlation_id, Some(correlation_id));
        assert_eq!(error.context.service_name, "api-gateway");
    }

    #[test]
    fn test_error_chain() {
        let mut error = TradingError::new(
            ErrorCode::InternalError,
            ErrorType::System {
                details: SystemError::InternalError {
                    component: "order-processor".to_string(),
                    error: "unexpected panic".to_string(),
                },
                component: Some("order-processor".to_string()),
                configuration: None,
            },
        );

        error = error.chain_error(
            "Database connection lost".to_string(),
            "DatabaseError".to_string(),
        );

        error = error.chain_error("Network timeout".to_string(), "NetworkError".to_string());

        assert_eq!(error.error_chain.len(), 2);
        assert_eq!(error.error_chain[0].message, "Database connection lost");
        assert_eq!(error.error_chain[1].message, "Network timeout");
    }

    #[test]
    fn test_retry_strategy() {
        let retry_strategy = RetryStrategy {
            should_retry: true,
            max_attempts: 3,
            delay_seconds: 5,
            backoff_strategy: BackoffStrategy::Exponential,
            retry_conditions: vec!["timeout".to_string(), "rate_limit".to_string()],
        };

        let error = TradingError::builder(
            ErrorCode::RateLimitExceeded,
            ErrorType::MarketData {
                details: MarketDataError::RateLimitExceeded {
                    provider: "alpha_vantage".to_string(),
                    retry_after: "60".to_string(),
                },
                symbol: Some("AAPL".to_string()),
                timeframe: Some("1m".to_string()),
            },
        )
        .retry_strategy(retry_strategy)
        .build();

        assert!(error.should_retry());
        assert_eq!(error.retry_strategy.as_ref().unwrap().max_attempts, 3);
        assert_eq!(
            error.retry_strategy.as_ref().unwrap().backoff_strategy,
            BackoffStrategy::Exponential
        );
    }

    #[test]
    fn test_error_severity_levels() {
        let severities = vec![
            ErrorSeverity::Trace,
            ErrorSeverity::Debug,
            ErrorSeverity::Info,
            ErrorSeverity::Warning,
            ErrorSeverity::Error,
            ErrorSeverity::Critical,
            ErrorSeverity::Fatal,
        ];

        for severity in severities {
            let error = TradingError::builder(
                ErrorCode::InternalError,
                ErrorType::System {
                    details: SystemError::InternalError {
                        component: "test".to_string(),
                        error: "test error".to_string(),
                    },
                    component: None,
                    configuration: None,
                },
            )
            .severity(severity.clone())
            .build();

            assert_eq!(error.severity, severity);
        }
    }

    #[test]
    fn test_error_code_display() {
        let error_code = ErrorCode::SymbolNotFound;
        assert_eq!(error_code.to_string(), "MD_001");

        let error_code = ErrorCode::InsufficientFunds;
        assert_eq!(error_code.to_string(), "TR_001");

        let error_code = ErrorCode::DatabaseUnavailable;
        assert_eq!(error_code.to_string(), "DB_006");
    }

    #[test]
    fn test_error_serialization() {
        let error = TradingError::builder(
            ErrorCode::InvalidCredentials,
            ErrorType::Authentication {
                details: AuthenticationError::InvalidCredentials {
                    user_id: "user123".to_string(),
                },
                user_id: Some("user123".to_string()),
                resource: Some("portfolio".to_string()),
            },
        )
        .user_message("Invalid login credentials")
        .developer_message("JWT token verification failed")
        .severity(ErrorSeverity::Warning)
        .build();

        let json = serde_json::to_string(&error).unwrap();
        let deserialized: TradingError = serde_json::from_str(&json).unwrap();

        assert_eq!(error.error_code, deserialized.error_code);
        assert_eq!(error.user_message, deserialized.user_message);
        assert_eq!(error.severity, deserialized.severity);
    }

    #[test]
    fn test_all_error_categories() {
        // Test each error category can be created
        let _market_data_error = ErrorType::MarketData {
            details: MarketDataError::SymbolNotFound {
                symbol: "TEST".to_string(),
            },
            symbol: Some("TEST".to_string()),
            timeframe: None,
        };

        let _trading_error = ErrorType::Trading {
            details: Box::new(TradingErrorDetails::InsufficientFunds {
                required: "1000".to_string(),
                available: "500".to_string(),
            }),
            order_id: Some("order123".to_string()),
            portfolio_id: None,
        };

        let _analysis_error = ErrorType::Analysis {
            details: AnalysisError::InsufficientDataForAnalysis {
                required: 100,
                available: 50,
            },
            analysis_type: Some("RSI".to_string()),
            parameters: HashMap::new(),
        };

        let _database_error = ErrorType::Database {
            details: DatabaseError::ConnectionFailed {
                database: "main".to_string(),
                error: "timeout".to_string(),
            },
            operation: Some("SELECT".to_string()),
            table: Some("users".to_string()),
        };

        let _network_error = ErrorType::Network {
            details: NetworkError::ConnectionTimeout {
                timeout_seconds: 30,
            },
            url: Some("https://api.example.com".to_string()),
            status_code: Some(408),
        };

        let _auth_error = ErrorType::Authentication {
            details: AuthenticationError::TokenExpired {
                expiry_time: "2024-01-01T00:00:00Z".to_string(),
            },
            user_id: Some("user123".to_string()),
            resource: Some("portfolio".to_string()),
        };

        let _validation_error = ErrorType::Validation {
            details: ValidationError::RequiredFieldMissing {
                field: "symbol".to_string(),
            },
            field: Some("symbol".to_string()),
            value: None,
        };

        let _system_error = ErrorType::System {
            details: SystemError::ServiceUnavailable {
                service: "analysis-engine".to_string(),
            },
            component: Some("analysis-engine".to_string()),
            configuration: None,
        };

        let _external_error = ErrorType::ExternalService {
            details: ExternalServiceError::ThirdPartyServiceDown {
                service: "alpha_vantage".to_string(),
            },
            service_name: "alpha_vantage".to_string(),
            endpoint: Some("/query".to_string()),
        };

        // If we reach here, all error types can be created successfully
        assert!(true);
    }

    #[test]
    fn test_error_context_metadata() {
        let mut context = ErrorContext::default();
        context
            .metadata
            .insert("key1".to_string(), "value1".to_string());
        context
            .metadata
            .insert("key2".to_string(), "value2".to_string());

        assert_eq!(context.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(context.metadata.get("key2"), Some(&"value2".to_string()));
        assert_eq!(context.metadata.get("nonexistent"), None);
    }
}
