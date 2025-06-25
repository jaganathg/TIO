use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetClass {
    #[serde(rename = "stock")]
    Stock,
    #[serde(rename = "forex")]
    Forex,
    #[serde(rename = "crypto")]
    Crypto,
    #[serde(rename = "commodity")]
    Commodity,
    #[serde(rename = "index")]
    Index,
    #[serde(rename = "bond")]
    Bond,
    #[serde(rename = "etf")]
    ETF,
    #[serde(rename = "option")]
    Option,
    #[serde(rename = "future")]
    Future,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Exchange {
    // US Stock Exchanges
    #[serde(rename = "NASDAQ")]
    NASDAQ,
    #[serde(rename = "NYSE")]
    NYSE,
    #[serde(rename = "AMEX")]
    AMEX,

    // Crypto Exchanges
    #[serde(rename = "BINANCE")]
    Binance,
    #[serde(rename = "COINBASE")]
    Coinbase,
    #[serde(rename = "KRAKEN")]
    Kraken,
    #[serde(rename = "BITFINEX")]
    Bitfinex,

    // Forex
    #[serde(rename = "FOREX")]
    Forex,

    // International Stock Exchanges
    #[serde(rename = "LSE")]
    LSE, // London Stock Exchange
    #[serde(rename = "TSE")]
    TSE, // Tokyo Stock Exchange
    #[serde(rename = "XETRA")]
    XETRA, // German exchange

    // Commodities
    #[serde(rename = "COMEX")]
    COMEX,
    #[serde(rename = "NYMEX")]
    NYMEX,

    // Custom/Other
    #[serde(rename = "OTHER")]
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "pre_market")]
    PreMarket,
    #[serde(rename = "after_market")]
    AfterMarket,
    #[serde(rename = "suspended")]
    Suspended,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
pub struct Symbol {
    /// Primary symbol code (e.g., "AAPL", "BTC-USD", "EUR/USD")
    #[validate(length(min = 1, max = 20))]
    pub code: String,

    /// Human-readable display name
    #[validate(length(min = 1, max = 100))]
    pub display_name: String,

    /// Asset classification
    pub asset_class: AssetClass,

    /// Exchange where this symbol is traded
    pub exchange: Exchange,

    /// Base currency (e.g., "USD", "EUR")
    #[validate(length(equal = 3))]
    pub currency: String,

    /// Quote currency for forex pairs (e.g., "USD" in "EUR/USD")
    pub quote_currency: Option<String>,

    /// Minimum price tick size (smallest price movement)
    pub tick_size: rust_decimal::Decimal,

    /// Contract size (1 for stocks, 100000 for standard forex lots)
    pub contract_size: rust_decimal::Decimal,

    /// Trading timezone (e.g., "America/New_York", "Europe/London")
    pub timezone: String,

    /// Current market status
    pub market_status: MarketStatus,

    /// Whether this symbol is actively tradeable
    pub is_active: bool,

    /// Sector for stocks (e.g., "Technology", "Healthcare")
    pub sector: Option<String>,

    /// Industry for stocks (e.g., "Software", "Pharmaceuticals")
    pub industry: Option<String>,

    /// Additional flexible metadata
    pub metadata: HashMap<String, String>,
}

#[derive(Error, Debug)]
pub enum SymbolError {
    #[error("Invalid symbol format: {0}")]
    InvalidFormat(String),
    #[error("Invalid currency code: {0}")]
    InvalidCurrency(String),
    #[error("Invalid asset class: {0}")]
    InvalidAssetClass(String),
    #[error("Invalid exchange: {0}")]
    InvalidExchange(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Symbol not found: {0}")]
    NotFound(String),
    #[error("Symbol is not active: {0}")]
    NotActive(String),
}

impl Symbol {
    /// Create a new Symbol with validation
    pub fn new(
        code: String,
        display_name: String,
        asset_class: AssetClass,
        exchange: Exchange,
        currency: String,
    ) -> Result<Self, SymbolError> {
        let symbol = Symbol {
            code: code.to_uppercase(),
            display_name,
            asset_class,
            exchange,
            currency: currency.to_uppercase(),
            quote_currency: None,
            tick_size: rust_decimal::Decimal::new(1, 4), // 0.0001 default
            contract_size: rust_decimal::Decimal::ONE,
            timezone: "UTC".to_string(),
            market_status: MarketStatus::Closed,
            is_active: true,
            sector: None,
            industry: None,
            metadata: HashMap::new(),
        };

        symbol
            .validate()
            .map_err(|e| SymbolError::ValidationError(e.to_string()))?;

        Ok(symbol)
    }

    /// Create a stock symbol
    pub fn stock(code: &str, name: &str, exchange: Exchange) -> Result<Self, SymbolError> {
        Self::new(
            code.to_string(),
            name.to_string(),
            AssetClass::Stock,
            exchange,
            "USD".to_string(),
        )
    }

