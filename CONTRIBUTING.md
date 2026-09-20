# Contributing to vz

## Prerequisites

- Rust via [rustup](https://rustup.rs/) (MSRV: 1.88+; CI validates it with `cargo +1.88.0 check --locked`)
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

# Run integration tests only (per-area targets: oneshot, flags, inputs,
# output, directory, diff, modes)
cargo test --test diff

# Run a single area, e.g. directory mode
cargo test --test directory

# Run snapshot tests only
cargo test --test snapshot_test

# Run with output shown
cargo test -- --nocapture
```

### Snapshot tests

Rendered output snapshots live in `tests/snapshot_test.rs` (files in
`tests/snapshots/`). New or changed rendering produces a pending
`*.snap.new` file and a test failure — this is expected (Red):

1. Run `cargo test --test snapshot_test`
2. Inspect the pending `tests/snapshots/*.snap.new` files (verify the
   rendering is correct, not just different)
3. Accept: rename `*.snap.new` → `*.snap` (or `cargo insta accept`
   if `cargo-insta` is installed)
4. Re-run to confirm green

Keep snapshots deterministic: snapshot helpers must pin `NO_COLOR=1` and
`COLUMNS=80` (see `tests/common/mod.rs::run_vz_stdout`). Verify new
snapshots render byte-identical output across two runs before accepting.

### Writing tests

- Integration tests go in per-area targets under `tests/` (`oneshot.rs`,
  `flags.rs`, `inputs.rs`, `output.rs`, `directory.rs`, `diff.rs`, `modes.rs`);
  shared binary construction, temp-file fixtures, and thresholds live in
  `tests/common/mod.rs` — reuse them instead of adding new helpers
- Unit tests go in the same file as the code, inside a `#[cfg(test)]` module
  (larger modules use sibling `tests.rs` / `*_tests.rs` files);
  shared `make_schema` / `make_recommendation` fixtures live in
  `src/test_helpers.rs` (test builds only)
- Use `pretty_assertions` for readable diffs
- Use `tempfile` for temporary file creation in tests

Example integration test:

```rust
use common::vz_binary;

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
- `access_log.csv` — Small log-style data (50 rows)
- `bom_sales.csv` — Edge case: UTF-8 BOM prefix
- `diff/` — Diff-mode fixture pairs (`sales_before/after`, `timeseries_*`, `ts_daily_*`, `identical`, `schema_mismatch`)
- `dir_test/` — Directory-mode cases (`dated`, `case_insensitive`, `empty`, `header_only`, `mixed_extensions`)
- `fixed_width/` — Space-aligned samples (`kubectl_*`, `df_h`, `ps_aux`, `lsblk`, …)
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
| `full_pipeline/csv_load_infer_1000` | End-to-end CSV load + infer |
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
- **Planes first**: put new logic in its owning plane
  ([ARCHITECTURE.md](docs/ARCHITECTURE.md#planes-and-dependency-rules)).
  Data/output code takes plain `*Params` structs, never `&Cli`;
  `println!/eprintln!` stays at the app-plane edge.
- Public surface is narrow by design (`lib.rs`: `chart`, `cli`, `filter`,
  `infer`, `loader`, `theme`, `util`, `run`, `apply_output_shorthands`,
  `infer_from_data`). New modules default to `pub(crate)` or private;
  do not add `pub mod` without a data-plane reuse case.
  `src/helpers/` was dissolved in Phase 1 and must not be recreated;
  put new logic in its owning plane.

## Pull Request Checklist

Before submitting a PR:

- [ ] `cargo fmt` — code is formatted
- [ ] `cargo clippy --all-targets -- -D warnings` — zero warnings
- [ ] `cargo test` — all tests pass
- [ ] New functionality has tests
- [ ] README.md updated if the CLI interface changed
- [ ] docs/ updated if the architecture or design changed
