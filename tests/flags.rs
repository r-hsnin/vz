//! Filter/sort/aggregation flags end-to-end tests for vz.

#[path = "common/mod.rs"]
mod common;
use common::vz_binary;

#[test]
fn test_sort_flag_bar_chart() {
    let file = common::temp_csv_with_suffix(
        ".csv",
        &["city,revenue", "Osaka,300", "Tokyo,500", "Nagoya,100"],
    );

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
fn test_where_filter_equality() {
    let output = common::vz_command()
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
    let output = common::vz_command()
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
    let output = common::vz_command()
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
fn test_where_filter_column_typo_suggests_close_match() {
    let output = common::vz_command()
        .args(["fixtures/sales.csv", "--where", "ctiy=Tokyo"])
        .output()
        .expect("failed to execute");

    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Did you mean 'city'?"),
        "Expected filter column suggestion, got: '{}'",
        combined
    );
}

#[test]
fn test_where_filter_doubled_operator_bails_loudly() {
    let output = common::vz_command()
        .args(["fixtures/sales.csv", "--where", "revenue>>100"])
        .output()
        .expect("failed to execute");

    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("single operator"),
        "Expected loud operator error, got: '{}'",
        combined
    );
    assert!(
        !combined.contains("No rows remain"),
        "Must not silently filter everything, got: '{}'",
        combined
    );
}

#[test]
fn test_where_filter_multiple() {
    let output = common::vz_command()
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
    let output = common::vz_command()
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
    let output = common::vz_command()
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
    let output = common::vz_command()
        .args(["fixtures/sales.csv", "--top", "1", "-t", "bar"])
        .output()
        .expect("failed to execute");

    assert!(output.status.success());
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
        stdout.lines().count() >= common::MIN_CHART_LINES,
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
fn test_bins_above_max_gives_clear_error() {
    // Unbounded bin counts allocate without limit (OOM-class); reject clearly.
    let output = vz_binary()
        .args([
            "fixtures/sales.csv",
            "-y",
            "revenue",
            "-t",
            "histogram",
            "--bins",
            "10001",
        ])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--bins must be between 1 and 10000"),
        "Expected clear error for --bins 10001, got: {}",
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
