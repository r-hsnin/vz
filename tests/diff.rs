//! Diff mode end-to-end tests for vz.

#[path = "common/mod.rs"]
mod common;
use common::{temp_csv, vz_binary};

#[test]
fn test_diff_insights_names_biggest_mover() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("💡 Biggest change: Tokyo grew from 1k to 1.2k (+20%)."),
        "expected diff insight, got: {}",
        stderr
    );
    assert!(
        stderr.contains("💡 2 improved, 1 declined."),
        "expected tally, got: {}",
        stderr
    );
}

#[test]
fn test_diff_json_has_insights() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let insights = v["insights"].as_array().expect("insights must be an array");
    assert!(
        insights
            .iter()
            .any(|s| s.as_str().unwrap_or("").contains("Tokyo grew")),
        "expected insight in diff JSON, got: {}",
        v["insights"]
    );
}

#[test]
fn test_diff_two_positional_files_bar() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Diff"),
        "Missing Diff header in: {}",
        stdout
    );
    assert!(stdout.contains("▲"), "Missing ▲ marker in: {}", stdout);
    assert!(stdout.contains("▼"), "Missing ▼ marker in: {}", stdout);
}

#[test]
fn test_diff_flag_syntax() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "--diff",
            "fixtures/diff/sales_after.csv",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("Diff"), "Missing Diff header");
    assert!(stdout.contains("▲"), "Missing ▲ marker");
}

#[test]
fn test_diff_spark_output() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--spark",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Δ"),
        "Missing Δ prefix in spark: {}",
        stdout
    );
    assert!(
        stdout.contains("revenue"),
        "Missing y column name: {}",
        stdout
    );
}

#[test]
fn test_diff_json_output() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--json",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    assert_eq!(json["mode"], "diff");
    assert_eq!(json["version"], 1);
    assert!(json["categories"].as_array().unwrap().len() == 4);
}

#[test]
fn test_diff_schema_mismatch_error() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/schema_mismatch.csv",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Schema mismatch")
            || String::from_utf8_lossy(&output.stdout).contains("Schema mismatch"),
        "Expected schema mismatch error, got: stderr={}, stdout={}",
        stderr,
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_diff_identical_files() {
    let output = vz_binary()
        .args(["fixtures/diff/identical.csv", "fixtures/diff/identical.csv"])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("0%"),
        "Expected 0% for identical files: {}",
        stdout
    );
}

#[test]
fn test_diff_timeseries() {
    let output = vz_binary()
        .args([
            "fixtures/diff/timeseries_before.csv",
            "fixtures/diff/timeseries_after.csv",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {}", stderr);
    // Temporal diff produces line overlay with "vs" title
    assert!(
        stderr.contains("Line") || stderr.contains("Diff"),
        "Expected Line or Diff chart for temporal diff, got stderr: '{}'",
        stderr
    );
    assert!(
        stderr.contains("date") || stdout.contains("date"),
        "Missing date column reference in output"
    );
}

#[test]
fn test_diff_with_x_y_override() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "-x",
            "city",
            "-y",
            "revenue",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("x=city"), "Missing x=city in: {}", stdout);
    assert!(
        stdout.contains("y=revenue"),
        "Missing y=revenue in: {}",
        stdout
    );
}

#[test]
fn test_diff_with_sort_desc() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--sort",
            "desc",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // First entry should be highest delta (Tokyo +200)
    let lines: Vec<&str> = stdout.lines().collect();
    let data_lines: Vec<&&str> = lines
        .iter()
        .filter(|l| l.contains("▲") || l.contains("▼") || l.contains("─"))
        .collect();
    assert!(!data_lines.is_empty());
    assert!(
        data_lines[0].contains("Tokyo"),
        "Highest delta should be first: {}",
        data_lines[0]
    );
}

#[test]
fn test_diff_with_top_limit() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--top",
            "2",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Only 2 entries should be shown (plus summary line)
    let data_lines: Vec<&str> = stdout.lines().filter(|l| l.starts_with("  ")).collect();
    assert_eq!(
        data_lines.len(),
        2,
        "Expected 2 entries with --top 2, got: {:?}",
        data_lines
    );
}