    /// Create a forex pair
    pub fn forex(base: &str, quote: &str) -> Result<Self, SymbolError> {
        let code = format!("{}/{}", base.to_uppercase(), quote.to_uppercase());
        let display_name = format!("{} vs {}", base.to_uppercase(), quote.to_uppercase());

        let mut symbol = Self::new(
            code,
            display_name,
            AssetClass::Forex,
            Exchange::Forex,
            base.to_uppercase(),
        )?;

        symbol.quote_currency = Some(quote.to_uppercase());
        symbol.contract_size = rust_decimal::Decimal::new(100000, 0); // Standard lot

        Ok(symbol)
    }

    /// Create a crypto pair
    pub fn crypto(base: &str, quote: &str, exchange: Exchange) -> Result<Self, SymbolError> {
        let code = format!("{}-{}", base.to_uppercase(), quote.to_uppercase());
        let display_name = format!("{} / {}", base.to_uppercase(), quote.to_uppercase());

        let mut symbol = Self::new(
            code,
            display_name,
            AssetClass::Crypto,
            exchange,
            quote.to_uppercase(),
        )?;

        symbol.quote_currency = Some(quote.to_uppercase());

        Ok(symbol)
    }

    /// Check if symbol is valid for the given exchange
    pub fn is_valid_for_exchange(&self) -> bool {
        match (&self.asset_class, &self.exchange) {
            (AssetClass::Stock, Exchange::NASDAQ | Exchange::NYSE | Exchange::AMEX) => true,
            (
                AssetClass::Crypto,
                Exchange::Binance | Exchange::Coinbase | Exchange::Kraken | Exchange::Bitfinex,
            ) => true,
            (AssetClass::Forex, Exchange::Forex) => true,
            (AssetClass::Commodity, Exchange::COMEX | Exchange::NYMEX) => true,
            _ => false,
        }
    }

    /// Get the full symbol identifier (code@exchange)
    pub fn full_identifier(&self) -> String {
        format!("{}@{}", self.code, self.exchange)
    }

    /// Update market status
    pub fn set_market_status(&mut self, status: MarketStatus) {
        self.market_status = status;
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Check if symbol is currently tradeable
    pub fn is_tradeable(&self) -> bool {
        self.is_active
            && matches!(
                self.market_status,
                MarketStatus::Open | MarketStatus::PreMarket | MarketStatus::AfterMarket
            )
    }

    /// Get display string for UI
    pub fn display(&self) -> String {
        format!("{} ({})", self.display_name, self.code)
    }
}

impl fmt::Display for AssetClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetClass::Stock => write!(f, "Stock"),
            AssetClass::Forex => write!(f, "Forex"),
            AssetClass::Crypto => write!(f, "Crypto"),
            AssetClass::Commodity => write!(f, "Commodity"),
            AssetClass::Index => write!(f, "Index"),
            AssetClass::Bond => write!(f, "Bond"),
            AssetClass::ETF => write!(f, "ETF"),
            AssetClass::Option => write!(f, "Option"),
            AssetClass::Future => write!(f, "Future"),
        }
    }
}

impl fmt::Display for Exchange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Exchange::NASDAQ => write!(f, "NASDAQ"),
            Exchange::NYSE => write!(f, "NYSE"),
            Exchange::AMEX => write!(f, "AMEX"),
            Exchange::Binance => write!(f, "BINANCE"),
            Exchange::Coinbase => write!(f, "COINBASE"),
            Exchange::Kraken => write!(f, "KRAKEN"),
            Exchange::Bitfinex => write!(f, "BITFINEX"),
            Exchange::Forex => write!(f, "FOREX"),
            Exchange::LSE => write!(f, "LSE"),
            Exchange::TSE => write!(f, "TSE"),
            Exchange::XETRA => write!(f, "XETRA"),
            Exchange::COMEX => write!(f, "COMEX"),
            Exchange::NYMEX => write!(f, "NYMEX"),
            Exchange::Other(name) => write!(f, "{}", name),
        }
    }
}

impl FromStr for AssetClass {
    type Err = SymbolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "stock" => Ok(AssetClass::Stock),
            "forex" => Ok(AssetClass::Forex),
            "crypto" => Ok(AssetClass::Crypto),
            "commodity" => Ok(AssetClass::Commodity),
            "index" => Ok(AssetClass::Index),
            "bond" => Ok(AssetClass::Bond),
            "etf" => Ok(AssetClass::ETF),
            "option" => Ok(AssetClass::Option),
            "future" => Ok(AssetClass::Future),
            _ => Err(SymbolError::InvalidAssetClass(s.to_string())),
        }
    }
}

