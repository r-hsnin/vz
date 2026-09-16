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

| Pattern | Detected Type | Examples |
|---------|--------------|----------|
| ISO 8601 / common date formats | `Temporal` | 2024-01-15, 2024/01/15 |
| Numeric (int or float) | `Quantitative` | 42, 3.14, -100 |
| Low cardinality (≤ 20 unique in sample) | `Categorical` | "Tokyo", "Osaka" |
| High cardinality text | `Nominal` | UUIDs, free text |

Sampling: first 100 rows for inference, full scan if ambiguous.

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
5. **Shared data_builder** — All 3 modes use the same data construction logic to avoid divergence
6. **Format auto-detection** — Extension first, then content heuristics (tabs vs commas, JSON detection)
