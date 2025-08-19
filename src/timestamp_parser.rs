use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};

/// Handles parsing timestamps from various formats used in Claude usage data
pub struct TimestampParser;

impl TimestampParser {
    /// Parse a timestamp string into a DateTime<Utc>
    /// Handles both Z suffix and timezone info formats
    pub fn parse(timestamp_str: &str) -> Result<DateTime<Utc>> {
        // Handle both Z suffix and timezone info
        let timestamp = if timestamp_str.ends_with('Z') {
            timestamp_str.replace('Z', "+00:00")
        } else {
            timestamp_str.to_string()
        };

        // Try parsing as ISO 8601
        if let Ok(dt) = DateTime::parse_from_rfc3339(&timestamp) {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try parsing as naive datetime and assume UTC
        if let Ok(naive) = NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%dT%H:%M:%S%.f") {
            return Ok(DateTime::from_naive_utc_and_offset(naive, Utc));
        }

        anyhow::bail!("Failed to parse timestamp: {}", timestamp_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_z_suffix() {
        let result = TimestampParser::parse("2024-01-01T12:00:00.000Z");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_timezone() {
        let result = TimestampParser::parse("2024-01-01T12:00:00.000+00:00");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_naive() {
        let result = TimestampParser::parse("2024-01-01T12:00:00.000");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid() {
        let result = TimestampParser::parse("invalid");
        assert!(result.is_err());
    }
}
