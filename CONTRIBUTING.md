# Contributing to vz

## Prerequisites

- Rust 1.85+ (install via [rustup](https://rustup.rs/))
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
├── main.rs              — Entry point, CLI dispatch (thin wrapper over lib.rs)
├── lib.rs               — Library crate: re-exports all modules for benchmarks
├── cli/                 — CLI argument definitions (clap derive)
│   ├── mod.rs           — Cli struct, Command enum, re-exports
│   ├── types.rs         — ValueEnum enums (SortOrder, OutputFormat, ThemeArg, etc.)
│   └── args.rs          — Cli impl methods (effective_sort, diff_pair, parse helpers)
├── loader/              — CSV/TSV/JSON/NDJSON/Space unified loader
│   ├── mod.rs           — Format dispatch, load_from_file, load_from_content
│   ├── tests.rs         — Loader unit tests
│   └── space/           — Fixed-width / space-aligned format parser
│       ├── mod.rs       — Public API re-exports, Column struct
│       ├── detect.rs    — looks_like_space_format, detect_columns
│       ├── parse.rs     — load_space, extract_row, find_gap_near
│       └── tests.rs     — Space parser unit tests
├── infer/               — Type inference engine
│   ├── mod.rs           — Schema inference entrypoint
│   ├── types.rs         — DataType enum, ColumnMeta, Schema
│   └── detector.rs      — Value-level type detection
├── pipeline.rs          — Render pipeline: infer → select → render → output
├── info.rs              — --info column metadata display
├── helpers/             — Shared helper functions
│   ├── mod.rs           — Re-exports
│   ├── args.rs          — CLI argument processing (build_render_options, resolve_theme)
│   ├── format.rs        — Format detection helpers (format_override)
│   └── data.rs          — Data transformation (apply_filters, build_recommendation)
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
│   ├── ansi.rs          — ANSI escape sequence output
│   └── tests.rs         — Unit tests (separated for file size)
├── output/              — Machine-readable output formats
│   ├── mod.rs           — InfoOutput struct, build_info_output
│   ├── chart_json.rs    — --output json chart data generation
│   ├── markdown.rs      — --output markdown (GFM tables)
│   ├── spark.rs         — --output spark (Unicode sparklines)
│   ├── stats_text.rs    — Column statistics text formatter
│   ├── svg.rs           — --output svg (monospace SVG image)
│   ├── html.rs          — --output html (self-contained interactive HTML)
│   └── table.rs         — --output table (formatted text)
├── diff/                — Diff mode: compare two data files
│   ├── mod.rs           — Public API, DiffEntry/DiffResult/DiffTimeSeries structs
│   ├── schema.rs        — Schema validation, column resolution
│   ├── compute.rs       — compute_diff, compute_diff_temporal, aggregate_by_category
│   ├── tests.rs         — Diff computation unit tests
│   └── render/          — Diff output rendering
│       ├── mod.rs       — render_diff / render_diff_line dispatch
│       ├── bar.rs       — Categorical bar chart (▲/▼ annotations)
│       ├── line.rs      — Temporal line overlay (before=gray, after=cyan)
│       ├── spark.rs     — Sparkline diff output
│       ├── json.rs      — JSON diff output
│       ├── markdown.rs  — Markdown table diff output
│       ├── html.rs      — HTML/SVG diff output
│       └── tests.rs     — Diff render unit tests
├── directory/           — Directory mode: auto-combine matching files
│   ├── mod.rs           — Entry point, run_directory
│   ├── scanner.rs       — File discovery & schema matching
│   ├── combiner.rs      — Multi-file row merging with _source column
│   ├── catalog.rs       — --catalog schema display
│   ├── date_extract.rs  — Date extraction from filenames
│   └── tests.rs         — Directory mode unit tests
├── explore/             — Interactive TUI mode
│   ├── mod.rs           — ExploreApp state & key handling
│   ├── render.rs        — TUI rendering (chart, table, status bar)
│   ├── diff.rs          — DiffExploreApp: diff exploration state & keys
│   ├── diff_render.rs   — Diff TUI rendering (bar/line chart, table)
│   ├── diff/tests.rs    — Diff explore unit tests
│   └── tests.rs         — Explore mode unit tests
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
- **~708 unit tests** — inline `#[cfg(test)]` modules in each source file
- **~221 integration tests** — `tests/integration_test.rs`, end-to-end binary tests
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
- [ ] README.md updated if CLI interface changed (update AUTO-GENERATED section)
- [ ] DESIGN.md updated if architecture changed

## Architecture Notes

See [DESIGN.md](DESIGN.md) for the full architecture document.

Key pipeline: **CLI → Data Loader → Type Inference → Chart Selection → Rendering**

Three output modes:
1. **One-shot** (default) — Renders chart to stdout via in-memory buffer
2. **Explore** — Interactive TUI with ratatui
3. **Present** — Slide presentation from Markdown with embedded charts
