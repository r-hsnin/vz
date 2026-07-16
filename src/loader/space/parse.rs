//! Parsing space-aligned content into LoadedData.

use anyhow::Result;

use super::Column;
use super::detect::detect_columns;
use super::is_separator_line;
use crate::loader::LoadedData;

/// Parse space-aligned content into LoadedData.
pub fn load_space(content: &str, no_header: bool) -> Result<LoadedData> {
    let lines: Vec<&str> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| !is_separator_line(l))
        .collect();

    if lines.is_empty() {
        return Ok(LoadedData {
            headers: vec![],
            rows: vec![],
        });
    }

    let (headers, data_lines) = if no_header || headers_are_numeric(lines[0]) {
        // Use first line to detect columns, but treat all lines as data
        let columns = detect_columns(lines[0]);
        let synthetic_headers: Vec<String> =
            (1..=columns.len()).map(|i| format!("col{}", i)).collect();
        let rows: Vec<Vec<String>> = lines
            .iter()
            .map(|line| extract_row(line, &columns))
            .collect();
        return Ok(LoadedData {
            headers: synthetic_headers,
            rows,
        });
    } else {
        let columns = detect_columns(lines[0]);
        let headers: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
        let rows: Vec<Vec<String>> = lines[1..]
            .iter()
            .map(|line| extract_row(line, &columns))
            .collect();
        (headers, rows)
    };

    Ok(LoadedData {
        headers,
        rows: data_lines,
    })
}

/// Check if a line is all-numeric tokens (indicating headerless data).
fn headers_are_numeric(line: &str) -> bool {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    !tokens.is_empty() && tokens.iter().all(|t| t.parse::<f64>().is_ok())
}

/// Extract cell values from a data row using pre-computed column boundaries.
///
/// Uses header-derived column positions as guides, but finds actual whitespace
/// gaps in the data row to split values correctly. This handles cases where
/// data values are slightly offset from header positions (e.g., right-aligned numbers).
fn extract_row(line: &str, columns: &[Column]) -> Vec<String> {
    if columns.is_empty() {
        return vec![];
    }

    let chars: Vec<char> = line.chars().collect();
    let line_len = chars.len();

    if line_len == 0 {
        return vec![String::new(); columns.len()];
    }

    // Find the actual split points in the data row.
    // For each column boundary (except the first), find the nearest space gap
    // in the data row around the expected position.
    let mut split_points: Vec<usize> = Vec::with_capacity(columns.len());
    split_points.push(columns[0].start.min(line_len));

    for col in columns.iter().skip(1) {
        let expected = col.start;
        // Search for a space gap near the expected position (±5 chars)
        let search_start = expected.saturating_sub(5).min(line_len);
        let search_end = (expected + 5).min(line_len);

        let split = find_gap_near(&chars, search_start, search_end, expected);
        split_points.push(split);
    }

    // Extract values between split points
    columns
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let start = split_points[i];
            let end = if i + 1 < split_points.len() {
                split_points[i + 1]
            } else {
                line_len
            };

            if start >= line_len {
                return String::new();
            }

            let actual_end = end.min(line_len);
            let value: String = chars[start..actual_end].iter().collect();
            value.trim().to_string()
        })
        .collect()
}

/// Find the best split point (start of a space gap) near an expected position.
/// Returns the position of the first space character in the gap closest to `expected`.
fn find_gap_near(chars: &[char], search_start: usize, search_end: usize, expected: usize) -> usize {
    let len = chars.len();

    // Look for runs of spaces in the search window
    // Find the gap whose start is closest to the expected position
    let mut best_gap_start = expected.min(len);
    let mut best_distance = usize::MAX;

    let mut i = search_start;
    while i < search_end && i < len {
        if chars[i] == ' ' {
            let gap_start = i;
            while i < len && chars[i] == ' ' {
                i += 1;
            }
            // This gap spans [gap_start..i)
            // The split point is the gap_start (value ends before this)
            // or i (next value starts here)
            // Use the END of the gap (where next column starts) as split point
            let dist = i.abs_diff(expected);
            if dist < best_distance {
                best_distance = dist;
                best_gap_start = i; // next column starts after the gap
            }
            // Also check gap_start-based distance
            let dist2 = gap_start.abs_diff(expected);
            if dist2 < best_distance {
                best_distance = dist2;
                best_gap_start = i; // still use end of gap as split
            }
        } else {
            i += 1;
        }
    }

    best_gap_start
}
