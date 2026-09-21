//! Snapshot tests for chart output using insta.
//! These tests capture the exact rendered output and detect visual regressions.

#[path = "common/mod.rs"]
mod common;

/// Run vz with NO_COLOR and fixed width, capturing stdout.
fn run_vz(args: &[&str]) -> String {
    common::run_vz_stdout(args)
}

#[test]
fn snapshot_bar_chart() {
    let output = run_vz(&["fixtures/sales.csv", "-x", "city", "-y", "revenue"]);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_line_chart_default() {
    let output = run_vz(&["fixtures/sales.csv"]);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_histogram() {
    let output = run_vz(&["fixtures/sales.csv", "-y", "revenue", "-t", "histogram"]);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_json_input() {
    let output = common::vz_no_color()
        .arg("-")
        .env("COLUMNS", "80")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            let stdin = child.stdin.as_mut().unwrap();
            stdin
                .write_all(
                    br#"[{"name":"Alice","score":85},{"name":"Bob","score":92},{"name":"Charlie","score":78}]"#,
                )
                .unwrap();
            child.wait_with_output()
        })
        .expect("Failed to run vz");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    insta::assert_snapshot!(stdout);
}

#[test]
fn snapshot_diff_bar() {
    let output = run_vz(&[
        "fixtures/diff/sales_before.csv",
        "fixtures/diff/sales_after.csv",
    ]);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_diff_temporal_line() {
    let output = run_vz(&[
        "fixtures/diff/ts_daily_before.csv",
        "fixtures/diff/ts_daily_after.csv",
    ]);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_svg_bar() {
    let output = run_vz(&[
        "fixtures/sales.csv",
        "-x",
        "city",
        "-y",
        "revenue",
        "--output",
        "svg",
    ]);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_html_default() {
    let output = run_vz(&["fixtures/sales.csv", "--output", "html"]);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_html_light_theme() {
    let output = run_vz(&["fixtures/sales.csv", "--html", "--theme", "light"]);
    insta::assert_snapshot!(output);
}

#[test]
fn snapshot_heatmap() {
    let output = run_vz(&["fixtures/departments.csv"]);
    insta::assert_snapshot!(output);
}
