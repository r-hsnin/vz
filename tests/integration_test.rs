//! Integration tests for vz — end-to-end pipeline tests.

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn vz_binary() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vz"));
    cmd.env("TERM", "dumb");
    cmd
}

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
    assert!(stdout.lines().count() >= 10, "Chart output too short");
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
    assert!(stdout.lines().count() >= 10);
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
fn test_csv_with_only_numeric_columns_renders_scatter() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "height,weight,age").unwrap();
    writeln!(f, "170,65,30").unwrap();
    writeln!(f, "175,72,28").unwrap();
    writeln!(f, "180,80,35").unwrap();
    writeln!(f, "165,58,25").unwrap();

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Scatter plot renders a chart with borders
    assert!(stdout.lines().count() >= 10);
    assert!(
        stdout.contains('│') || stdout.contains('─') || stdout.contains("Scatter"),
        "Scatter chart not rendered"
    );
}

#[test]
fn test_csv_single_numeric_column_renders_histogram() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "score").unwrap();
    writeln!(f, "85").unwrap();
    writeln!(f, "90").unwrap();
    writeln!(f, "78").unwrap();
    writeln!(f, "92").unwrap();
    writeln!(f, "88").unwrap();

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Histogram renders with bin labels and bars
    assert!(stdout.lines().count() >= 10);
    assert!(
        stdout.contains("Distribution") || stdout.contains("score") || stdout.contains('│'),
        "Histogram not rendered:\n{}",
        stdout
    );
}

#[test]
fn test_csv_categorical_only_renders_chart() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "department,status").unwrap();
    writeln!(f, "Engineering,Active").unwrap();
    writeln!(f, "Sales,Active").unwrap();
    writeln!(f, "Engineering,Inactive").unwrap();
    writeln!(f, "Marketing,Active").unwrap();

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success());
    // Should auto-select heatmap for two categorical columns
    assert!(
        stderr.contains("Heatmap"),
        "Expected Heatmap chart type: {stderr}"
    );
    // Chart should render some output
    assert!(
        stdout.lines().count() >= 2,
        "Expected chart output, got:\n{stdout}"
    );
}

#[test]
fn test_csv_with_empty_values_renders() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "date,value").unwrap();
    writeln!(f, "2024-01-01,100").unwrap();
    writeln!(f, "2024-02-01,").unwrap();
    writeln!(f, "2024-03-01,300").unwrap();
    writeln!(f, ",400").unwrap();

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should still render a chart despite nulls
    assert!(stdout.lines().count() >= 10);
}

#[test]
fn test_csv_with_comma_numbers_renders() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "city,population").unwrap();
    writeln!(f, "Tokyo,\"13,960,000\"").unwrap();
    writeln!(f, "Osaka,\"2,753,000\"").unwrap();
    writeln!(f, "Nagoya,\"2,320,000\"").unwrap();

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should render a bar chart (labels may be truncated)
    assert!(stdout.lines().count() >= 10);
    assert!(
        stdout.contains("Tok")
            || stdout.contains("Osa")
            || stdout.contains("Nag")
            || stdout.contains("population"),
        "City labels or title not found in output:\n{}",
        stdout
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
fn test_large_csv_renders() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "id,value,category").unwrap();
    for i in 0..1000 {
        writeln!(
            f,
            "{},{},{}",
            i,
            i as f64 * 1.5,
            if i % 3 == 0 {
                "A"
            } else if i % 3 == 1 {
                "B"
            } else {
                "C"
            }
        )
        .unwrap();
    }

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should handle 1000 rows and render a chart
    assert!(stdout.lines().count() >= 10);
}

#[test]
fn test_unicode_column_names_renders() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "日付,都市,売上").unwrap();
    writeln!(f, "2024-01-01,東京,1000").unwrap();
    writeln!(f, "2024-02-01,大阪,1500").unwrap();
    writeln!(f, "2024-03-01,名古屋,800").unwrap();

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should render a chart with unicode column names
    assert!(stdout.lines().count() >= 10);
    // The chart title includes column names (may have spaces between wide chars)
    assert!(
        stdout.contains('売') || stdout.contains('日'),
        "Unicode column name characters not in chart:\n{}",
        stdout
    );
}

#[test]
fn test_tsv_input() {
    let mut f = NamedTempFile::with_suffix(".tsv").unwrap();
    writeln!(f, "city\trevenue\tprofit").unwrap();
    writeln!(f, "Tokyo\t1000\t200").unwrap();
    writeln!(f, "Osaka\t1500\t350").unwrap();
    writeln!(f, "Nagoya\t800\t150").unwrap();

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should render a bar chart with city labels
    assert!(stdout.lines().count() >= 10);
    assert!(
        stdout.contains("Tokyo") || stdout.contains("Osaka") || stdout.contains("Nagoya"),
        "TSV city labels not found:\n{}",
        stdout
    );
}

#[test]
fn test_stdin_pipe() {
    use std::process::Stdio;

    let mut child = vz_binary()
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn vz");

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"x,y\n1,10\n2,20\n3,30\n4,40\n").unwrap();
    }

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.lines().count() >= 10);
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
fn test_json_array_input() {
    let mut f = NamedTempFile::with_suffix(".json").unwrap();
    writeln!(
        f,
        r#"[
        {{"date": "2024-01-01", "revenue": 1000}},
        {{"date": "2024-02-01", "revenue": 1500}},
        {{"date": "2024-03-01", "revenue": 1200}},
        {{"date": "2024-04-01", "revenue": 1800}}
    ]"#
    )
    .unwrap();

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.lines().count() >= 10);
    assert!(
        stdout.contains("Line") || stdout.contains("revenue"),
        "JSON array should render a line chart:\n{}",
        stdout
    );
}

#[test]
fn test_ndjson_input() {
    let mut f = NamedTempFile::with_suffix(".ndjson").unwrap();
    writeln!(f, r#"{{"x": 1, "y": 10}}"#).unwrap();
    writeln!(f, r#"{{"x": 2, "y": 20}}"#).unwrap();
    writeln!(f, r#"{{"x": 3, "y": 30}}"#).unwrap();
    writeln!(f, r#"{{"x": 4, "y": 25}}"#).unwrap();

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.lines().count() >= 10);
    assert!(
        stdout.contains("Scatter") || stdout.contains('•') || stdout.contains('│'),
        "NDJSON should render a scatter chart:\n{}",
        stdout
    );
}

#[test]
fn test_json_stdin_pipe() {
    use std::process::Stdio;

    let mut child = vz_binary()
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn vz");

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(b"[{\"name\":\"Alice\",\"score\":85},{\"name\":\"Bob\",\"score\":92},{\"name\":\"Charlie\",\"score\":78}]")
            .unwrap();
    }

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.lines().count() >= 10);
    // Should render a bar chart (categorical×quantitative)
    assert!(
        stdout.contains("Alice") || stdout.contains("Bob") || stdout.contains("score"),
        "JSON stdin should render with column names:\n{}",
        stdout
    );
}

#[test]
fn test_ndjson_stdin_pipe() {
    use std::process::Stdio;

    let mut child = vz_binary()
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn vz");

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(b"{\"date\":\"2024-01-01\",\"value\":100}\n{\"date\":\"2024-02-01\",\"value\":200}\n{\"date\":\"2024-03-01\",\"value\":300}\n")
            .unwrap();
    }

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.lines().count() >= 10);
}

#[test]
fn test_no_color_strips_ansi() {
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .arg("fixtures/sales.csv")
        .env("NO_COLOR", "1")
        .env_remove("FORCE_COLOR")
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should NOT contain any ANSI escape sequences
    assert!(
        !stdout.contains('\x1b'),
        "NO_COLOR=1 should strip all ANSI codes, but found escape sequences:\n{}",
        stdout.chars().take(200).collect::<String>()
    );
    // Should still contain chart content
    assert!(stdout.contains("revenue") || stdout.contains("Line"));
}

#[test]
fn test_skipped_rows_warning() {
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "city,revenue").unwrap();
    writeln!(f, "Tokyo,1000").unwrap();
    writeln!(f, "Osaka,N/A").unwrap();
    writeln!(f, "Nagoya,2000").unwrap();
    writeln!(f, "Fukuoka,bad").unwrap();
    writeln!(f, "Kyoto,1500").unwrap();

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
    use std::io::Write;
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "x,y").unwrap();
    for i in 0..10000 {
        writeln!(file, "{},{}", i, i * 2).unwrap();
    }
    file.flush().unwrap();

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
    assert!(stdout.lines().count() >= 10);
}

#[test]
fn test_summary_line_goes_to_stderr() {
    let output = vz_binary()
        .arg("fixtures/sales.csv")
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Summary (metadata) should be on stderr, not stdout
    assert!(
        stderr.contains("Line") && stderr.contains("rows"),
        "Expected summary line on stderr, got stderr: '{}'",
        stderr
    );
    // stdout should NOT contain the summary line (just the chart)
    assert!(
        !stdout.contains("│ x=date │"),
        "Summary line should not be on stdout (for clean piping)"
    );
}

#[test]
fn test_header_only_csv_gives_clear_error() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "x,y").unwrap();
    file.flush().unwrap();

    let output = vz_binary()
        .arg(file.path())
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Should mention "no data" or "0 rows", NOT "Nominal"
    let lower = combined.to_lowercase();
    assert!(
        lower.contains("no data") || lower.contains("0 row") || lower.contains("empty"),
        "Expected clear 'no data' error, got: '{}'",
        combined
    );
    assert!(
        !combined.contains("Nominal"),
        "Should not expose internal type 'Nominal' to user"
    );
}

