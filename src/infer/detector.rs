use crate::infer::types::DataType;

/// Maximum unique values for a column to be considered categorical.
const CATEGORICAL_THRESHOLD: usize = 20;

/// Number of rows sampled for type inference.
/// Shared by [`infer_column_type`] and the loaded-data fast path in `pipeline`.
pub(crate) const SAMPLE_SIZE: usize = 100;

/// Detect the data type of a single value string.
pub fn detect_value_type(value: &str) -> DataType {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return DataType::Nominal;
    }

    if is_temporal(trimmed) {
        return DataType::Temporal;
    }

    if is_quantitative(trimmed) {
        return DataType::Quantitative;
    }

    DataType::Nominal
}

/// Infer column type from a sample of values.
/// Returns the majority type among non-null values.
/// Empty and non-finite (`NaN`/`inf`) values are ignored in the vote,
/// consistent with downstream aggregation skipping them.
pub fn infer_column_type(values: &[&str]) -> DataType {
    if values.is_empty() {
        return DataType::Nominal;
    }

    let sample: Vec<&str> = values.iter().take(SAMPLE_SIZE).copied().collect();
    let non_empty: Vec<&str> = sample
        .iter()
        .filter(|v| !v.trim().is_empty() && !is_non_finite_number(v))
        .copied()
        .collect();

    if non_empty.is_empty() {
        return DataType::Nominal;
    }

    // Count type votes
    let mut temporal_count = 0usize;
    let mut quantitative_count = 0usize;

    for val in &non_empty {
        match detect_value_type(val) {
            DataType::Temporal => temporal_count += 1,
            DataType::Quantitative => quantitative_count += 1,
            _ => {}
        }
    }

    let total = non_empty.len();
    let threshold = (total * 80 / 100).max(1); // 80% majority needed, minimum 1

    if temporal_count >= threshold {
        return DataType::Temporal;
    }

    if quantitative_count >= threshold {
        return DataType::Quantitative;
    }

    // For predominantly nominal or mixed types, classify by cardinality
    classify_by_cardinality(&non_empty)
}

/// Classify as Categorical or Nominal based on unique value count.
fn classify_by_cardinality(values: &[&str]) -> DataType {
    let unique_count = unique_values(values);
    if unique_count <= CATEGORICAL_THRESHOLD {
        DataType::Categorical
    } else {
        DataType::Nominal
    }
}

fn unique_values(values: &[&str]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for v in values {
        seen.insert(*v);
    }
    seen.len()
}

fn is_temporal(value: &str) -> bool {
    use std::sync::LazyLock;

    static TEMPORAL_PATTERNS: LazyLock<[regex::Regex; 5]> = LazyLock::new(|| {
        [
            // YYYY-MM-DD (with optional time)
            regex::Regex::new(r"^\d{4}-\d{2}-\d{2}").expect("valid temporal regex"),
            // YYYY/MM/DD
            regex::Regex::new(r"^\d{4}/\d{2}/\d{2}").expect("valid temporal regex"),
            // MM/DD/YYYY
            regex::Regex::new(r"^\d{2}/\d{2}/\d{4}").expect("valid temporal regex"),
            // DD-Mon-YYYY
            regex::Regex::new(r"^\d{2}-[A-Za-z]{3}-\d{4}").expect("valid temporal regex"),
            // YYYY-MM (year-month only)
            regex::Regex::new(r"^\d{4}-\d{2}$").expect("valid temporal regex"),
        ]
    });

    TEMPORAL_PATTERNS.iter().any(|re| re.is_match(value))
}

fn is_quantitative(value: &str) -> bool {
    // Single numeric parser shared with every chart/filter/stats path:
    // "1,000", "$100", "45%", "10k", "10GiB" all vote Quantitative.
    // Non-finite values (NaN/inf) must not vote Quantitative — they are
    // skipped downstream. `detect_value_type` keeps them Nominal.
    crate::util::parse_number(value).is_some()
}

