//! Error diagnostics: contextual hints for common errors.

use std::path::Path;

/// Supported data file extensions for discovery.
const DATA_EXTENSIONS: &[&str] = &["csv", "tsv", "json", "ndjson", "jsonl", "tab"];

/// Check if a filename has a recognized data extension.
pub fn is_data_file(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|ext| DATA_EXTENSIONS.contains(&ext))
}

/// Generate contextual hints for common errors.
pub fn error_hint(err: &anyhow::Error, file: Option<&Path>) -> Option<String> {
    let msg = format!("{:#}", err);
    // File not found: suggest similar files in the same directory
    if msg.contains("No such file")
        && let Some(file) = file
    {
        let parent = file.parent().unwrap_or(Path::new("."));
        let stem = file.file_name()?.to_str()?;
        let suggestions = find_similar_files(parent, stem);
        if !suggestions.is_empty() {
            let list = suggestions
                .iter()
                .map(|s| format!("    • {}", s))
                .collect::<Vec<_>>()
                .join("\n");
            return Some(format!("  Did you mean?\n{}", list));
        }
        return Some("  Tip: use vz - to read from stdin".to_string());
    }
    // Empty data
    if msg.contains("No data rows") {
        return Some(
            "  Tip: check that the file contains data rows below the header.\n  \
             For headerless data, try: vz file.csv --no-header"
                .to_string(),
        );
    }
    None
}

/// Find files in `dir` with names similar to `target`.
pub fn find_similar_files(dir: &Path, target: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let target_lower = target.to_lowercase();
    let all_data_files: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().ok().is_some_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| is_data_file(name))
        .collect();

    // First: find files similar to the target name
    let prefix: String = target_lower.chars().take(4).collect();
    let similar: Vec<String> = all_data_files
        .iter()
        .filter(|name| {
            let name_lower = name.to_lowercase();
            let shared = target_lower
                .chars()
                .zip(name_lower.chars())
                .take_while(|(a, b)| a == b)
                .count();
            shared >= 3 || (!prefix.is_empty() && name_lower.contains(&prefix))
        })
        .take(3)
        .cloned()
        .collect();

    if !similar.is_empty() {
        return similar;
    }
    // Fallback: show any data files in the directory
    all_data_files.into_iter().take(3).collect()
}

/// Suggest the closest column name for a typo'd column reference.
///
/// Prefers a case-insensitive match first (headers are case-sensitive, so
/// `Revenue` vs `revenue` deserves a hint), then falls back to an edit
/// distance of at most 2 on lowercased names. Returns `None` for exact
/// matches, empty inputs, or distant names.
pub fn suggest_column(available: &[String], target: &str) -> Option<String> {
    if target.is_empty() || available.is_empty() {
        return None;
    }
    if available.iter().any(|c| c == target) {
        return None;
    }
    if let Some(case_match) = available.iter().find(|c| c.eq_ignore_ascii_case(target)) {
        return Some(case_match.clone());
    }
    let target_lower = target.to_lowercase();
    let mut best: Option<(usize, usize, &String)> = None;
    for cand in available {
        let dist = levenshtein(&target_lower, &cand.to_lowercase());
        if dist > 2 {
            continue;
        }
        let key = (dist, cand.chars().count());
        if best.is_none_or(|(d, n, _)| key < (d, n)) {
            best = Some((dist, cand.chars().count(), cand));
        }
    }
    best.map(|(_, _, cand)| cand.clone())
}

/// Format the `Did you mean ...?` suffix for column-not-found errors.
/// Appends a case-sensitivity note when the target only differs by case.
pub fn format_column_suffix(suggestion: Option<&str>, target: &str) -> String {
    match suggestion {
        Some(s) if s.eq_ignore_ascii_case(target) && s != target => {
            format!(" Did you mean '{s}'? Note: column names are case-sensitive.")
        }
        Some(s) => format!(" Did you mean '{s}'?"),
        None => String::new(),
    }
}