impl FromStr for Exchange {
    type Err = SymbolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NASDAQ" => Ok(Exchange::NASDAQ),
            "NYSE" => Ok(Exchange::NYSE),
            "AMEX" => Ok(Exchange::AMEX),
            "BINANCE" => Ok(Exchange::Binance),
            "COINBASE" => Ok(Exchange::Coinbase),
            "KRAKEN" => Ok(Exchange::Kraken),
            "BITFINEX" => Ok(Exchange::Bitfinex),
            "FOREX" => Ok(Exchange::Forex),
            "LSE" => Ok(Exchange::LSE),
            "TSE" => Ok(Exchange::TSE),
            "XETRA" => Ok(Exchange::XETRA),
            "COMEX" => Ok(Exchange::COMEX),
            "NYMEX" => Ok(Exchange::NYMEX),
            _ => Ok(Exchange::Other(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_create_stock_symbol() {
        let symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap();

        assert_eq!(symbol.code, "AAPL");
        assert_eq!(symbol.display_name, "Apple Inc.");
        assert_eq!(symbol.asset_class, AssetClass::Stock);
        assert_eq!(symbol.exchange, Exchange::NASDAQ);
        assert_eq!(symbol.currency, "USD");
        assert!(symbol.is_active);
    }

    #[test]
    fn test_create_forex_symbol() {
        let symbol = Symbol::forex("EUR", "USD").unwrap();

        assert_eq!(symbol.code, "EUR/USD");
        assert_eq!(symbol.display_name, "EUR vs USD");
        assert_eq!(symbol.asset_class, AssetClass::Forex);
        assert_eq!(symbol.exchange, Exchange::Forex);
        assert_eq!(symbol.currency, "EUR");
        assert_eq!(symbol.quote_currency, Some("USD".to_string()));
        assert_eq!(symbol.contract_size, Decimal::new(100000, 0));
    }

    #[test]
    fn test_create_crypto_symbol() {
        let symbol = Symbol::crypto("BTC", "USD", Exchange::Coinbase).unwrap();

        assert_eq!(symbol.code, "BTC-USD");
        assert_eq!(symbol.display_name, "BTC / USD");
        assert_eq!(symbol.asset_class, AssetClass::Crypto);
        assert_eq!(symbol.exchange, Exchange::Coinbase);
        assert_eq!(symbol.currency, "USD");
        assert_eq!(symbol.quote_currency, Some("USD".to_string()));
    }

    #[test]
    fn test_symbol_validation() {
        // Valid symbol
        let valid = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ);
        assert!(valid.is_ok());

        // Invalid currency (not 3 chars)
        let invalid = Symbol::new(
            "AAPL".to_string(),
            "Apple Inc.".to_string(),
            AssetClass::Stock,
            Exchange::NASDAQ,
            "US".to_string(), // Invalid: should be 3 chars
        );
        assert!(invalid.is_err());
    }

    #[test]
    fn test_exchange_validation() {
        let stock_symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap();
        assert!(stock_symbol.is_valid_for_exchange());

        let invalid_symbol = Symbol::new(
            "AAPL".to_string(),
            "Apple Inc.".to_string(),
            AssetClass::Stock,
            Exchange::Binance, // Wrong exchange for stock
            "USD".to_string(),
        )
        .unwrap();
        assert!(!invalid_symbol.is_valid_for_exchange());
    }

    #[test]
    fn test_market_status_and_tradeability() {
        let mut symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap();

        // Initially closed and active
        symbol.set_market_status(MarketStatus::Closed);
        assert!(!symbol.is_tradeable());

        // Open market should be tradeable
        symbol.set_market_status(MarketStatus::Open);
        assert!(symbol.is_tradeable());

        // Inactive symbol should not be tradeable even when market is open
        symbol.is_active = false;
        assert!(!symbol.is_tradeable());
    }

    #[test]
    fn test_metadata() {
        let mut symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap();

        symbol.add_metadata("sector", "Technology");
        symbol.add_metadata("market_cap", "3000000000000");

        assert_eq!(
            symbol.get_metadata("sector"),
            Some(&"Technology".to_string())
        );
        assert_eq!(
            symbol.get_metadata("market_cap"),
            Some(&"3000000000000".to_string())
        );
        assert_eq!(symbol.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_full_identifier() {
        let symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap();
        assert_eq!(symbol.full_identifier(), "AAPL@NASDAQ");
    }

    #[test]
    fn test_display_methods() {
        let symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap();
        assert_eq!(symbol.display(), "Apple Inc. (AAPL)");
    }

    #[test]
    fn test_asset_class_from_str() {
        assert_eq!(AssetClass::from_str("stock").unwrap(), AssetClass::Stock);
        assert_eq!(AssetClass::from_str("FOREX").unwrap(), AssetClass::Forex);
        assert_eq!(AssetClass::from_str("Crypto").unwrap(), AssetClass::Crypto);
        assert!(AssetClass::from_str("invalid").is_err());
    }

    #[test]
    fn test_exchange_from_str() {
        assert_eq!(Exchange::from_str("nasdaq").unwrap(), Exchange::NASDAQ);
        assert_eq!(Exchange::from_str("BINANCE").unwrap(), Exchange::Binance);

        // Custom exchange
        if let Exchange::Other(name) = Exchange::from_str("CUSTOM_EXCHANGE").unwrap() {
            assert_eq!(name, "CUSTOM_EXCHANGE");
        } else {
            panic!("Expected Other variant");
        }
    }

    #[test]
    fn test_serde_serialization() {
        let symbol = Symbol::stock("AAPL", "Apple Inc.", Exchange::NASDAQ).unwrap();

        let json = serde_json::to_string(&symbol).unwrap();
        let deserialized: Symbol = serde_json::from_str(&json).unwrap();

        assert_eq!(symbol, deserialized);
    }
}
