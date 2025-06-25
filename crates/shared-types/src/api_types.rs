use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

use crate::{Symbol, TimeFrame, OHLCV};

// ============================================================================
// Generic API Response Structure
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Unique request ID for tracking
    pub request_id: Uuid,

    /// Response status
    pub status: ApiStatus,

    /// Response data (None if error)
    pub data: Option<T>,

    /// Error details (None if success)
    pub error: Option<ApiError>,

    /// Response metadata
    pub metadata: ResponseMetadata,

    /// Server timestamp when response was generated
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApiStatus {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "partial")]
    Partial,
    #[serde(rename = "pending")]
    Pending,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Processing time in milliseconds
    pub processing_time_ms: u64,

    /// API version that handled the request
    pub api_version: String,

    /// Source service that generated the response
    pub source: String,

    /// Pagination info (if applicable)
    pub pagination: Option<PaginationInfo>,

    /// Rate limiting info
    pub rate_limit: Option<RateLimitInfo>,

    /// Additional metadata
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
    pub total_items: u64,
    pub has_next: bool,
    pub has_previous: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimitInfo {
    pub requests_remaining: u32,
    pub reset_time: DateTime<Utc>,
    pub window_size_seconds: u32,
}

// ============================================================================
// API Error Types
// ============================================================================

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApiError {
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
    },

    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    #[error("Authorization failed: {message}")]
    Authorization { message: String },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Option<DateTime<Utc>>,
    },

    #[error("External service error: {service} - {message}")]
    ExternalService {
        service: String,
        message: String,
        status_code: Option<u16>,
    },

    #[error("Database error: {message}")]
    Database { message: String },

    #[error("Internal server error: {message}")]
    Internal { message: String, error_id: Uuid },

    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    #[error("Bad request: {message}")]
    BadRequest { message: String },
}

// ============================================================================
// Market Data Request/Response Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketDataRequest {
    /// Trading symbol to get data for
    pub symbol: Symbol,

    /// Timeframe for the data
    pub timeframe: TimeFrame,

    /// Start time for historical data (None for real-time)
    pub start_time: Option<DateTime<Utc>>,

    /// End time for historical data (None for latest)
    pub end_time: Option<DateTime<Utc>>,

    /// Maximum number of bars to return
    pub limit: Option<u32>,

    /// Whether to include extended hours data
    pub include_extended_hours: bool,

    /// Data adjustment type
    pub adjustment: DataAdjustment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataAdjustment {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "splits")]
    Splits,
    #[serde(rename = "dividends")]
    Dividends,
    #[serde(rename = "all")]
    All,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketDataResponse {
    /// The requested symbol
    pub symbol: Symbol,

    /// The requested timeframe
    pub timeframe: TimeFrame,

    /// OHLCV data bars
    pub bars: Vec<OHLCV>,

    /// Data adjustment applied
    pub adjustment: DataAdjustment,

    /// Whether data includes extended hours
    pub includes_extended_hours: bool,

    /// Data freshness timestamp
    pub last_updated: DateTime<Utc>,
}

