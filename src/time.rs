//! Minimal UTC timestamp formatting for the `saveDateTime` attribute, so we
//! do not need a date/time dependency.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current UTC time as `YYYY-MM-DDTHH:MM:SS`.
pub(crate) fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_timestamp(secs)
}

/// Formats seconds since the Unix epoch as `YYYY-MM-DDTHH:MM:SS` (UTC).
pub(crate) fn format_timestamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}",
        rem / 3600,
        (rem / 60) % 60,
        rem % 60
    )
}

/// Days since 1970-01-01 -> (year, month, day), proleptic Gregorian.
/// Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch() {
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00");
    }

    #[test]
    fn known_date() {
        // 2026-07-18T07:51:05Z, verified with `date -u`.
        assert_eq!(format_timestamp(1_784_361_065), "2026-07-18T07:51:05");
    }

    #[test]
    fn leap_day() {
        // 2000-02-29T00:00:00Z
        assert_eq!(format_timestamp(951_782_400), "2000-02-29T00:00:00");
    }
}
