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
├── output/mod.rs        — Machine-readable output (JSON metadata, column stats)
├── sparkline.rs         — Shared sparkline generation (Unicode block chars)
└── table.rs             — Text table output (--output table)
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

**変更影響マップ:**
- `loader/` を変更 → 全モードに影響。統合テスト全実行必須。
- `filter.rs` を変更 → oneshot + explore モードに影響（present は chart block 内 where: フィールドで適用）。
- `infer/` を変更 → chart selection + 全モードに影響。
- `chart/selector.rs` を変更 → 全モードに影響。
- `chart/data_builder.rs` を変更 → oneshot, explore, present すべてに影響。
- `render/` を変更 → 対応チャート種別のみ影響。
- `oneshot/`, `explore/`, `present/` → そのモードのみ影響。

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

### Implemented (v1)
- [x] CSV / TSV input (stdin + file)
- [x] JSON / NDJSON input (array-of-objects, newline-delimited)
- [x] Format auto-detection (extension + content sniffing)
- [x] Type inference (temporal, quantitative, categorical, nominal)
- [x] Auto chart selection (line, bar, scatter, histogram, heatmap)
- [x] Terminal rendering via ratatui
- [x] One-shot mode: Buffer → ANSI stdout with summary stats
- [x] Explore mode: interactive chart switching, column selection, data table view
- [x] Present mode: markdown with ```chart blocks
- [x] Legend from column names, label override with `:`
- [x] Nice-numbers axis tick algorithm
- [x] Multi-series: color grouping (`-c`) and multi-Y (`-y a,b,c`)
- [x] Row filtering (`--where col=val`, `--where col>val`)
- [x] Bar chart sorting (`--sort desc/asc`)
- [x] Bar chart limiting (`--top N`, `--tail N`)
- [x] Stdin auto-detect (pipe without `-` argument)
- [x] Column metadata (`--info`)
- [x] Heatmap (categorical × categorical, count-based)
- [x] File watch mode (`--watch`)
- [x] Machine-readable output (`--output json`, `--output table`, `--output spark`)
- [x] Aggregation functions (`--agg sum/mean/count/max/min`)
- [x] Custom chart title (`--title`)
- [x] Value labels on bar charts (`--labels`)
- [x] Shell completions (`vz completions <shell>`)
- [x] Sampling for large datasets (`--sample N`)
- [x] All-Y overlay (`-Y` / `--all-y`)

### Out (future)
- Parquet / SQLite / DB connections
- Export (PNG, SVG)
- ~~Custom themes / color configuration~~ → **実装済み** (`--theme dark|light|high-contrast`)
- Streaming / live data (部分的に `--watch` で実現)

## Key Design Decisions

1. **Ratatui for rendering** — Mature, active, Rust-native
2. **No external data engine** — Keep binary small, no Polars/DuckDB dep for v1
3. **In-memory processing** — v1 targets files that fit in memory (< 1GB)
4. **Convention-first CLI** — Minimal flags needed for 80% of use cases
5. **Shared data_builder** — All 3 modes use the same data construction logic to avoid divergence
6. **Format auto-detection** — Extension first, then content heuristics (tabs vs commas, JSON detection)
