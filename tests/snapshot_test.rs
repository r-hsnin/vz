//! Snapshot tests for chart output using insta.
//! These tests capture the exact rendered output and detect visual regressions.

use std::process::Command;

/// Run vz with NO_COLOR and fixed width, capturing stdout.
fn run_vz(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .args(args)
        .env("NO_COLOR", "1")
        .env_remove("FORCE_COLOR")
        .env("COLUMNS", "80")
        .output()
        .expect("Failed to run vz");

    assert!(
        output.status.success(),
        "vz failed with: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("Invalid UTF-8 in output")
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
    let output = Command::new(env!("CARGO_BIN_EXE_vz"))
        .arg("-")
        .env("NO_COLOR", "1")
        .env_remove("FORCE_COLOR")
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
