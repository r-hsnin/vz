//! One-shot rendering end-to-end tests for vz.

#[path = "common/mod.rs"]
mod common;
use common::vz_binary;

#[test]
fn test_basic_csv_renders_chart() {
    let output = vz_binary()
        .arg("fixtures/sales.csv")
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // One-shot mode should render a chart with borders and content
    assert!(
        stdout.lines().count() >= common::MIN_CHART_LINES,
        "Chart output too short"
    );
    // Should contain box-drawing characters from the chart border
    assert!(
        stdout.contains('│') || stdout.contains('─') || stdout.contains('┌'),
        "No chart border characters found in output:\n{}",
        stdout
    );
    // Should contain the chart title (revenue vs date for sales.csv)
    assert!(
        stdout.contains("revenue") || stdout.contains("Revenue"),
        "Chart title not found in output"
    );
}

#[test]
fn test_axis_override_renders_bar_chart() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-x", "city", "-y", "revenue"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Bar chart should show city labels (may be truncated to bar_width)
    assert!(
        stdout.contains("Tok") || stdout.contains("Osa") || stdout.contains("Nag"),
        "Bar chart labels not found in output:\n{}",
        stdout
    );
    // Title should reference the chart content
    assert!(
        stdout.contains("revenue") || stdout.contains("city"),
        "Chart title not found"
    );
}

#[test]
fn test_chart_type_override() {
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-t",
            "bar",
            "-x",
            "city",
            "-y",
            "revenue",
        ])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should render a bar chart (truncated labels + border)
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
    assert!(
        stdout.contains("Tok") || stdout.contains("Osa") || stdout.contains("revenue"),
        "Bar chart content not found:\n{}",
        stdout
    );
}

#[test]
fn test_nonexistent_file_error() {
    let output = vz_binary()
        .arg("nonexistent.csv")
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read file") || stderr.contains("nonexistent"));
}

#[test]
fn test_nonexistent_non_ascii_file_does_not_panic() {
    let output = vz_binary()
        .arg("日本語不存在.csv")
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "suggestion must not panic:\n{}",
        stderr
    );
    assert!(stderr.contains("日本語不存在.csv"));
}

#[test]
fn test_no_file_argument_error() {
    use std::process::Stdio;
    // When stdin is a terminal (not pipe), vz should show usage error.
    // When stdin is an empty pipe, it'll try to read and fail with no-data error.
    // Using Stdio::null() simulates no-pipe (no data available).
    let output = vz_binary()
        .stdin(Stdio::null())
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("No input file")
            || combined.contains("No data rows")
            || combined.contains("is empty"),
        "Expected error, got: '{}'",
        combined
    );
}

#[test]
fn test_nonexistent_column_hint() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-x", "nonexistent", "-y", "revenue"])
        .output()
        .expect("Failed to run vz");

    // Should fail gracefully with helpful message
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") && stderr.contains("Available columns"),
        "Expected helpful error message, got: {stderr}"
    );
}

#[test]
fn test_column_typo_suggests_close_match() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-x", "date", "-y", "revnue"])
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Did you mean 'revenue'?"),
        "Expected column suggestion, got: {stderr}"
    );
}

#[test]
fn test_column_case_typo_marks_case_sensitivity() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-x", "date", "-y", "Revenue"])
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Did you mean 'revenue'?") && stderr.contains("case-sensitive"),
        "Expected case-sensitivity note, got: {stderr}"
    );
}

#[test]
fn test_color_column_produces_multi_series() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-c", "city"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Multi-series chart should have legend with city names
    assert!(
        stdout.contains("Tokyo") || stdout.contains("Osaka") || stdout.contains("Nagoya"),
        "Color column legend not found:\n{}",
        stdout
    );
}

#[test]
fn test_color_column_not_found_errors() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-c", "nonexistent"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Color column 'nonexistent' not found"),
        "Expected color column not found error, got: {}",
        stderr
    );
}

#[test]
fn test_color_column_typo_suggests_close_match() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-c", "City"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Did you mean 'city'?") && stderr.contains("case-sensitive"),
        "Expected color suggestion with case note, got: {stderr}"
    );
}

#[test]
fn test_skipped_rows_warning() {
    let f = common::temp_csv(&[
        "city,revenue",
        "Tokyo,1000",
        "Osaka,N/A",
        "Nagoya,2000",
        "Fukuoka,bad",
        "Kyoto,1500",
    ]);

    let output = vz_binary()
        .args([
            f.path().to_str().unwrap(),
            "-x",
            "city",
            "-y",
            "revenue",
            "-t",
            "bar",
        ])
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("non-parseable values") || stderr.contains("were skipped"),
        "Expected skip warning in stderr, got: {stderr}"
    );
}

#[test]
fn test_no_skip_warning_on_clean_data() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-x", "city", "-y", "revenue"])
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("skipped"),
        "Should not warn on clean data, but stderr: {stderr}"
    );
}