#[test]
fn test_info_flag_shows_column_metadata() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--info"])
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show column names and types
    assert!(stdout.contains("date"), "Missing column 'date'");
    assert!(stdout.contains("city"), "Missing column 'city'");
    assert!(stdout.contains("revenue"), "Missing column 'revenue'");
    assert!(stdout.contains("Date/Time"), "Missing type 'Date/Time'");
    assert!(stdout.contains("Categorical"), "Missing type 'Categorical'");
    assert!(stdout.contains("Numeric"), "Missing type 'Numeric'");
    // Should show row count
    assert!(stdout.contains("6"), "Missing row count '6'");
}

#[test]
fn test_no_header_flag_treats_first_row_as_data() {
    let mut file = NamedTempFile::with_suffix(".csv").unwrap();
    writeln!(file, "1,10").unwrap();
    writeln!(file, "2,20").unwrap();
    writeln!(file, "3,30").unwrap();
    file.flush().unwrap();

    let output = vz_binary()
        .args([file.path().to_str().unwrap(), "--no-header"])
        .output()
        .expect("Failed to run vz");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should have 3 rows (all data, no header consumed)
    assert!(
        stderr.contains("3 rows"),
        "Expected 3 rows with --no-header, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_numeric_header_auto_detected() {
    let mut file = NamedTempFile::with_suffix(".csv").unwrap();
    // All-numeric "headers" — should auto-detect as no-header
    writeln!(file, "1,100").unwrap();
    writeln!(file, "2,200").unwrap();
    writeln!(file, "3,300").unwrap();
    file.flush().unwrap();

    let output = vz_binary()
        .arg(file.path())
        .output()
        .expect("Failed to run vz");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should have 3 rows (auto-detected no-header since first row is all-numeric)
    assert!(
        stderr.contains("3 rows"),
        "Expected 3 rows with auto-detected no-header, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_info_flag_shows_statistics() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--info"])
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Quantitative columns should show min/max
    assert!(
        stdout.contains("800") || stdout.contains("Min"),
        "Expected min value or Min header for quantitative column, got:\n{}",
        stdout
    );
    // Categorical columns should show unique count
    assert!(
        stdout.contains("unique") || stdout.contains("Unique"),
        "Expected unique count info, got:\n{}",
        stdout
    );
}

#[test]
fn test_sort_flag_bar_chart() {
    let mut file = NamedTempFile::with_suffix(".csv").unwrap();
    writeln!(file, "city,revenue").unwrap();
    writeln!(file, "Osaka,300").unwrap();
    writeln!(file, "Tokyo,500").unwrap();
    writeln!(file, "Nagoya,100").unwrap();
    file.flush().unwrap();

    // With --sort desc, bars should be ordered by value descending
    let output = vz_binary()
        .args([file.path().to_str().unwrap(), "-t", "bar", "--sort", "desc"])
        .output()
        .expect("Failed to run vz");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Tokyo (500) should appear before Osaka (300) which should appear before Nagoya (100)
    let tokyo_pos = stdout.find("Tokyo").expect("Tokyo not in output");
    let osaka_pos = stdout.find("Osaka").expect("Osaka not in output");
    let nagoya_pos = stdout.find("Nagoya").expect("Nagoya not in output");
    assert!(
        tokyo_pos < osaka_pos && osaka_pos < nagoya_pos,
        "Expected desc order (Tokyo < Osaka < Nagoya pos), got: T={}, O={}, N={}",
        tokyo_pos,
        osaka_pos,
        nagoya_pos
    );
}

#[test]
fn test_sort_invalid_value_gives_error() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--sort", "invalid"])
        .output()
        .expect("Failed to run vz");

    assert!(
        !output.status.success(),
        "Expected error for invalid --sort value, but got success"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("possible values"),
        "Expected error mentioning invalid value, got: '{}'",
        stderr
    );
}

#[test]
fn test_all_unparseable_y_values_gives_clear_error() {
    let mut file = NamedTempFile::with_suffix(".csv").unwrap();
    writeln!(file, "date,revenue").unwrap();
    writeln!(file, "2024-01-01,N/A").unwrap();
    writeln!(file, "2024-02-01,missing").unwrap();
    writeln!(file, "2024-03-01,").unwrap();
    file.flush().unwrap();

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
fn test_summary_shows_unused_columns() {
    let mut file = NamedTempFile::with_suffix(".csv").unwrap();
    writeln!(file, "date,revenue,profit,city").unwrap();
    writeln!(file, "2024-01-01,100,50,Tokyo").unwrap();
    writeln!(file, "2024-02-01,200,80,Osaka").unwrap();
    file.flush().unwrap();

    let output = vz_binary()
        .arg(file.path())
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Summary should mention unused columns
    assert!(
        stderr.contains("profit") || stderr.contains("+1"),
        "Expected summary to mention unused column 'profit', got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_sort_on_line_chart_warns() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--sort", "desc"])
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should warn that --sort has no effect on line charts
    assert!(
        stderr.contains("--sort") && stderr.contains("bar"),
        "Expected warning about --sort only applying to bar charts, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_top_on_non_bar_chart_warns() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--top", "3"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--top/--tail") && stderr.contains("bar"),
        "Expected warning about --top/--tail only applying to bar charts, got stderr: '{}'",
        stderr
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
    let bin = env!("CARGO_BIN_EXE_vz");
    let output = std::process::Command::new(bin)
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
    let bin = env!("CARGO_BIN_EXE_vz");
    let output = std::process::Command::new(bin)
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
    let bin = env!("CARGO_BIN_EXE_vz");
    let output = std::process::Command::new(bin)
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
    let bin = env!("CARGO_BIN_EXE_vz");
    let output = std::process::Command::new(bin)
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
    let bin = env!("CARGO_BIN_EXE_vz");
    let output = std::process::Command::new(bin)
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
fn test_format_flag_forces_tsv() {
    // Create a TSV file with .txt extension (would be detected as CSV without --format)
    let mut tmp = NamedTempFile::with_suffix(".txt").unwrap();
    writeln!(tmp, "city\trevenue").unwrap();
    writeln!(tmp, "Tokyo\t1000").unwrap();
    writeln!(tmp, "Osaka\t2000").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args([tmp.path().to_str().unwrap(), "--format", "tsv"])
        .output()
        .expect("failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("x=city") && stderr.contains("y=revenue"),
        "TSV format should parse columns correctly, got: '{}'",
        stderr
    );
}

#[test]
fn test_format_flag_short() {
    let mut tmp = NamedTempFile::with_suffix(".dat").unwrap();
    writeln!(tmp, "city\trevenue").unwrap();
    writeln!(tmp, "Tokyo\t1000").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args([tmp.path().to_str().unwrap(), "-f", "tsv"])
        .output()
        .expect("failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("x=city"),
        "-f short flag should work, got: '{}'",
        stderr
    );
}

#[test]
fn test_where_filter_equality() {
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args(["fixtures/sales.csv", "--where", "city=Tokyo"])
        .output()
        .expect("failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "Failed: {}", stderr);
    assert!(
        stderr.contains("3 rows"),
        "Expected 3 rows for Tokyo, got: '{}'",
        stderr
    );
}

#[test]
fn test_where_filter_numeric_gt() {
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args(["fixtures/sales.csv", "--where", "revenue>1500"])
        .output()
        .expect("failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "Failed: {}", stderr);
    assert!(
        stderr.contains("2 rows"),
        "Expected 2 rows with revenue>1500, got: '{}'",
        stderr
    );
}

#[test]
fn test_where_filter_invalid_column() {
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args(["fixtures/sales.csv", "--where", "missing=x"])
        .output()
        .expect("failed to execute");

    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("not found"),
        "Expected error about missing column, got: '{}'",
        combined
    );
}

#[test]
fn test_where_filter_multiple() {
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args([
            "fixtures/sales.csv",
            "--where",
            "city=Tokyo",
            "--where",
            "revenue>1500",
        ])
        .output()
        .expect("failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "Failed: {}", stderr);
    // Tokyo + revenue>1500 should give 1 row
    assert!(
        stderr.contains("1 row"),
        "Expected 1 filtered row, got: '{}'",
        stderr
    );
}

#[test]
fn test_top_flag_limits_bars() {
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args([
            "fixtures/sales.csv",
            "-x",
            "city",
            "-y",
            "revenue",
            "-t",
            "bar",
            "--top",
            "2",
        ])
        .output()
        .expect("failed to execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show only 2 bars: Tokyo and Osaka (top 2 by revenue)
    assert!(stdout.contains("Tokyo"), "Should contain Tokyo");
    assert!(stdout.contains("Osaka"), "Should contain Osaka");
    // Nagoya should be excluded (lowest revenue)
    assert!(
        !stdout.contains("Nagoya"),
        "Nagoya should be excluded by --top 2"
    );
}

#[test]
fn test_tail_flag_limits_bars() {
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args([
            "fixtures/sales.csv",
            "-x",
            "city",
            "-y",
            "revenue",
            "-t",
            "bar",
            "--tail",
            "1",
        ])
        .output()
        .expect("failed to execute");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show only the bottom 1: Nagoya (lowest total revenue)
    assert!(
        stdout.contains("Nagoya"),
        "Should contain Nagoya (lowest revenue)"
    );
}

#[test]
fn test_top_flag_cli_parsing() {
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args(["fixtures/sales.csv", "--top", "1", "-t", "bar"])
        .output()
        .expect("failed to execute");

    assert!(output.status.success());
}

#[test]
fn test_stdin_auto_detect_without_dash() {
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_vz"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn");

    use std::io::Write;
    let stdin = child.stdin.as_mut().unwrap();
    stdin
        .write_all(b"city,revenue\nTokyo,1000\nOsaka,2000\n")
        .unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("failed to wait");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("2 rows"),
        "Should auto-read stdin, got stderr: {}",
        stderr
    );
}

#[test]
fn test_present_nonexistent_file_errors() {
    let output = vz_binary()
        .args(["present", "nonexistent.md"])
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to read") || stderr.contains("No such file"),
        "Expected file-not-found error for present, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_explore_nonexistent_file_errors() {
    let output = vz_binary()
        .args(["explore", "nonexistent.csv"])
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to read") || stderr.contains("No such file"),
        "Expected file-not-found error for explore, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_present_empty_file_errors() {
    let mut file = NamedTempFile::with_suffix(".md").unwrap();
    file.flush().unwrap();

    let output = vz_binary()
        .args(["present", file.path().to_str().unwrap()])
        .output()
        .expect("Failed to run vz");

    // Empty file should either error or handle gracefully (not panic)
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    // Should not contain "panicked" — that would be a crash
    assert!(
        !combined.contains("panicked"),
        "Present mode should not panic on empty file, got: '{}'",
        combined
    );
}

#[test]
fn test_explore_empty_csv_errors() {
    let mut file = NamedTempFile::with_suffix(".csv").unwrap();
    writeln!(file).unwrap();
    file.flush().unwrap();

    let output = vz_binary()
        .args(["explore", file.path().to_str().unwrap()])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    // Should not contain "panicked"
    assert!(
        !combined.contains("panicked"),
        "Explore mode should not panic on empty csv, got: '{}'",
        combined
    );
}

#[test]
fn test_present_no_file_argument_errors() {
    let output = vz_binary()
        .args(["present"])
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should complain about missing file argument
    assert!(
        stderr.contains("required") || stderr.contains("FILE") || stderr.contains("Usage"),
        "Expected usage/required error for present without file, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_explore_no_file_argument_errors() {
    let output = vz_binary()
        .args(["explore"])
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should complain about missing file argument
    assert!(
        stderr.contains("required") || stderr.contains("FILE") || stderr.contains("Usage"),
        "Expected usage/required error for explore without file, got stderr: '{}'",
        stderr
    );
}

#[test]
fn test_info_shows_chart_recommendation() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--info"])
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // --info should show a chart recommendation
    assert!(
        stdout.contains("Recommendation:"),
        "Expected chart recommendation in --info output, got:\n{}",
        stdout
    );
    // For sales.csv (temporal + quantitative), should recommend Line
    assert!(
        stdout.contains("Line"),
        "Expected Line recommendation for temporal+quantitative data, got:\n{}",
        stdout
    );
}

#[test]
fn test_stdin_literal_newline_gives_helpful_hint() {
    // When user pipes data with literal \n (not expanded), vz auto-expands them
    let mut cmd = vz_binary();
    cmd.args(["-"]);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("Failed to spawn vz");
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"a,b\\n1,2\\n3,4").unwrap();
    }
    // stdin is dropped here, sending EOF
    let output = child.wait_with_output().expect("Failed to wait");
    assert!(
        output.status.success(),
        "Expected success after auto-expanding literal \\n, got stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should render a chart with some meaningful output
    assert!(
        stdout.lines().count() >= 5,
        "Expected chart output after auto-expanding, got:\n{}",
        stdout
    );
}

#[test]
fn test_where_eq_filter() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--where", "city=Tokyo"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "vz failed with --where city=Tokyo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Tokyo"),
        "Expected Tokyo in filtered output"
    );
}

