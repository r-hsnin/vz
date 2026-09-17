use std::path::Path;

/// Extract the file stem from a path as a `&str`.
///
/// Returns the file stem (filename without extension) if it is valid UTF-8,
/// otherwise returns `"unknown"`.
pub fn path_label(p: &Path) -> &str {
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown")
}

/// Parse a display-formatted number: strips `,`/spaces, currency symbols
/// (`$€£¥₹`), trailing `%` (percent → fraction), and unit suffixes
/// (`K/M/B/T`, `KB/MB/GB/TB`, `GiB`…). Returns `None` for non-numeric
/// strings (names, dates, `N/A`) and non-finite values (`NaN`/`inf`).
/// This is the single numeric parser for all chart/filter/stats paths —
/// inference, aggregation, series, diff, sparkline, and JSON samples
/// must all go through here so `$100` means 100 everywhere, not just
/// on the axis that happens to strip commas today.
pub fn parse_number(raw: &str) -> Option<f64> {
    let t = strip_outer(raw);
    if t.is_empty() {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "nan" | "inf" | "+inf" | "-inf" | "infinity" | "+infinity" | "-infinity"
    ) {
        return None;
    }
    // Percent: "45%" → 0.45. Missing number ("%" alone) is not numeric.
    if let Some(num) = lower.strip_suffix('%') {
        let v = parse_scaled(num.strip_suffix("pp").unwrap_or(num))?;
        return Some(v / 100.0);
    }
    parse_scaled(&lower)
}

/// Strip surrounding whitespace/quotes/parens, leading currency symbols,
/// and embedded `,`/space/`_`/`'` digit separators. `(42)` → `-42`.
fn strip_outer(raw: &str) -> String {
    let mut t = raw.trim().to_string();
    // strip one layer of surrounding quotes
    if t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')))
    {
        t = t[1..t.len() - 1].to_string();
        t = t.trim().to_string();
    }
    let negative_paren = t.starts_with('(') && t.ends_with(')') && t.len() >= 2;
    if negative_paren {
        t = t[1..t.len() - 1].to_string();
    }
    // leading sign (+/-), then currency symbols, then digits
    let mut t = t.trim().to_string();
    let mut sign = if negative_paren { "-" } else { "" }.to_string();
    if t.starts_with('+') || t.starts_with('-') {
        if t.starts_with('-') && sign.is_empty() {
            sign = "-".to_string();
        }
        t = t[1..].trim_start().to_string();
    }
    // strip leading currency symbols / ISO codes (USD 100, $100)
    for sym in [
        "$", "€", "£", "¥", "₹", "usd", "eur", "gbp", "jpy", "cny", "inr",
    ] {
        if t.to_ascii_lowercase().starts_with(sym) {
            t = t[sym.len()..].trim_start().to_string();
            break;
        }
    }
    // trailing currency symbol ($-suffix "100€")
    for sym in ["$", "€", "£", "¥", "₹"] {
        if t.ends_with(sym) {
            t = t[..t.len() - sym.len()].trim_end().to_string();
            break;
        }
    }
    let mut cleaned: String = t
        .chars()
        .filter(|c| *c != ',' && *c != ' ' && *c != '_' && *c != '\'')
        .collect();
    if !sign.is_empty() && !cleaned.starts_with('-') && !cleaned.starts_with('+') {
        cleaned = format!("{sign}{cleaned}");
    }
    cleaned
}

/// Parse the stripped body, honoring trailing unit suffixes
/// (`k/m/b/t`, `kb/mb/gb/tb`, `kib/mib/gib/tib`, `bps`).
fn parse_scaled(lower: &str) -> Option<f64> {
    let (num, mult) = strip_unit_suffix(lower)?;
    let v: f64 = num.parse().ok()?;
    if !v.is_finite() {
        return None;
    }
    Some(v * mult)
}

/// Split trailing unit suffix; returns (number_text, multiplier).
fn strip_unit_suffix(lower: &str) -> Option<(&str, f64)> {
    // longest suffixes first so "gib"/"gb" win over "b"
    const SUFFIXES: &[(&str, f64)] = &[
        ("tib", 1_099_511_627_776.0),
        ("gib", 1_073_741_824.0),
        ("mib", 1_048_576.0),
        ("kib", 1024.0),
        ("tb", 1e12),
        ("gb", 1e9),
        ("mb", 1e6),
        ("kb", 1e3),
        ("bps", 1.0),
        ("t", 1e12),
        ("g", 1e9),
        ("b", 1e9),
        ("m", 1e6),
        ("k", 1e3),
    ];
    for (suf, mult) in SUFFIXES {
        if let Some(num) = lower.strip_suffix(suf) {
            if num.is_empty() {
                return None;
            }
            // require the char before the suffix to be a digit or '.'
            // so "abc" / "m" don't parse but "10m" does
            if num
                .chars()
                .last()
                .is_some_and(|c| c.is_ascii_digit() || c == '.')
            {
                return Some((num, *mult));
            }
            return None;
        }
    }
    Some((lower, 1.0))
}

