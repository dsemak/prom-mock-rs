//! Time utilities for parsing relative time expressions.
//!
//! This module provides utilities for parsing and resolving relative time
//! expressions like "now-15m" into absolute timestamps.

use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

/// Result of resolving a time/interval parameter.
pub enum ResolvedParam {
    /// Original string left as-is (format not recognized)
    Raw(String),
    /// Resolved to absolute UNIX timestamp in seconds (as string)
    Absolute(String),
    /// Relative but we could compute absolute (used for comparison)
    Relative(String),
}

/// Resolve relative time expressions to absolute timestamps.
///
/// Supports:
/// - "now", "now-15m", "now-1h", "now-30s", "now-2d"
/// - ISO-8601 (RFC3339)
/// - UNIX seconds (string of digits)
///
/// # Parameters
///
/// - `input` - Time expression string to resolve
/// - `now` - Optional fixed time for relative resolution, uses current time if None
///
/// # Returns
///
/// Returns `ResolvedParam` containing the resolved timestamp or raw string if unparseable.
pub fn resolve_relative(input: &str, now: Option<OffsetDateTime>) -> ResolvedParam {
    let s = input.trim();

    // UNIX seconds
    if s.chars().all(|c| c.is_ascii_digit()) {
        return ResolvedParam::Absolute(s.to_string());
    }

    // ISO-8601
    if time::OffsetDateTime::parse(s, &Rfc3339).is_ok() {
        return ResolvedParam::Absolute(s.to_string());
    }

    // now / now-<N><unit>
    if let Some(now) = now {
        if s == "now" {
            return ResolvedParam::Relative(now.unix_timestamp().to_string());
        }
        if let Some(rest) = s.strip_prefix("now-") {
            if let Some((num, unit)) = split_num_unit(rest) {
                if let Ok(n) = num.parse::<i64>() {
                    let dur = match unit {
                        "s" => Duration::seconds(n),
                        "m" => Duration::minutes(n),
                        "h" => Duration::hours(n),
                        "d" => Duration::days(n),
                        _ => return ResolvedParam::Raw(s.to_string()),
                    };
                    let ts = (now - dur).unix_timestamp();
                    return ResolvedParam::Relative(ts.to_string());
                }
            }
        }
    }

    ResolvedParam::Raw(s.to_string())
}

