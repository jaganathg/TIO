use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeUnit {
    #[serde(rename = "m")]
    Minutes,
    #[serde(rename = "h")]
    Hours,
    #[serde(rename = "d")]
    Days,
    #[serde(rename = "w")]
    Weeks,
    #[serde(rename = "M")]
    Months,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TimeFrame {
    // Standard timeframes
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
    OneMonth,
    
    // Custom timeframe
    Custom { value: u32, unit: TimeUnit },
}

#[derive(Error, Debug)]
pub enum TimeFrameError {
    #[error("Invalid timeframe format: {0}")]
    InvalidFormat(String),
    #[error("Invalid time unit: {0}")]
    InvalidUnit(String),
    #[error("Invalid time value: {0}")]
    InvalidValue(String),
    #[error("Zero or negative timeframe values are not allowed")]
    ZeroOrNegativeValue,
}

impl TimeFrame {
    /// Convert timeframe to total seconds
    pub fn to_seconds(&self) -> u64 {
        match self {
            TimeFrame::OneMinute => 60,
            TimeFrame::FiveMinutes => 300,
            TimeFrame::FifteenMinutes => 900,
            TimeFrame::ThirtyMinutes => 1800,
            TimeFrame::OneHour => 3600,
            TimeFrame::FourHours => 14400,
            TimeFrame::OneDay => 86400,
            TimeFrame::OneWeek => 604800,
            TimeFrame::OneMonth => 2592000, // 30 days approximation
            TimeFrame::Custom { value, unit } => {
                let base_seconds = match unit {
                    TimeUnit::Minutes => 60,
                    TimeUnit::Hours => 3600,
                    TimeUnit::Days => 86400,
                    TimeUnit::Weeks => 604800,
                    TimeUnit::Months => 2592000, // 30 days approximation
                };
                (*value as u64) * base_seconds
            }
        }
    }

    /// Convert timeframe to Duration
    pub fn to_duration(&self) -> Duration {
        Duration::from_secs(self.to_seconds())
    }

    /// Check if this is a standard predefined timeframe
    pub fn is_standard(&self) -> bool {
        !matches!(self, TimeFrame::Custom { .. })
    }

    /// Get all standard timeframes
    pub fn standard_timeframes() -> Vec<TimeFrame> {
        vec![
            TimeFrame::OneMinute,
            TimeFrame::FiveMinutes,
            TimeFrame::FifteenMinutes,
            TimeFrame::ThirtyMinutes,
            TimeFrame::OneHour,
            TimeFrame::FourHours,
            TimeFrame::OneDay,
            TimeFrame::OneWeek,
            TimeFrame::OneMonth,
        ]
    }

    /// Create a custom timeframe
    pub fn custom(value: u32, unit: TimeUnit) -> Result<Self, TimeFrameError> {
        if value == 0 {
            return Err(TimeFrameError::ZeroOrNegativeValue);
        }
        Ok(TimeFrame::Custom { value, unit })
    }
}

impl FromStr for TimeFrame {
    type Err = TimeFrameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        
        // Handle standard timeframes first
        match s {
            "1m" => return Ok(TimeFrame::OneMinute),
            "5m" => return Ok(TimeFrame::FiveMinutes),
            "15m" => return Ok(TimeFrame::FifteenMinutes),
            "30m" => return Ok(TimeFrame::ThirtyMinutes),
            "1h" => return Ok(TimeFrame::OneHour),
            "4h" => return Ok(TimeFrame::FourHours),
            "1d" => return Ok(TimeFrame::OneDay),
            "1w" => return Ok(TimeFrame::OneWeek),
            "1M" => return Ok(TimeFrame::OneMonth),
            _ => {}
        }

        // Parse custom timeframe
        if s.len() < 2 {
            return Err(TimeFrameError::InvalidFormat(s.to_string()));
        }

        let (number_part, unit_part) = if s.ends_with('m') && s != "1m" {
            (&s[..s.len()-1], "m")
        } else if s.ends_with('h') {
            (&s[..s.len()-1], "h")
        } else if s.ends_with('d') {
            (&s[..s.len()-1], "d")
        } else if s.ends_with('w') {
            (&s[..s.len()-1], "w")
        } else if s.ends_with('M') {
            (&s[..s.len()-1], "M")
        } else {
            return Err(TimeFrameError::InvalidFormat(s.to_string()));
        };

        let value: u32 = number_part
            .parse()
            .map_err(|_| TimeFrameError::InvalidValue(number_part.to_string()))?;
        
        if value == 0 {
            return Err(TimeFrameError::ZeroOrNegativeValue);
        }

        let unit = match unit_part {
            "m" => TimeUnit::Minutes,
            "h" => TimeUnit::Hours,
            "d" => TimeUnit::Days,
            "w" => TimeUnit::Weeks,
            "M" => TimeUnit::Months,
            _ => return Err(TimeFrameError::InvalidUnit(unit_part.to_string())),
        };

        Ok(TimeFrame::Custom { value, unit })
    }
}

