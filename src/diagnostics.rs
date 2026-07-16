//! Error diagnostics: contextual hints for common errors.

use std::path::Path;

use crate::cli::Cli;

/// Supported data file extensions for discovery.
const DATA_EXTENSIONS: &[&str] = &["csv", "tsv", "json", "ndjson", "jsonl", "tab"];

/// Check if a filename has a recognized data extension.
pub fn is_data_file(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|ext| DATA_EXTENSIONS.contains(&ext))
}

/// Generate contextual hints for common errors.
pub fn error_hint(err: &anyhow::Error, cli: &Cli) -> Option<String> {
    let msg = format!("{:#}", err);
    // File not found: suggest similar files in the same directory
    if msg.contains("No such file")
        && let Some(file) = cli.primary_file()
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
    let similar: Vec<String> = all_data_files
        .iter()
        .filter(|name| {
            let name_lower = name.to_lowercase();
            let shared = target_lower
                .chars()
                .zip(name_lower.chars())
                .take_while(|(a, b)| a == b)
                .count();
            shared >= 3 || name_lower.contains(&target_lower[..target_lower.len().min(4)])
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
}
