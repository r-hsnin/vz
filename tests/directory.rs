//! Directory mode end-to-end tests for vz.

#[path = "common/mod.rs"]
mod common;
use common::vz_binary;

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
fn test_directory_bins_zero_gives_clear_error() {
    // Limit validation runs before the directory branch, so directory mode
    // no longer silently renders an empty histogram for --bins 0.
    let output = vz_binary()
        .args(["fixtures/dir_test/same_schema/", "--bins", "0"])
        .output()
        .expect("Failed to run vz");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--bins must be at least 1"),
        "Expected clear error for directory --bins 0, got: {}",
        stderr
    );
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
