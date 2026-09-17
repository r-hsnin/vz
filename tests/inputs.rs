//! Input formats end-to-end tests for vz.

#[path = "common/mod.rs"]
mod common;
use common::vz_binary;
use std::io::Write;

#[test]
fn test_csv_with_only_numeric_columns_renders_scatter() {
    let f = common::temp_csv(&[
        "height,weight,age",
        "170,65,30",
        "175,72,28",
        "180,80,35",
        "165,58,25",
    ]);

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Scatter plot renders a chart with borders
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
    assert!(
        stdout.contains('│') || stdout.contains('─') || stdout.contains("Scatter"),
        "Scatter chart not rendered"
    );
}

#[test]
fn test_csv_single_numeric_column_renders_histogram() {
    let f = common::temp_csv(&["score", "85", "90", "78", "92", "88"]);

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Histogram renders with bin labels and bars
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
    assert!(
        stdout.contains("Distribution") || stdout.contains("score") || stdout.contains('│'),
        "Histogram not rendered:\n{}",
        stdout
    );
}

#[test]
fn test_csv_categorical_only_renders_chart() {
    let f = common::temp_csv(&[
        "department,status",
        "Engineering,Active",
        "Sales,Active",
        "Engineering,Inactive",
        "Marketing,Active",
    ]);

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
    let f = common::temp_csv(&[
        "date,value",
        "2024-01-01,100",
        "2024-02-01,",
        "2024-03-01,300",
        ",400",
    ]);

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should still render a chart despite nulls
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
}

#[test]
fn test_csv_with_comma_numbers_renders() {
    let f = common::temp_csv(&[
        "city,population",
        "Tokyo,\"13,960,000\"",
        "Osaka,\"2,753,000\"",
        "Nagoya,\"2,320,000\"",
    ]);

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should render a bar chart (labels may be truncated)
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
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
fn test_large_csv_renders() {
    let mut rows = vec!["id,value,category".to_string()];
    for i in 0..1000 {
        rows.push(format!(
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
        ));
    }
    let f = common::temp_csv(&rows);

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should handle 1000 rows and render a chart
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
}

#[test]
fn test_unicode_column_names_renders() {
    let f = common::temp_csv(&[
        "日付,都市,売上",
        "2024-01-01,東京,1000",
        "2024-02-01,大阪,1500",
        "2024-03-01,名古屋,800",
    ]);

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should render a chart with unicode column names
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
    // The chart title includes column names (may have spaces between wide chars)
    assert!(
        stdout.contains('売') || stdout.contains('日'),
        "Unicode column name characters not in chart:\n{}",
        stdout
    );
}

#[test]
fn test_tsv_input() {
    let f = common::temp_csv_with_suffix(
        ".tsv",
        &[
            "city\trevenue\tprofit",
            "Tokyo\t1000\t200",
            "Osaka\t1500\t350",
            "Nagoya\t800\t150",
        ],
    );

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    // Should render a bar chart with city labels
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
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
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
}

#[test]
fn test_json_array_input() {
    let f = common::temp_csv_with_suffix(
        ".json",
        &[
            "[",
            "        {\"date\": \"2024-01-01\", \"revenue\": 1000},",
            "        {\"date\": \"2024-02-01\", \"revenue\": 1500},",
            "        {\"date\": \"2024-03-01\", \"revenue\": 1200},",
            "        {\"date\": \"2024-04-01\", \"revenue\": 1800}",
            "    ]",
        ],
    );

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
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
    assert!(
        stdout.contains("Line") || stdout.contains("revenue"),
        "JSON array should render a line chart:\n{}",
        stdout
    );
}

#[test]
fn test_ndjson_input() {
    let f = common::temp_csv_with_suffix(
        ".ndjson",
        &[
            "{\"x\": 1, \"y\": 10}",
            "{\"x\": 2, \"y\": 20}",
            "{\"x\": 3, \"y\": 30}",
            "{\"x\": 4, \"y\": 25}",
        ],
    );

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
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
    assert!(
        stdout.contains("Scatter") || stdout.contains('•') || stdout.contains('│'),
        "NDJSON should render a scatter chart:\n{}",
        stdout
    );
}

#[test]
fn test_ndjson_invalid_line_reports_line_number() {
    let f = common::temp_csv_with_suffix(
        ".ndjson",
        &[
            "{\"x\": 1, \"y\": 10}",
            "{\"x\": 2, \"y\": 20}",
            "not json",
            "{\"x\": 4, \"y\": 25}",
        ],
    );

    let output = vz_binary()
        .arg(f.path())
        .output()
        .expect("Failed to run vz");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("line 3"),
        "stderr should name the bad line:\n{}",
        stderr
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
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
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
    assert!(stdout.lines().count() >= common::MIN_CHART_LINES);
}

#[test]
fn test_header_only_csv_gives_clear_error() {
    let file = common::temp_csv(&["x,y"]);

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
fn test_no_header_flag_treats_first_row_as_data() {
    let file = common::temp_csv_with_suffix(".csv", &["1,10", "2,20", "3,30"]);

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
    // All-numeric "headers" — should auto-detect as no-header
    let file = common::temp_csv_with_suffix(".csv", &["1,100", "2,200", "3,300"]);

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
fn test_format_flag_forces_tsv() {
    // Create a TSV file with .txt extension (would be detected as CSV without --format)
    let tmp =
        common::temp_csv_with_suffix(".txt", &["city\trevenue", "Tokyo\t1000", "Osaka\t2000"]);

    let output = common::vz_command()
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
    let tmp = common::temp_csv_with_suffix(".dat", &["city\trevenue", "Tokyo\t1000"]);

    let output = common::vz_command()
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
fn test_stdin_auto_detect_without_dash() {
    use std::process::Stdio;
    let mut child = common::vz_command()
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
fn test_malformed_csv_row_warning() {
    // The csv crate with flexible(true) tolerates most malformations.
    // Verify that vz handles edge cases gracefully without crashing.
    // Fewer/more fields than the header (flexible mode tolerates them).
    let f = common::temp_csv(&["name,val", "Alice,10", "Bob", "Charlie,30,extra"]);

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
fn test_empty_stdin_gives_clear_error() {
    // Empty stdin should say "empty input" not "only headers"
    let output = common::vz_command()
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
fn test_header_only_input_no_duplicate_tip() {
    let output = common::vz_command()
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
