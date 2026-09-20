//! Output formats end-to-end tests for vz.

#[path = "common/mod.rs"]
mod common;
use common::vz_binary;

#[test]
fn test_no_color_strips_ansi() {
    let output = common::vz_no_color()
        .arg("fixtures/sales.csv")
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
fn test_summary_shows_unused_columns() {
    let file = common::temp_csv_with_suffix(
        ".csv",
        &[
            "date,revenue,profit,city",
            "2024-01-01,100,50,Tokyo",
            "2024-02-01,200,80,Osaka",
        ],
    );

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
    let file = common::temp_csv(&["city,revenue", "Tokyo,1000", ",500", "Osaka,800"]);

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
fn test_json_histogram_x_only_bins_the_x_column() {
    // `-x <quantitative>` alone must bin the x column, like oneshot text does
    // (regression: JSON fell back to the Y slot and produced `"bins": []`).
    let output = vz_binary()
        .args([
            "fixtures/temperature.csv",
            "-o",
            "json",
            "-t",
            "histogram",
            "-x",
            "temperature",
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
        "`-x temperature` histogram must bin temperature, got: {}",
        serde_json::to_string_pretty(&json["chart_data"]).unwrap()
    );
    assert_eq!(json["query"]["x"], "temperature");
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
fn test_output_table_respects_sort_asc() {
    let output = common::vz_command()
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
    let output = common::vz_command()
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
    let output = common::vz_command()
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
    let output = common::vz_command()
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
fn test_spark_output_shows_column_context() {
    let output = common::vz_command()
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
fn test_help_lists_html_output_format() {
    let output = vz_binary()
        .arg("--help")
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let output_line = stdout
        .lines()
        .find(|l| l.contains("Output format"))
        .expect("--help should describe the output format");
    assert!(
        output_line.contains("html"),
        "--help should document the html output format, got: {}",
        output_line
    );
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
fn test_spark_bar_limit_truncates_via_canonical_adapter() {
    // --top 2 keeps 2 categories → 2 sparkline chars/range (3.3k–4.2k)
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
            "--top",
            "2",
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
        2,
        "Expected 2 sparkline chars after --top 2, got {} in: {}",
        spark_chars.len(),
        parts[1]
    );
}

#[test]
fn test_spark_histogram_no_y_uses_canonical_bins() {
    // Single quantitative column + no Y → histogram; --bins 4 → 4 sparkline chars
    let output = vz_binary()
        .args([
            "fixtures/temperature.csv",
            "--spark",
            "-t",
            "histogram",
            "-x",
            "temperature",
            "--bins",
            "4",
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
        4,
        "Expected 4 histogram bins after --bins 4, got {} in: {}",
        spark_chars.len(),
        parts[1]
    );
    assert!(
        trimmed.contains("36 rows"),
        "Expected canonical histogram row count in: {}",
        trimmed
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
        stdout.contains("| Tokyo | 4.2k |"),
        "Expected Tokyo sum 4.2k, got: {}",
        stdout
    );
    assert!(
        stdout.contains("| Osaka | 3.3k |"),
        "Expected Osaka sum 3.3k, got: {}",
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
fn help_shows_examples_legend_and_stream_split() {
    let output = vz_binary()
        .arg("--help")
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Examples:"), "missing Examples:\n{stdout}");
    assert!(
        stdout.contains("vz sales.csv -x city"),
        "missing example:\n{stdout}"
    );
    assert!(stdout.contains("→ stable"), "missing legend:\n{stdout}");
    assert!(
        stdout.contains("summary and warnings go to stderr"),
        "missing stream split:\n{stdout}"
    );
}

#[test]
fn test_json_non_finite_becomes_null_not_zero() {
    let f = common::temp_csv(&["city,revenue", "Tokyo,NaN", "Osaka,inf", "Kyoto,100"]);
    let output = common::vz_no_color()
        .arg(f.path())
        .arg("-o")
        .arg("json")
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let data = v["data"].as_array().unwrap();
    assert!(
        data[0]["revenue"].is_null(),
        "NaN must be null, got {}",
        data[0]["revenue"]
    );
    assert!(
        data[1]["revenue"].is_null(),
        "inf must be null, got {}",
        data[1]["revenue"]
    );
}

#[test]
fn test_json_truncated_flag() {
    let mut rows = vec!["date,revenue".to_string()];
    for i in 0..150 {
        rows.push(format!("2024-01-{:02},{}", (i % 28) + 1, 1000 + i));
    }
    let f = common::temp_csv(&rows);
    let output = common::vz_no_color()
        .arg(f.path())
        .arg("-o")
        .arg("json")
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["rows"], 150);
    assert_eq!(v["data"].as_array().unwrap().len(), 100);
    assert_eq!(v["truncated"], true);
}

#[test]
fn test_spark_skips_non_finite_values() {
    let f = common::temp_csv(&[
        "date,revenue",
        "2024-01-01,100",
        "2024-02-01,NaN",
        "2024-03-01,inf",
        "2024-04-01,200",
    ]);
    let output = common::vz_no_color()
        .arg(f.path())
        .arg("-o")
        .arg("spark")
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.to_lowercase().contains("inf") && !stdout.to_lowercase().contains("nan"),
        "non-finite leaked into spark output: {}",
        stdout
    );
}

#[test]
fn test_chart_json_series_skips_non_finite_points() {
    let f = common::temp_csv(&[
        "date,revenue",
        "2024-01-01,100",
        "2024-02-01,NaN",
        "2024-03-01,inf",
        "2024-04-01,200",
    ]);
    let output = common::vz_no_color()
        .arg(f.path())
        .arg("-o")
        .arg("json")
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let series = v["chart_data"]["series"].as_array().unwrap();
    let data = series[0]["data"].as_array().unwrap();
    assert_eq!(data.len(), 2, "non-finite points must be skipped: {}", v);
    assert!(data.iter().all(|p| p["y"].is_number()));
}

#[test]
fn test_output_table_count_agg_auto_for_categorical_y() {
    // departments.csv has two categorical columns; -t bar must auto-count
    // (same effective_agg path as chart rendering), not sum garbage.
    let output = vz_binary()
        .args(["fixtures/departments.csv", "-t", "bar", "-o", "table"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("count("),
        "Expected count() agg header, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Engineering") && stdout.contains('3'),
        "Expected Engineering count 3, got:\n{}",
        stdout
    );
}

#[test]
fn test_output_table_truncates_large_data() {
    let f = common::temp_csv_with_suffix(".csv", &{
        let mut rows = vec!["x,y".to_string()];
        rows.extend((0..150).map(|i| format!("a{i},{i}")));
        rows
    });
    let output = vz_binary()
        .args([f.path().to_str().unwrap(), "-o", "table"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // header + separator + 100 rows
    assert_eq!(
        stdout.lines().count(),
        102,
        "Expected truncation to 100 rows, got {} lines",
        stdout.lines().count()
    );
    assert!(
        stderr.contains("showing 100/150 rows"),
        "Expected truncation notice on stderr, got: {}",
        stderr
    );
}

#[test]
fn test_output_table_warns_on_top_for_non_bar() {
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-t",
            "line",
            "-o",
            "table",
            "--top",
            "2",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--top/--tail has no effect"),
        "Expected top-ignore warning, got: {}",
        stderr
    );
}

#[test]
fn test_output_markdown_escapes_pipe_cells() {
    let f = common::temp_csv_with_suffix(".csv", &["a,b", "\"x|y\",2", "q,3"]);
    let output = vz_binary()
        .args([f.path().to_str().unwrap(), "-o", "markdown"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("x\\|y"),
        "Expected escaped pipe cell, got:\n{}",
        stdout
    );
}

#[test]
fn test_spark_multi_y_emits_one_line_per_series() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-y", "revenue,profit", "-o", "spark"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "Expected 2 spark lines, got:\n{}", stdout);
    assert!(lines[0].starts_with("revenue"), "First line: {}", lines[0]);
    assert!(lines[1].starts_with("profit"), "Second line: {}", lines[1]);
}

#[test]
fn test_spark_bar_has_no_trend_arrow() {
    // Bar categories have no time order; endpoint trend would mislead.
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
    assert!(
        !stdout.contains('↑') && !stdout.contains('↓'),
        "Bar spark must not show trend arrow, got: {}",
        stdout
    );
    assert!(stdout.contains("(3.3k–4.2k)"), "Range: {}", stdout);
}

#[test]
fn test_json_query_provenance_records_overrides() {
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-x",
            "city",
            "-y",
            "revenue",
            "-t",
            "bar",
            "--agg",
            "mean",
            "--sort",
            "desc",
            "--top",
            "2",
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let q = &v["query"];
    assert_eq!(q["chart_type"], "bar", "query: {}", q);
    assert_eq!(q["agg"], "mean", "query: {}", q);
    assert_eq!(q["sort"], "desc", "query: {}", q);
    assert_eq!(q["limit"], 2, "query: {}", q);
    assert_eq!(q["x"], "city", "query: {}", q);
    assert_eq!(q["y"], "revenue", "query: {}", q);
}

#[test]
fn test_json_query_records_extra_y_and_filters() {
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-y",
            "revenue,profit",
            "-w",
            "revenue>1000",
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let q = &v["query"];
    assert_eq!(q["extra_y"], serde_json::json!(["profit"]), "query: {}", q);
    assert_eq!(
        q["filters"],
        serde_json::json!(["revenue>1000"]),
        "query: {}",
        q
    );
}

#[test]
fn test_insights_line_on_stderr() {
    let output = vz_binary()
        .args(["fixtures/sales.csv"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("💡 revenue rose 80% overall (1k → 1.8k)."),
        "expected line insight, got: {}",
        stderr
    );
    assert!(
        stderr.contains("💡 Peaked at 2k (2024-05-01)"),
        "expected extremes, got: {}",
        stderr
    );
}

#[test]
fn test_insights_bar_names_leader() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-x", "city", "-y", "revenue"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("💡 Tokyo leads with 4.2k (51% of total)."),
        "expected bar insight, got: {}",
        stderr
    );
}

#[test]
fn test_insights_histogram_and_heatmap() {
    let hist = vz_binary()
        .args(["fixtures/sales.csv", "-t", "histogram", "-y", "revenue"])
        .output()
        .expect("Failed to run vz");
    assert!(hist.status.success());
    let stderr = String::from_utf8_lossy(&hist.stderr);
    assert!(
        stderr.contains("💡 Most revenue values"),
        "expected histogram insight, got: {}",
        stderr
    );

    let heat = vz_binary()
        .args(["fixtures/departments.csv"])
        .output()
        .expect("Failed to run vz");
    assert!(heat.status.success());
    let stderr = String::from_utf8_lossy(&heat.stderr);
    assert!(
        stderr.contains("💡 Most common combo: Marketing × Active"),
        "expected heatmap insight, got: {}",
        stderr
    );
}

#[test]
fn test_insights_silent_for_single_point() {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::with_suffix(".csv").unwrap();
    writeln!(f, "date,revenue\n2024-01,100").unwrap();
    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("💡"),
        "single point must stay silent, got: {}",
        stderr
    );
}

#[test]
fn test_json_insights_field_matches_stderr() {
    let output = vz_binary()
        .args(["fixtures/sales.csv", "-o", "json"])
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let insights = v["insights"].as_array().expect("insights must be an array");
    assert!(
        insights
            .iter()
            .any(|s| s.as_str().unwrap_or("").contains("rose 80%")),
        "expected insight in JSON, got: {}",
        v["insights"]
    );
}

#[test]
fn test_insights_no_ansi_when_piped() {
    let output = vz_binary()
        .args(["fixtures/sales.csv"])
        .env_remove("FORCE_COLOR")
        .output()
        .expect("Failed to run vz");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("\x1b["),
        "insights must not add ANSI when piped, got: {:?}",
        stderr
    );
}