#[test]
fn test_heatmap_type_shows_warning() {
    let output = vz_binary()
        .arg("fixtures/sales.csv")
        .args(["-x", "city", "-y", "revenue", "-t", "heatmap"])
        .env("NO_COLOR", "1")
        .env("COLUMNS", "80")
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Heatmap is now implemented — no warning expected
    assert!(
        stderr.contains("Heatmap"),
        "Expected Heatmap chart type in summary: {stderr}"
    );
    assert!(
        !stderr.contains("not yet implemented"),
        "Warning should be gone: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.lines().count() >= 2);
}

#[test]
fn test_height_flag_controls_output_height() {
    let output_short = vz_binary()
        .arg("fixtures/sales.csv")
        .args(["-H", "10"])
        .env("NO_COLOR", "1")
        .env("COLUMNS", "80")
        .output()
        .expect("Failed to run vz");

    let output_tall = vz_binary()
        .arg("fixtures/sales.csv")
        .args(["-H", "30"])
        .env("NO_COLOR", "1")
        .env("COLUMNS", "80")
        .output()
        .expect("Failed to run vz");

    assert!(output_short.status.success());
    assert!(output_tall.status.success());

    let short_lines = String::from_utf8_lossy(&output_short.stdout)
        .lines()
        .count();
    let tall_lines = String::from_utf8_lossy(&output_tall.stdout).lines().count();

    // Short should have fewer lines than tall
    assert!(
        short_lines < tall_lines,
        "Expected -H 10 ({short_lines} lines) < -H 30 ({tall_lines} lines)"
    );
}

#[test]
fn test_width_flag_controls_output_width() {
    let output = vz_binary()
        .arg("fixtures/sales.csv")
        .args(["-W", "50"])
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // All chart lines (skip summary) should be ≤ 50 display chars
    let max_line_width = stdout
        .lines()
        .skip(1)
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_line_width <= 50,
        "Expected max line width ≤ 50 with -W 50, got {max_line_width}"
    );
}

#[test]
fn test_y_only_flag_is_honored() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-y", "profit"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The chart should use profit as Y axis, not revenue
    assert!(
        stdout.contains("profit"),
        "Expected 'profit' in output when -y profit is specified, got:\n{}",
        stdout
    );
}

#[test]
fn test_x_only_flag_is_honored() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-x", "city"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Bar chart with city as X axis
    assert!(
        stdout.contains("Bar") || stdout.contains("city"),
        "Expected bar chart or city reference when -x city is specified, got:\n{}",
        stdout
    );
}

#[test]
fn test_invalid_chart_type_emits_warning() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "pie"])
        .output()
        .expect("Failed to run vz");

    // Should fail at parse time (ValueEnum validation)
    assert!(!output.status.success());
    // Should show clap error with possible values
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value") || stderr.contains("possible values"),
        "Expected clap parse error for invalid chart type, got: '{}'",
        stderr
    );
}

#[test]
fn test_large_dataset_sampling() {
    let mut rows = vec!["x,y".to_string()];
    for i in 0..10000 {
        rows.push(format!("{},{}", i, i * 2));
    }
    let file = common::temp_csv(&rows);

    let output = vz_binary()
        .arg(file.path())
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should mention sampling
    assert!(
        stderr.contains("sampled"),
        "Expected sampling info message, got stderr: '{}'",
        stderr
    );
    // Should render successfully
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
}

#[test]
fn test_all_unparseable_y_values_gives_clear_error() {
    let file = common::temp_csv_with_suffix(
        ".csv",
        &[
            "date,revenue",
            "2024-01-01,N/A",
            "2024-02-01,missing",
            "2024-03-01,",
        ],
    );

    // Force line chart type to exercise the rendering path with unparseable Y
    let output = vz_binary()
        .args([
            file.path().to_str().unwrap(),
            "-t",
            "line",
            "-x",
            "date",
            "-y",
            "revenue",
        ])
        .output()
        .expect("Failed to run vz");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Should warn about skipped/unparseable data
    let lower = combined.to_lowercase();
    assert!(
        lower.contains("no valid") || lower.contains("skipped") || lower.contains("non-parseable"),
        "Expected warning about unparseable data, got: '{}'",
        combined
    );
}