fn split_num_unit(s: &str) -> Option<(&str, &str)> {
    let i = s.find(|c: char| !c.is_ascii_digit())?;
    Some((&s[..i], &s[i..]))
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    /// Test resolving absolute timestamps and formats.
    #[test]
    fn test_resolve_absolute_formats() {
        // UNIX timestamp
        match resolve_relative("1640995200", None) {
            ResolvedParam::Absolute(ts) => assert_eq!(ts, "1640995200"),
            _ => panic!("Expected absolute timestamp"),
        }

        // ISO-8601 / RFC3339
        let iso_time = "2022-01-01T00:00:00Z";
        match resolve_relative(iso_time, None) {
            ResolvedParam::Absolute(ts) => assert_eq!(ts, iso_time),
            _ => panic!("Expected absolute timestamp"),
        }

        // Invalid format should return raw
        match resolve_relative("invalid-time", None) {
            ResolvedParam::Raw(raw) => assert_eq!(raw, "invalid-time"),
            _ => panic!("Expected raw string"),
        }
    }

    /// Test resolving "now" relative expressions.
    #[test]
    fn test_resolve_now_expressions() {
        let fixed_time = datetime!(2022-01-01 12:00:00 UTC);

        // Simple "now"
        match resolve_relative("now", Some(fixed_time)) {
            ResolvedParam::Relative(ts) => assert_eq!(ts, "1641038400"), // 2022-01-01 12:00:00
            _ => panic!("Expected relative timestamp"),
        }

        // now-15m (15 minutes ago)
        match resolve_relative("now-15m", Some(fixed_time)) {
            ResolvedParam::Relative(ts) => assert_eq!(ts, "1641037500"), // 15 minutes earlier
            _ => panic!("Expected relative timestamp"),
        }

        // now-2h (2 hours ago)
        match resolve_relative("now-2h", Some(fixed_time)) {
            ResolvedParam::Relative(ts) => assert_eq!(ts, "1641031200"), // 2 hours earlier
            _ => panic!("Expected relative timestamp"),
        }

        // now-30s (30 seconds ago)
        match resolve_relative("now-30s", Some(fixed_time)) {
            ResolvedParam::Relative(ts) => assert_eq!(ts, "1641038370"), // 30 seconds earlier
            _ => panic!("Expected relative timestamp"),
        }

        // now-1d (1 day ago)
        match resolve_relative("now-1d", Some(fixed_time)) {
            ResolvedParam::Relative(ts) => assert_eq!(ts, "1640952000"), // 1 day earlier
            _ => panic!("Expected relative timestamp"),
        }
    }

    /// Test invalid relative expressions.
    #[test]
    fn test_resolve_invalid_relative() {
        let fixed_time = datetime!(2022-01-01 12:00:00 UTC);

        // Invalid unit
        match resolve_relative("now-15x", Some(fixed_time)) {
            ResolvedParam::Raw(raw) => assert_eq!(raw, "now-15x"),
            _ => panic!("Expected raw string for invalid unit"),
        }

        // Invalid number
        match resolve_relative("now-abcm", Some(fixed_time)) {
            ResolvedParam::Raw(raw) => assert_eq!(raw, "now-abcm"),
            _ => panic!("Expected raw string for invalid number"),
        }

        // Missing number
        match resolve_relative("now-m", Some(fixed_time)) {
            ResolvedParam::Raw(raw) => assert_eq!(raw, "now-m"),
            _ => panic!("Expected raw string for missing number"),
        }

        // Without fixed time, relative expressions should return raw
        match resolve_relative("now", None) {
            ResolvedParam::Raw(raw) => assert_eq!(raw, "now"),
            _ => panic!("Expected raw string when no fixed time provided"),
        }
    }

    /// Test edge cases and boundary conditions.
    #[test]
    fn test_edge_cases() {
        let fixed_time = datetime!(2022-01-01 12:00:00 UTC);

        // Whitespace handling
        match resolve_relative("  now-1h  ", Some(fixed_time)) {
            ResolvedParam::Relative(ts) => assert_eq!(ts, "1641034800"),
            _ => panic!("Expected relative timestamp with whitespace trimming"),
        }

        // Zero duration
        match resolve_relative("now-0s", Some(fixed_time)) {
            ResolvedParam::Relative(ts) => assert_eq!(ts, "1641038400"), // Same as "now"
            _ => panic!("Expected relative timestamp for zero duration"),
        }

        // Large numbers
        match resolve_relative("now-999m", Some(fixed_time)) {
            ResolvedParam::Relative(ts) => assert_eq!(ts, "1640978460"), // 999 minutes earlier
            _ => panic!("Expected relative timestamp for large duration"),
        }

        // Empty string (all digits check passes for empty string)
        match resolve_relative("", None) {
            ResolvedParam::Absolute(ts) => assert_eq!(ts, ""),
            _ => panic!("Expected absolute timestamp for empty input"),
        }
    }

    /// Test split_num_unit helper function.
    #[test]
    fn test_split_num_unit() {
        assert_eq!(split_num_unit("15m"), Some(("15", "m")));
        assert_eq!(split_num_unit("123s"), Some(("123", "s")));
        assert_eq!(split_num_unit("0h"), Some(("0", "h")));
        assert_eq!(split_num_unit("42days"), Some(("42", "days")));

        // Edge cases
        assert_eq!(split_num_unit("m"), Some(("", "m"))); // No digits at start
        assert_eq!(split_num_unit("123"), None); // No unit (all digits)
        assert_eq!(split_num_unit(""), None); // Empty string
        assert_eq!(split_num_unit("abc123def"), Some(("", "abc123def"))); // Starts with non-digit
    }
}
