//! Format detection and column boundary detection.

use super::Column;
use super::is_separator_line;

/// Detect whether content looks like space-aligned fixed-width data.
///
/// Returns true if:
/// - No tabs in first line
/// - No commas in first line
/// - 2+ data lines (header + at least 1 data row)
/// - First line has 2+ tokens separated by spaces
pub fn looks_like_space_format(content: &str) -> bool {
    let lines: Vec<&str> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| !is_separator_line(l))
        .collect();

    if lines.len() < 2 {
        return false;
    }

    let first_line = lines[0];

    // Must not contain tabs (would be TSV)
    if first_line.contains('\t') {
        return false;
    }

    // Must not contain commas (would be CSV)
    if first_line.contains(',') {
        return false;
    }

    // Must have 2+ tokens separated by spaces
    let columns = detect_columns(first_line);
    if columns.len() < 2 {
        return false;
    }

    // Validate: check that data rows also split into a similar number of columns
    // This avoids false-positives on prose text
    if let Some(data_line) = lines.get(1) {
        let data_tokens: Vec<&str> = data_line.split_whitespace().collect();
        // Data should have at least 2 fields
        if data_tokens.len() < 2 {
            return false;
        }
    }

    true
}

/// Detect column boundaries from a header line.
///
/// Two strategies:
/// 1. If the header has 2+ consecutive space gaps → use those as column boundaries
///    (handles multi-word headers like "Mounted on" which have only 1 space internally)
/// 2. If no 2+ space gaps exist (or the wide-gap approach produces too few columns
///    relative to the number of tokens) → each whitespace-separated token is a column
///    (handles lsblk, ps aux, etc.)
pub(crate) fn detect_columns(header_line: &str) -> Vec<Column> {
    let chars: Vec<char> = header_line.chars().collect();
    let len = chars.len();

    if len == 0 {
        return Vec::new();
    }

    // Count whitespace-separated tokens
    let token_count = header_line.split_whitespace().count();

    // Check if header has any 2+ space gaps
    let has_wide_gaps = has_two_plus_space_gaps(&chars);

    if has_wide_gaps {
        let mut columns = Vec::new();
        detect_columns_wide_gaps(&chars, &mut columns);

        // Use wide-gap columns if they capture most tokens.
        // Allow at most 2 fewer columns than tokens (for multi-word names like "Mounted on").
        // If wide-gap produces significantly fewer, the header likely has many columns
        // with inconsistent spacing — fall back to single-space.
        if columns.len() + 2 >= token_count {
            return columns;
        }
        // Fall through to single-space strategy
    }

    // Strategy 2: Each whitespace-separated token is a column
    let mut columns = Vec::new();
    detect_columns_single_space(&chars, &mut columns);
    columns
}

/// Check if a char sequence contains at least one gap of 2+ consecutive spaces
/// between non-space characters.
fn has_two_plus_space_gaps(chars: &[char]) -> bool {
    let mut i = 0;
    let len = chars.len();

    // Skip leading spaces
    while i < len && chars[i] == ' ' {
        i += 1;
    }

    loop {
        // Skip non-spaces (token)
        while i < len && chars[i] != ' ' {
            i += 1;
        }
        if i >= len {
            return false;
        }
        // Count spaces
        let space_start = i;
        while i < len && chars[i] == ' ' {
            i += 1;
        }
        if i >= len {
            return false;
        }
        if i - space_start >= 2 {
            return true;
        }
    }
}

/// Detect columns using 2+ space gaps as boundaries.
/// Single spaces within a header name are preserved (e.g., "Mounted on").
fn detect_columns_wide_gaps(chars: &[char], columns: &mut Vec<Column>) {
    let len = chars.len();
    let mut i = 0;

    // Skip leading spaces
    while i < len && chars[i] == ' ' {
        i += 1;
    }

    if i >= len {
        return;
    }

    let mut token_start = i;

    loop {
        // Find end of current token (next space)
        while i < len && chars[i] != ' ' {
            i += 1;
        }

        if i >= len {
            // Token extends to end of line
            let name: String = chars[token_start..i].iter().collect();
            columns.push(Column {
                start: token_start,
                name: name.trim().to_string(),
            });
            break;
        }

        // Count consecutive spaces
        let space_start = i;
        while i < len && chars[i] == ' ' {
            i += 1;
        }
        let space_count = i - space_start;

        if i >= len {
            // Token followed by trailing spaces
            let name: String = chars[token_start..space_start].iter().collect();
            columns.push(Column {
                start: token_start,
                name: name.trim().to_string(),
            });
            break;
        }

        if space_count >= 2 {
            // 2+ spaces = column boundary
            let name: String = chars[token_start..space_start].iter().collect();
            columns.push(Column {
                start: token_start,
                name: name.trim().to_string(),
            });
            token_start = i;
        }
        // else: single space within a column name (e.g., "Mounted on")
    }
}

/// Detect columns by treating each whitespace-separated token as a column.
/// Used when the header has only single-space gaps.
fn detect_columns_single_space(chars: &[char], columns: &mut Vec<Column>) {
    let len = chars.len();
    let mut i = 0;

    loop {
        // Skip spaces
        while i < len && chars[i] == ' ' {
            i += 1;
        }
        if i >= len {
            break;
        }

        let token_start = i;

        // Find end of token
        while i < len && chars[i] != ' ' {
            i += 1;
        }

        let name: String = chars[token_start..i].iter().collect();
        columns.push(Column {
            start: token_start,
            name,
        });
    }
}
