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
├── main.rs                 — binary entry, CLI dispatch
├── lib.rs                  — library crate re-exports (for benches/tests)
├── pipeline.rs             — render pipeline: infer → select → build → render → output
├── cli/                    — clap definitions (mod.rs, args.rs, types.rs)
├── helpers/                — CLI arg processing, format detection, data transforms
├── loader/                 — CSV/TSV/JSON/NDJSON/space unified loader, format auto-detect
├── filter.rs               — --where predicate engine
├── infer/                  — type inference (types.rs: Schema/ColumnMeta, detector.rs)
├── chart/                  — selector.rs (types → chart), data_builder.rs (rows → chart data)
├── render/                 — ratatui widgets: line, bar, scatter, histogram, heatmap, nice_numbers
├── oneshot/                — stdout rendering: builders, summary, ansi
├── info.rs                 — --info column metadata
├── output/                 — machine-readable exporters: chart_json, markdown, spark, stats_text, svg, html, table
├── diff/                   — two-file comparison: schema, compute, render/{bar,line,spark,json,markdown,html}
├── directory/              — directory mode: scanner, combiner, catalog, date_extract
├── explore/                — interactive TUI: app, state, render, diff, diff_render
├── present/                — slides: parser, render, chart_loader
├── watch.rs                — --watch auto-redraw
├── theme.rs                — color themes (dark/light/high-contrast)
├── sparkline.rs            — shared sparkline generation
├── util.rs                 — shared numeric utilities
└── diagnostics.rs          — error hints & file suggestions

Unit tests live beside their module (`tests.rs` / `*_tests.rs`); end-to-end tests in `tests/`.
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

## Chart Selection Design

The user-facing selection table and behavior live in [README.md](README.md#chart-selection-rules).

Design intent: the selector maps inferred column types to a chart type, normalizes
reversed axes (e.g. Quantitative × Temporal) to the canonical orientation, and falls
back to Bar for unmatched type pairs.

## CLI Design

Flag definitions and examples live in [README.md](README.md#usage); `vz --help` is authoritative at runtime.

Design guideline: the common case needs no flags (`vz data.csv`), and every override
(axes, type, aggregation, filtering) is opt-in.

## Scope

### In Scope (v0.2)
- File-based batch visualization (CSV/TSV/JSON/NDJSON)
- Auto-inference of column types and chart selection
- Three output modes: oneshot (stdout), explore (TUI), present (slides)
- Machine-readable exports: JSON, SVG, HTML, Markdown, sparkline, table
- Row filtering, aggregation, sampling
- Color themes (dark, light, high-contrast)
- File watch mode for iterative exploration
- Diff mode (two-file comparison)
- Directory mode (multi-file combine)
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
