---
name: vz
description: >
  Zero-config terminal BI CLI that charts tabular data. Use when an agent needs
  to visualize, inspect, compare, or export CSV/TSV/JSON/NDJSON/fixed-width data
  or piped stdin: auto-select a chart (line, bar, scatter, histogram, heatmap),
  discover a schema, aggregate/filter/sort categories, diff two files, combine a
  directory, or emit machine-readable JSON/SVG/HTML/Markdown/sparkline. Triggers
  include "visualize data", "make a chart", "plot this CSV", "chart", "graph",
  "inspect columns", "schema", "compare two files", "データ可視化", "グラフ作成",
  "可視化して". Do NOT use for web dashboards, notebooks, or generating
  matplotlib/plotly code.
---

# vz — Terminal Data Visualization

`vz` loads a table, infers column types, picks a chart, and renders it — one
command, no config. Default chart goes to stdout; summary, insights, and warnings
go to **stderr**. For programmatic use prefer `-o json`.

## When to use / not to use

Use when the task needs a quick chart, aggregate, or schema read from tabular
data on disk or in a pipe, including two-file comparison and directory-wide
combine. Do not use for web dashboards, notebook output, or generating Python/R
plotting code. `explore` and `present` are interactive TUIs and need a real TTY.

## Core recipes

```bash
# Auto-chart (infers axes + type)
vz data.csv

# Pick axes; rename a series with col:Label
vz data.csv -x month -y revenue
vz data.csv -y revenue:"Revenue (USD)"

# Multi-series overlay and grouping
vz data.csv -x month -y revenue,profit
vz data.csv -x month -y revenue -c city      # line/scatter split by color; bars = legend only
vz data.csv -Y                               # overlay every quantitative column

# Force chart type / bar shaping
vz data.csv -x city -y revenue -t bar
vz data.csv -x city -y revenue -t bar --sort desc --top 5
vz data.csv -x city -y revenue -t bar --agg mean --labels
vz data.csv -y age -t histogram --bins 20
vz data.csv -x city                          # categorical -x, no numeric Y -> bar of row counts

# Filter rows (repeatable, ANDed); operators: = != > >= < <=
vz data.csv -x city -y revenue -w "revenue>=1000" -w "region=West"

# JSON for agent consumption; --info for schema discovery
vz data.csv -o json
vz data.csv --info
vz data.csv --info -o json | jq '.columns[] | {name, type, stats}'

# Compact text / doc exports
vz data.csv -y latency --spark
vz data.csv -o table
vz data.csv --markdown
vz data.csv --svg > chart.svg
vz data.csv --html > chart.html

# stdin (use - ; force format with -f when piping)
cat data.csv | vz -
kubectl top pods | vz - -f space

# Directory combine + schema catalog
vz logs/                                     # combine same-schema files
vz data/ --recurse --glob "sales_*.csv"
vz data/ -c _source                          # group by originating file
vz data/ --catalog                           # list schemas instead of charting
vz data/ --catalog -o json

# Diff two files (two positionals or --diff)
vz before.csv after.csv
vz before.csv --diff after.csv
vz q1.csv q2.csv --sort desc --top 5         # largest increases first
vz q1.csv q2.csv -o json                     # or -o spark / -o markdown / -o html

# Interactive (TTY only)
vz explore data.csv
vz present slides.md
```

Headerless data: pass `--no-header` when the first row is all-numeric.

## Output & exit behavior (agent contract)

- **Streams:** charts and machine formats (json, table, markdown, spark, svg,
  html, info) go to **stdout**; the summary line, `💡` insights, and all warnings
  go to **stderr**. Do not parse stderr as data.
- **Errors:** stderr + exit **1** with no chart. Gotcha: if the resolved format is
  JSON (`-o json`/`--json`, including `--info -o json`), the error is pretty JSON
  `{ "version": 1, "error": ... }` on **stdout** instead. Always check the exit
  code, never just whether stdout parsed.
- **`-o json` schema:** `version`, `file`, `rows`, `columns[]` (`name`, `type`,
  `nulls`, `stats`), optional `recommendation` (`chart_type`, `x`, `y`, `color`),
  `data[]` (first 100 rows as objects), `truncated`, plus `chart_data`
  (type-specific), `query` (`chart_type`, `x`, `y`, `extra_y`, `color`, `agg`,
  `sort`, `limit`, `bins`, `filters`, `sample`), and `insights[]`. With
  `--info -o json` only the info object is emitted.