#[test]
fn test_bar_type_override_prefers_categorical_x() {
    // sales.csv has: date(temporal), city(categorical), revenue(quantitative), profit(quantitative)
    // When user says -t bar without -x, the X axis should be categorical (city) not temporal (date)
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "bar"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Summary should show x=city (categorical) not x=date (temporal) for bar chart
    assert!(
        stderr.contains("x=city"),
        "Expected bar chart to use categorical x=city, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_color_legend_shows_series_mapping() {
    let output = common::vz_command()
        .args(["fixtures/sales.csv", "-c", "city"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should show color-to-series mapping in summary
    assert!(
        stderr.contains("Tokyo=cyan"),
        "Expected color legend with Tokyo=cyan, got stderr: '{}'",
        stderr
    );
    assert!(
        stderr.contains("Osaka=yellow"),
        "Expected color legend with Osaka=yellow, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_multi_y_columns() {
    let output = common::vz_command()
        .args(["fixtures/sales.csv", "-y", "revenue,profit"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Summary should show extra Y column
    assert!(
        stderr.contains("y+=profit"),
        "Expected y+=profit in summary, got stderr: '{}'",
        stderr
    );
    // Chart should contain both series in legend
    assert!(
        stdout.contains("revenue") && stdout.contains("profit"),
        "Expected both series in chart output, got stdout:\n{}",
        stdout
    );
}

#[test]
fn test_multi_y_with_labels() {
    let output = common::vz_command()
        .args(["fixtures/sales.csv", "-y", "revenue:Rev,profit:Prof"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Summary should show the label "Prof" not the column name
    assert!(
        stderr.contains("y+=Prof"),
        "Expected y+=Prof in summary, got stderr: '{}'",
        stderr
    );
    // Chart legend should use labels
    assert!(
        stdout.contains("Prof"),
        "Expected 'Prof' label in chart, got stdout:\n{}",
        stdout
    );
}

#[test]
fn test_heatmap_auto_select() {
    let output = common::vz_command()
        .args(["fixtures/departments.csv"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should auto-detect as Heatmap for two categorical columns
    assert!(
        stderr.contains("Heatmap"),
        "Expected Heatmap chart type in summary, got stderr: '{}'",
        stderr
    );
    // Should NOT contain the old fallback warning
    assert!(
        !stderr.contains("not yet implemented"),
        "Heatmap should be implemented now, got stderr: '{}'",
        stderr
    );
    // Chart should render (non-empty stdout)
    assert!(
        !stdout.is_empty(),
        "Expected chart output, got empty stdout"
    );
}

#[test]
fn test_heatmap_explicit_type() {
    let output = common::vz_command()
        .args([
            "fixtures/sales.csv",
            "-t",
            "heatmap",
            "-x",
            "city",
            "-y",
            "date",
        ])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should render as Heatmap when explicitly requested
    assert!(
        stderr.contains("Heatmap"),
        "Expected Heatmap type in summary, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_sample_flag() {
    // Create a large-ish dataset
    let mut rows = vec!["x,y".to_string()];
    for i in 0..1000 {
        rows.push(format!("{},{}", i, i * 2));
    }
    let f = common::temp_csv(&rows);

    let output = vz_binary()
        .args([f.path().to_str().unwrap(), "--sample", "50", "-o", "json"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    // Should report sampled row count
    assert_eq!(json["rows"], 50);
    // Stderr should contain info about sampling
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("sampled 50/1000"),
        "Expected sampling info in stderr, got: {}",
        stderr
    );
}

#[test]
fn test_sample_zero_gives_clear_error() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--sample", "0"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--sample must be at least 1"),
        "Expected clear error for --sample 0, got: {}",
        stderr
    );
}

#[test]
fn test_error_hint_did_you_mean() {
    // Use a wrong filename that's close to an actual fixture
    let output = vz_binary()
        .args(["fixtures/sale.csv"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Did you mean?"),
        "Expected 'Did you mean?' suggestion, got: {}",
        stderr
    );
    assert!(
        stderr.contains("sales.csv"),
        "Expected 'sales.csv' suggestion, got: {}",
        stderr
    );
}

#[test]
fn test_error_hint_stdin_tip() {
    // Nonexistent file with no similar files around
    let output = vz_binary()
        .args(["zzz_no_match_xyz.csv"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should show at least the stdin tip or nearby data files
    assert!(
        stderr.contains("Tip:") || stderr.contains("Did you mean?"),
        "Expected hint in error output, got: {}",
        stderr
    );
}

#[test]
fn test_invalid_chart_type_rejected() {
    // -t should reject invalid values like --sort and --output do
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "pizza"])
        .output()
        .expect("Failed to run vz");
    assert!(
        !output.status.success(),
        "Expected failure for invalid chart type, but got success"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value") || stderr.contains("possible values"),
        "Expected clap error message, got stderr: {}, stdout: {}",
        stderr,
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_valid_chart_types_accepted() {
    for t in &["line", "bar", "scatter", "histogram", "heatmap"] {
        let output = vz_binary()
            .args(["fixtures/sales.csv", "-t", t])
            .output()
            .expect("Failed to run vz");
        assert!(
            output.status.success(),
            "Expected success for -t {}, got exit code {:?}",
            t,
            output.status
        );
    }
}

#[test]
fn test_year_month_temporal_produces_line_chart() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("year_month.csv");
    std::fs::write(
        &file_path,
        "month,revenue\n2024-01,100\n2024-02,150\n2024-03,200\n2024-04,180\n2024-05,250\n",
    )
    .unwrap();

    let output = vz_binary()
        .arg(file_path.as_os_str())
        .output()
        .expect("Failed to run vz");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "vz failed: {}", stderr);
    assert!(
        stderr.contains("Line"),
        "Expected Line chart for YYYY-MM temporal data, got stderr: '{}'",
        stderr
    );
}
