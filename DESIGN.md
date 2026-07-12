# vz — Design Document

## Vision

CLI BI tool that auto-visualizes data in the terminal with zero configuration.
Three output modes: **One-shot** (default stdout), **Explore** (interactive TUI), **Present** (slide-based).

## Core Philosophy

- **Convention over Configuration** — Data types determine visualization
- **Zero-config by default** — Override only when needed
- **Terminal-native** — No browser, no GUI, just your terminal
- **Instant value** — `vz data.csv` produces a meaningful chart immediately

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                    CLI (clap 4)                      │
│   vz <file>  |  vz explore <file>  |  vz present   │
└────────────────────────┬────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────┐
│               Data Loader (loader/)                  │
│   CSV / TSV / JSON / NDJSON  (format auto-detect)   │
└────────────────────────┬────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────┐
│            Type Inference Engine (infer/)            │
│   temporal / quantitative / categorical / nominal    │
└────────────────────────┬────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────┐
│         Chart Selection (chart/selector.rs)          │
│   Types → Best chart (line/bar/scatter/histogram)   │
│         Data Builder (chart/data_builder.rs)         │
│   Schema + Rows → Renderable chart data structures  │
└────────────────────────┬────────────────────────────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  One-shot    │ │   Explore    │ │   Present    │
│  (stdout)    │ │   (TUI)      │ │  (Slides)    │
│  oneshot/    │ │  explore/    │ │  present/    │
└──────────────┘ └──────────────┘ └──────────────┘
```

## Module Structure

```
src/
├── main.rs              — Entry point, CLI dispatch
├── cli/mod.rs           — clap4 argument definitions
├── loader/mod.rs        — Unified data loader (CSV/TSV/JSON/NDJSON, format auto-detect)
├── filter.rs            — Row filtering (--where predicates)
├── infer/               — Type inference engine
│   ├── mod.rs           — Schema inference entrypoint
│   ├── types.rs         — DataType enum, ColumnMeta, Schema
│   └── detector.rs      — Value-level type detection
├── chart/               — Chart selection & data building
│   ├── mod.rs           — Module re-exports
│   ├── selector.rs      — Type combination → chart type mapping
│   └── data_builder.rs  — Schema + rows → renderable chart data (shared across modes)
├── render/              — Terminal chart rendering (ratatui widgets)
│   ├── mod.rs           — Shared types (Axis, Series, ChartConfig, BarChartData, HistogramData)
│   ├── line.rs          — Line chart widget
│   ├── bar.rs           — Bar chart widget
│   ├── scatter.rs       — Scatter plot widget
│   ├── histogram.rs     — Histogram widget
│   ├── heatmap.rs       — Heatmap widget (categorical × categorical)
│   └── nice_numbers.rs  — Axis tick calculation (nice numbers algorithm)
├── oneshot/mod.rs       — One-shot stdout rendering (Buffer → ANSI, multi-series, summary)
│   ├── ansi.rs          — ANSI color output, print_buffer
│   ├── builders.rs      — Chart data construction (bar, histogram, heatmap, line/scatter)
│   └── summary.rs       — Summary line formatting (sparkline, trend, hints)
├── explore/mod.rs       — Interactive TUI mode (chart switching, column selection, data table)
├── present/mod.rs       — Slide presentation mode (Markdown + ```chart blocks)
│   ├── parser.rs        — Markdown→Slide AST parser
│   ├── render.rs        — Slide rendering (draw_slide, element rendering)
│   └── chart_loader.rs  — Chart data loading for embedded chart blocks
├── watch.rs             — File watching mode (--watch, auto-redraw on changes)
├── output/              — Output format renderers (machine-readable & export)
│   ├── mod.rs           — Column stats computation, JSON metadata output
│   ├── chart_json.rs    — Chart data as JSON (series, labels, bins)
│   ├── markdown.rs      — Markdown table output (--output markdown)
│   ├── spark.rs         — Sparkline output mode (--output spark)
│   ├── stats_text.rs    — Text formatting for column statistics (--info)
│   ├── svg.rs           — SVG image export (Buffer → SVG document)
│   └── table.rs         — Text table output (--output table)
├── diagnostics.rs       — Error hints & file suggestions for common errors
├── theme.rs             — Color theme definitions (dark, light, high-contrast)
├── util.rs              — Shared numeric utilities (min_max)
├── sparkline.rs         — Shared sparkline generation (Unicode block chars)
```

## Data Flow & Dependencies

```
main.rs ─── cli/        (parse args)
   │
   ├──────── loader/    (file → LoadedData{headers, rows})
   │
   ├──────── filter/    (--where predicates → row subset)
   │
   ├──────── infer/     (LoadedData → Schema{columns: Vec<ColumnMeta>})
   │
   ├──────── chart/
   │         ├── selector   (Schema → ChartType)
   │         └── data_builder (Schema + rows → chart-specific data structs)
   │
   └──────── render/    (data structs → ratatui Buffer)
             │
             ├── oneshot/  (builders.rs → ChartData → render_chart_data → Buffer → ANSI)
             ├── explore/  (inline construction → ChartData → ChartWidget → TUI)
             └── present/  (chart_loader.rs → ChartData → render_chart_data → slide)
```

Each mode has a **mode-specific builder layer** that adapts the shared `ChartData`
structures before passing them to `render_chart_data()`:
- `oneshot/builders.rs` — sorting, truncation, label fitting, theme application
- `explore/mod.rs` — interactive column selection → ChartData construction
- `present/chart_loader.rs` — Markdown chart block → ChartData
```

**Change Impact Map:**
- `loader/` change → affects all modes. Run full integration tests.
- `filter.rs` change → affects oneshot + explore (present applies via chart block `where:` field).
- `infer/` change → affects chart selection + all modes.
- `chart/selector.rs` change → affects all modes.
- `chart/data_builder.rs` change → affects oneshot, explore, present.
- `render/` change → affects only the corresponding chart type.
- `oneshot/`, `explore/`, `present/` → affects only that mode.

## Type Inference Rules

| Pattern | Detected Type | Examples |
|---------|--------------|----------|
| ISO 8601 / common date formats | `Temporal` | 2024-01-15, 2024/01/15 |
| Numeric (int or float) | `Quantitative` | 42, 3.14, -100 |
| Low cardinality (≤ 20 unique in sample) | `Categorical` | "Tokyo", "Osaka" |
| High cardinality text | `Nominal` | UUIDs, free text |

Sampling: first 100 rows for inference, full scan if ambiguous.

## Chart Selection Rules

| X type | Y type | Chart |
|--------|--------|-------|
| Temporal | Quantitative | Line |
| Categorical | Quantitative | Bar |
| Quantitative | Quantitative | Scatter |
| (single column) | Quantitative | Histogram |
| Categorical | Categorical | Heatmap (count) |

## CLI Interface

```bash
# Auto mode — one-shot chart to stdout (default)
vz data.csv

# Specify axes
vz data.csv -x month -y revenue

# Override chart type
vz data.csv -x month -y revenue --type bar

# Label override
vz data.csv -y revenue:"Revenue (USD)"

# Multi-Y series (comma-separated)
vz data.csv -y revenue,profit

# Color grouping (multi-series by category)
vz data.csv -c city

# Filter rows
vz data.csv --where "city=Tokyo" --where "revenue>1000"

# Sort bar chart (desc/asc)
vz data.csv -t bar --sort desc

# Limit bar chart to top/bottom N
vz data.csv -t bar --top 5
vz data.csv -t bar --tail 3

# Column metadata
vz data.csv --info

# Stdin pipe (auto-detected, no '-' needed)
cat data.csv | vz

# Explore mode — interactive TUI
vz explore data.csv

# Present mode — slides from markdown
vz present slides.md
```

## Scope

### In Scope (v0.1)
- File-based batch visualization (CSV/TSV/JSON/NDJSON)
- Auto-inference of column types and chart selection
- Three output modes: oneshot (stdout), explore (TUI), present (slides)
- Machine-readable exports: JSON, SVG, Markdown, sparkline, table
- Row filtering, aggregation, sampling
- Color themes (dark, light, high-contrast)
- File watch mode for iterative exploration
- Shell completions

### Non-goals (for now)
- Database connections (Parquet, SQLite, PostgreSQL)
- PNG raster export
- Streaming / real-time data beyond `--watch`
- Data transformation / ETL operations
- Custom color palettes (beyond the 3 built-in themes)

## Key Design Decisions

1. **Ratatui for rendering** — Mature, active, Rust-native
2. **No external data engine** — Keep binary small, no Polars/DuckDB dep for v1
3. **In-memory processing** — v1 targets files that fit in memory (< 1GB)
4. **Convention-first CLI** — Minimal flags needed for 80% of use cases
5. **Shared data_builder** — All 3 modes use the same data construction logic to avoid divergence
6. **Format auto-detection** — Extension first, then content heuristics (tabs vs commas, JSON detection)