/// Compute the minimum and maximum of a slice of f64 values.
pub fn min_max(values: &[f64]) -> Option<(f64, f64)> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in values {
        if !v.is_finite() {
            continue;
        }
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    if min == f64::INFINITY {
        return None;
    }
    Some((min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_path_label_with_extension() {
        assert_eq!(path_label(Path::new("sales.csv")), "sales");
    }

    #[test]
    fn test_path_label_without_extension() {
        assert_eq!(path_label(Path::new("data")), "data");
    }

    #[test]
    fn test_path_label_multiple_dots() {
        assert_eq!(path_label(Path::new("data.backup.csv")), "data.backup");
    }

    #[test]
    fn test_path_label_full_path() {
        assert_eq!(path_label(Path::new("/tmp/reports/q1.csv")), "q1");
    }

    #[test]
    fn test_path_label_empty_path() {
        assert_eq!(path_label(Path::new("")), "unknown");
    }

    #[test]
    fn test_parse_number_comma_formatted() {
        assert_eq!(parse_number("1,000"), Some(1000.0));
        assert_eq!(parse_number("1,234,567"), Some(1234567.0));
        assert_eq!(parse_number("1 000"), Some(1000.0));
    }

    #[test]
    fn test_parse_number_currency_prefix() {
        assert_eq!(parse_number("$100"), Some(100.0));
        assert_eq!(parse_number("€50"), Some(50.0));
        assert_eq!(parse_number("¥1000"), Some(1000.0));
        assert_eq!(parse_number("USD 42"), Some(42.0));
        assert_eq!(parse_number("-$5"), Some(-5.0));
    }

    #[test]
    fn test_parse_number_percent_is_fraction() {
        assert_eq!(parse_number("45%"), Some(0.45));
        assert_eq!(parse_number("100%"), Some(1.0));
        assert!(parse_number("%").is_none());
    }

    #[test]
    fn test_parse_number_unit_suffixes() {
        assert_eq!(parse_number("10k"), Some(10_000.0));
        assert_eq!(parse_number("2.5M"), Some(2_500_000.0));
        // df-style "100G" = 100 gigabytes; round-trips via format_number → "100G"
        assert_eq!(parse_number("100G"), Some(100_000_000_000.0));
    }

    #[test]
    fn test_parse_number_storage_double_units() {
        assert_eq!(parse_number("10GiB"), Some(10.0 * 1024.0 * 1024.0 * 1024.0));
        assert_eq!(parse_number("10GB"), Some(10_000_000_000.0));
        // decimal and binary stay distinct: 10GB ≠ 10GiB
        assert!(parse_number("10GB") != parse_number("10GiB"));
    }

    #[test]
    fn test_parse_number_accounting_parens() {
        assert_eq!(parse_number("(42)"), Some(-42.0));
        assert_eq!(parse_number("\"1,000\""), Some(1000.0));
    }

    #[test]
    fn test_parse_number_rejects_text_and_non_finite() {
        assert!(parse_number("Tokyo").is_none());
        assert!(parse_number("N/A").is_none());
        assert!(parse_number("").is_none());
        assert!(parse_number("NaN").is_none());
        assert!(parse_number("inf").is_none());
        assert!(parse_number("-inf").is_none());
    }

    #[test]
    fn test_min_max_normal() {
        assert_eq!(min_max(&[3.0, 1.0, 4.0, 1.5, 9.0]), Some((1.0, 9.0)));
    }

    #[test]
    fn test_min_max_single() {
        assert_eq!(min_max(&[42.0]), Some((42.0, 42.0)));
    }

    #[test]
    fn test_min_max_empty() {
        assert_eq!(min_max(&[]), None);
    }

    #[test]
    fn test_min_max_negative() {
        assert_eq!(min_max(&[-5.0, -1.0, -10.0]), Some((-10.0, -1.0)));
    }

    #[test]
    fn test_min_max_ignores_non_finite() {
        assert_eq!(min_max(&[1.0, f64::NAN, 3.0]), Some((1.0, 3.0)));
        assert_eq!(
            min_max(&[f64::INFINITY, 2.0, f64::NEG_INFINITY]),
            Some((2.0, 2.0))
        );
    }

    #[test]
    fn test_min_max_all_non_finite_returns_none() {
        assert_eq!(min_max(&[f64::NAN, f64::INFINITY]), None);
    }
}