// ============================================================================
// Symbol Search Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolSearchRequest {
    /// Search query (symbol code, company name, etc.)
    pub query: String,

    /// Asset class filter
    pub asset_class: Option<crate::AssetClass>,

    /// Exchange filter
    pub exchange: Option<crate::Exchange>,

    /// Maximum results to return
    pub limit: Option<u32>,

    /// Include inactive symbols
    pub include_inactive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolSearchResponse {
    /// Search query that was executed
    pub query: String,

    /// Matching symbols
    pub symbols: Vec<SymbolMatch>,

    /// Total matches found (may be more than returned)
    pub total_matches: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolMatch {
    /// The matched symbol
    pub symbol: Symbol,

    /// Match confidence score (0.0 to 1.0)
    pub match_score: f64,

    /// Which fields matched the query
    pub matched_fields: Vec<String>,
}

// ============================================================================
// Technical Analysis Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnicalAnalysisRequest {
    /// Symbol to analyze
    pub symbol: Symbol,

    /// Timeframe for analysis
    pub timeframe: TimeFrame,

    /// Technical indicators to calculate
    pub indicators: Vec<TechnicalIndicator>,

    /// Number of periods to analyze
    pub periods: Option<u32>,

    /// Custom parameters for indicators
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TechnicalIndicator {
    #[serde(rename = "rsi")]
    RSI,
    #[serde(rename = "macd")]
    MACD,
    #[serde(rename = "bollinger_bands")]
    BollingerBands,
    #[serde(rename = "moving_average")]
    MovingAverage,
    #[serde(rename = "stochastic")]
    Stochastic,
    #[serde(rename = "atr")]
    ATR,
    #[serde(rename = "fibonacci")]
    Fibonacci,
    #[serde(rename = "support_resistance")]
    SupportResistance,
    #[serde(rename = "pattern_recognition")]
    PatternRecognition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnicalAnalysisResponse {
    /// Analyzed symbol
    pub symbol: Symbol,

    /// Analysis timeframe
    pub timeframe: TimeFrame,

    /// Calculated indicator results
    pub indicators: HashMap<String, IndicatorResult>,

    /// Detected chart patterns
    pub patterns: Vec<ChartPattern>,

    /// Support and resistance levels
    pub levels: Vec<PriceLevel>,

    /// Overall technical summary
    pub summary: TechnicalSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorResult {
    /// Indicator name
    pub name: String,

    /// Current value
    pub current_value: Decimal,

    /// Previous value (for comparison)
    pub previous_value: Option<Decimal>,

    /// Signal interpretation
    pub signal: TechnicalSignal,

    /// Confidence in the signal (0.0 to 1.0)
    pub confidence: f64,

    /// Historical values
    pub values: Vec<TimestampedValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimestampedValue {
    pub timestamp: DateTime<Utc>,
    pub value: Decimal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TechnicalSignal {
    #[serde(rename = "bullish")]
    Bullish,
    #[serde(rename = "bearish")]
    Bearish,
    #[serde(rename = "neutral")]
    Neutral,
    #[serde(rename = "oversold")]
    Oversold,
    #[serde(rename = "overbought")]
    Overbought,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartPattern {
    /// Pattern type
    pub pattern_type: String,

    /// Pattern confidence (0.0 to 1.0)
    pub confidence: f64,

    /// Time range of the pattern
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,

    /// Key price levels
    pub price_levels: Vec<Decimal>,

    /// Pattern description
    pub description: String,

    /// Predicted direction
    pub prediction: TechnicalSignal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceLevel {
    /// Level type
    pub level_type: LevelType,

    /// Price level
    pub price: Decimal,

    /// Strength of the level (0.0 to 1.0)
    pub strength: f64,

    /// Number of times price touched this level
    pub touch_count: u32,

    /// Last time price touched this level
    pub last_touch: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LevelType {
    #[serde(rename = "support")]
    Support,
    #[serde(rename = "resistance")]
    Resistance,
    #[serde(rename = "pivot")]
    Pivot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TechnicalSummary {
    /// Overall technical bias
    pub overall_signal: TechnicalSignal,

    /// Confidence in overall signal (0.0 to 1.0)
    pub confidence: f64,

    /// Short-term outlook
    pub short_term: TechnicalSignal,

    /// Medium-term outlook
    pub medium_term: TechnicalSignal,

    /// Long-term outlook
    pub long_term: TechnicalSignal,

    /// Key insights
    pub insights: Vec<String>,
}

// ============================================================================
// AI Analysis Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AIAnalysisRequest {
    /// Symbol to analyze
    pub symbol: Symbol,

    /// Type of AI analysis requested
    pub analysis_type: AIAnalysisType,

    /// Context for the analysis
    pub context: AnalysisContext,

    /// Custom prompt or specific questions
    pub custom_prompt: Option<String>,

    /// Include technical data in analysis
    pub include_technical: bool,

    /// Include sentiment data in analysis
    pub include_sentiment: bool,

    /// Include news data in analysis
    pub include_news: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AIAnalysisType {
    #[serde(rename = "market_summary")]
    MarketSummary,
    #[serde(rename = "trade_idea")]
    TradeIdea,
    #[serde(rename = "risk_assessment")]
    RiskAssessment,
    #[serde(rename = "price_prediction")]
    PricePrediction,
    #[serde(rename = "correlation_analysis")]
    CorrelationAnalysis,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisContext {
    /// Timeframe for analysis
    pub timeframe: TimeFrame,

    /// Investment horizon
    pub horizon: InvestmentHorizon,

    /// Risk tolerance
    pub risk_tolerance: RiskTolerance,

    /// Additional context information
    pub additional_context: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InvestmentHorizon {
    #[serde(rename = "scalping")]
    Scalping, // Minutes to hours
    #[serde(rename = "day_trading")]
    DayTrading, // Intraday
    #[serde(rename = "swing")]
    Swing, // Days to weeks
    #[serde(rename = "position")]
    Position, // Weeks to months
    #[serde(rename = "long_term")]
    LongTerm, // Months to years
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskTolerance {
    #[serde(rename = "conservative")]
    Conservative,
    #[serde(rename = "moderate")]
    Moderate,
    #[serde(rename = "aggressive")]
    Aggressive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AIAnalysisResponse {
    /// Analyzed symbol
    pub symbol: Symbol,

    /// Type of analysis performed
    pub analysis_type: AIAnalysisType,

    /// Main AI-generated insights
    pub insights: String,

    /// Structured recommendations
    pub recommendations: Vec<Recommendation>,

    /// Risk assessment
    pub risk_assessment: RiskAssessment,

    /// Confidence in the analysis (0.0 to 1.0)
    pub confidence: f64,

    /// Data sources used
    pub data_sources: Vec<String>,

    /// Analysis timestamp
    pub analysis_time: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    /// Recommendation type
    pub recommendation_type: RecommendationType,

    /// Action to take
    pub action: String,

    /// Reasoning behind the recommendation
    pub reasoning: String,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,

    /// Time horizon for this recommendation
    pub time_horizon: InvestmentHorizon,

    /// Target price levels (if applicable)
    pub target_levels: Vec<TargetLevel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecommendationType {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
    #[serde(rename = "hold")]
    Hold,
    #[serde(rename = "avoid")]
    Avoid,
    #[serde(rename = "watch")]
    Watch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetLevel {
    /// Level type
    pub level_type: TargetLevelType,

    /// Price level
    pub price: Decimal,

    /// Probability of reaching this level
    pub probability: f64,

    /// Timeframe to reach this level
    pub timeframe: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TargetLevelType {
    #[serde(rename = "entry")]
    Entry,
    #[serde(rename = "stop_loss")]
    StopLoss,
    #[serde(rename = "take_profit")]
    TakeProfit,
    #[serde(rename = "resistance")]
    Resistance,
    #[serde(rename = "support")]
    Support,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk level
    pub risk_level: RiskLevel,

    /// Risk factors identified
    pub risk_factors: Vec<RiskFactor>,

    /// Volatility assessment
    pub volatility: VolatilityLevel,

    /// Liquidity assessment
    pub liquidity: LiquidityLevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    #[serde(rename = "very_low")]
    VeryLow,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "very_high")]
    VeryHigh,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Factor name
    pub factor: String,

    /// Impact level
    pub impact: RiskLevel,

    /// Description
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VolatilityLevel {
    #[serde(rename = "very_low")]
    VeryLow,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "extreme")]
    Extreme,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiquidityLevel {
    #[serde(rename = "very_high")]
    VeryHigh,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "very_low")]
    VeryLow,
}

// ============================================================================
// WebSocket Message Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSocketMessage {
    /// Message ID for tracking
    pub message_id: Uuid,

    /// Message type
    pub message_type: WebSocketMessageType,

    /// Message payload
    pub payload: serde_json::Value,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WebSocketMessageType {
    // Client to Server
    #[serde(rename = "subscribe")]
    Subscribe,
    #[serde(rename = "unsubscribe")]
    Unsubscribe,
    #[serde(rename = "request")]
    Request,

    // Server to Client
    #[serde(rename = "market_data")]
    MarketData,
    #[serde(rename = "analysis_update")]
    AnalysisUpdate,
    #[serde(rename = "alert")]
    Alert,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "heartbeat")]
    Heartbeat,
    #[serde(rename = "response")]
    Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    /// Subscription type
    pub subscription_type: SubscriptionType,

    /// Symbol to subscribe to
    pub symbol: Symbol,

    /// Timeframe (for market data)
    pub timeframe: Option<TimeFrame>,

    /// Additional parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SubscriptionType {
    #[serde(rename = "market_data")]
    MarketData,
    #[serde(rename = "technical_analysis")]
    TechnicalAnalysis,
    #[serde(rename = "ai_insights")]
    AIInsights,
    #[serde(rename = "alerts")]
    Alerts,
}

// ============================================================================
// Helper Implementations
// ============================================================================

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(request_id: Uuid, data: T, metadata: ResponseMetadata) -> Self {
        Self {
            request_id,
            status: ApiStatus::Success,
            data: Some(data),
            error: None,
            metadata,
            timestamp: Utc::now(),
        }
    }

    /// Create an error response
    pub fn error(request_id: Uuid, error: ApiError, metadata: ResponseMetadata) -> Self {
        Self {
            request_id,
            status: ApiStatus::Error,
            data: None,
            error: Some(error),
            metadata,
            timestamp: Utc::now(),
        }
    }

    /// Check if response is successful
    pub fn is_success(&self) -> bool {
        matches!(self.status, ApiStatus::Success)
    }

    /// Check if response is an error
    pub fn is_error(&self) -> bool {
        matches!(self.status, ApiStatus::Error)
    }
}

impl Default for ResponseMetadata {
    fn default() -> Self {
        Self {
            processing_time_ms: 0,
            api_version: "1.0.0".to_string(),
            source: "unknown".to_string(),
            pagination: None,
            rate_limit: None,
            extra: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Exchange;

    fn create_test_symbol() -> Symbol {
        Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap()
    }

    #[test]
    fn test_api_response_success() {
        let request_id = Uuid::new_v4();
        let data = "test data";
        let metadata = ResponseMetadata::default();

        let response = ApiResponse::success(request_id, data, metadata);

        assert!(response.is_success());
        assert!(!response.is_error());
        assert_eq!(response.data, Some("test data"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let request_id = Uuid::new_v4();
        let error = ApiError::NotFound {
            resource: "symbol".to_string(),
        };
        let metadata = ResponseMetadata::default();

        let response: ApiResponse<String> = ApiResponse::error(request_id, error.clone(), metadata);

        assert!(!response.is_success());
        assert!(response.is_error());
        assert!(response.data.is_none());
        assert_eq!(response.error, Some(error));
    }

    #[test]
    fn test_market_data_request() {
        let symbol = create_test_symbol();
        let request = MarketDataRequest {
            symbol,
            timeframe: TimeFrame::OneHour,
            start_time: None,
            end_time: None,
            limit: Some(100),
            include_extended_hours: false,
            adjustment: DataAdjustment::All,
        };

        assert_eq!(request.timeframe, TimeFrame::OneHour);
        assert_eq!(request.limit, Some(100));
        assert!(!request.include_extended_hours);
    }

    #[test]
    fn test_technical_analysis_request() {
        let symbol = create_test_symbol();
        let request = TechnicalAnalysisRequest {
            symbol,
            timeframe: TimeFrame::OneDay,
            indicators: vec![TechnicalIndicator::RSI, TechnicalIndicator::MACD],
            periods: Some(20),
            parameters: HashMap::new(),
        };

        assert_eq!(request.indicators.len(), 2);
        assert!(request.indicators.contains(&TechnicalIndicator::RSI));
        assert!(request.indicators.contains(&TechnicalIndicator::MACD));
    }

    #[test]
    fn test_websocket_message() {
        let message = WebSocketMessage {
            message_id: Uuid::new_v4(),
            message_type: WebSocketMessageType::MarketData,
            payload: serde_json::json!({"test": "data"}),
            timestamp: Utc::now(),
        };

        assert_eq!(message.message_type, WebSocketMessageType::MarketData);
    }

    #[test]
    fn test_serde_serialization() {
        let symbol = create_test_symbol();
        let request = MarketDataRequest {
            symbol,
            timeframe: TimeFrame::OneHour,
            start_time: None,
            end_time: None,
            limit: Some(100),
            include_extended_hours: false,
            adjustment: DataAdjustment::All,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: MarketDataRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request, deserialized);
    }
}
