# Contributing to vz

## Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
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

<!-- AUTO-GENERATED: from src/ directory structure -->

```
src/
├── main.rs              — Entry point, CLI dispatch, data loading
├── cli/mod.rs           — CLI argument definitions (clap derive)
├── loader/mod.rs        — CSV/TSV/JSON/NDJSON unified loader
├── infer/               — Type inference engine
│   ├── mod.rs           — Schema inference entrypoint
│   ├── types.rs         — DataType enum, ColumnMeta, Schema
│   └── detector.rs      — Value-level type detection
├── chart/               — Chart selection & data building
│   ├── mod.rs           — Module re-exports
│   ├── selector.rs      — Type combination → chart type mapping
│   └── data_builder.rs  — Schema+rows → rendering data structures
├── filter.rs            — Row filtering engine (--where predicates)
├── render/              — Terminal chart rendering (ratatui widgets)
│   ├── mod.rs           — ChartData enum, ChartWidget, dispatch
│   ├── line.rs          — Line/Scatter unified widget (XYChart)
│   ├── bar.rs           — Bar chart widget
│   ├── scatter.rs       — Scatter re-export (thin wrapper)
│   ├── histogram.rs     — Histogram widget
│   ├── heatmap.rs       — Heatmap widget (Cat×Cat count matrix)
│   └── nice_numbers.rs  — Axis tick calculation
├── oneshot/             — One-shot stdout rendering (Buffer → ANSI)
│   ├── mod.rs           — Render orchestration
│   ├── builders.rs      — Chart data builders (bar/histogram/line)
│   ├── summary.rs       — Summary line & color legend
│   ├── tests.rs         — Unit tests (separated for file size)
│   └── ansi.rs          — ANSI escape sequence output
├── output/              — Machine-readable output formats
│   ├── mod.rs           — InfoOutput struct, build_info_output
│   ├── chart_json.rs    — --output json chart data generation
│   ├── markdown.rs      — --output markdown (GFM tables)
│   ├── spark.rs         — --output spark (Unicode sparklines)
│   ├── stats_text.rs    — Column statistics text formatter
│   ├── svg.rs           — --output svg (monospace SVG image)
│   └── table.rs         — --output table (formatted text)
├── explore/             — Interactive TUI mode
│   ├── mod.rs           — ExploreApp state & key handling
│   └── render.rs        — TUI rendering (chart, table, status bar)
├── present/             — Slide presentation mode
│   ├── mod.rs           — PresentApp state & key handling
│   ├── parser.rs        — Markdown slide parser
│   ├── render.rs        — Slide rendering (elements, charts)
│   ├── chart_loader.rs  — Chart data loading & type inference
│   └── tests.rs         — Unit tests (separated for file size)
├── diagnostics.rs       — Error hints & file suggestions
├── sparkline.rs         — Shared sparkline generation utility
├── theme.rs             — Color theme definitions (dark/light/high-contrast)
├── util.rs              — Numeric utilities (min_max)
└── watch.rs             — File watch & auto-redraw (--watch)
```

<!-- /AUTO-GENERATED -->

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

This runs:
- **439 unit tests** — inline `#[cfg(test)]` modules in each source file
- **137 integration tests** — `tests/integration_test.rs`, end-to-end binary tests
- **4 snapshot tests** — `tests/snapshot_test.rs`, visual regression tests

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
- [ ] README.md updated if CLI interface changed (update AUTO-GENERATED section)
- [ ] DESIGN.md updated if architecture changed

## Architecture Notes

See [DESIGN.md](DESIGN.md) for the full architecture document.

Key pipeline: **CLI → Data Loader → Type Inference → Chart Selection → Rendering**

Three output modes:
1. **One-shot** (default) — Renders chart to stdout via in-memory buffer
2. **Explore** — Interactive TUI with ratatui
3. **Present** — Slide presentation from Markdown with embedded charts