- **`data[]` is capped at 100 rows**; use `chart_data`/`query` for the full
  aggregate. Values are numeric-parsed (`$100` → `100`); non-finite (`NaN`/`inf`)
  become `null` and are omitted from series.
- No file argument: with piped stdin it reads stdin; with a TTY it errors (never
  shows help). Insights silently disappear when they cannot be computed.

## Chart selection quick rules

Summary only — the normative source is `../../README.md#chart-selection-rules`.

With both axes given, the chart comes from the type pair; reversed pairs are
normalized (`-x revenue -y date` renders `x=date`, a Line).

| X type | Y type | Chart |
|---|---|---|
| Temporal | Quantitative | Line |
| Categorical | Quantitative | Bar |
| Quantitative | Quantitative | Scatter |
| Quantitative | Temporal | Line (axes flipped) |
| Quantitative | Categorical | Bar (axes flipped) |
| Categorical | Categorical | Heatmap |
| anything else (Nominal, Temporal×Temporal) | — | Bar (fallback + warning) |

The fallback + warning fires **only** for an explicit resolved pair with no
dedicated rule. Auto-selection never falls back: a lone Quantitative → Histogram,
≥2 Quantitative → Scatter, and Nominal columns → the no-chart error.

Hint behavior when one axis is omitted or nothing is given:

- **Only Y:** first Temporal → Line when Y is Quantitative (else Bar + fallback
  warning); else first Categorical → Bar when Y is Quantitative, else Heatmap
  `x=y=Y`; else another Quantitative → Scatter; else lone Y (Quantitative or
  Nominal) → Histogram.
- **Only X:** first other Quantitative → pair chart; else categorical X → Bar of
  row counts (`y=count(x)`); else quantitative X → Histogram; else error.
- **No hints:** Temporal×Quant → Line; Cat×Quant → Bar; ≥2 Quant → Scatter;
  1 Quant → Histogram; ≥2 Cat → Heatmap; else the no-chart error.
- Auto color = first categorical not used as X/Y, applied via `find_color_column`
  on most hinted/explicit paths (Line/Scatter split series; on Bar it feeds the
  legend only). Some paths leave it unset — e.g. only-Y paired with a Quantitative
  X, and the no-hint Categorical×Quantitative Bar. Extra `-y` columns suppress
  auto color.

Override any choice with `-t line|bar|scatter|histogram|heatmap`.

## Gotchas to avoid

- Bar ignores `-c` for data (grouped bars unsupported); an explicit `-c` warns.
- `--bins` (1–10000) and `--top`/`--tail` (≥1) are validated in every mode, even
  formats that ignore them. `--sample 0` is rejected only on single-file/directory
  paths; `--sample` is ignored by diff.
- Diff `--sort` sorts by **signed** delta: `desc` = largest increase, not largest
  absolute change. `diff-explore` is the exception — its interactive sort orders by
  `|Δ|` (non-diff explore uses the canonical signed sort). Categories with no
  before value render `▲ new`/`▼ new`; removed ones render `▼ -100%`.
- Diff silently ignores `-t`, `--labels`, `--sample`, `--all-y`, `--bins` and has
  **no table or SVG output** — `-o table`/`-o svg` fall back to the text renderer.
  Diff also warns (no effect) for `-w`/`--agg`/`--color`.
- `explore` and `present` require an interactive TTY; do not invoke them in
  non-interactive agent runs. (`explore` no-ops under `VZ_TEST_HEADLESS`.)
- All-numeric first row is treated as data (headerless); pass `--no-header` when
  headers are numeric-looking.
- `NO_COLOR` (any non-empty value) disables ANSI; a piped stdout forces chart
  width to 80 regardless of `COLUMNS`.
- CSV rows that fail to parse are skipped with a `warning: skipping row N`.

## Pointers

- `../../README.md` — full CLI reference, install, present-mode markdown syntax.
- `../../docs/GOTCHAS.md` — catalog of non-obvious behavior and known issues.
- `../../docs/ARCHITECTURE.md` — modules, data flow, mode/plane map.
- `../../docs/DESIGN.md` — design intent and decisions.
- `../../CONTRIBUTING.md` — dev setup, tests, snapshots, benchmarks.
