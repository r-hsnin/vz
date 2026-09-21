---
name: vz
description: >
  Zero-config terminal BI CLI that charts tabular data in one command. Use when
  the task is to plot, chart, aggregate, inspect, compare, or export a
  CSV/TSV/JSON/NDJSON/fixed-width file or piped stdin with the `vz` command:
  "plot this CSV", "chart revenue by month", "aggregate sales by region",
  "compare two files", "CSVを可視化して", "グラフにして", "比較して".
  Do NOT use for pandas/matplotlib/plotly code, notebooks, or web dashboards.
---

# vz — Terminal Data Visualization

`vz` loads a table, infers column types, picks a chart, and renders it — one
command, no config. Charts go to **stdout**; the summary, `💡` insights, and
warnings go to **stderr**. For programmatic use prefer `-o json`.

## Quick start

```bash
vz data.csv                          # auto-chart (infers axes + type)
vz data.csv -x month -y revenue      # choose axes
vz data.csv -x city -y revenue -t bar --sort desc --top 5
vz data.csv -y revenue:"Revenue (USD)"   # rename a series
vz data.csv -y latency --spark       # compact text
vz data.csv -o json                  # machine-readable
vz data.csv --info                   # schema instead of a chart
cat data.csv | vz -                  # stdin
kubectl top pods | vz - -f space     # force format when piping
vz before.csv after.csv              # diff two files
vz logs/                             # combine same-schema files
```

Full flag list and every recipe: [references/cli.md](references/cli.md).

## Agent contract

- **Streams:** charts and machine formats go to stdout; the summary line, `💡`
  insights, and all warnings go to stderr. Never parse stderr as data.
- **Exit codes:** success `0`; failure `1` with no chart. Check the exit code,
  not just whether stdout parsed.
- **JSON errors go to stdout.** If the resolved format is JSON (`-o json`,
  `--json`, or `--info -o json`), an error is pretty JSON
  `{"version":1,"error":...}` on **stdout** — the one case an error is not on
  stderr.
- **`-o json` is the stable surface:** `version`, `file`, `rows`, `columns[]`,
  optional `recommendation`, `data[]` (first 100 rows), `truncated`, plus
  `chart_data`, `query`, and `insights[]` on chart output.
- **Values are numeric-parsed** (`$100` → `100`); `NaN`/`inf` become `null` in
  `data[]` and are omitted from series. `data[]` is capped at 100 rows — use
  `chart_data`/`query` for the full aggregate.

Formats, JSON fields, and schema details: [references/output-and-json.md](references/output-and-json.md).

## Gotchas that bite first

- **No argument does not mean help.** With a TTY stdin it errors; with piped
  stdin it reads stdin. Pass a file, `-`, or pipe data.
- **Bar ignores `-c` for data** (no grouped or stacked bars); the color column
  only reaches the legend, and an explicit `-c` warns.
- **Diff `--sort` is signed delta:** `desc` = largest increase, not largest
  absolute change (diff-explore's interactive sort is the `|Δ|` exception).
- **Diff silently drops `-t`, `--labels`, `--sample`, `--all-y`, `--bins`** and
  has no table/SVG output (`-o table`/`-o svg` fall back to the text renderer).
- **All-numeric first row is treated as data (headerless).** Pass `--no-header`
  when the header is numeric-looking.
- **`--bins` (1–10000) and `--top`/`--tail` (≥1) are validated in every mode**,
  even where the flag has no effect.
- **`explore` and `present` need an interactive TTY** — skip them in
  non-interactive runs.
- **Column names are case-sensitive;** unknown names error with a
  `Did you mean '...'?` hint instead of silently rendering.

More (inference, filters, directory, present, known bug): [references/troubleshooting.md](references/troubleshooting.md).

## Chart selection

With both axes given, the chart follows the resolved type pair —
Temporal × Quantitative ⇒ Line, Categorical × Quantitative ⇒ Bar,
Quantitative × Quantitative ⇒ Scatter, Categorical × Categorical ⇒ Heatmap —
and reversed pairs normalize (`-x revenue -y date` renders `x=date`, a Line).
With one or no axis hinted, auto-selection picks the chart from the column
types. Override with `-t line|bar|scatter|histogram|heatmap`.

Full rules and hint behavior: [references/selection.md](references/selection.md).

## Done when

The intended format is on stdout and the exit code is `0`. On failure, read the
error (stderr, or the JSON object on stdout for JSON output) before retrying —
do not re-run the same command blindly.
