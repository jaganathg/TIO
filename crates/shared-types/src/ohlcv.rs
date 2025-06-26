use chrono::{DateTime, Utc};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

use crate::validation::{validate_non_negative_volume, validate_positive_price};
use crate::{Symbol, TimeFrame};
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
pub struct OHLCV {
    #[validate(nested)]
    /// The trading symbol this candlestick represents
    pub symbol: Symbol,

    /// The timeframe of this candlestick
    pub timeframe: TimeFrame,

    /// Timestamp of when this candlestick period started (UTC)
    pub timestamp: DateTime<Utc>,

    /// Opening price at the start of the period
    #[validate(custom(function = "validate_positive_price"))]
    pub open: Decimal,

    /// Highest price during the period
    #[validate(custom(function = "validate_positive_price"))]
    pub high: Decimal,

    /// Lowest price during the period
    #[validate(custom(function = "validate_positive_price"))]
    pub low: Decimal,

    /// Closing price at the end of the period
    #[validate(custom(function = "validate_positive_price"))]
    pub close: Decimal,

    /// Total volume traded during the period
    #[validate(custom(function = "validate_non_negative_volume"))]
    pub volume: Decimal,

    /// Additional metadata for future extensibility
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Error, Debug)]
pub enum OHLCVError {
    #[error("Invalid price data: High ({high}) must be >= Low ({low})")]
    InvalidHighLow { high: Decimal, low: Decimal },

    #[error("Invalid open price: Open ({open}) must be between High ({high}) and Low ({low})")]
    InvalidOpen {
        open: Decimal,
        high: Decimal,
        low: Decimal,
    },

    #[error("Invalid close price: Close ({close}) must be between High ({high}) and Low ({low})")]
    InvalidClose {
        close: Decimal,
        high: Decimal,
        low: Decimal,
    },

    #[error("Invalid volume: Volume cannot be negative")]
    NegativeVolume,

    #[error("Invalid price: All prices must be positive")]
    NonPositivePrice,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Missing data: {field}")]
    MissingData { field: String },

    #[error("Data consistency error: {0}")]
    DataConsistency(String),
}

impl OHLCV {
    /// Create a new OHLCV with validation
    pub fn new(
        symbol: Symbol,
        timeframe: TimeFrame,
        timestamp: DateTime<Utc>,
        open: Decimal,
        high: Decimal,
        low: Decimal,
        close: Decimal,
        volume: Decimal,
    ) -> Result<Self, OHLCVError> {
        let ohlcv = OHLCV {
            symbol,
            timeframe,
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            metadata: HashMap::new(),
        };

        ohlcv.validate_prices()?;

        Ok(ohlcv)
    }

    /// Validate price relationships
    fn validate_prices(&self) -> Result<(), OHLCVError> {
        // Check all prices are positive
        if self.open <= Decimal::ZERO
            || self.high <= Decimal::ZERO
            || self.low <= Decimal::ZERO
            || self.close <= Decimal::ZERO
        {
            return Err(OHLCVError::NonPositivePrice);
        }

        // Check volume is non-negative
        if self.volume < Decimal::ZERO {
            return Err(OHLCVError::NegativeVolume);
        }

        // High must be >= Low
        if self.high < self.low {
            return Err(OHLCVError::InvalidHighLow {
                high: self.high,
                low: self.low,
            });
        }

        // Open must be between High and Low
        if self.open > self.high || self.open < self.low {
            return Err(OHLCVError::InvalidOpen {
                open: self.open,
                high: self.high,
                low: self.low,
            });
        }

        // Close must be between High and Low
        if self.close > self.high || self.close < self.low {
            return Err(OHLCVError::InvalidClose {
                close: self.close,
                high: self.high,
                low: self.low,
            });
        }

        Ok(())
    }

    /// Builder for creating OHLCV with method chaining
    pub fn builder(symbol: Symbol, timeframe: TimeFrame, timestamp: DateTime<Utc>) -> OHLCVBuilder {
        OHLCVBuilder::new(symbol, timeframe, timestamp)
    }

    /// Get the price change from open to close
    pub fn price_change(&self) -> Decimal {
        self.close - self.open
    }

    /// Get the price change percentage
    pub fn price_change_percent(&self) -> Decimal {
        if self.open == Decimal::ZERO {
            return Decimal::ZERO;
        }
        (self.price_change() / self.open) * Decimal::new(100, 0)
    }

    /// Get the price range (high - low)
    pub fn price_range(&self) -> Decimal {
        self.high - self.low
    }