#[test]
fn test_where_not_eq_filter() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--where", "city!=Tokyo"])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "vz failed with --where city!=Tokyo"
    );
}

#[test]
fn test_where_gte_filter() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--where", "revenue>=1500"])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "vz failed with --where revenue>=1500"
    );
}

#[test]
fn test_where_lte_filter() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--where", "revenue<=1000"])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "vz failed with --where revenue<=1000"
    );
}

#[test]
fn test_where_invalid_column_errors() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--where", "nonexist=foo"])
        .output()
        .expect("Failed to run vz");
    assert!(
        !output.status.success(),
        "Expected failure for invalid filter column"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found"),
        "Expected 'not found' in error for invalid column, got: {}",
        stderr
    );
}

#[test]
fn test_agg_mean_flag() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "bar", "--agg", "mean"])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "Expected success with --agg mean, got stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should render a bar chart with axis labels
    assert!(
        stdout.lines().count() >= 10,
        "Expected chart output with --agg mean, got:\n{}",
        stdout
    );
}

#[test]
fn test_agg_count_flag() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "bar", "--agg", "count"])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "Expected success with --agg count, got stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_agg_warns_on_non_bar_chart() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "line", "--agg", "mean"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--agg has no effect"),
        "Expected warning about --agg on non-bar chart, got stderr:\n{}",
        stderr
    );
}

#[test]
fn test_output_json_basic() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--output", "json"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON output");
    assert_eq!(json["version"], 1);
    assert_eq!(json["rows"], 6);
    assert_eq!(json["columns"].as_array().unwrap().len(), 4);
    assert_eq!(json["recommendation"]["chart_type"], "line");
    assert_eq!(json["recommendation"]["x"], "date");
}

#[test]
fn test_output_json_column_types() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-o", "json"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON output");
    let cols = json["columns"].as_array().unwrap();
    assert_eq!(cols[0]["type"], "temporal");
    assert_eq!(cols[1]["type"], "categorical");
    assert_eq!(cols[2]["type"], "quantitative");
    // Quantitative stats
    assert!(cols[2]["stats"]["min"].is_f64());
    assert!(cols[2]["stats"]["max"].is_f64());
    assert!(cols[2]["stats"]["mean"].is_f64());
    // Categorical stats
    assert_eq!(cols[1]["stats"]["unique"], 3);
    assert!(cols[1]["stats"]["values"].is_array());
}

#[test]
fn test_output_json_chart_data_line() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--output", "json"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let chart_data = &json["chart_data"];
    assert_eq!(chart_data["type"], "line");
    let series = chart_data["series"].as_array().unwrap();
    assert!(!series.is_empty());
    assert_eq!(series[0]["name"], "revenue");
    let data = series[0]["data"].as_array().unwrap();
    assert_eq!(data.len(), 6);
    assert_eq!(data[0]["x"], "2024-01-01");
    assert_eq!(data[0]["y"], 1000.0);
}

#[test]
fn test_output_json_chart_data_bar_sorted() {
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-t",
            "bar",
            "-x",
            "city",
            "--sort",
            "desc",
            "--output",
            "json",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let chart_data = &json["chart_data"];
    assert_eq!(chart_data["type"], "bar");
    let cats = chart_data["categories"].as_array().unwrap();
    let vals = chart_data["values"].as_array().unwrap();
    assert_eq!(cats.len(), 3);
    // Sorted desc: Tokyo(4200) > Osaka(3300) > Nagoya(800)
    assert_eq!(cats[0], "Tokyo");
    assert!(vals[0].as_f64().unwrap() > vals[1].as_f64().unwrap());
}

#[test]
fn test_output_svg_basic() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--output", "svg"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("<svg"), "SVG should start with <svg tag");
    assert!(stdout.contains("</svg>"), "SVG should have closing tag");
    assert!(stdout.contains("viewBox"), "SVG should have viewBox");
    assert!(stdout.contains("revenue"), "SVG should contain data labels");
}

#[test]
fn test_output_svg_light_theme() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--svg", "--theme", "light"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fill=\"#ffffff\""),
        "Light theme SVG should have white background"
    );
}

#[test]
fn test_output_json_info_flag() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--info", "--output", "json"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON output");
    assert_eq!(json["version"], 1);
    assert!(json["recommendation"].is_object());
}

#[test]
fn test_output_json_stdin() {
    let output = vz_binary()
        .args(["-", "-o", "json"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"name,val\nAlice,10\nBob,20\n")
                .unwrap();
            child.wait_with_output()
        })
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON output");
    assert_eq!(json["rows"], 2);
}

#[test]
fn test_deterministic_pipe_width() {
    // When piped, output should always be 80 columns wide (deterministic)
    let output1 = vz_binary()
        .args(["fixtures/sales.csv", "-t", "bar"])
        .output()
        .expect("Failed to run vz");
    let output2 = vz_binary()
        .args(["fixtures/sales.csv", "-t", "bar"])
        .output()
        .expect("Failed to run vz");
    assert!(output1.status.success());
    assert!(output2.status.success());
    // Same input, piped → identical output
    assert_eq!(
        String::from_utf8_lossy(&output1.stdout),
        String::from_utf8_lossy(&output2.stdout),
        "Piped output should be deterministic"
    );
}