/// Edit distance over chars (self-contained; avoids a new dependency).
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut curr = vec![i + 1];
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr.push((prev[j] + cost).min((curr[j] + 1).min(prev[j + 1] + 1)));
        }
        prev = curr;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_data_file() {
        assert!(is_data_file("sales.csv"));
        assert!(is_data_file("data.tsv"));
        assert!(is_data_file("records.json"));
        assert!(is_data_file("stream.ndjson"));
        assert!(is_data_file("stream.jsonl"));
        assert!(!is_data_file("readme.md"));
        assert!(!is_data_file("image.png"));
        assert!(!is_data_file("noext"));
    }

    #[test]
    fn test_find_similar_files_returns_empty_for_nonexistent_dir() {
        let result = find_similar_files(Path::new("/nonexistent_dir_xyz"), "test.csv");
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_similar_files_finds_fixtures() {
        let result = find_similar_files(Path::new("fixtures"), "sales");
        assert!(!result.is_empty());
        assert!(result.iter().any(|f| f.contains("sales")));
    }

    #[test]
    fn test_find_similar_files_non_ascii_target_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("日本語データ.csv"), "a,b\n1,2\n").unwrap();
        // Shared prefix < 3 chars forces evaluation of the 4-byte prefix slice,
        // which panics on a byte-wise cut inside a multibyte char.
        let result = find_similar_files(dir.path(), "zz日本語.csv");
        assert_eq!(result, vec!["日本語データ.csv".to_string()]);
    }

    #[test]
    fn test_find_similar_files_emoji_target_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data.csv"), "a\n1\n").unwrap();
        // "a📊.csv"[..4] cuts inside the emoji on byte-wise slicing.
        let result = find_similar_files(dir.path(), "a📊.csv");
        assert_eq!(result, vec!["data.csv".to_string()]);
    }

    #[test]
    fn test_find_similar_files_empty_target_matches_nothing_similar() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data.csv"), "a\n1\n").unwrap();
        // Empty target must not vacuously match; only the fallback applies.
        let result = find_similar_files(dir.path(), "");
        assert_eq!(result, vec!["data.csv".to_string()]);
    }

    #[test]
    fn test_suggest_column_exact_match() {
        let available = ["date".to_string(), "city".to_string()];
        assert_eq!(suggest_column(&available, "date"), None);
        assert_eq!(suggest_column(&available, ""), None);
        assert_eq!(suggest_column(&[], "date"), None);
    }

    #[test]
    fn test_suggest_column_prefers_case_match() {
        let available = ["date".to_string(), "revenue".to_string()];
        assert_eq!(
            suggest_column(&available, "Revenue"),
            Some("revenue".to_string())
        );
    }

    #[test]
    fn test_suggest_column_typo_within_distance_two() {
        let available = ["date".to_string(), "revenue".to_string()];
        assert_eq!(
            suggest_column(&available, "revnue"),
            Some("revenue".to_string())
        );
        assert_eq!(suggest_column(&available, "zzzz"), None);
    }

    #[test]
    fn test_format_column_suffix_marks_case_sensitivity() {
        assert_eq!(
            format_column_suffix(Some("revenue"), "Revenue"),
            " Did you mean 'revenue'? Note: column names are case-sensitive."
        );
        assert_eq!(
            format_column_suffix(Some("revenue"), "revnue"),
            " Did you mean 'revenue'?"
        );
        assert_eq!(format_column_suffix(None, "zzzz"), "");
    }

    #[test]
    fn test_error_hint_no_such_file_with_similar() {
        let err = anyhow::anyhow!("No such file or directory: fixtures/sale.csv");
        let hint = error_hint(&err, Some(Path::new("fixtures/sale.csv")));
        let hint = hint.expect("expected a hint for a mistyped filename");
        assert!(hint.contains("Did you mean?"), "got: {hint}");
        assert!(hint.contains("sales.csv"), "got: {hint}");
    }

    #[test]
    fn test_error_hint_no_such_file_without_path_returns_none() {
        let err = anyhow::anyhow!("No such file or directory: fixtures/sale.csv");
        assert_eq!(error_hint(&err, None), None);
    }

    #[test]
    fn test_error_hint_no_data_rows_needs_no_path() {
        let err = anyhow::anyhow!("No data rows found");
        let hint = error_hint(&err, None).expect("expected a hint for empty data");
        assert!(hint.contains("--no-header"), "got: {hint}");
    }

    #[test]
    fn test_error_hint_unrelated_returns_none() {
        let err = anyhow::anyhow!("something else went wrong");
        assert_eq!(error_hint(&err, None), None);
    }
}
