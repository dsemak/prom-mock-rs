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