impl fmt::Display for TimeFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeFrame::OneMinute => write!(f, "1m"),
            TimeFrame::FiveMinutes => write!(f, "5m"),
            TimeFrame::FifteenMinutes => write!(f, "15m"),
            TimeFrame::ThirtyMinutes => write!(f, "30m"),
            TimeFrame::OneHour => write!(f, "1h"),
            TimeFrame::FourHours => write!(f, "4h"),
            TimeFrame::OneDay => write!(f, "1d"),
            TimeFrame::OneWeek => write!(f, "1w"),
            TimeFrame::OneMonth => write!(f, "1M"),
            TimeFrame::Custom { value, unit } => {
                let unit_str = match unit {
                    TimeUnit::Minutes => "m",
                    TimeUnit::Hours => "h",
                    TimeUnit::Days => "d",
                    TimeUnit::Weeks => "w",
                    TimeUnit::Months => "M",
                };
                write!(f, "{}{}", value, unit_str)
            }
        }
    }
}

// Implement ordering based on total seconds
impl PartialOrd for TimeFrame {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeFrame {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_seconds().cmp(&other.to_seconds())
    }
}

// Custom serde implementation to serialize as string
impl Serialize for TimeFrame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for TimeFrame {
    fn deserialize<D>(deserializer: D) -> Result<TimeFrame, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        TimeFrame::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_timeframes() {
        assert_eq!(TimeFrame::OneMinute.to_seconds(), 60);
        assert_eq!(TimeFrame::FiveMinutes.to_seconds(), 300);
        assert_eq!(TimeFrame::OneHour.to_seconds(), 3600);
        assert_eq!(TimeFrame::OneDay.to_seconds(), 86400);
    }

    #[test]
    fn test_custom_timeframes() {
        let tf = TimeFrame::custom(2, TimeUnit::Hours).unwrap();
        assert_eq!(tf.to_seconds(), 7200);
        
        let tf = TimeFrame::custom(3, TimeUnit::Days).unwrap();
        assert_eq!(tf.to_seconds(), 259200);
    }

    #[test]
    fn test_from_str_standard() {
        assert_eq!(TimeFrame::from_str("1m").unwrap(), TimeFrame::OneMinute);
        assert_eq!(TimeFrame::from_str("5m").unwrap(), TimeFrame::FiveMinutes);
        assert_eq!(TimeFrame::from_str("1h").unwrap(), TimeFrame::OneHour);
        assert_eq!(TimeFrame::from_str("1d").unwrap(), TimeFrame::OneDay);
        assert_eq!(TimeFrame::from_str("1M").unwrap(), TimeFrame::OneMonth);
    }

    #[test]
    fn test_month_vs_minute() {
        assert_eq!(TimeFrame::from_str("1m").unwrap(), TimeFrame::OneMinute);
        assert_eq!(TimeFrame::from_str("1M").unwrap(), TimeFrame::OneMonth);
        // Should NOT be equal!
        assert_ne!(TimeFrame::from_str("1m").unwrap(), TimeFrame::from_str("1M").unwrap());
    }

    #[test]
    fn test_from_str_custom() {
        let tf = TimeFrame::from_str("2h").unwrap();
        assert_eq!(tf, TimeFrame::Custom { value: 2, unit: TimeUnit::Hours });
        
        let tf = TimeFrame::from_str("3d").unwrap();
        assert_eq!(tf, TimeFrame::Custom { value: 3, unit: TimeUnit::Days });
        
        let tf = TimeFrame::from_str("10m").unwrap();
        assert_eq!(tf, TimeFrame::Custom { value: 10, unit: TimeUnit::Minutes });
    }

    #[test]
    fn test_to_string() {
        assert_eq!(TimeFrame::OneMinute.to_string(), "1m");
        assert_eq!(TimeFrame::OneHour.to_string(), "1h");
        
        let tf = TimeFrame::Custom { value: 2, unit: TimeUnit::Hours };
        assert_eq!(tf.to_string(), "2h");
    }

    #[test]
    fn test_ordering() {
        assert!(TimeFrame::OneMinute < TimeFrame::FiveMinutes);
        assert!(TimeFrame::OneHour > TimeFrame::ThirtyMinutes);
        
        let custom_2h = TimeFrame::Custom { value: 2, unit: TimeUnit::Hours };
        assert!(TimeFrame::OneHour < custom_2h);
    }

    #[test]
    fn test_serde() {
        let tf = TimeFrame::OneHour;
        let json = serde_json::to_string(&tf).unwrap();
        assert_eq!(json, "\"1h\"");
        
        let deserialized: TimeFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, TimeFrame::OneHour);
    }

    #[test]
    fn test_invalid_formats() {
        assert!(TimeFrame::from_str("").is_err());
        assert!(TimeFrame::from_str("x").is_err());
        assert!(TimeFrame::from_str("0m").is_err());
        assert!(TimeFrame::from_str("1x").is_err());
        assert!(TimeFrame::from_str("-1m").is_err());
    }

    #[test]
    fn test_is_standard() {
        assert!(TimeFrame::OneHour.is_standard());
        
        let custom = TimeFrame::Custom { value: 2, unit: TimeUnit::Hours };
        assert!(!custom.is_standard());
    }

    #[test]
    fn test_duration_conversion() {
        let tf = TimeFrame::OneHour;
        let duration = tf.to_duration();
        assert_eq!(duration, Duration::from_secs(3600));
    }
}