#[test]
fn test_output_json_error_format() {
    let output = vz_binary()
        .args(["nonexistent.csv", "-o", "json"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Error output should be valid JSON");
    assert_eq!(json["version"], 1);
    assert!(
        json["error"].as_str().unwrap().contains("No such file"),
        "Expected file not found error in JSON, got: {}",
        json["error"]
    );
}

#[test]
fn test_malformed_csv_row_warning() {
    // The csv crate with flexible(true) tolerates most malformations.
    // Verify that vz handles edge cases gracefully without crashing.
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "name,val").unwrap();
    writeln!(f, "Alice,10").unwrap();
    writeln!(f, "Bob").unwrap(); // fewer fields
    writeln!(f, "Charlie,30,extra").unwrap(); // more fields
    f.flush().unwrap();

    let output = vz_binary()
        .args([f.path().to_str().unwrap(), "-o", "json"])
        .output()
        .expect("Failed to run vz");
    // Should succeed — flexible mode handles field count differences
    assert!(
        output.status.success(),
        "Should handle inconsistent field counts. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should produce valid JSON");
    // All 3 data rows should be present (flexible mode allows them)
    assert_eq!(json["rows"], 3);
}

#[test]
fn test_sample_flag() {
    // Create a large-ish dataset
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "x,y").unwrap();
    for i in 0..1000 {
        writeln!(f, "{},{}", i, i * 2).unwrap();
    }
    f.flush().unwrap();

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
fn test_title_flag() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "bar", "--title", "Custom Title"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Custom Title"),
        "Expected custom title in output, got: {}",
        stdout
    );
}

#[test]
fn test_explore_where_flag_parsed() {
    // explore subcommand should accept --where flag (even though we can't test TUI output,
    // verify the binary doesn't reject the flag with a parse error)
    let output = vz_binary()
        .args(["explore", "fixtures/sales.csv", "--where", "city=Tokyo"])
        .env("VZ_TEST_HEADLESS", "1")
        .output()
        .expect("Failed to run vz");
    // Should not fail with "unexpected argument" or similar CLI parse error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "explore should accept --where flag, got: {}",
        stderr
    );
}

#[test]
fn test_explore_directory_does_not_panic() {
    // vz explore <dir> should combine files and enter TUI (headless exits immediately)
    let output = vz_binary()
        .args(["explore", "fixtures/dir_test/same_schema/"])
        .env("VZ_TEST_HEADLESS", "1")
        .output()
        .expect("Failed to run vz");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        !combined.contains("panicked"),
        "explore dir should not panic, got: '{}'",
        combined
    );
    assert!(
        output.status.success(),
        "explore dir should succeed, stderr: '{}'",
        stderr
    );
}

#[test]
fn test_explore_directory_with_case_insensitive() {
    // vz explore <dir> with case-insensitive schema files
    let output = vz_binary()
        .args(["explore", "fixtures/dir_test/case_insensitive/"])
        .env("VZ_TEST_HEADLESS", "1")
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "explore dir with case-insensitive schema should succeed, stderr: '{}'",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_where_filter_shows_feedback() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--where", "city=Tokyo"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("filtered 3/6 rows"),
        "Expected filter feedback in stderr, got: {}",
        stderr
    );
}

#[test]
fn test_all_y_flag_overlays_all_numeric_columns() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-Y"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "vz -Y should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // With --all-y, the summary should show multi-Y (e.g. "y=revenue,profit")
    // and should NOT show "+1: profit" hint (since it's already plotted)
    assert!(
        !stderr.contains("+1:"),
        "With -Y, no columns should be listed as unused. Got: {}",
        stderr
    );
}

#[test]
fn test_bar_summary_shows_aggregated_values() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "bar"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Tokyo has revenue 1000+1200+2000=4200, which is >2000 (the raw max).
    // Summary should show the aggregated max (4.2k), not raw max (2.0k).
    assert!(
        !stderr.contains("800\u{2013}2.0k"),
        "Summary should NOT show raw range 800-2.0k for bar chart; got: {}",
        stderr
    );
    assert!(
        stderr.contains("4.2k"),
        "Summary should show aggregated max 4.2k for bar chart; got: {}",
        stderr
    );
}

#[test]
fn test_bar_skip_warning_blames_x_column() {
    // Create CSV with empty category (X) labels — bar chart should blame X, not Y
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "city,revenue").unwrap();
    writeln!(file, "Tokyo,1000").unwrap();
    writeln!(file, ",500").unwrap(); // empty X label
    writeln!(file, "Osaka,800").unwrap();
    file.flush().unwrap();

    let output = vz_binary()
        .args([file.path().to_str().unwrap(), "-t", "bar"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("rows skipped") {
        assert!(
            stderr.contains("'city'"),
            "Bar chart skip warning should blame X column 'city', got: {}",
            stderr
        );
        assert!(
            !stderr.contains("'revenue'"),
            "Bar chart skip warning should NOT blame Y column 'revenue', got: {}",
            stderr
        );
    }
}

#[test]
fn test_labels_flag_shows_percentage_on_bars() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "bar", "--labels"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "vz --labels should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With --labels, bar values should show percentage (e.g., "51%" or similar)
    assert!(
        stdout.contains('%'),
        "Expected percentage labels on bars with --labels flag, got stdout: {}",
        &stdout[..stdout.len().min(500)]
    );
}

#[test]
fn test_output_table_shows_formatted_data() {
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-x",
            "city",
            "-y",
            "revenue",
            "-t",
            "bar",
            "-o",
            "table",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "vz -o table should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Table should show column headers
    assert!(
        stdout.contains("city"),
        "Table should contain 'city' header"
    );
    assert!(
        stdout.contains("revenue"),
        "Table should contain 'revenue' header"
    );
    // Table should show aggregated values (since -t bar implies aggregation)
    assert!(stdout.contains("Tokyo"), "Table should contain 'Tokyo' row");
}

#[test]
fn test_output_table_includes_color_column() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-c", "city", "-o", "table"])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "vz -c city -o table should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Table should include the color/group column
    assert!(
        stdout.contains("city"),
        "Table should show color column header: {stdout}"
    );
    // Should show the data values for the color column
    assert!(
        stdout.contains("Tokyo"),
        "Table should show color column values: {stdout}"
    );
    assert!(
        stdout.contains("Osaka"),
        "Table should show color column values: {stdout}"
    );
}

#[test]
fn test_json_flag_shorthand() {
    // --json should produce the same output as -o json
    let json_flag = vz_binary()
        .args(["fixtures/sales.csv", "--json"])
        .output()
        .expect("Failed to run vz --json");
    let o_json = vz_binary()
        .args(["fixtures/sales.csv", "-o", "json"])
        .output()
        .expect("Failed to run vz -o json");
    assert!(json_flag.status.success(), "vz --json should succeed");
    assert!(o_json.status.success(), "vz -o json should succeed");
    let out1 = String::from_utf8_lossy(&json_flag.stdout);
    let out2 = String::from_utf8_lossy(&o_json.stdout);
    assert_eq!(
        out1, out2,
        "--json and -o json should produce identical output"
    );
}

