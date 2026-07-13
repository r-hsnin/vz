use crate::infer::types::DataType;

/// Maximum unique values for a column to be considered categorical.
const CATEGORICAL_THRESHOLD: usize = 20;

/// Number of rows to sample for type inference.
const SAMPLE_SIZE: usize = 100;

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
pub fn infer_column_type(values: &[&str]) -> DataType {
    if values.is_empty() {
        return DataType::Nominal;
    }

    let sample: Vec<&str> = values.iter().take(SAMPLE_SIZE).copied().collect();
    let non_empty: Vec<&str> = sample
        .iter()
        .filter(|v| !v.trim().is_empty())
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

    static TEMPORAL_PATTERNS: LazyLock<[regex::Regex; 4]> = LazyLock::new(|| {
        [
            // YYYY-MM-DD (with optional time)
            regex::Regex::new(r"^\d{4}-\d{2}-\d{2}").expect("valid temporal regex"),
            // YYYY/MM/DD
            regex::Regex::new(r"^\d{4}/\d{2}/\d{2}").expect("valid temporal regex"),
            // MM/DD/YYYY
            regex::Regex::new(r"^\d{2}/\d{2}/\d{4}").expect("valid temporal regex"),
            // DD-Mon-YYYY
            regex::Regex::new(r"^\d{2}-[A-Za-z]{3}-\d{4}").expect("valid temporal regex"),
        ]
    });

    TEMPORAL_PATTERNS.iter().any(|re| re.is_match(value))
}

fn is_quantitative(value: &str) -> bool {
    // Strip common number formatting
    let cleaned: String = value.chars().filter(|c| *c != ',' && *c != ' ').collect();
    cleaned.parse::<f64>().is_ok()
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
    fn test_detect_percentage_string_is_nominal() {
        // "45%" contains non-numeric char '%', should NOT be quantitative
        assert_eq!(detect_value_type("45%"), DataType::Nominal);
        assert_eq!(detect_value_type("100%"), DataType::Nominal);
        assert_eq!(detect_value_type("0.5%"), DataType::Nominal);
    }

    #[test]
    fn test_detect_currency_string_is_nominal() {
        // "$100" and "€50" contain currency symbols, should NOT be quantitative
        assert_eq!(detect_value_type("$100"), DataType::Nominal);
        assert_eq!(detect_value_type("€50"), DataType::Nominal);
        assert_eq!(detect_value_type("¥1000"), DataType::Nominal);
    }
}
