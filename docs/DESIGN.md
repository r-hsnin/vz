# vz — Design Document

Design intent: vision, philosophy, behavioral rules, and key decisions.
Structural facts (modules, data flow, change impact) live in [ARCHITECTURE.md](ARCHITECTURE.md).

## Vision

CLI BI tool that auto-visualizes data in the terminal with zero configuration.
Three output modes: **One-shot** (default stdout), **Explore** (interactive TUI), **Present** (slide-based).

## Core Philosophy

- **Convention over Configuration** — Data types determine visualization
- **Zero-config by default** — Override only when needed
- **Terminal-native** — No browser, no GUI, just your terminal
- **Instant value** — `vz data.csv` produces a meaningful chart immediately

## Type Inference Rules

Implemented in `infer/detector.rs`. Each value is classified first, then the
column type is decided by majority vote:

| Value pattern | Detected as | Notes |
|---------------|-------------|-------|
| `YYYY-MM-DD` (optional time), `YYYY/MM/DD`, `MM/DD/YYYY`, `DD-Mon-YYYY`, `YYYY-MM` | `Temporal` | Checked before numeric |
| Display-formatted numbers via `util::parse_number` | `Quantitative` | `1,000`, `$100`, `€50`, `45%` (= 0.45), `10k`, `10GiB`, `(42)` (= -42), `USD 100` all parse → Quantitative. One parser shared by inference, aggregation, series, filters, diff, sparkline, and JSON samples, so a value means the same number on every path. `%` is a fraction (`50%` = 0.5); storage suffixes are decimal except `KiB/MiB/GiB/TiB` (binary). `--where` equality is numeric too (`revenue=2000` matches `$2,000`) |
| Trend annotation via `util::trend_label`/`trend_from_slice` | `→ stable` band ±5% | Single implementation shared by oneshot summary lines and spark suffixes; denominator is `first.abs()` (`-100 → -50` = `↑ +50%`), near-zero start yields no trend |
| `NaN`, `inf`, `-inf`, `Infinity` | `Nominal` (excluded from column vote) | `parse_number` returns `None` for non-finite → treated like nulls: skipped in inference, aggregation, and all chart paths |
| Empty string | `Nominal` (ignored in column vote) | Nulls don't vote |
| Anything else | `Nominal` | e.g. UUIDs, free text |

Column decision (first 100 rows only, empty values excluded):

1. ≥ 80% of sampled values Temporal → `Temporal`
2. ≥ 80% Quantitative → `Quantitative`
3. Otherwise by cardinality: ≤ 20 unique values → `Categorical`, else `Nominal`

Only the first 100 rows are sampled; there is no full-scan fallback.
The same 100-row cap is applied in `pipeline::infer_from_data`.

## Chart Selection Design

The user-facing selection table and behavior live in [README.md](../README.md#chart-selection-rules).

Design intent: the selector maps inferred column types to a chart type, normalizes
reversed axes (e.g. Quantitative × Temporal) to the canonical orientation, and falls
back to Bar for unmatched type pairs.

## CLI Design

Flag definitions and examples live in [README.md](../README.md#usage); `vz --help` is authoritative at runtime.

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
5. **Shared data_builder** — All 3 modes build on the same `ChartData`
   structures from `chart/data_builder.rs` to avoid divergence; each mode keeps
   only a thin adaptation layer (sorting, truncation, slide wiring, see
   ARCHITECTURE.md) on top
6. **Format auto-detection** — Extension first, then content heuristics (tabs vs commas, JSON detection)