#[test]
fn test_sparkline_in_summary_line() {
    let output = vz_binary()
        .args(["fixtures/sales.csv"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Summary should contain Unicode block characters (sparkline)
    let has_spark = stderr.chars().any(|c| "▁▂▃▄▅▆▇█".contains(c));
    assert!(
        has_spark,
        "Expected sparkline characters in summary line, got: {}",
        stderr
    );
}

#[test]
fn test_trend_annotation_in_summary() {
    // sales.csv revenue goes from 1000 to 1800 (first to last row) → uptrend
    let output = vz_binary()
        .args(["fixtures/sales.csv"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should contain an arrow indicator
    let has_trend = stderr.contains('↑') || stderr.contains('↓') || stderr.contains('→');
    assert!(
        has_trend,
        "Expected trend annotation (↑/↓/→) in summary, got: {}",
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
fn test_spark_output_mode() {
    // --spark should output the y-column name followed by a sparkline
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-o", "spark"])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "Expected success, got: {:?}",
        output
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    assert!(!trimmed.is_empty(), "Expected sparkline output");
    // Format: "column_name  ▂▅▃▁█▇  (min–max) ↑ +N%"
    assert!(
        trimmed.contains("revenue"),
        "Expected column name in spark output, got: {}",
        trimmed
    );
    let parts: Vec<&str> = trimmed.split("  ").collect();
    assert!(
        parts.len() >= 2,
        "Expected at least label and sparkline parts, got: {}",
        trimmed
    );
    let spark_part = parts[1];
    assert!(
        spark_part.chars().all(|c| "▁▂▃▄▅▆▇█".contains(c)),
        "Expected only sparkline chars in second segment, got: {}",
        spark_part
    );
    // Stats suffix should contain range info
    if parts.len() >= 3 {
        let stats_part = parts[2..].join("  ");
        assert!(
            stats_part.contains('(') && stats_part.contains(')'),
            "Expected range in stats suffix, got: {}",
            stats_part
        );
    }
}

#[test]
fn test_spark_with_color_grouped() {
    // With -c, should show one sparkline per group
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-o", "spark", "-c", "city"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    // Should have multiple lines (one per city)
    assert!(
        lines.len() >= 2,
        "Expected multiple sparkline lines for grouped data, got: {}",
        stdout
    );
    // Each line should contain group name and sparkline
    assert!(
        lines[0].contains("Tokyo") || lines[0].contains("Osaka") || lines[0].contains("Nagoya"),
        "Expected group name in output, got: {}",
        lines[0]
    );
}

#[test]
fn test_spark_shorthand_flag() {
    // --spark should be equivalent to -o spark
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--spark"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    // Format: "column_name  ▂▅▃▁█▇  (min–max) ↑ +N%"
    let parts: Vec<&str> = trimmed.split("  ").collect();
    assert!(
        parts.len() >= 2,
        "Expected at least label and sparkline, got: {}",
        trimmed
    );
    let spark_part = parts[1];
    assert!(
        spark_part.chars().all(|c| "▁▂▃▄▅▆▇█".contains(c)),
        "Expected sparkline from --spark, got: {}",
        trimmed
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
fn test_completions_bash() {
    let output = vz_binary()
        .args(["completions", "bash"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "completions bash should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Bash completions should contain the binary name
    assert!(
        stdout.contains("vz") && stdout.contains("complete"),
        "Expected bash completion script, got: {}",
        &stdout[..stdout.len().min(200)]
    );
}

#[test]
fn test_completions_zsh() {
    let output = vz_binary()
        .args(["completions", "zsh"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "completions zsh should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("vz") || stdout.contains("compdef"),
        "Expected zsh completion script"
    );
}

#[test]
fn test_completions_fish() {
    let output = vz_binary()
        .args(["completions", "fish"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "completions fish should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("vz") && stdout.contains("complete"),
        "Expected fish completion script"
    );
}

#[test]
fn test_where_filter_eliminates_all_rows_gives_clear_message() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--where", "city=Nonexistent"])
        .output()
        .expect("Failed to run vz");
    assert!(
        !output.status.success(),
        "Should fail when filter eliminates all rows"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should NOT say "appears to contain only headers" — that's misleading
    assert!(
        !stderr.contains("only headers"),
        "Error should not misleadingly mention 'only headers' when filter eliminated all rows. Got: {}",
        stderr
    );
    // Should indicate that filtering removed all data
    assert!(
        stderr.contains("filter") && stderr.contains("0"),
        "Error should mention filter as cause. Got: {}",
        stderr
    );
}

#[test]
fn test_json_array_of_primitives_gives_helpful_error() {
    use std::io::Write;
    let mut child = vz_binary()
        .args(["-", "-f", "json"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn vz");

    child.stdin.take().unwrap().write_all(b"[1, 2, 3]").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("object"),
        "Expected helpful error about objects, got: {}",
        stderr
    );
}

#[test]
fn test_watch_flag_accepted_and_rerenders_on_change() {
    use std::io::Write;
    use std::time::Duration;

    // Create a temporary CSV file
    let mut tmpfile = NamedTempFile::new().unwrap();
    writeln!(tmpfile, "x,y").unwrap();
    writeln!(tmpfile, "a,1").unwrap();
    writeln!(tmpfile, "b,2").unwrap();
    tmpfile.flush().unwrap();

    let path = tmpfile.path().to_path_buf();

    // Start vz with --watch
    let mut child = vz_binary()
        .arg(path.to_str().unwrap())
        .arg("--watch")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start vz --watch");

    // Give it time to render once and start watching
    std::thread::sleep(Duration::from_millis(500));

    // Modify the file to trigger re-render
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        writeln!(f, "x,y").unwrap();
        writeln!(f, "a,10").unwrap();
        writeln!(f, "b,20").unwrap();
        writeln!(f, "c,30").unwrap();
        f.flush().unwrap();
    }

    // Give it time to detect and re-render
    std::thread::sleep(Duration::from_millis(1000));

    // Kill the watch process
    child.kill().ok();
    let output = child.wait_with_output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Watch mode should print re-render info to stderr
    assert!(
        stderr.contains("Watching") || stderr.contains("Re-rendering"),
        "Expected watch feedback in stderr, got: {}",
        stderr
    );
}

#[test]
fn test_watch_flag_on_nonexistent_file_errors() {
    let output = vz_binary()
        .arg("nonexistent_data.csv")
        .arg("--watch")
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
}

#[test]
fn test_watch_flag_on_stdin_errors() {
    let output = vz_binary()
        .arg("-")
        .arg("--watch")
        .stdin(std::process::Stdio::piped())
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() || stderr.contains("--watch cannot be used with stdin"),
        "Expected error for --watch with stdin, got: {}",
        stderr
    );
}

#[test]
fn test_labels_on_non_bar_chart_warns() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--labels"])
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--labels has no effect"),
        "Expected --labels warning, got: {}",
        stderr
    );
}

#[test]
fn test_theme_flag_dark() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--theme", "dark"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
}

#[test]
fn test_theme_flag_light() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--theme", "light"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
}

#[test]
fn test_theme_flag_high_contrast() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--theme", "high-contrast"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
}

#[test]
fn test_theme_flag_invalid_rejected() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--theme", "neon"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
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

#[test]
fn test_summary_shows_skipped_rows() {
    let output = vz_binary()
        .args([
            "fixtures/mixed_values.csv",
            "-t",
            "line",
            "-x",
            "date",
            "-y",
            "revenue",
        ])
        .output()
        .expect("Failed to run vz");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("5 rows (2 skipped)"),
        "Expected '5 rows (2 skipped)' in stderr: {}",
        stderr
    );
}

#[test]
fn test_theme_light_produces_output() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--theme", "light"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "vz --theme light failed");
    assert!(!output.stdout.is_empty(), "Expected chart output");
}

#[test]
fn test_theme_high_contrast_produces_output() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--theme", "high-contrast"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "vz --theme high-contrast failed");
    assert!(!output.stdout.is_empty(), "Expected chart output");
}

#[test]
fn test_theme_invalid_value_errors() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--theme", "neon"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success(), "Expected error for invalid theme");
}

#[test]
fn test_bins_flag_controls_histogram_bin_count() {
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-y",
            "revenue",
            "-t",
            "histogram",
            "--bins",
            "5",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "vz --bins 5 failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // With 5 bins, we should have fewer distinct bin labels than with 10
    assert!(!stdout.is_empty(), "Should produce histogram output");
}

#[test]
fn test_bins_zero_gives_clear_error() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--bins", "0"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--bins must be at least 1"),
        "Expected clear error for --bins 0, got: {}",
        stderr
    );
}

#[test]
fn test_top_zero_gives_clear_error() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--top", "0"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--top must be at least 1"),
        "Expected clear error for --top 0, got: {}",
        stderr
    );
}

#[test]
fn test_tail_zero_gives_clear_error() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--tail", "0"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--tail must be at least 1"),
        "Expected clear error for --tail 0, got: {}",
        stderr
    );
}

#[test]
fn test_bins_flag_warns_on_non_histogram() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-t", "bar", "--bins", "20"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--bins"),
        "Should warn that --bins has no effect on non-histogram charts. stderr: {}",
        stderr
    );
}

#[test]
fn test_output_table_shows_all_columns_by_default() {
    // When -o table is used without -t bar, all columns should be shown
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-o", "table"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success(), "vz -o table should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // ALL columns from the CSV should be present, not just chart-selected ones
    assert!(
        stdout.contains("profit"),
        "Table should show ALL columns including 'profit'. Got:\n{stdout}"
    );
    assert!(
        stdout.contains("date"),
        "Table should show 'date' column. Got:\n{stdout}"
    );
    assert!(
        stdout.contains("city"),
        "Table should show 'city' column. Got:\n{stdout}"
    );
    assert!(
        stdout.contains("revenue"),
        "Table should show 'revenue' column. Got:\n{stdout}"
    );
}

#[test]
fn test_spark_output_respects_bar_aggregation() {
    // When -t bar is specified with spark output, values should be aggregated
    // sales.csv: Tokyo=4200, Osaka=3300, Nagoya=800 → 3 aggregated categories
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-t",
            "bar",
            "-x",
            "city",
            "-y",
            "revenue",
            "-o",
            "spark",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    // Format: "revenue  █▆▁  (min–max) trend" — sparkline is second segment
    let parts: Vec<&str> = trimmed.split("  ").collect();
    assert!(
        parts.len() >= 2,
        "Expected at least label and sparkline, got: {}",
        trimmed
    );
    let spark_part = parts[1];
    let spark_chars: Vec<char> = spark_part.chars().collect();
    // Should have 3 characters (one per aggregated category), not 6 (one per raw row)
    assert_eq!(
        spark_chars.len(),
        3,
        "Spark with -t bar should show 3 aggregated values, got {}: '{}'",
        spark_chars.len(),
        trimmed
    );
}

#[test]
fn test_spark_output_respects_sort_and_top() {
    // With --sort desc --top 2, spark should show only top 2 categories
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-t",
            "bar",
            "-x",
            "city",
            "-y",
            "revenue",
            "-o",
            "spark",
            "--sort",
            "desc",
            "--top",
            "2",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    // Format: "revenue  █▇  (min–max) trend" — sparkline is second segment
    let parts: Vec<&str> = trimmed.split("  ").collect();
    assert!(
        parts.len() >= 2,
        "Expected at least label and sparkline, got: {}",
        trimmed
    );
    let spark_part = parts[1];
    let spark_chars: Vec<char> = spark_part.chars().collect();
    assert_eq!(
        spark_chars.len(),
        2,
        "Spark with --top 2 should show 2 values, got {}: '{}'",
        spark_chars.len(),
        trimmed
    );
}

#[test]
fn test_json_output_respects_color_grouping() {
    // When -c city is specified, JSON should produce multiple series grouped by city
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-c", "city", "-o", "json"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    let series = json["chart_data"]["series"]
        .as_array()
        .expect("chart_data.series should be an array");
    // Should have 3 series (Tokyo, Osaka, Nagoya), not 1
    assert!(
        series.len() >= 3,
        "Expected at least 3 series for 3 cities, got {}: {}",
        series.len(),
        serde_json::to_string_pretty(&json["chart_data"]).unwrap()
    );
    // Each series should have a name matching a city
    let names: Vec<&str> = series.iter().filter_map(|s| s["name"].as_str()).collect();
    assert!(
        names.contains(&"Tokyo"),
        "Should have Tokyo series, got: {:?}",
        names
    );
    assert!(
        names.contains(&"Osaka"),
        "Should have Osaka series, got: {:?}",
        names
    );
}

