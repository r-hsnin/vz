//! Fixed-width / space-aligned format parser.
//!
//! Determines column boundaries from the header line:
//! tokens separated by 2+ consecutive spaces define column start positions.

use anyhow::Result;

use super::LoadedData;

/// Column boundary definition.
#[derive(Debug, Clone)]
struct Column {
    start: usize,
    name: String,
}

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

/// Detect whether a line is a separator (e.g., "---", "===", "─────").
fn is_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .chars()
        .all(|c| matches!(c, '-' | '=' | '─' | '┼' | '+' | '|' | ' '))
}

/// Detect column boundaries from a header line.
///
/// Two strategies:
/// 1. If the header has 2+ consecutive space gaps → use those as column boundaries
///    (handles multi-word headers like "Mounted on" which have only 1 space internally)
/// 2. If no 2+ space gaps exist (or the wide-gap approach produces too few columns
///    relative to the number of tokens) → each whitespace-separated token is a column
///    (handles lsblk, ps aux, etc.)
fn detect_columns(header_line: &str) -> Vec<Column> {
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Detection Tests ---

    #[test]
    fn test_looks_like_space_format_kubectl_top() {
        let content = "NAME        CPU(cores)   MEMORY(bytes)\npod1        100m         256Mi\n";
        assert!(looks_like_space_format(content));
    }

    #[test]
    fn test_looks_like_space_format_not_csv() {
        let content = "name,cpu,mem\npod1,100m,256Mi\n";
        assert!(!looks_like_space_format(content));
    }

    #[test]
    fn test_looks_like_space_format_not_tsv() {
        let content = "name\tcpu\tmem\npod1\t100m\t256Mi\n";
        assert!(!looks_like_space_format(content));
    }

    #[test]
    fn test_looks_like_space_format_single_line() {
        let content = "NAME        CPU(cores)   MEMORY(bytes)\n";
        assert!(!looks_like_space_format(content));
    }

    #[test]
    fn test_looks_like_space_format_with_separators() {
        let content = "Name        Score\n----------  -----\nAlice       95\n";
        assert!(looks_like_space_format(content));
    }

    // --- Column Detection Tests ---

    #[test]
    fn test_detect_columns_simple() {
        let cols = detect_columns("NAME        CPU(cores)   MEMORY(bytes)");
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].name, "NAME");
        assert_eq!(cols[1].name, "CPU(cores)");
        assert_eq!(cols[2].name, "MEMORY(bytes)");
    }

    #[test]
    fn test_detect_columns_multi_word_header() {
        // "Mounted on" has only 1 space — should be one column (wide-gap strategy)
        // Header: 7 tokens, wide-gap gives 6 columns (6+2=8 >= 7) → uses wide-gap
        let cols = detect_columns("Filesystem      Size  Used  Avail  Use%  Mounted on");
        assert_eq!(cols.len(), 6);
        assert_eq!(cols[0].name, "Filesystem");
        assert_eq!(cols[5].name, "Mounted on");
    }

    #[test]
    fn test_detect_columns_varying_gaps() {
        // lsblk: 7 tokens, wide-gap gives 3 columns (3+2=5 < 7) → falls back to single-space
        let cols = detect_columns("NAME   MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS");
        assert_eq!(cols.len(), 7);
        assert_eq!(cols[0].name, "NAME");
        assert_eq!(cols[1].name, "MAJ:MIN");
        assert_eq!(cols[6].name, "MOUNTPOINTS");
    }

    // --- Parsing Tests ---

    #[test]
    fn test_load_space_kubectl_top() {
        let content = "\
NAME                                    CPU(cores)   MEMORY(bytes)
frontend-deploy-7b4c9f8d6-abc12        100m         256Mi
backend-deploy-5d6e7f8a9-def34         200m         512Mi
redis-master-0                          50m          128Mi
";
        let data = load_space(content, false).unwrap();
        assert_eq!(data.headers, vec!["NAME", "CPU(cores)", "MEMORY(bytes)"]);
        assert_eq!(data.rows.len(), 3);
        assert_eq!(data.rows[0][0], "frontend-deploy-7b4c9f8d6-abc12");
        assert_eq!(data.rows[0][1], "100m");
        assert_eq!(data.rows[0][2], "256Mi");
        assert_eq!(data.rows[2][0], "redis-master-0");
        assert_eq!(data.rows[2][1], "50m");
    }

    #[test]
    fn test_load_space_separator_lines_skipped() {
        let content = "\
Name        Score   Grade
----------  ------  -----
Alice       95      A
Bob         82      B
Charlie     71      C
";
        let data = load_space(content, false).unwrap();
        assert_eq!(data.headers, vec!["Name", "Score", "Grade"]);
        assert_eq!(data.rows.len(), 3);
        assert_eq!(data.rows[0], vec!["Alice", "95", "A"]);
    }

    #[test]
    fn test_load_space_empty_trailing_values() {
        let content = "\
NAME        STATUS    ERROR
service-a   Running
service-b   Failed    timeout
service-c   Running
";
        let data = load_space(content, false).unwrap();
        assert_eq!(data.headers, vec!["NAME", "STATUS", "ERROR"]);
        assert_eq!(data.rows.len(), 3);
        assert_eq!(data.rows[0][0], "service-a");
        assert_eq!(data.rows[0][1], "Running");
        assert_eq!(data.rows[0][2], "");
        assert_eq!(data.rows[1][2], "timeout");
    }

    #[test]
    fn test_load_space_single_row() {
        let content = "\
NAME         CPU    MEM
my-pod       50m    128Mi
";
        let data = load_space(content, false).unwrap();
        assert_eq!(data.headers, vec!["NAME", "CPU", "MEM"]);
        assert_eq!(data.rows.len(), 1);
        assert_eq!(data.rows[0], vec!["my-pod", "50m", "128Mi"]);
    }

    #[test]
    fn test_load_space_no_header() {
        let content = "\
100   200   300
150   250   350
200   300   400
";
        let data = load_space(content, true).unwrap();
        assert_eq!(data.headers, vec!["col1", "col2", "col3"]);
        assert_eq!(data.rows.len(), 3);
        assert_eq!(data.rows[0], vec!["100", "200", "300"]);
    }

    #[test]
    fn test_load_space_auto_detect_headerless() {
        // All-numeric first row: auto-treat as headerless
        let content = "\
100   200   300
150   250   350
";
        let data = load_space(content, false).unwrap();
        assert_eq!(data.headers, vec!["col1", "col2", "col3"]);
        assert_eq!(data.rows.len(), 2);
    }

    #[test]
    fn test_load_space_lsblk() {
        let content = "\
NAME   MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS
sda      8:0    0 476.9G  0 disk
sda1     8:1    0   512M  0 part /boot/efi
sda2     8:2    0 476.4G  0 part /
";
        let data = load_space(content, false).unwrap();
        assert_eq!(data.headers.len(), 7);
        assert_eq!(data.headers[0], "NAME");
        assert_eq!(data.headers[1], "MAJ:MIN");
        assert_eq!(data.headers[2], "RM");
        assert_eq!(data.headers[5], "TYPE");
        assert_eq!(data.headers[6], "MOUNTPOINTS");
        assert_eq!(data.rows.len(), 3);
        // With single-space fallback, each token is a column
        assert_eq!(data.rows[0][0], "sda");
        assert_eq!(data.rows[0][5], "disk");
        assert_eq!(data.rows[0][6], ""); // no mountpoint for disk
        assert_eq!(data.rows[2][6], "/");
    }

    #[test]
    fn test_load_space_df_h() {
        let content = "\
Filesystem      Size  Used  Avail  Use%  Mounted on
/dev/sda1        50G   35G    15G   70%  /
tmpfs           7.8G     0   7.8G    0%  /dev/shm
";
        let data = load_space(content, false).unwrap();
        assert_eq!(data.headers.len(), 6);
        assert_eq!(data.headers[0], "Filesystem");
        assert_eq!(data.headers[5], "Mounted on");
        assert_eq!(data.rows.len(), 2);
        assert_eq!(data.rows[0][0], "/dev/sda1");
        assert_eq!(data.rows[0][4], "70%");
        assert_eq!(data.rows[0][5], "/");
        assert_eq!(data.rows[1][5], "/dev/shm");
    }

    #[test]
    fn test_load_space_ps_aux_last_column_has_spaces() {
        // With single-space tokenization fallback, each whitespace-separated
        // token becomes a column. For ps aux, the COMMAND column with spaces
        // gets split into multiple columns.
        // The primary use case (kubectl top) handles this correctly via wide-gap.
        // For ps aux, the user can use -f space with custom column selection.
        let content = "\
USER       PID  %CPU  %MEM  COMMAND
root         1   0.0   0.1  /sbin/init
www-data  1234   1.5   2.3  nginx
";
        let data = load_space(content, false).unwrap();
        assert_eq!(data.headers[0], "USER");
        assert_eq!(data.headers[4], "COMMAND");
        assert_eq!(data.rows.len(), 2);
        assert_eq!(data.rows[0][0], "root");
        assert_eq!(data.rows[0][4], "/sbin/init");
        assert_eq!(data.rows[1][4], "nginx");
    }

    #[test]
    fn test_load_space_empty_content() {
        let data = load_space("", false).unwrap();
        assert_eq!(data.headers.len(), 0);
        assert_eq!(data.rows.len(), 0);
    }

    #[test]
    fn test_load_space_trims_whitespace() {
        let content = "\
NAME        VALUE
foo         bar
baz         qux
";
        let data = load_space(content, false).unwrap();
        assert_eq!(data.rows[0][0], "foo");
        assert_eq!(data.rows[0][1], "bar");
    }

    // --- Separator Line Detection ---

    #[test]
    fn test_is_separator_line_dashes() {
        assert!(is_separator_line("----------  ------  -----"));
    }

    #[test]
    fn test_is_separator_line_equals() {
        assert!(is_separator_line("==========  ======  ====="));
    }

    #[test]
    fn test_is_separator_line_mixed() {
        assert!(is_separator_line("---+---+---"));
    }

    #[test]
    fn test_is_separator_line_not_data() {
        assert!(!is_separator_line("Alice       95      A"));
    }

    #[test]
    fn test_is_separator_line_empty() {
        assert!(!is_separator_line(""));
    }
}