#[test]
fn test_diff_nonexistent_file_error() {
    let output = vz_binary()
        .args(["fixtures/diff/sales_before.csv", "nonexistent.csv"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
}

// --- Temporal diff tests ---

#[test]
fn test_diff_temporal_renders_line_chart() {
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
        ])
        .output()
        .expect("Failed to run vz");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", stderr);
    // Summary line should be on stderr with "Line" prefix
    assert!(
        stderr.contains("Line"),
        "Missing Line in summary: {}",
        stderr
    );
    assert!(
        stderr.contains("x=date"),
        "Missing x=date in summary: {}",
        stderr
    );
    // Should NOT contain bar chart markers
    assert!(
        !stdout.contains("▲"),
        "Should not contain ▲ (bar diff marker): {}",
        stdout
    );
    assert!(
        !stdout.contains("▼"),
        "Should not contain ▼ (bar diff marker): {}",
        stdout
    );
}

#[test]
fn test_diff_temporal_summary_format() {
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
        ])
        .output()
        .expect("Failed to run vz");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success());
    // Verify summary format: Line │ x=date │ ... vs ... │ Δ ... │ ... rows
    assert!(stderr.contains("Line │ x=date │ ts_daily_before vs ts_daily_after │ Δ +13% │ 6 rows"));
}

#[test]
fn test_diff_temporal_spark_output() {
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
            "--spark",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("ts_daily_before"),
        "Missing before name in spark: {}",
        stdout
    );
    assert!(
        stdout.contains("ts_daily_after"),
        "Missing after name in spark: {}",
        stdout
    );
}

#[test]
fn test_diff_temporal_json_output() {
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
            "--json",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    assert_eq!(json["mode"], "diff");
    assert_eq!(json["chart_type"], "line");
    assert_eq!(json["x_column"], "date");
    assert_eq!(json["y_column"], "revenue");
    assert!(json["dates"].is_array());
    assert!(json["before"]["series"].is_array());
    assert!(json["after"]["series"].is_array());
}

#[test]
fn test_diff_temporal_with_width_height() {
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
            "-W",
            "100",
            "-H",
            "30",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_diff_categorical_still_uses_bar() {
    // Regression guard: categorical X should still produce bar diff
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(
        stdout.contains("Diff"),
        "Should contain Diff for categorical: {}",
        stdout
    );
    assert!(stdout.contains("▲"), "Should contain ▲ for categorical");
}

// --- Diff Markdown output tests ---

#[test]
fn test_diff_markdown_output() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "-o",
            "markdown",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("|---|"),
        "Missing table separator in: {}",
        stdout
    );
    assert!(stdout.contains("| city |"), "Missing header in: {}", stdout);
    assert!(stdout.contains("Tokyo"), "Missing Tokyo in: {}", stdout);
    assert!(stdout.contains("Osaka"), "Missing Osaka in: {}", stdout);
    assert!(stdout.contains("▲"), "Missing ▲ marker in: {}", stdout);
    assert!(stdout.contains("▼"), "Missing ▼ marker in: {}", stdout);
}

#[test]
fn test_diff_markdown_shorthand() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--markdown",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("|---|"),
        "Missing table separator in: {}",
        stdout
    );
    assert!(stdout.contains("| city |"), "Missing header in: {}", stdout);
}

#[test]
fn test_diff_markdown_temporal() {
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
            "--markdown",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("|---|"),
        "Missing table separator in: {}",
        stdout
    );
    assert!(
        stdout.contains("| date |"),
        "Missing date header in: {}",
        stdout
    );
    assert!(
        stdout.contains("2024-01-01"),
        "Missing first date in: {}",
        stdout
    );
    assert!(
        stdout.contains("2024-01-06"),
        "Missing last date in: {}",
        stdout
    );
}

#[test]
fn test_diff_markdown_with_sort_and_top() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--markdown",
            "--sort",
            "desc",
            "--top",
            "2",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Header + separator + 2 data rows (+ optional overall line)
    let data_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("| ") && !l.contains("city") && !l.starts_with("|---"))
        .collect();
    assert_eq!(
        data_lines.len(),
        2,
        "Expected 2 data rows with --top 2, got: {:?}",
        data_lines
    );
}