#[test]
fn test_json_histogram_produces_nonempty_bins() {
    // JSON histogram output should have populated bins, not empty array
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-o",
            "json",
            "-t",
            "histogram",
            "-y",
            "revenue",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("Should be valid JSON");
    let bins = json["chart_data"]["bins"]
        .as_array()
        .expect("chart_data.bins should be an array");
    assert!(
        !bins.is_empty(),
        "Histogram bins should not be empty, got: {}",
        serde_json::to_string_pretty(&json["chart_data"]).unwrap()
    );
    // Each bin should have range and count
    assert!(bins[0]["range"].is_string());
    assert!(bins[0]["count"].is_number());
}

#[test]
fn test_stderr_summary_no_ansi_when_piped() {
    // When stderr is piped (as in test harness), summary should NOT contain ANSI escape codes
    let output = vz_binary()
        .args(["fixtures/sales.csv"])
        .env_remove("FORCE_COLOR")
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Summary line should be present but without escape sequences
    assert!(!stderr.is_empty(), "stderr should contain a summary line");
    assert!(
        !stderr.contains("\x1b["),
        "stderr should not contain ANSI escapes when piped, got: {:?}",
        stderr
    );
}

#[test]
fn test_output_markdown_produces_valid_table() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-o", "markdown"])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain markdown table separators
    assert!(
        stdout.contains("|---"),
        "Expected markdown table separator, got:\n{}",
        stdout
    );
    // Should contain header columns
    assert!(stdout.contains("date"), "Expected 'date' column in output");
    assert!(
        stdout.contains("revenue"),
        "Expected 'revenue' column in output"
    );
}

#[test]
fn test_output_markdown_shorthand_flag() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--markdown"])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("|---"));
}

#[test]
fn test_output_markdown_with_bar_chart() {
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-o",
            "markdown",
            "-t",
            "bar",
            "-x",
            "city",
            "-y",
            "revenue",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Bar chart markdown should have aggregated data
    assert!(
        stdout.contains("city"),
        "Expected 'city' in markdown output"
    );
    assert!(
        stdout.contains("revenue"),
        "Expected 'revenue' in markdown output"
    );
    assert!(stdout.contains("|---"));
}

#[test]
fn test_empty_stdin_gives_clear_error() {
    // Empty stdin should say "empty input" not "only headers"
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vz"))
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(
        !combined.contains("only headers"),
        "Empty stdin should NOT say 'only headers'. Got: {}",
        combined
    );
    assert!(
        combined.contains("empty") || combined.contains("no data"),
        "Empty stdin should mention 'empty' or 'no data'. Got: {}",
        combined
    );
}

#[test]
fn test_output_table_respects_sort_asc() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vz"))
        .args([
            "fixtures/sales.csv",
            "-o",
            "table",
            "-x",
            "city",
            "-y",
            "revenue",
            "--sort",
            "asc",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Ascending: Nagoya (800) should come before Osaka (3300) before Tokyo (4200)
    let nagoya_pos = stdout.find("Nagoya").expect("Nagoya not in output");
    let osaka_pos = stdout.find("Osaka").expect("Osaka not in output");
    let tokyo_pos = stdout.find("Tokyo").expect("Tokyo not in output");
    assert!(
        nagoya_pos < osaka_pos && osaka_pos < tokyo_pos,
        "Expected ascending sort: Nagoya < Osaka < Tokyo. Got:\n{}",
        stdout
    );
}

#[test]
fn test_output_table_respects_top_flag() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vz"))
        .args([
            "fixtures/sales.csv",
            "-o",
            "table",
            "-x",
            "city",
            "-y",
            "revenue",
            "--top",
            "2",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    // Header + separator + 2 data rows = 4 lines
    assert_eq!(
        lines.len(),
        4,
        "Expected 4 lines (header + sep + 2 rows), got {}:\n{}",
        lines.len(),
        stdout
    );
}

#[test]
fn test_output_markdown_respects_top_flag() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vz"))
        .args([
            "fixtures/sales.csv",
            "-o",
            "markdown",
            "-x",
            "city",
            "-y",
            "revenue",
            "--top",
            "2",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    // Markdown: header + separator + 2 data rows = 4 lines
    assert_eq!(
        lines.len(),
        4,
        "Expected 4 lines (header + sep + 2 rows), got {}:\n{}",
        lines.len(),
        stdout
    );
}

#[test]
fn test_output_markdown_respects_sort_desc() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vz"))
        .args([
            "fixtures/sales.csv",
            "-o",
            "markdown",
            "-x",
            "city",
            "-y",
            "revenue",
            "--sort",
            "desc",
        ])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Descending: Tokyo (4200) should come before Osaka (3300) before Nagoya (800)
    let tokyo_pos = stdout.find("Tokyo").expect("Tokyo not in output");
    let osaka_pos = stdout.find("Osaka").expect("Osaka not in output");
    let nagoya_pos = stdout.find("Nagoya").expect("Nagoya not in output");
    assert!(
        tokyo_pos < osaka_pos && osaka_pos < nagoya_pos,
        "Expected descending sort: Tokyo < Osaka < Nagoya. Got:\n{}",
        stdout
    );
}

#[test]
fn test_header_only_input_no_duplicate_tip() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vz"))
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"a,b\n").unwrap();
            child.wait_with_output()
        })
        .unwrap();
    let combined = String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr);
    // The tip should appear exactly ONCE, not twice
    let tip_count = combined.matches("vz file.csv --no-header").count();
    assert_eq!(
        tip_count, 1,
        "Tip should appear exactly once, but appeared {} times.\nOutput:\n{}",
        tip_count, combined
    );
}

#[test]
fn test_spark_output_shows_column_context() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vz"))
        .args(["fixtures/sales.csv", "-o", "spark"])
        .output()
        .expect("Failed to run vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show the Y column name somewhere in the output
    assert!(
        stdout.contains("revenue"),
        "Spark output should show the Y column name. Got:\n{}",
        stdout
    );
}

// === Directory mode integration tests ===

#[test]
fn test_directory_same_schema_renders_chart() {
    let output = vz_binary()
        .arg("fixtures/dir_test/same_schema/")
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("3 files, 9 rows"), "stderr: {stderr}");
    assert!(stdout.contains("revenue"), "stdout: {stdout}");
}

#[test]
fn test_directory_with_color_source() {
    let output = vz_binary()
        .args(["fixtures/dir_test/same_schema/", "-c", "_source", "--spark"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stdout.contains("sales_2024-01"), "stdout: {stdout}");
    assert!(stdout.contains("sales_2024-02"), "stdout: {stdout}");
    assert!(stdout.contains("sales_2024-03"), "stdout: {stdout}");
}

#[test]
fn test_directory_info_shows_columns() {
    let output = vz_binary()
        .args(["fixtures/dir_test/same_schema/", "--info"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("_source"));
    assert!(stdout.contains("date"));
    assert!(stdout.contains("revenue"));
    assert!(stdout.contains("Rows: 9"));
}

#[test]
fn test_directory_mixed_schema_skips_mismatch() {
    let output = vz_binary()
        .arg("fixtures/dir_test/mixed_schema/")
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success());
    assert!(stderr.contains("1 skipped"), "stderr: {stderr}");
    assert!(stderr.contains("schema mismatch"), "stderr: {stderr}");
}

#[test]
fn test_directory_ragged_csv_does_not_panic() {
    // Directory with reordered columns AND ragged rows (fewer fields than header).
    // Must not panic — should combine gracefully with empty strings for missing fields.
    let output = vz_binary()
        .args(["fixtures/dir_test/ragged/", "--json"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "Should handle ragged CSV without panic. stderr: {stderr}"
    );
    // Should combine both files (2 files, 6 rows total)
    assert!(stderr.contains("2 files"), "stderr: {stderr}");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");
    assert!(parsed.is_object(), "stdout: {stdout}");
}

#[test]
fn test_directory_empty_fails_with_error() {
    let output = vz_binary()
        .arg("fixtures/dir_test/empty/")
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("no data files"),
        "stderr: {stderr}"
    );
}

#[test]
fn test_directory_glob_filters_files() {
    let output = vz_binary()
        .args([
            "fixtures/dir_test/same_schema/",
            "--glob",
            "sales_2024-01*",
            "--spark",
        ])
        .output()
        .expect("Failed to run vz");

    let _stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("1 files"), "stderr: {stderr}");
}

