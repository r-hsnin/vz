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

Requires Rust 1.85+.

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
kubectl top pods | vz - -f tsv
```

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
- `--markdown` — Markdown table (paste into README, issues)
- `--output table` — formatted text table

## Key Flags

| Flag | Purpose |
|------|---------|
| `-x COL` | X axis column |
| `-y COL[,COL2]` | Y axis column(s), supports `col:Label` |
| `-t TYPE` | Force chart type |
| `-c COL` | Color/group-by column |
| `-f FORMAT` | Force input format (csv/tsv/json/ndjson) |
| `-W N` / `-H N` | Width/height in terminal cells |
| `--where "col=val"` | Filter rows: `=`, `!=`, `>`, `<`, `>=`, `<=` (repeatable) |
| `--title TEXT` | Custom chart title |
| `--sort desc\|asc` | Sort bar chart |
| `--top N` / `--tail N` | Limit categories |
| `--agg sum\|mean\|count\|max\|min` | Aggregation function |
| `--sample N` | Systematic sampling for large data |
| `--labels` | Show value+percentage on bars |
| `--no-header` | First row is data, not header |
| `-Y` / `--all-y` | Plot all numeric columns |
| `--watch` | Auto-redraw on file change |
| `--theme dark\|light\|high-contrast` | Color theme |
| `--bins N` | Histogram bin count (default: 10) |

## Subcommands

### Explore (interactive TUI)

```bash
vz explore data.csv
vz explore data.csv --where "city=Tokyo"
```

Vim-style navigation: `h/l` change X, `j/k` change Y, `c` cycle color,
`d`/`Tab` toggle chart↔table, `1-4` force chart type, `y` show CLI command.

### Present (terminal slides)

```bash
vz present slides.md
```

Markdown with embedded chart blocks:

````markdown
```chart
source: sales.csv
x: month
y: revenue
type: line
```
````

Navigate: `h/l` or `←/→`, jump: `g/G`, quit: `q`.

## Input Formats

- CSV (`.csv`, comma-separated)
- TSV (`.tsv`/`.tab`, tab-separated, auto-detected)
- JSON (`.json`, array of objects)
- NDJSON (`.ndjson`/`.jsonl`, newline-delimited JSON)
- Stdin (`-`), with optional `-f` to force format

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