#[test]
fn test_diff_markdown_temporal_zero_before_uses_new_marker() {
    let before = temp_csv(&[
        "date,revenue",
        "2024-01-01,0",
        "2024-01-02,5",
        "2024-01-03,10",
    ]);
    let after = temp_csv(&[
        "date,revenue",
        "2024-01-01,10",
        "2024-01-02,6",
        "2024-01-03,11",
    ]);
    let output = vz_binary()
        .args([
            before.path().to_str().unwrap(),
            after.path().to_str().unwrap(),
            "--markdown",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("▲ new"),
        "Zero-before rows should use the canonical new marker: {}",
        stdout
    );
    assert!(
        stdout.contains("▲ +20%"),
        "Non-zero before rows should keep percentage deltas: {}",
        stdout
    );
}

// --- HTML output tests ---

#[test]
fn test_explore_diff_two_files_does_not_panic() {
    let output = vz_binary()
        .args([
            "explore",
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
        ])
        .env("VZ_TEST_HEADLESS", "1")
        .output()
        .expect("Failed to run vz");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("panicked"),
        "Should not panic: {}",
        combined
    );
    assert!(output.status.success());
}

#[test]
fn test_explore_diff_temporal_does_not_panic() {
    let output = vz_binary()
        .args([
            "explore",
            "fixtures/diff/timeseries_before.csv",
            "fixtures/diff/timeseries_after.csv",
        ])
        .env("VZ_TEST_HEADLESS", "1")
        .output()
        .expect("Failed to run vz");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("panicked"),
        "Should not panic: {}",
        combined
    );
    assert!(output.status.success());
}

#[test]
fn test_explore_diff_schema_mismatch_errors() {
    let output = vz_binary()
        .args([
            "explore",
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/schema_mismatch.csv",
        ])
        .env("VZ_TEST_HEADLESS", "1")
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Schema mismatch"),
        "Expected schema mismatch error, got: {}",
        stderr
    );
}

#[test]
fn test_explore_diff_identical_files() {
    let output = vz_binary()
        .args([
            "explore",
            "fixtures/diff/identical.csv",
            "fixtures/diff/identical.csv",
        ])
        .env("VZ_TEST_HEADLESS", "1")
        .output()
        .expect("Failed to run vz");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("panicked"),
        "Should not panic: {}",
        combined
    );
    assert!(output.status.success());
}

#[test]
fn test_explore_diff_daily_temporal() {
    let output = vz_binary()
        .args([
            "explore",
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
        ])
        .env("VZ_TEST_HEADLESS", "1")
        .output()
        .expect("Failed to run vz");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("panicked"),
        "Should not panic: {}",
        combined
    );
    assert!(output.status.success());
}

// --- Diff HTML output tests ---

#[test]
fn test_diff_html_output() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "-o",
            "html",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("<!DOCTYPE html>"),
        "Missing DOCTYPE in: {}",
        &stdout[..200.min(stdout.len())]
    );
    assert!(stdout.contains("<svg"), "Missing <svg> in HTML output");
    assert!(stdout.contains("</svg>"), "Missing </svg> in HTML output");
    assert!(
        stdout.contains("<script>"),
        "Missing <script> in HTML output"
    );
    assert!(stdout.contains("</html>"), "Missing </html> in output");
}

#[test]
fn test_diff_html_bars_include_percent_labels() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--html",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // After values 1200/1350/950/600 (total 4100): Tokyo holds 29%.
    assert!(
        stdout.contains("(29%)"),
        "Diff HTML bars should keep share-percent labels: {}",
        &stdout[..600.min(stdout.len())]
    );
}

#[test]
fn test_diff_html_shorthand() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--html",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("<!DOCTYPE html>"),
        "Missing DOCTYPE in HTML output"
    );
    assert!(stdout.contains("<svg"), "Missing <svg> in HTML output");
}

#[test]
fn test_diff_html_temporal() {
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
            "--html",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("<!DOCTYPE html>"),
        "Missing DOCTYPE in HTML output"
    );
    assert!(stdout.contains("<svg"), "Missing <svg> in HTML output");
    assert!(stdout.contains("viewBox"), "Missing viewBox in SVG output");
}

#[test]
fn test_diff_html_with_sort_and_top() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--html",
            "--sort",
            "desc",
            "--top",
            "2",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("<!DOCTYPE html>"),
        "Missing DOCTYPE in HTML output"
    );
    assert!(stdout.contains("<svg"), "Missing <svg> in HTML output");
}

