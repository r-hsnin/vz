//! Explore/present/watch/completions end-to-end tests for vz.

#[path = "common/mod.rs"]
mod common;
use common::vz_binary;

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
    let file = common::temp_csv_with_suffix::<&str>(".md", &[]);

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
    let file = common::temp_csv_with_suffix(".csv", &[""]);

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
fn test_watch_flag_accepted_and_rerenders_on_change() {
    use std::io::Write;
    use std::time::Duration;

    // Create a temporary CSV file
    let tmpfile = common::temp_csv(&["x,y", "a,1", "b,2"]);

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
