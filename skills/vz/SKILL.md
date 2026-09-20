---
name: vz
description: >
  CLI data visualization tool for terminals. Use when the user wants to
  visualize CSV/TSV/JSON/NDJSON data, create charts (line, bar, scatter,
  histogram, heatmap), inspect column metadata, or produce machine-readable
  chart output (JSON, SVG, Markdown, sparkline). Also activate on
  "データ可視化", "グラフ作成", "chart", "plot", "visualize data",
  "terminal chart", "explore data", "present slides", or when piping
  command output to a visualization. Do NOT use for: web-based dashboards,
  Jupyter notebooks, matplotlib/plotly code generation, or GUI chart tools.
---

# vz — Terminal Data Visualization

Zero-config CLI BI tool. Data types determine the chart automatically.

## Install

```bash
cargo install --git https://github.com/r-hsnin/vz
```

Requires Rust 1.88+.

## Core Workflow

```bash
# Auto-visualize (infers chart type from column types)
vz data.csv

# Specify axes explicitly
vz data.csv -x month -y revenue

# Override chart type
vz data.csv -x city -y revenue -t bar

# Multi-series (comma-separated Y columns)
vz data.csv -y revenue,profit

# Group by color column
vz data.csv -c region

# Stdin pipe
cat data.csv | vz -
kubectl top pods | vz - -f space
```

All flags: `vz --help`.

## Chart Selection Rules

vz infers the best chart from column types:

| X type | Y type | Chart |
|--------|--------|-------|
| Temporal | Quantitative | Line |
| Categorical | Quantitative | Bar |
| Quantitative | Quantitative | Scatter |
| Single Quantitative | — | Histogram |
| Categorical | Categorical | Heatmap |

Override with `-t line|bar|scatter|histogram|heatmap`.

## Agent-Optimized Outputs

For programmatic consumption, prefer `--output json`:

```bash
# Structured JSON with metadata + chart data
vz data.csv --json

# Column metadata only (schema inspection)
vz data.csv --info --json
```

JSON output includes:
- `version`, `file`, `rows` — metadata
- `columns[]` — name, type (temporal/quantitative/categorical/nominal), nulls, stats
- `recommendation` — inferred chart_type, x, y, color
- `data[]` — first 100 rows as objects
- `chart_data` — aggregated/processed chart-ready data

Other output formats:
- `--spark` — single-line sparkline (embed in dashboards, logs)
- `--svg` — vector image (embed in docs, reports)
- `--html` — self-contained interactive HTML page (hover tooltips)
- `--markdown` — Markdown table (paste into README, issues)
- `--output table` — formatted text table

## Subcommands

```bash
vz explore data.csv          # Interactive TUI (vim-style: h/l X, j/k Y, c color, d/Tab chart↔table)
vz present slides.md         # Terminal slides from Markdown with ```chart blocks
vz completions <SHELL>       # Shell completions (bash/zsh/fish/elvish/powershell)
```

## Gotchas

- **No file argument shows help.** Always pass a file or `-` for stdin.
- **Bar chart aggregates by default (sum).** Use `--agg mean` if you want averages.
- **Column names are case-sensitive.** Check with `vz data.csv --info`.
- **TSV detection relies on extension or tab prevalence.** When piping, use `-f tsv` explicitly.
- **Large datasets (>100k rows):** Use `--sample N` to keep rendering fast.
- **JSON output includes only first 100 rows in `data[]`.** The `chart_data` field contains the full aggregated result.

## Typical Agent Patterns

```bash
# 1. Inspect schema before visualizing
vz data.csv --info --json | jq '.columns[] | {name, type}'

# 2. Get chart data for downstream processing
vz sales.csv -x month -y revenue --json | jq '.chart_data'

# 3. Quick trend check via sparkline
vz metrics.csv -y latency --spark

# 4. Generate SVG for embedding in reports
vz data.csv --svg > chart.svg

# 5. Filter + aggregate for specific insight
vz logs.csv --where "status=500" -x endpoint -y count -t bar --sort desc --top 10 --json
```