#[test]
fn test_directory_json_output() {
    let output = vz_binary()
        .args(["fixtures/dir_test/same_schema/", "--json"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // JSON output should be parseable
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");
    assert!(parsed.is_object());
}

#[test]
fn test_directory_single_file() {
    let output = vz_binary()
        .args(["fixtures/dir_test/single_file/", "--spark"])
        .output()
        .expect("Failed to run vz");

    let _stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("1 files, 3 rows"), "stderr: {stderr}");
}

// === --recurse flag integration tests (Cycle 1) ===

#[test]
fn test_directory_recurse_finds_nested_files() {
    let output = vz_binary()
        .args(["fixtures/dir_test/nested/", "--recurse", "--spark"])
        .output()
        .expect("Failed to run vz");

    let _stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stderr: {stderr}");
    // Should find 5 files (top_a, top_b, sub1/deep_a, sub1/sub1_inner/bottom, sub2/deep_b)
    assert!(stderr.contains("5 files"), "stderr: {stderr}");
}

#[test]
fn test_directory_recurse_short_flag() {
    let output = vz_binary()
        .args(["fixtures/dir_test/nested/", "-R", "--spark"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("5 files"), "stderr: {stderr}");
}

#[test]
fn test_directory_recurse_source_shows_relative_path() {
    let output = vz_binary()
        .args(["fixtures/dir_test/nested/", "-R", "--json"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // _source should contain relative path entries in JSON output
    assert!(
        stdout.contains("sub1/deep_a"),
        "stdout should contain relative path 'sub1/deep_a' in _source: {stdout}"
    );
    assert!(
        stdout.contains("sub2/deep_b"),
        "stdout should contain relative path 'sub2/deep_b' in _source: {stdout}"
    );
}

#[test]
fn test_directory_recurse_excludes_hidden_dirs() {
    let output = vz_binary()
        .args(["fixtures/dir_test/nested/", "-R", "--spark"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    // 5 files total (hidden dir excluded), total rows = 3+3+3+2+3 = 14
    assert!(stderr.contains("14 rows"), "stderr: {stderr}");
}

#[test]
fn test_directory_no_recurse_only_top_level() {
    let output = vz_binary()
        .args(["fixtures/dir_test/nested/", "--spark"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    // Without --recurse, only top-level files: top_a + top_b = 2 files, 6 rows
    assert!(stderr.contains("2 files"), "stderr: {stderr}");
    assert!(stderr.contains("6 rows"), "stderr: {stderr}");
}

#[test]
fn test_directory_recurse_with_glob() {
    let output = vz_binary()
        .args([
            "fixtures/dir_test/nested/",
            "-R",
            "--glob",
            "deep_*",
            "--spark",
        ])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    // Only deep_a.csv and deep_b.csv match the glob at any level
    assert!(stderr.contains("2 files"), "stderr: {stderr}");
}

// === Catalog mode integration tests ===

#[test]
fn test_catalog_flag_basic() {
    let output = vz_binary()
        .args(["fixtures/dir_test/same_schema/", "--catalog"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("date"), "stdout: {stdout}");
    assert!(stdout.contains("city"), "stdout: {stdout}");
    assert!(stdout.contains("revenue"), "stdout: {stdout}");
    assert!(stdout.contains("sales_2024-01"), "stdout: {stdout}");
    assert!(stdout.contains("3 files"), "stdout: {stdout}");
}

#[test]
fn test_catalog_json_output() {
    let output = vz_binary()
        .args(["fixtures/dir_test/mixed_schema/", "--catalog", "--json"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse catalog JSON");
    assert_eq!(parsed["version"], 1);
    assert!(parsed["groups"].is_array());
    assert_eq!(parsed["groups"].as_array().unwrap().len(), 2);
}

#[test]
fn test_catalog_with_recurse() {
    let output = vz_binary()
        .args(["fixtures/dir_test/nested/", "--catalog", "-R"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Should contain relative paths with /
    assert!(
        stdout.contains("sub1/") || stdout.contains("sub2/"),
        "recursive catalog should show relative paths: {stdout}"
    );
}

#[test]
fn test_catalog_on_file_errors() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--catalog"])
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("directory"),
        "should mention directory requirement: {stderr}"
    );
}

#[test]
fn test_catalog_with_glob_filter() {
    let output = vz_binary()
        .args([
            "fixtures/dir_test/same_schema/",
            "--catalog",
            "--glob",
            "sales_2024-01*",
        ])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("sales_2024-01"), "stdout: {stdout}");
    assert!(
        !stdout.contains("sales_2024-02"),
        "should not include non-matching files: {stdout}"
    );
    assert!(stdout.contains("1 file"), "stdout: {stdout}");
}

#[test]
fn test_catalog_empty_directory_errors() {
    let output = vz_binary()
        .args(["fixtures/dir_test/empty/", "--catalog"])
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no data files"),
        "should report no data files: {stderr}"
    );
}

// === Auto-sampling integration tests ===

#[test]
fn test_directory_auto_sampling_triggers_warning() {
    // Create a temp dir with enough rows to exceed 1M
    let dir = tempfile::tempdir().unwrap();
    // 20 files × 60,000 rows = 1,200,000 total rows (exceeds 1M limit)
    for i in 0..20 {
        let path = dir.path().join(format!("data_{:02}.csv", i));
        let mut f = std::fs::File::create(&path).unwrap();
        use std::io::Write;
        writeln!(f, "x,y").unwrap();
        for j in 0..60_000 {
            writeln!(f, "{},{}", j, j * 2 + i).unwrap();
        }
    }

    let output = vz_binary()
        .args([dir.path().to_str().unwrap(), "--json"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(
        stderr.contains("auto-sampled"),
        "Expected auto-sampling warning, got: {stderr}"
    );
}

#[test]
fn test_directory_no_limit_flag_bypasses_sampling() {
    // Create a temp dir with rows exceeding 1M, use --no-limit
    let dir = tempfile::tempdir().unwrap();
    // 20 files × 60,000 rows = 1,200,000 total rows
    for i in 0..20 {
        let path = dir.path().join(format!("data_{:02}.csv", i));
        let mut f = std::fs::File::create(&path).unwrap();
        use std::io::Write;
        writeln!(f, "x,y").unwrap();
        for j in 0..60_000 {
            writeln!(f, "{},{}", j, j * 2 + i).unwrap();
        }
    }

    let output = vz_binary()
        .args([dir.path().to_str().unwrap(), "--no-limit", "--json"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(
        !stderr.contains("auto-sampled"),
        "Should NOT auto-sample with --no-limit: {stderr}"
    );
}

#[test]
fn test_directory_explicit_sample_flag_independent() {
    // --sample works independently from auto-sampling (small dataset, no auto-sampling)
    let output = vz_binary()
        .args(["fixtures/dir_test/same_schema/", "--sample", "3", "--json"])
        .output()
        .expect("Failed to run vz");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");

    // Should NOT mention "auto-sampled" — dataset is small
    assert!(
        !stderr.contains("auto-sampled"),
        "Should not auto-sample small data: {stderr}"
    );

    // Should show sample info from --sample flag
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "Expected JSON output");
}

// === Mixed format directory tests (Cycle 5) ===

#[test]
fn test_directory_mixed_format_renders_chart() {
    let output = vz_binary()
        .arg("fixtures/dir_test/mixed_format/")
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("3 files, 6 rows"), "stderr: {stderr}");
    assert!(stdout.contains("revenue"), "stdout: {stdout}");
}

#[test]
fn test_directory_mixed_format_spark_output() {
    let output = vz_binary()
        .args(["fixtures/dir_test/mixed_format/", "--spark"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stderr.contains("3 files, 6 rows"), "stderr: {stderr}");
    assert!(stdout.contains("revenue"), "stdout: {stdout}");
}

#[test]
fn test_directory_mixed_format_json_output() {
    let output = vz_binary()
        .args(["fixtures/dir_test/mixed_format/", "--json"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse JSON output");
    // Should have 6 rows of data
    let data = parsed["data"].as_array().unwrap();
    assert_eq!(data.len(), 6);

    // Verify data from all 3 formats is present via _source
    let sources: Vec<&str> = data
        .iter()
        .map(|r| r["_source"].as_str().unwrap())
        .collect();
    assert!(sources.contains(&"sales"), "missing CSV data");
    assert!(sources.contains(&"stats"), "missing JSON data");
    assert!(sources.contains(&"summary"), "missing TSV data");
}

#[test]
fn test_directory_mixed_format_info() {
    let output = vz_binary()
        .args(["fixtures/dir_test/mixed_format/", "--info"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("date"), "stdout: {stdout}");
    assert!(stdout.contains("city"), "stdout: {stdout}");
    assert!(stdout.contains("revenue"), "stdout: {stdout}");
    assert!(stdout.contains("Rows: 6"), "stdout: {stdout}");
}

#[test]
fn test_directory_mixed_format_color_by_source() {
    let output = vz_binary()
        .args([
            "fixtures/dir_test/mixed_format/",
            "-c",
            "_source",
            "--spark",
        ])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stderr: {stderr}");
    // Should show all 3 source names
    assert!(stdout.contains("sales"), "stdout: {stdout}");
    assert!(stdout.contains("stats"), "stdout: {stdout}");
    assert!(stdout.contains("summary"), "stdout: {stdout}");
}

#[test]
fn test_fixed_width_kubectl_top_from_file() {
    let output = vz_binary()
        .args(["fixtures/fixed_width/kubectl_top_pods.txt", "--info"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("Rows: 3"), "stdout: {stdout}");
    assert!(stdout.contains("NAME"), "stdout: {stdout}");
    assert!(stdout.contains("CPU(cores)"), "stdout: {stdout}");
    assert!(stdout.contains("MEMORY(bytes)"), "stdout: {stdout}");
}

#[test]
fn test_fixed_width_stdin_auto_detect() {
    let mut child = vz_binary()
        .args(["-", "--info"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn vz");

    let stdin = child.stdin.as_mut().unwrap();
    stdin
        .write_all(b"NAME        CPU    MEM\npod1        100m   256Mi\npod2        200m   512Mi\n")
        .unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("Failed to wait for vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("Rows: 2"), "stdout: {stdout}");
    assert!(stdout.contains("NAME"), "stdout: {stdout}");
    assert!(stdout.contains("CPU"), "stdout: {stdout}");
    assert!(stdout.contains("MEM"), "stdout: {stdout}");
}

#[test]
fn test_fixed_width_format_flag_space() {
    let output = vz_binary()
        .args([
            "fixtures/fixed_width/kubectl_top_pods.txt",
            "-f",
            "space",
            "--info",
        ])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("Rows: 3"), "stdout: {stdout}");
    assert!(stdout.contains("CPU(cores)"), "stdout: {stdout}");
}

#[test]
fn test_fixed_width_stdin_spark_output() {
    let mut child = vz_binary()
        .args(["-", "--spark"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn vz");

    let stdin = child.stdin.as_mut().unwrap();
    stdin
        .write_all(b"NAME CPU MEM\npod1 100m 256Mi\npod2 200m 512Mi\npod3 50m 128Mi\n")
        .unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("Failed to wait for vz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Should produce some spark output
    assert!(!stdout.trim().is_empty(), "stdout should not be empty");
}

#[test]
fn test_fixed_width_separator_lines_handled() {
    let output = vz_binary()
        .args(["fixtures/fixed_width/separator_lines.txt", "--info"])
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("Rows: 3"), "stdout: {stdout}");
    assert!(stdout.contains("Name"), "stdout: {stdout}");
    assert!(stdout.contains("Score"), "stdout: {stdout}");
}

// === Diff mode integration tests ===

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

// --- HTML output tests ---

#[test]
fn test_output_html_basic() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--output", "html"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<!DOCTYPE html>"),
        "HTML should start with doctype"
    );
    assert!(stdout.contains("<svg"), "HTML should contain embedded SVG");
    assert!(
        stdout.contains("</svg>"),
        "HTML should have closing SVG tag"
    );
    assert!(
        stdout.contains("<script>"),
        "HTML should have inline JS for tooltips"
    );
    assert!(
        stdout.contains("</html>"),
        "HTML should have closing html tag"
    );
}

#[test]
fn test_output_html_shorthand_flag() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--html"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(stdout.contains("<svg"));
}

#[test]
fn test_output_html_with_o_flag() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-o", "html"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!DOCTYPE html>"));
}

#[test]
fn test_output_html_no_external_resources() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--html"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // No external script sources (xmlns namespace URIs are fine)
    assert!(
        !stdout.contains("src=\"http"),
        "HTML must not load external scripts"
    );
    assert!(
        !stdout.contains("href=\"http"),
        "HTML must not load external stylesheets"
    );
    assert!(
        !stdout.contains("src=\"//"),
        "HTML must not load protocol-relative scripts"
    );
}

#[test]
fn test_output_html_with_custom_title() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--html", "--title", "My Sales Chart"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<title>My Sales Chart</title>"));
}

#[test]
fn test_output_html_contains_viewbox() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--html"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("viewBox"),
        "Embedded SVG should have viewBox attribute"
    );
}

#[test]
fn test_output_html_with_theme_light() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--html", "--theme", "light"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(
        stdout.contains("#ffffff"),
        "Light theme should use white background"
    );
}

#[test]
fn test_output_html_conflicts_with_svg() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--html", "--svg"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success(), "--html and --svg should conflict");
}

#[test]
fn test_output_html_with_bar_chart() {
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-x",
            "city",
            "-y",
            "revenue",
            "-t",
            "bar",
            "--html",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(stdout.contains("<svg"));
}

#[test]
fn test_output_html_with_custom_dimensions() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--html", "-W", "120", "-H", "30"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(stdout.contains("<svg"));
}

#[test]
fn test_output_html_from_stdin() {
    use std::io::Write;
    let mut child = vz_binary()
        .args(["-", "--html"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start vz");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"city,revenue\nTokyo,1000\nOsaka,2000\n")
        .unwrap();
    let output = child.wait_with_output().expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!DOCTYPE html>"));
    assert!(stdout.contains("<svg"));
}

// ========== Explore Diff Mode Tests ==========

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
fn test_bom_csv_file_renders_correctly() {
    let output = vz_binary()
        .arg("fixtures/bom_sales.csv")
        .output()
        .expect("Failed to run vz");

    assert!(
        output.status.success(),
        "BOM CSV should render. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.lines().count() >= 5, "Chart output too short");
}

#[test]
fn test_bom_csv_x_flag_matches_first_column() {
    let output = vz_binary()
        .args(["fixtures/bom_sales.csv", "-x", "date", "-y", "revenue"])
        .output()
        .expect("Failed to run vz");

    assert!(
        output.status.success(),
        "BOM should not prevent -x from matching first column. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_bom_csv_info_shows_clean_column_names() {
    let output = vz_binary()
        .args(["fixtures/bom_sales.csv", "--info"])
        .output()
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("date"),
        "First column should be 'date' without BOM prefix"
    );
    assert!(
        !stdout.contains('\u{feff}'),
        "BOM character leaked into --info output"
    );
}

#[test]
fn test_bom_stdin_pipe() {
    use std::process::Stdio;

    let mut child = vz_binary()
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn vz");

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(
                b"\xEF\xBB\xBFdate,city,revenue\n2024-01-01,Tokyo,1000\n2024-02-01,Osaka,1500\n",
            )
            .unwrap();
    }

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "BOM in stdin should be handled. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// =============================================================================
// Cycle 27: Output value assertions (spark, markdown, html, diff)
// =============================================================================

#[test]
fn test_spark_output_values_range_and_trend() {
    // sales.csv has revenue: 1000, 1500, 1200, 800, 2000, 1800
    // Range: 800–2000 → formatted as (800–2k), Trend: first=1000, last=1800 → ↑ +80%
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--spark"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("(800–2k)"),
        "Expected range (800–2k) in spark output, got: {}",
        stdout
    );
    assert!(
        stdout.contains("↑ +80%"),
        "Expected trend ↑ +80% in spark output, got: {}",
        stdout
    );
}

#[test]
fn test_spark_output_sparkline_char_count() {
    // sales.csv has 6 rows → sparkline should have exactly 6 chars
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--spark"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    let parts: Vec<&str> = trimmed.split("  ").collect();
    assert!(parts.len() >= 2, "Expected parts, got: {}", trimmed);
    let spark_chars: Vec<char> = parts[1].chars().collect();
    assert_eq!(
        spark_chars.len(),
        6,
        "Expected 6 sparkline chars for 6 rows, got {} in: {}",
        spark_chars.len(),
        parts[1]
    );
}

#[test]
fn test_spark_bar_aggregation_values() {
    // Bar with 3 categories → 3 sparkline chars, range (800–4.2k)
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "--spark",
            "-t",
            "bar",
            "-x",
            "city",
            "-y",
            "revenue",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    let parts: Vec<&str> = trimmed.split("  ").collect();
    assert!(parts.len() >= 2, "Expected parts, got: {}", trimmed);
    let spark_chars: Vec<char> = parts[1].chars().collect();
    assert_eq!(
        spark_chars.len(),
        3,
        "Expected 3 sparkline chars for 3 categories, got {} in: {}",
        spark_chars.len(),
        parts[1]
    );
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
fn test_output_markdown_bar_aggregated_values() {
    // Bar chart markdown should show aggregated sums
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-o",
            "markdown",
            "-t",
            "bar",
            "-x",
            "city",
            "-y",
            "revenue",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("| city | revenue |"),
        "Expected header, got: {}",
        stdout
    );
    assert!(
        stdout.contains("| Tokyo | 4200 |"),
        "Expected Tokyo sum 4200, got: {}",
        stdout
    );
    assert!(
        stdout.contains("| Osaka | 3300 |"),
        "Expected Osaka sum 3300, got: {}",
        stdout
    );
    assert!(
        stdout.contains("| Nagoya | 800 |"),
        "Expected Nagoya sum 800, got: {}",
        stdout
    );
    // Exactly 3 data rows
    let data_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("| ") && !l.contains("city") && !l.starts_with("|---"))
        .collect();
    assert_eq!(
        data_lines.len(),
        3,
        "Expected 3 data rows for 3 cities, got: {:?}",
        data_lines
    );
}

#[test]
fn test_output_markdown_raw_data_values() {
    // Raw markdown should have all data rows with exact values
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-o", "markdown"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // All 4 columns present in header
    assert!(
        stdout.contains("| date | city | revenue | profit |"),
        "Expected all 4 columns in header, got: {}",
        stdout
    );
    // Verify first and last data rows
    assert!(
        stdout.contains("| 2024-01-01 | Tokyo | 1000 | 200 |"),
        "Expected first row, got: {}",
        stdout
    );
    assert!(
        stdout.contains("| 2024-06-01 | Osaka | 1800 | 400 |"),
        "Expected last row, got: {}",
        stdout
    );
    // 6 data rows total
    let data_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.starts_with("| 2024-"))
        .collect();
    assert_eq!(
        data_lines.len(),
        6,
        "Expected 6 data rows, got: {:?}",
        data_lines
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
fn test_output_html_title_default() {
    // Default HTML title should be "vz chart"
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--html"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<title>vz chart</title>"),
        "Expected default title 'vz chart'"
    );
}

#[test]
fn test_output_html_structure_complete() {
    // Verify complete HTML document structure
    let output = vz_binary()
        .args(["fixtures/sales.csv", "--html"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Exactly one html open/close
    assert_eq!(
        stdout.matches("<html").count(),
        1,
        "Expected exactly one <html> tag"
    );
    assert_eq!(
        stdout.matches("</html>").count(),
        1,
        "Expected exactly one </html> tag"
    );
    // Has viewport meta for responsive
    assert!(
        stdout.contains("width=device-width, initial-scale=1"),
        "Expected responsive viewport meta"
    );
    // Has chart-container div
    assert!(
        stdout.contains("chart-container"),
        "Expected chart-container div"
    );
    // viewBox with default dimensions
    assert!(
        stdout.contains("viewBox=\"0 0 640 384\""),
        "Expected default viewBox dimensions"
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