    /// Check if this is a bullish candle (close > open)
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// Check if this is a bearish candle (close < open)
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// Check if this is a doji candle (close ≈ open)
    pub fn is_doji(&self, tolerance_percent: Decimal) -> bool {
        let tolerance = self.open * tolerance_percent / Decimal::new(100, 0);
        (self.close - self.open).abs() <= tolerance
    }

    /// Get the typical price (H+L+C)/3
    pub fn typical_price(&self) -> Decimal {
        (self.high + self.low + self.close) / Decimal::new(3, 0)
    }

    /// Get the weighted close price (H+L+2C)/4
    pub fn weighted_close(&self) -> Decimal {
        (self.high + self.low + (self.close * Decimal::new(2, 0))) / Decimal::new(4, 0)
    }

    /// Get the median price (H+L)/2
    pub fn median_price(&self) -> Decimal {
        (self.high + self.low) / Decimal::new(2, 0)
    }

    /// Calculate Volume Weighted Average Price (VWAP) for this candle
    /// Note: This is simplified for a single candle; true VWAP needs multiple periods
    pub fn vwap(&self) -> Decimal {
        if self.volume == Decimal::ZERO {
            return self.typical_price();
        }
        self.typical_price() // Simplified - in practice you'd need tick data
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: &str, value: serde_json::Value) {
        self.metadata.insert(key.to_string(), value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    /// Get unique identifier for this OHLCV bar
    pub fn identifier(&self) -> String {
        format!(
            "{}@{}:{}:{}",
            self.symbol.code,
            self.symbol.exchange,
            self.timeframe,
            self.timestamp.timestamp()
        )
    }

    /// Convert to array format [timestamp, open, high, low, close, volume]
    pub fn to_array(&self) -> [f64; 6] {
        [
            self.timestamp.timestamp() as f64,
            self.open.to_f64().unwrap_or(0.0),
            self.high.to_f64().unwrap_or(0.0),
            self.low.to_f64().unwrap_or(0.0),
            self.close.to_f64().unwrap_or(0.0),
            self.volume.to_f64().unwrap_or(0.0),
        ]
    }

    /// Create from array format [timestamp, open, high, low, close, volume]
    pub fn from_array(
        symbol: Symbol,
        timeframe: TimeFrame,
        data: [f64; 6],
    ) -> Result<Self, OHLCVError> {
        let timestamp =
            DateTime::from_timestamp(data[0] as i64, 0).ok_or_else(|| OHLCVError::MissingData {
                field: "timestamp".to_string(),
            })?;

        let open = Decimal::from_f64_retain(data[1]).ok_or_else(|| OHLCVError::MissingData {
            field: "open".to_string(),
        })?;
        let high = Decimal::from_f64_retain(data[2]).ok_or_else(|| OHLCVError::MissingData {
            field: "high".to_string(),
        })?;
        let low = Decimal::from_f64_retain(data[3]).ok_or_else(|| OHLCVError::MissingData {
            field: "low".to_string(),
        })?;
        let close = Decimal::from_f64_retain(data[4]).ok_or_else(|| OHLCVError::MissingData {
            field: "close".to_string(),
        })?;
        let volume = Decimal::from_f64_retain(data[5]).ok_or_else(|| OHLCVError::MissingData {
            field: "volume".to_string(),
        })?;

        Self::new(symbol, timeframe, timestamp, open, high, low, close, volume)
    }
}

/// Builder pattern for OHLCV construction
pub struct OHLCVBuilder {
    symbol: Symbol,
    timeframe: TimeFrame,
    timestamp: DateTime<Utc>,
    open: Option<Decimal>,
    high: Option<Decimal>,
    low: Option<Decimal>,
    close: Option<Decimal>,
    volume: Option<Decimal>,
    metadata: HashMap<String, serde_json::Value>,
}

impl OHLCVBuilder {
    pub fn new(symbol: Symbol, timeframe: TimeFrame, timestamp: DateTime<Utc>) -> Self {
        Self {
            symbol,
            timeframe,
            timestamp,
            open: None,
            high: None,
            low: None,
            close: None,
            volume: None,
            metadata: HashMap::new(),
        }
    }

    pub fn open(mut self, open: Decimal) -> Self {
        self.open = Some(open);
        self
    }

    pub fn high(mut self, high: Decimal) -> Self {
        self.high = Some(high);
        self
    }

    pub fn low(mut self, low: Decimal) -> Self {
        self.low = Some(low);
        self
    }

    pub fn close(mut self, close: Decimal) -> Self {
        self.close = Some(close);
        self
    }

    pub fn volume(mut self, volume: Decimal) -> Self {
        self.volume = Some(volume);
        self
    }

    pub fn metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    pub fn build(self) -> Result<OHLCV, OHLCVError> {
        let open = self.open.ok_or_else(|| OHLCVError::MissingData {
            field: "open".to_string(),
        })?;
        let high = self.high.ok_or_else(|| OHLCVError::MissingData {
            field: "high".to_string(),
        })?;
        let low = self.low.ok_or_else(|| OHLCVError::MissingData {
            field: "low".to_string(),
        })?;
        let close = self.close.ok_or_else(|| OHLCVError::MissingData {
            field: "close".to_string(),
        })?;
        let volume = self.volume.ok_or_else(|| OHLCVError::MissingData {
            field: "volume".to_string(),
        })?;

        let mut ohlcv = OHLCV::new(
            self.symbol,
            self.timeframe,
            self.timestamp,
            open,
            high,
            low,
            close,
            volume,
        )?;
        ohlcv.metadata = self.metadata;
        Ok(ohlcv)
    }
}

// Implement ordering by timestamp for time series operations
impl PartialOrd for OHLCV {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OHLCV {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl Eq for OHLCV {}

impl fmt::Display for OHLCV {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} [{}] O:{} H:{} L:{} C:{} V:{}",
            self.symbol.code,
            self.timeframe,
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.open,
            self.high,
            self.low,
            self.close,
            self.volume
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Exchange;
    use chrono::TimeZone;
    use rust_decimal::Decimal;

    fn create_test_symbol() -> Symbol {
        Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap()
    }

    fn create_test_timestamp() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 1, 9, 30, 0).unwrap()
    }

    #[test]
    fn test_create_valid_ohlcv() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let ohlcv = OHLCV::new(
            symbol,
            TimeFrame::OneHour,
            timestamp,
            Decimal::new(100, 0),  // open
            Decimal::new(105, 0),  // high
            Decimal::new(99, 0),   // low
            Decimal::new(103, 0),  // close
            Decimal::new(1000, 0), // volume
        )
        .unwrap();

        assert_eq!(ohlcv.open, Decimal::new(100, 0));
        assert_eq!(ohlcv.high, Decimal::new(105, 0));
        assert_eq!(ohlcv.low, Decimal::new(99, 0));
        assert_eq!(ohlcv.close, Decimal::new(103, 0));
        assert_eq!(ohlcv.volume, Decimal::new(1000, 0));
    }

