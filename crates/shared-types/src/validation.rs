use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use crate::Symbol;


// Custom validation functions for trading data
pub fn validate_symbol(symbol: &str) -> Result<(), ValidationError> {
    if symbol.is_empty() || symbol.len() > 20 {
        return Err(ValidationError::new("invalid_symbol_length"));
    }
    
    if !symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
        return Err(ValidationError::new("invalid_symbol_format"));
    }
    
    Ok(())
}

pub fn validate_positive_price(price: &Decimal) -> Result<(), ValidationError> {
    if *price <= Decimal::ZERO {
        return Err(ValidationError::new("price_must_be_positive"));
    }
    Ok(())
}

pub fn validate_non_negative_volume(volume: &Decimal) -> Result<(), ValidationError> {
    if *volume < Decimal::ZERO {
        return Err(ValidationError::new("volume_cannot_be_negative"));
    }
    Ok(())
}


#[derive(Debug, Serialize, Deserialize)]
pub struct AIInsight {
    pub summary: String,
    pub recommendation: TradingRecommendation,
    pub confidence: f32,
    pub reasoning: Vec<String>,
    pub risk_factors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradingRecommendation {
    StrongBuy,
    Buy,
    Hold,
    Sell,
    StrongSell,
    NoRecommendation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern_name: String,
    pub confidence: f32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub description: String,
}

// Portfolio and Position types

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Position {
    #[validate(nested)]
    pub symbol: Symbol,
    
    #[validate(custom(function = "validate_non_negative_volume"))]
    pub quantity: Decimal,
    
    #[validate(custom(function = "validate_positive_price"))]
    pub average_price: Decimal,
    
    pub side: PositionSide,
    
    #[serde(with = "chrono::serde::ts_seconds")]
    pub opened_at: DateTime<Utc>,
    
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionSide {
    Long,
    Short,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct Portfolio {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    
    pub positions: Vec<Position>,
    pub cash_balance: Decimal,
    pub total_value: Decimal,
    pub total_pnl: Decimal,
    
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_updated: DateTime<Utc>,
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::{Symbol, TimeFrame, OHLCV, Exchange};
    use std::collections::HashMap;
    
    #[test]
    fn test_symbol_validation() {
        // Valid symbol
        let valid_symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ);
        assert!(valid_symbol.is_ok());
        
        // Invalid symbol - empty
        let invalid_symbol = Symbol::stock("", "Jaguar", Exchange::NYSE);
        assert!(invalid_symbol.is_err());
        
        // Invalid symbol - too long
        let long_symbol = Symbol::forex("VERYLONGSYMBOLNAME123", "USDGBP");
        assert!(long_symbol.is_err());
    }
    
    #[test]
    fn test_ohlcv_serialization() {
        let symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap();
        let ohlcv = OHLCV {
            symbol,
            timestamp: Utc::now(),
            open: Decimal::new(15000, 2), // 150.00
            high: Decimal::new(15500, 2), // 155.00
            low: Decimal::new(14800, 2),  // 148.00
            close: Decimal::new(15200, 2), // 152.00
            volume: Decimal::new(1000000, 0),
            timeframe: TimeFrame::OneDay,
            metadata: HashMap::new(),
        };
        
        // Test serialization
        let json = serde_json::to_string(&ohlcv).unwrap();
        assert!(json.contains("AAPL"));
        
        // Test deserialization
        let deserialized: OHLCV = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.symbol.code, "AAPL");
    }
    
    #[test]
    fn test_ohlcv_validation() {
        let symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap();
        let mut ohlcv = OHLCV {
            symbol,
            timestamp: Utc::now(),
            open: Decimal::new(15000, 2),
            high: Decimal::new(15500, 2),
            low: Decimal::new(14800, 2),
            close: Decimal::new(15200, 2),
            volume: Decimal::new(1000000, 0),
            timeframe: TimeFrame::OneDay,
            metadata: HashMap::new(),
        };
        
        // Valid OHLCV should pass validation
        assert!(ohlcv.validate().is_ok());
        
        // Test negative price - should fail
        ohlcv.open = Decimal::new(-100, 2);
        assert!(ohlcv.validate().is_err());
        
        // Reset and test negative volume - should fail
        ohlcv.open = Decimal::new(15000, 2);
        ohlcv.volume = Decimal::new(-100, 0);
        assert!(ohlcv.validate().is_err());
        
        // Reset volume and test all price validations
        ohlcv.volume = Decimal::new(1000000, 0);
        
        // Test negative high price
        ohlcv.high = Decimal::new(-100, 2);
        assert!(ohlcv.validate().is_err());
        
        // Test negative low price
        ohlcv.high = Decimal::new(15500, 2);
        ohlcv.low = Decimal::new(-100, 2);
        assert!(ohlcv.validate().is_err());
        
        // Test negative close price
        ohlcv.low = Decimal::new(14800, 2);
        ohlcv.close = Decimal::new(-100, 2);
        assert!(ohlcv.validate().is_err());
    }
    
    #[test]
    fn test_websocket_message_serialization() {
        use crate::api_types::{WebSocketMessage, WebSocketMessageType};
        use uuid::Uuid;
        
        let symbol = Symbol::crypto("BTC", "USD", Exchange::Binance).unwrap();
        let message = WebSocketMessage {
            message_id: Uuid::new_v4(),
            message_type: WebSocketMessageType::Subscribe,
            payload: serde_json::json!({
                "symbols": vec![symbol],
                "timeframes": vec![TimeFrame::OneMinute, TimeFrame::OneHour]
            }),
            timestamp: Utc::now(),
        };
        
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.message_type, WebSocketMessageType::Subscribe);
        
        // Test the payload contains our data
        if let Some(symbols_array) = deserialized.payload.get("symbols") {
            assert!(symbols_array.is_array());
        }
        if let Some(timeframes_array) = deserialized.payload.get("timeframes") {
            assert!(timeframes_array.is_array());
        }
    }
}