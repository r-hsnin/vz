# Chart Types

`vz` infers each column's type — temporal, quantitative, categorical, or
nominal — and picks a chart from the resolved axis types. Override any choice
with `-t line|bar|scatter|histogram|heatmap`.

## Explicit axis pairs

With both axes given, the chart comes from the resolved type pair:

| X type | Y type | Chart |
|--------|--------|-------|
| Temporal | Quantitative | Line |
| Categorical | Quantitative | Bar |
| Quantitative | Quantitative | Scatter |
| Categorical | Categorical | Heatmap |
| Quantitative | Temporal | Line (axes normalized so `x` is temporal) |
| Quantitative | Categorical | Bar (axes normalized so `x` is categorical) |
| Any other resolved pair | — | Bar fallback + warning |

Reversed pairs are normalized: `-x revenue -y date` renders `x=date` as a Line,
and `-x revenue -y city` renders `x=city` as a Bar. The fallback and its
`warning: no chart rule for ...` message apply only to an explicitly resolved
axis pair with no dedicated rule, such as a Nominal column or
Temporal × Temporal. They do not fire for automatic selection.

## One axis hinted

**Only `-y`:**

- First temporal column ⇒ Line; otherwise the first categorical ⇒ Bar; otherwise
  the first quantitative other than Y ⇒ Scatter.
- When Y is the only column, it maps by type: quantitative or nominal Y ⇒
  Histogram, categorical Y ⇒ Heatmap with `x=y=Y`, temporal Y ⇒ Bar with the
  fallback warning.

**Only `-x`:**

- First quantitative column other than X ⇒ its pair chart.
- Otherwise a categorical X ⇒ Bar of row counts.
- Otherwise a quantitative X ⇒ Histogram.
- Otherwise an error.

## No hints

Auto-selection prefers:

| Data shape | Chart |
|---|---|
| Temporal × Quantitative | Line (color = first categorical) |
| Categorical × Quantitative | Bar (color none) |
| 2+ Quantitative | Scatter (color = first categorical) |
| 1 Quantitative | Histogram |
| 2+ Categorical | Heatmap |
| Anything else | Error listing the detected columns |

Nominal columns do not auto-produce a Bar.

## Bar charts

Bar charts aggregate by `sum` by default. `-x city` with no numeric Y counts rows
per category (`y=count(city)`); `count` is also applied when `-t bar` has no
explicit Y and Y is non-quantitative.

```bash
vz sales.csv -x city -y revenue -t bar --sort desc
vz sales.csv -x city -y revenue -t bar --top 5
vz sales.csv -x city -y revenue -t bar --tail 5
vz sales.csv -x city -y revenue -t bar --agg mean
vz sales.csv -x city -y revenue -t bar --labels
```

- `--sort desc|asc|none` orders by value; it warns for chart types that ignore it.
- `--top N` keeps the top N categories by Y and implies `--sort desc`; `--tail N`
  keeps the bottom N and implies `--sort asc`.
- `--agg` accepts `sum` (default), `mean`, `count`, `max`, and `min`.
- `--labels` prints value and percentage labels on the bars.

## Histograms

Histograms show the distribution of a single quantitative column. Set the bin
count with `--bins` (1–10000, default 10):

```bash
vz data.csv -y age -t histogram --bins 20
vz data.csv -y age -t histogram
```

## Multiple series

- Comma-separate Y columns: `-y revenue,profit`.
- Rename a series with `col:Label`: `-y revenue:"Revenue (USD)"`.
- Overlay every quantitative column with `-Y`/`--all-y`.
- `-c city` splits line and scatter charts into series.

Column names are case-sensitive. Unknown `-x`, `-y`, or `-c` names error with a
`Did you mean '...'?` hint; second and later `-y` columns are validated too, so
`-y revenue,revnue` fails instead of silently rendering one series.

## Color on bars

Bar charts never split data by color: grouped and stacked bars do not exist.
Heights stay aggregated over all rows, and the color column reaches only the
summary legend. An explicit `-c` warns; an auto-detected color does not.

## Explore interactively

`vz explore FILE` opens a TUI that cycles through the same choices live
(requires an interactive terminal).

| Key | Action |
|-----|--------|
| `h` / `l` (←/→) | Change X axis column |
| `j` / `k` (↑/↓) | Change Y axis column (chart) / scroll rows (table) |
| `g` / `G` (`Home`/`End`) | Jump to first/last row (table) |
| `PgUp` / `PgDn` | Page through the table |
| `c` | Cycle color/group-by column (reports "legend only" on bars) |
| `s` | Cycle sort order (desc/asc/none) |
| `a` | Cycle aggregation (sum/mean/count/max/min) |
| `y` | Yank the equivalent one-shot command |
| `d` / `Tab` | Toggle chart ↔ table view |
| `1`–`5` | Force chart type: Line/Bar/Scatter/Histogram/Heatmap |
| `0` | Reset to auto chart type |
| `?` | Show/hide help |
| `q` / `Esc` | Quit |

Diff-explore (two files) supports only sort, table scrolling, the table toggle,
and yank; axis, color, and aggregation keys report "N/A in diff mode".

## Next steps

- [Getting Started](./getting-started.md) — install and first steps.
- [Output Modes](./output-modes.md) — render the same chart as JSON, SVG, or HTML.
- [Diff Mode](./diff-mode.md) — compare chart values between two files.