fn is_non_finite_number(value: &str) -> bool {
    // parse_number already returns None for NaN/inf, so detect them
    // directly here: strip formatting and check for a non-finite f64.
    let cleaned: String = value
        .chars()
        .filter(|c| *c != ',' && *c != ' ' && *c != '_' && *c != '\'')
        .collect();
    cleaned.parse::<f64>().is_ok_and(|v| !v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- detect_value_type tests ---

    #[test]
    fn test_detect_iso_date() {
        assert_eq!(detect_value_type("2024-01-15"), DataType::Temporal);
    }

    #[test]
    fn test_detect_iso_datetime() {
        assert_eq!(detect_value_type("2024-01-15T10:30:00"), DataType::Temporal);
    }

    #[test]
    fn test_detect_slash_date() {
        assert_eq!(detect_value_type("2024/01/15"), DataType::Temporal);
    }

    #[test]
    fn test_detect_us_date() {
        assert_eq!(detect_value_type("01/15/2024"), DataType::Temporal);
    }

    #[test]
    fn test_detect_integer() {
        assert_eq!(detect_value_type("42"), DataType::Quantitative);
    }

    #[test]
    fn test_detect_negative_float() {
        assert_eq!(detect_value_type("-3.14"), DataType::Quantitative);
    }

    #[test]
    fn test_detect_comma_number() {
        assert_eq!(detect_value_type("1,234,567"), DataType::Quantitative);
    }

    #[test]
    fn test_detect_text() {
        assert_eq!(detect_value_type("Tokyo"), DataType::Nominal);
    }

    #[test]
    fn test_detect_empty() {
        assert_eq!(detect_value_type(""), DataType::Nominal);
    }

    #[test]
    fn test_detect_whitespace_only() {
        assert_eq!(detect_value_type("   "), DataType::Nominal);
    }

    // --- infer_column_type tests ---

    #[test]
    fn test_infer_temporal_column() {
        let values = vec!["2024-01-01", "2024-02-01", "2024-03-01", "2024-04-01"];
        assert_eq!(infer_column_type(&values), DataType::Temporal);
    }

    #[test]
    fn test_infer_quantitative_column() {
        let values = vec!["10", "20.5", "30", "-5", "100"];
        assert_eq!(infer_column_type(&values), DataType::Quantitative);
    }

    #[test]
    fn test_infer_categorical_column() {
        let values = vec!["Tokyo", "Osaka", "Tokyo", "Nagoya", "Osaka", "Tokyo"];
        assert_eq!(infer_column_type(&values), DataType::Categorical);
    }

    #[test]
    fn test_infer_nominal_high_cardinality() {
        // More than 20 unique values
        let values: Vec<String> = (0..30).map(|i| format!("uuid-{}", i)).collect();
        let refs: Vec<&str> = values.iter().map(|s| s.as_str()).collect();
        assert_eq!(infer_column_type(&refs), DataType::Nominal);
    }

    #[test]
    fn test_infer_empty_column() {
        let values: Vec<&str> = vec![];
        assert_eq!(infer_column_type(&values), DataType::Nominal);
    }

    #[test]
    fn test_infer_with_nulls() {
        let values = vec!["2024-01-01", "", "2024-03-01", "", "2024-05-01"];
        assert_eq!(infer_column_type(&values), DataType::Temporal);
    }

    #[test]
    fn test_infer_mixed_but_majority_numeric() {
        let values = vec!["10", "20", "30", "40", "50", "N/A"];
        assert_eq!(infer_column_type(&values), DataType::Quantitative);
    }

    #[test]
    fn test_infer_single_numeric_value() {
        // A single numeric value must not be misclassified as Temporal
        let values = vec!["1"];
        assert_eq!(infer_column_type(&values), DataType::Quantitative);
    }

    #[test]
    fn test_infer_single_date_value() {
        let values = vec!["2024-01-01"];
        assert_eq!(infer_column_type(&values), DataType::Temporal);
    }

    #[test]
    fn test_classify_by_cardinality_categorical() {
        // Few unique values → Categorical
        let values = vec!["A", "B", "C", "A", "B", "C"];
        assert_eq!(classify_by_cardinality(&values), DataType::Categorical);
    }

    #[test]
    fn test_classify_by_cardinality_nominal() {
        // Many unique values → Nominal
        let values: Vec<&str> = (0..30)
            .map(|i| Box::leak(format!("val_{i}").into_boxed_str()) as &str)
            .collect();
        assert_eq!(classify_by_cardinality(&values), DataType::Nominal);
    }

    #[test]
    fn test_detect_year_month() {
        assert_eq!(detect_value_type("2024-01"), DataType::Temporal);
        assert_eq!(detect_value_type("2023-12"), DataType::Temporal);
        assert_eq!(detect_value_type("1999-06"), DataType::Temporal);
    }

    #[test]
    fn test_detect_year_month_single_digit_not_temporal() {
        // Single-digit month should not match YYYY-MM
        assert_eq!(detect_value_type("2024-1"), DataType::Nominal);
    }

    #[test]
    fn test_infer_year_month_column() {
        let values = vec!["2024-01", "2024-02", "2024-03", "2024-04", "2024-05"];
        assert_eq!(infer_column_type(&values), DataType::Temporal);
    }

    #[test]
    fn test_detect_percentage_string_is_quantitative_fraction() {
        // "45%" parses to 0.45 via the shared numeric parser — it is data,
        // not free text. (Breaking change: was Nominal before parse unification.)
        assert_eq!(detect_value_type("45%"), DataType::Quantitative);
        assert_eq!(detect_value_type("100%"), DataType::Quantitative);
        assert_eq!(detect_value_type("0.5%"), DataType::Quantitative);
    }

    #[test]
    fn test_detect_currency_string_is_quantitative() {
        // "$100" means 100 everywhere now (inference, charts, filters, stats).
        // (Breaking change: was Nominal before parse unification.)
        assert_eq!(detect_value_type("$100"), DataType::Quantitative);
        assert_eq!(detect_value_type("€50"), DataType::Quantitative);
        assert_eq!(detect_value_type("¥1000"), DataType::Quantitative);
        assert_eq!(detect_value_type("1,000"), DataType::Quantitative);
        assert_eq!(detect_value_type("10k"), DataType::Quantitative);
    }

    #[test]
    fn test_detect_non_finite_is_nominal() {
        // NaN/inf parse as f64 but must never vote Quantitative
        assert_eq!(detect_value_type("NaN"), DataType::Nominal);
        assert_eq!(detect_value_type("inf"), DataType::Nominal);
        assert_eq!(detect_value_type("-inf"), DataType::Nominal);
        assert_eq!(detect_value_type("Infinity"), DataType::Nominal);
    }

    #[test]
    fn test_infer_ignores_non_finite_in_vote() {
        // 2 finite of 2 voting values → Quantitative despite NaN/inf rows
        let values = vec!["100", "NaN", "inf", "200"];
        assert_eq!(infer_column_type(&values), DataType::Quantitative);
    }

    #[test]
    fn test_infer_all_non_finite_is_nominal() {
        let values = vec!["NaN", "inf", "-inf"];
        assert_eq!(infer_column_type(&values), DataType::Nominal);
    }
}