#[test]
fn test_diff_spark_output_values() {
    // Categorical diff: 4 categories, overall +5%
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--spark",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    assert!(
        trimmed.starts_with("Δ revenue"),
        "Expected Δ revenue prefix, got: {}",
        trimmed
    );
    assert!(
        trimmed.contains("(+5%)"),
        "Expected overall (+5%) in diff spark, got: {}",
        trimmed
    );
    // 4 categories → 4 sparkline chars
    let parts: Vec<&str> = trimmed.split("  ").collect();
    assert!(parts.len() >= 2, "Expected parts, got: {}", trimmed);
    let spark_chars: Vec<char> = parts[1]
        .chars()
        .filter(|c| "▁▂▃▄▅▆▇█".contains(*c))
        .collect();
    assert_eq!(
        spark_chars.len(),
        4,
        "Expected 4 sparkline chars for 4 categories, got {} in: {}",
        spark_chars.len(),
        parts[1]
    );
}

#[test]
fn test_diff_spark_temporal_output_values() {
    // Temporal diff: 2 lines, 6 data points each, overall +13%
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
            "--spark",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "Expected 2 lines for temporal diff spark, got: {:?}",
        lines
    );
    assert!(
        lines[0].contains("ts_daily_before"),
        "First line should contain before filename, got: {}",
        lines[0]
    );
    assert!(
        lines[1].contains("ts_daily_after"),
        "Second line should contain after filename, got: {}",
        lines[1]
    );
    assert!(
        lines[1].contains("(+13%)"),
        "Second line should have overall (+13%), got: {}",
        lines[1]
    );
    // Each line should have 6 sparkline chars
    for (i, line) in lines.iter().enumerate() {
        let spark_count = line.chars().filter(|c| "▁▂▃▄▅▆▇█".contains(*c)).count();
        assert_eq!(
            spark_count, 6,
            "Line {} should have 6 sparkline chars, got {} in: {}",
            i, spark_count, line
        );
    }
}

#[test]
fn test_diff_markdown_output_values() {
    // Verify actual cell values in diff markdown
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "-o",
            "markdown",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Header columns
    assert!(
        stdout.contains("| city | Before | After | Change |"),
        "Expected header row, got: {}",
        stdout
    );
    // Value assertions for each city
    assert!(
        stdout.contains("| Tokyo | 1k | 1.2k | ▲ +20% |"),
        "Expected Tokyo row values, got: {}",
        stdout
    );
    assert!(
        stdout.contains("| Osaka | 1.5k | 1.4k | ▼ -10% |"),
        "Expected Osaka row values, got: {}",
        stdout
    );
    assert!(
        stdout.contains("| Nagoya | 800 | 950 | ▲ +19% |"),
        "Expected Nagoya row values, got: {}",
        stdout
    );
    assert!(
        stdout.contains("| Fukuoka | 600 | 600 | ─ 0% |"),
        "Expected Fukuoka row values, got: {}",
        stdout
    );
    // Overall summary line
    assert!(
        stdout.contains("*Overall: ▲ +5%*"),
        "Expected overall summary line, got: {}",
        stdout
    );
}

#[test]
fn test_diff_markdown_temporal_values() {
    // Verify temporal diff markdown cell values
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
            "-o",
            "markdown",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Header
    assert!(
        stdout.contains("| date | Before | After | Change |"),
        "Expected temporal header, got: {}",
        stdout
    );
    // First row: 2024-01-01, before=100, after=110, +10%
    assert!(
        stdout.contains("| 2024-01-01 | 100 | 110 | ▲ +10% |"),
        "Expected first temporal row, got: {}",
        stdout
    );
    // Last row: 2024-01-06, before=180, after=220, +22%
    assert!(
        stdout.contains("| 2024-01-06 | 180 | 220 | ▲ +22% |"),
        "Expected last temporal row, got: {}",
        stdout
    );
    // Overall
    assert!(
        stdout.contains("*Overall: ▲ +13%*"),
        "Expected overall +13%, got: {}",
        stdout
    );
    // Row count: header + separator + 6 data rows + empty + overall = 10 lines
    let non_empty_lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        non_empty_lines.len(),
        9,
        "Expected 9 non-empty lines (header + sep + 6 data + overall), got: {:?}",
        non_empty_lines
    );
}