    #[test]
    fn test_invalid_high_low() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let result = OHLCV::new(
            symbol,
            TimeFrame::OneHour,
            timestamp,
            Decimal::new(100, 0),
            Decimal::new(99, 0), // high < low - invalid!
            Decimal::new(101, 0),
            Decimal::new(100, 5),
            Decimal::new(1000, 0),
        );

        assert!(matches!(result, Err(OHLCVError::InvalidHighLow { .. })));
    }

    #[test]
    fn test_invalid_open_price() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let result = OHLCV::new(
            symbol,
            TimeFrame::OneHour,
            timestamp,
            Decimal::new(106, 0), // open > high - invalid!
            Decimal::new(105, 0),
            Decimal::new(99, 0),
            Decimal::new(103, 0),
            Decimal::new(1000, 0),
        );

        assert!(matches!(result, Err(OHLCVError::InvalidOpen { .. })));
    }

    #[test]
    fn test_negative_volume() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let result = OHLCV::new(
            symbol,
            TimeFrame::OneHour,
            timestamp,
            Decimal::new(100, 0),
            Decimal::new(105, 0),
            Decimal::new(99, 0),
            Decimal::new(103, 0),
            Decimal::new(-100, 0), // negative volume - invalid!
        );

        assert!(matches!(result, Err(OHLCVError::NegativeVolume)));
    }

    #[test]
    fn test_price_calculations() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let ohlcv = OHLCV::new(
            symbol,
            TimeFrame::OneHour,
            timestamp,
            Decimal::new(100, 0),
            Decimal::new(105, 0),
            Decimal::new(99, 0),
            Decimal::new(103, 0),
            Decimal::new(1000, 0),
        )
        .unwrap();

        assert_eq!(ohlcv.price_change(), Decimal::new(3, 0)); // 103 - 100
        assert_eq!(ohlcv.price_change_percent(), Decimal::new(3, 0)); // 3%
        assert_eq!(ohlcv.price_range(), Decimal::new(6, 0)); // 105 - 99
        assert!(ohlcv.is_bullish()); // close > open
        assert!(!ohlcv.is_bearish());
        assert!(!ohlcv.is_doji(Decimal::new(1, 0))); // change > 1%
    }

    #[test]
    fn test_technical_prices() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let ohlcv = OHLCV::new(
            symbol,
            TimeFrame::OneHour,
            timestamp,
            Decimal::new(100, 0),
            Decimal::new(105, 0),
            Decimal::new(99, 0),
            Decimal::new(102, 0),
            Decimal::new(1000, 0),
        )
        .unwrap();

        // Typical price = (H+L+C)/3 = (105+99+102)/3 = 102
        assert_eq!(ohlcv.typical_price(), Decimal::new(102, 0));

        // Median price = (H+L)/2 = (105+99)/2 = 102
        assert_eq!(ohlcv.median_price(), Decimal::new(102, 0));

        // Weighted close = (H+L+2C)/4 = (105+99+204)/4 = 102
        assert_eq!(ohlcv.weighted_close(), Decimal::new(102, 0));
    }

    #[test]
    fn test_builder_pattern() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let ohlcv = OHLCV::builder(symbol, TimeFrame::OneHour, timestamp)
            .open(Decimal::new(100, 0))
            .high(Decimal::new(105, 0))
            .low(Decimal::new(99, 0))
            .close(Decimal::new(103, 0))
            .volume(Decimal::new(1000, 0))
            .metadata("source", serde_json::Value::String("test".to_string()))
            .build()
            .unwrap();

        assert_eq!(ohlcv.open, Decimal::new(100, 0));
        assert_eq!(
            ohlcv.get_metadata("source"),
            Some(&serde_json::Value::String("test".to_string()))
        );
    }

    #[test]
    fn test_array_conversion() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let ohlcv = OHLCV::new(
            symbol.clone(),
            TimeFrame::OneHour,
            timestamp,
            Decimal::new(100, 0),
            Decimal::new(105, 0),
            Decimal::new(99, 0),
            Decimal::new(103, 0),
            Decimal::new(1000, 0),
        )
        .unwrap();

        let array = ohlcv.to_array();
        let restored = OHLCV::from_array(symbol, TimeFrame::OneHour, array).unwrap();

        assert_eq!(ohlcv.timestamp, restored.timestamp);
        assert_eq!(ohlcv.open, restored.open);
        assert_eq!(ohlcv.high, restored.high);
        assert_eq!(ohlcv.low, restored.low);
        assert_eq!(ohlcv.close, restored.close);
        assert_eq!(ohlcv.volume, restored.volume);
    }

    #[test]
    fn test_ordering_by_timestamp() {
        let symbol = create_test_symbol();
        let timestamp1 = Utc.with_ymd_and_hms(2024, 1, 1, 9, 30, 0).unwrap();
        let timestamp2 = Utc.with_ymd_and_hms(2024, 1, 1, 10, 30, 0).unwrap();

        let ohlcv1 = OHLCV::new(
            symbol.clone(),
            TimeFrame::OneHour,
            timestamp1,
            Decimal::new(100, 0),
            Decimal::new(105, 0),
            Decimal::new(99, 0),
            Decimal::new(103, 0),
            Decimal::new(1000, 0),
        )
        .unwrap();
        let ohlcv2 = OHLCV::new(
            symbol,
            TimeFrame::OneHour,
            timestamp2,
            Decimal::new(103, 0),
            Decimal::new(108, 0),
            Decimal::new(102, 0),
            Decimal::new(106, 0),
            Decimal::new(1200, 0),
        )
        .unwrap();

        assert!(ohlcv1 < ohlcv2);

        let mut ohlcv_vec = vec![ohlcv2.clone(), ohlcv1.clone()];
        ohlcv_vec.sort();
        assert_eq!(ohlcv_vec[0], ohlcv1);
        assert_eq!(ohlcv_vec[1], ohlcv2);
    }

    #[test]
    fn test_serde_serialization() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let ohlcv = OHLCV::new(
            symbol,
            TimeFrame::OneHour,
            timestamp,
            Decimal::new(100, 0),
            Decimal::new(105, 0),
            Decimal::new(99, 0),
            Decimal::new(103, 0),
            Decimal::new(1000, 0),
        )
        .unwrap();

        let json = serde_json::to_string(&ohlcv).unwrap();
        let deserialized: OHLCV = serde_json::from_str(&json).unwrap();

        assert_eq!(ohlcv, deserialized);
    }

    #[test]
    fn test_display() {
        let symbol = create_test_symbol();
        let timestamp = create_test_timestamp();

        let ohlcv = OHLCV::new(
            symbol,
            TimeFrame::OneHour,
            timestamp,
            Decimal::new(100, 0),
            Decimal::new(105, 0),
            Decimal::new(99, 0),
            Decimal::new(103, 0),
            Decimal::new(1000, 0),
        )
        .unwrap();

        let display_str = format!("{}", ohlcv);
        assert!(display_str.contains("AAPL"));
        assert!(display_str.contains("1h"));
        assert!(display_str.contains("2024-01-01"));
    }
}
