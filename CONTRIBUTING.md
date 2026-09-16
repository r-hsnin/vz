# Contributing to vz

## Prerequisites

- Rust 1.87+ (install via [rustup](https://rustup.rs/))
- Cargo (comes with rustup)

## Development Setup

```bash
# Clone the repository
git clone <repo-url>
cd vz

# Build the project
cargo build

# Run the binary
cargo run -- fixtures/sales.csv

# Run in release mode
cargo run --release -- fixtures/sales.csv
```

## Project Structure

Module layout and architecture live in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#module-structure).

## Available Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Build the project |
| `cargo run -- <args>` | Run with arguments |
| `cargo test` | Run all tests (unit + integration) |
| `cargo clippy --all-targets -- -D warnings` | Lint with zero warnings |
| `cargo fmt` | Format code |
| `cargo doc --open` | Generate and open API docs |

## Testing

### Run all tests

```bash
cargo test
```

`cargo test` also prints the current unit / integration / snapshot counts; there is
no need to keep counts in this document.

### Run specific tests

```bash
# Run a single test by name
cargo test test_basic_csv

# Run all tests in a module
cargo test oneshot::tests

# Run integration tests only
cargo test --test integration_test

# Run snapshot tests only
cargo test --test snapshot_test

# Run with output shown
cargo test -- --nocapture
```

### Writing tests

- Unit tests go in the same file as the code, inside a `#[cfg(test)]` module
  (larger modules use sibling `tests.rs` / `*_tests.rs` files)
- Integration tests go in `tests/integration_test.rs`
- Use `pretty_assertions` for readable diffs
- Use `tempfile` for temporary file creation in tests

Example integration test:

```rust
#[test]
fn test_my_feature() {
    let output = vz_binary()
        .arg("fixtures/sales.csv")
        .output()
        .expect("Failed to run vz");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("expected output"));
}
```

### Test fixtures

Test data lives in `fixtures/`:
- `sales.csv` — Sample sales data (date, city, revenue, profit)
- `departments.csv` — Categorical×Categorical data (department, status)
- `stock.csv` — Time-series stock data
- `temperature.csv` — Multi-point temperature measurements
- `exam_scores.csv` — Numeric exam scores
- `body_measurements.csv` — Quantitative×Quantitative data
- `access_log.csv` — Large-ish log-style data (2000 rows)
- `messy_data.csv` — Edge case: missing values, mixed types
- `mixed_values.csv` — Edge case: mixed parseable/non-parseable Y values
- `scores.json` — JSON array format test data
- `demo.md` — Sample presentation file with chart blocks
- `code_demo.md` — Presentation with code blocks

## Benchmarking

Performance benchmarks use [Criterion.rs](https://github.com/bheisler/criterion.rs):

```bash
# Run all benchmarks
cargo bench

# Run a specific benchmark
cargo bench -- csv_parse

# Run with filtering
cargo bench -- "pipeline"
```

Benchmark suite (`benches/loading.rs`) covers:

| Benchmark | What it measures |
|-----------|-----------------|
| `csv_parse_1000` | CSV parsing (1000 rows) |
| `json_parse_1000` | JSON array parsing (1000 rows) |
| `space_parse_1000` | Space-aligned format parsing (1000 rows) |
| `infer_1000` | Type inference (1000 rows) |
| `infer_10000_rows` | Type inference scaling (10000 rows, sampled) |
| `pipeline_csv_1000` | Full render pipeline (CSV → chart selection) |
| `full_pipeline/json_load_infer_1000` | End-to-end JSON load + infer |

Results are stored in `target/criterion/` with HTML reports. After running benchmarks, open `target/criterion/report/index.html` for a visual comparison.

When optimizing hot paths, run benchmarks before and after to verify improvement:

```bash
# Save baseline
cargo bench -- --save-baseline before

# Make changes, then compare
cargo bench -- --baseline before
```

## Code Style

- **Formatter**: `cargo fmt` (rustfmt with default settings)
- **Linter**: `cargo clippy --all-targets -- -D warnings` (zero warnings policy)
- Keep functions under 50 lines where possible
- Keep files focused and under 800 lines
- Use descriptive names; no abbreviations in public APIs

## Pull Request Checklist

Before submitting a PR:

- [ ] `cargo fmt` — code is formatted
- [ ] `cargo clippy --all-targets -- -D warnings` — zero warnings
- [ ] `cargo test` — all tests pass
- [ ] New functionality has tests
- [ ] README.md updated if the CLI interface changed
- [ ] docs/ updated if the architecture or design changed