#[test]
fn test_diff_json_categorical_values() {
    // Verify actual numeric values in diff JSON
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--json",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");

    // Structural assertions
    assert_eq!(json["x_column"], "city");
    assert_eq!(json["y_column"], "revenue");

    // Category value assertions
    let categories = json["categories"].as_array().unwrap();
    let tokyo = &categories[0];
    assert_eq!(tokyo["label"], "Tokyo");
    assert_eq!(tokyo["before"], 1000.0);
    assert_eq!(tokyo["after"], 1200.0);
    assert_eq!(tokyo["delta"], 200.0);
    assert_eq!(tokyo["pct_change"], 20.0);

    let osaka = &categories[1];
    assert_eq!(osaka["label"], "Osaka");
    assert_eq!(osaka["before"], 1500.0);
    assert_eq!(osaka["after"], 1350.0);
    assert_eq!(osaka["delta"], -150.0);
    assert_eq!(osaka["pct_change"], -10.0);

    let fukuoka = &categories[3];
    assert_eq!(fukuoka["label"], "Fukuoka");
    assert_eq!(fukuoka["delta"], 0.0);
    assert_eq!(fukuoka["pct_change"], 0.0);

    // Overall percentage
    let overall = json["overall_delta_pct"].as_f64().unwrap();
    assert!(
        (overall - 5.128).abs() < 0.01,
        "Expected overall ~5.128%, got: {}",
        overall
    );
}

#[test]
fn test_diff_json_temporal_values() {
    // Verify temporal diff JSON data values
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
            "--json",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");

    assert_eq!(json["chart_type"], "line");
    assert_eq!(json["x_column"], "date");
    assert_eq!(json["y_column"], "revenue");

    // Dates array
    let dates = json["dates"].as_array().unwrap();
    assert_eq!(dates.len(), 6);
    assert_eq!(dates[0], "2024-01-01");
    assert_eq!(dates[5], "2024-01-06");

    // Before series values
    let before_series = json["before"]["series"].as_array().unwrap();
    assert_eq!(before_series[0]["value"], 100.0);
    assert_eq!(before_series[5]["value"], 180.0);

    // After series values
    let after_series = json["after"]["series"].as_array().unwrap();
    assert_eq!(after_series[0]["value"], 110.0);
    assert_eq!(after_series[5]["value"], 220.0);

    // Overall delta
    let overall = json["overall_delta_pct"].as_f64().unwrap();
    assert!(
        (overall - 13.253).abs() < 0.01,
        "Expected overall ~13.253%, got: {}",
        overall
    );
}

#[test]
fn test_diff_html_output_title_and_structure() {
    // Diff HTML should have specific title with filenames
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "--html",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<title>Diff: sales_before vs sales_after</title>"),
        "Expected diff title with filenames, got first 500: {}",
        &stdout[..500.min(stdout.len())]
    );
    // Tooltip container
    assert!(
        stdout.contains("class=\"tooltip\""),
        "Expected tooltip CSS class in HTML"
    );
    // Responsive SVG
    assert!(
        stdout.contains("max-width: 100%"),
        "Expected responsive SVG CSS"
    );
    assert!(
        stdout.contains("height: auto"),
        "Expected height: auto for responsive SVG"
    );
    // Dark theme background
    assert!(
        stdout.contains("background: #1e1e1e"),
        "Expected dark theme background"
    );
    // Interactive script with hover
    assert!(
        stdout.contains("mouseenter") || stdout.contains("mousemove"),
        "Expected interactive hover event handlers"
    );
}

#[test]
fn test_diff_html_temporal_title() {
    // Temporal diff HTML should also have diff title
    let output = vz_binary()
        .args([
            "fixtures/diff/ts_daily_before.csv",
            "fixtures/diff/ts_daily_after.csv",
            "--html",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<title>Diff: ts_daily_before vs ts_daily_after</title>"),
        "Expected temporal diff title with filenames"
    );
}

#[test]
fn test_diff_text_columns_report_no_fabricated_change() {
    let before = common::temp_csv(&["city,name", "A,x", "B,y"]);
    let after = common::temp_csv(&["city,name", "A,x", "B,z"]);
    let output = vz_binary()
        .args([
            before.path().to_str().unwrap(),
            after.path().to_str().unwrap(),
            "-y",
            "name",
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("-100.0"),
        "text diff must not fabricate -100%: {}",
        stdout
    );
}

#[test]
fn test_diff_ignored_flags_warn() {
    let output = vz_binary()
        .args([
            "fixtures/diff/sales_before.csv",
            "fixtures/diff/sales_after.csv",
            "-w",
            "city=Tokyo",
            "--agg",
            "mean",
            "-c",
            "city",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no effect in diff mode"),
        "missing ignored-flag warning: {}",
        stderr
    );
}
