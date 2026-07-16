//! Date extraction from filenames for `_file_date` virtual column.
//!
//! Recognizes three patterns (tried in priority order):
//! 1. YYYY-MM-DD (e.g. `sales_2024-01-15`)
//! 2. YYYY_MM_DD (e.g. `data_2024_01_15`)
//! 3. YYYYMMDD   (e.g. `report_20240115`)
//!
//! Returns the date normalized to ISO 8601 (YYYY-MM-DD) or empty string if not found.

use std::sync::LazyLock;

use regex::Regex;

/// Regex for YYYY-MM-DD pattern.
static RE_DASH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap());

/// Regex for YYYY_MM_DD pattern.
static RE_UNDERSCORE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{4})_(\d{2})_(\d{2})").unwrap());

/// Regex for YYYYMMDD pattern (exactly 8 consecutive digits bounded by non-digits or string edges).
static RE_COMPACT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[^\d])(\d{4})(\d{2})(\d{2})(?:$|[^\d])").unwrap());

/// Extract a date from a filename stem.
///
/// Searches the stem (filename without extension) for date patterns.
/// Returns normalized ISO date (`YYYY-MM-DD`) or empty string if no valid date found.
///
/// In recursive mode where stem may contain path separators (e.g. `sub/file_2024-01-15`),
/// the function still correctly extracts from the full string.
pub fn extract_file_date(stem: &str) -> String {
    // Try patterns in priority order: YYYY-MM-DD > YYYY_MM_DD > YYYYMMDD
    if let Some(caps) = RE_DASH.captures(stem) {
        let (y, m, d) = (&caps[1], &caps[2], &caps[3]);
        if is_valid_date(y, m, d) {
            return format!("{y}-{m}-{d}");
        }
    }

    if let Some(caps) = RE_UNDERSCORE.captures(stem) {
        let (y, m, d) = (&caps[1], &caps[2], &caps[3]);
        if is_valid_date(y, m, d) {
            return format!("{y}-{m}-{d}");
        }
    }

    if let Some(caps) = RE_COMPACT.captures(stem) {
        let (y, m, d) = (&caps[1], &caps[2], &caps[3]);
        if is_valid_date(y, m, d) {
            return format!("{y}-{m}-{d}");
        }
    }

    String::new()
}

/// Basic calendar validation: year 1900-2099, month 01-12, day 01-31.
fn is_valid_date(year: &str, month: &str, day: &str) -> bool {
    let y: u16 = match year.parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let m: u8 = match month.parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let d: u8 = match day.parse() {
        Ok(v) => v,
        Err(_) => return false,
    };

    (1900..=2099).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d)
}

#[cfg(test)]
mod tests {
    use super::*;

    // === YYYY-MM-DD pattern ===

    #[test]
    fn test_extract_yyyy_mm_dd_middle() {
        assert_eq!(extract_file_date("sales_2024-01-15"), "2024-01-15");
    }

    #[test]
    fn test_extract_yyyy_mm_dd_at_start() {
        assert_eq!(extract_file_date("2024-03-01_report"), "2024-03-01");
    }

    #[test]
    fn test_extract_yyyy_mm_dd_at_end() {
        assert_eq!(extract_file_date("report_2024-12-31"), "2024-12-31");
    }

    // === YYYYMMDD pattern ===

    #[test]
    fn test_extract_yyyymmdd() {
        assert_eq!(extract_file_date("data_20240115"), "2024-01-15");
    }

    #[test]
    fn test_extract_yyyymmdd_at_start() {
        assert_eq!(extract_file_date("20240301_metrics"), "2024-03-01");
    }

    #[test]
    fn test_extract_yyyymmdd_entire_stem() {
        assert_eq!(extract_file_date("20240315"), "2024-03-15");
    }

    // === YYYY_MM_DD pattern ===

    #[test]
    fn test_extract_yyyy_underscore_mm_dd() {
        assert_eq!(extract_file_date("data_2024_06_15"), "2024-06-15");
    }

    #[test]
    fn test_extract_yyyy_underscore_mm_dd_with_suffix() {
        assert_eq!(extract_file_date("sales_2024_01_15_final"), "2024-01-15");
    }

    // === No date / edge cases ===

    #[test]
    fn test_extract_no_date_returns_empty() {
        assert_eq!(extract_file_date("report_final"), "");
    }

    #[test]
    fn test_extract_empty_stem_returns_empty() {
        assert_eq!(extract_file_date(""), "");
    }

    #[test]
    fn test_extract_numbers_but_not_date() {
        assert_eq!(extract_file_date("sales_v2_rev3"), "");
    }

    #[test]
    fn test_extract_partial_date_no_match() {
        // year-month only (no day) should not match
        assert_eq!(extract_file_date("sales_2024-01"), "");
    }

    #[test]
    fn test_extract_multiple_dates_picks_first() {
        assert_eq!(extract_file_date("2024-01-01_to_2024-06-30"), "2024-01-01");
    }

    #[test]
    fn test_extract_invalid_month_returns_empty() {
        assert_eq!(extract_file_date("data_2024-13-01"), "");
    }

    #[test]
    fn test_extract_invalid_day_returns_empty() {
        assert_eq!(extract_file_date("data_2024-01-32"), "");
    }

    #[test]
    fn test_extract_invalid_yyyymmdd_returns_empty() {
        assert_eq!(extract_file_date("data_20241345"), "");
    }

    #[test]
    fn test_extract_normalizes_to_iso() {
        // All three formats output YYYY-MM-DD
        assert_eq!(extract_file_date("a_2024-03-15_b"), "2024-03-15");
        assert_eq!(extract_file_date("a_2024_03_15_b"), "2024-03-15");
        assert_eq!(extract_file_date("a_20240315_b"), "2024-03-15");
    }

    #[test]
    fn test_extract_with_path_prefix() {
        // In recursive mode, stem may contain path separators
        assert_eq!(extract_file_date("sub1/sales_2024-03-01"), "2024-03-01");
    }

    #[test]
    fn test_extract_nine_digits_no_false_positive() {
        // 9 digits should not falsely match YYYYMMDD
        assert_eq!(extract_file_date("id_123456789"), "");
    }
}
