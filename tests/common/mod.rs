//! Shared helpers for integration and snapshot tests.
//!
//! Centralizes binary construction (single definition of the `vz` under test),
//! environment presets, temp-file fixtures, and common assertion thresholds so
//! that `tests/*.rs` don't re-implement them per file.

// Each test target compiles its own copy of this module but uses only a
// subset, so unused-item warnings would be false positives here.
#![allow(dead_code)]

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Minimum chart height (lines) expected by smoke assertions.
pub const MIN_CHART_LINES: usize = 10;

/// Binary with a dumb terminal (stable width behavior for chart output).
pub fn vz_binary() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vz"));
    cmd.env("TERM", "dumb");
    cmd
}

/// Binary without environment overrides (preserves default terminal behavior).
pub fn vz_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vz"))
}

/// Binary with color disabled (for ANSI-stripping assertions).
pub fn vz_no_color() -> Command {
    let mut cmd = vz_command();
    cmd.env("NO_COLOR", "1").env_remove("FORCE_COLOR");
    cmd
}

/// Deterministic run for snapshot tests: no color, fixed width.
/// Returns stdout, asserting the command succeeded.
pub fn run_vz_stdout(args: &[&str]) -> String {
    let output = vz_no_color()
        .env("COLUMNS", "80")
        .args(args)
        .output()
        .expect("Failed to run vz");

    assert!(
        output.status.success(),
        "vz failed with: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("Invalid UTF-8 in output")
}

/// Write rows (one per line) into a temp file, flushed and ready to read.
pub fn temp_csv<S: AsRef<str>>(rows: &[S]) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    for row in rows {
        writeln!(f, "{}", row.as_ref()).unwrap();
    }
    f.flush().unwrap();
    f
}

/// Same as [`temp_csv`], with a file suffix (e.g. `".csv"`) for
/// extension-based format detection.
pub fn temp_csv_with_suffix<S: AsRef<str>>(suffix: &str, rows: &[S]) -> NamedTempFile {
    let mut f = NamedTempFile::with_suffix(suffix).unwrap();
    for row in rows {
        writeln!(f, "{}", row.as_ref()).unwrap();
    }
    f.flush().unwrap();
    f
}
