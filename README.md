# vz

[![CI](https://github.com/r-hsnin/vz/actions/workflows/ci.yml/badge.svg)](https://github.com/r-hsnin/vz/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)

CLI BI tool with smart visualization and terminal presentation.

`vz data.csv` loads a tabular file, infers each column's type, picks a chart, and renders it in the terminal.
Convention over configuration: column types determine the visualization automatically.

[Contributing](CONTRIBUTING.md) · [Architecture](docs/ARCHITECTURE.md) · [Design](docs/DESIGN.md) · [Gotchas](docs/GOTCHAS.md) · [Runbook](docs/RUNBOOK.md)

## Features

- **Auto-inference** — Detects temporal, quantitative, categorical, and nominal columns.
- **Smart chart selection** — Picks line/bar/scatter/histogram/heatmap from the resolved axis types.
- **One-shot rendering** — Prints a chart to stdout and exits; no TUI required.
- **Plain-language insights** — 0–3 `💡` takeaways on stderr (growth, leader, clusters); also `insights` in JSON and in diff output.
- **Multi-series** — Groups data by a color column, with a legend.
- **Explore mode** — Interactive TUI with vim-style navigation and a table view.
- **Present mode** — Terminal slides that embed charts from Markdown `chart` blocks.
- **Diff mode** — Compares two files with per-category change annotations or a temporal line overlay.
- **Directory mode** — Combines files with matching schemas and catalogs them.
- **Many outputs** — Text, JSON, table, Markdown, sparkline, SVG, and self-contained HTML.
- **Flexible inputs** — CSV, TSV, JSON array, NDJSON, fixed-width/space-aligned, and stdin.
- **Formatted numbers** — `1,000`, `$100`, `45%`, `10k`, `10GiB` parse as numbers on every path.

## Install

From the git repository:

```bash
cargo install --git https://github.com/r-hsnin/vz
```

Or from a local clone:

```bash
cargo install --path .
```

Requires Rust 1.88+ (MSRV).

## Usage

```bash
# Auto-visualize: infer axes and chart type
vz sales.csv

# Choose axes
vz sales.csv -x month -y revenue

# Override the chart type
vz sales.csv -x city -y revenue -t bar

# Rename a Y series
vz sales.csv -y revenue:"Revenue (USD)"

# Multi-series Y
vz sales.csv -x month -y revenue,profit

# Group by a color column (line/scatter get multi-series; bars show a legend only)
vz sales.csv -x month -y revenue -c city

# Read from stdin, optionally forcing the format
cat data.csv | vz -
kubectl top pods | vz - -f space

# Fixed-width / space-aligned input (auto-detected from content)
df -h | vz - -x "Mounted on" -y "Use%"

# Inspect column metadata instead of charting
vz sales.csv --info

# Export formats
vz sales.csv --svg > chart.svg
vz sales.csv --html > chart.html
vz sales.csv --markdown
vz sales.csv --spark

# Compare two files (diff mode)
vz before.csv after.csv

# Custom dimensions
vz sales.csv -W 80 -H 20

# Bar options: sort, top N, mean aggregation, value labels
vz sales.csv -x city -y revenue -t bar --sort desc
vz sales.csv -x city -y revenue -t bar --top 5
vz sales.csv -x city -y revenue -t bar --agg mean
vz sales.csv -x city -y revenue -t bar --labels

# Count rows per category (categorical -x with no numeric Y)
vz sales.csv -x city

# Histogram bin count
vz data.csv -y age -t histogram --bins 20

# Filter rows
vz sales.csv -x city -y revenue -w "revenue>=1000"

# Headerless data
vz raw_numbers.csv --no-header

# Interactive explore and slides
vz explore sales.csv
vz present slides.md
```

### CLI Options

`FILE` takes zero to two paths. Two paths trigger diff mode; a directory triggers directory mode; `-` reads stdin.
Flags marked **diff: ignored** are accepted but have no effect in diff mode.

| Flag | Value / Default | Description |
|------|-----------------|-------------|
| `FILE` | 0–2 paths | Input file(s); `-` for stdin; two files ⇒ diff; a directory ⇒ directory mode |
| `--diff` | `FILE2` | Second file for diff mode (alternative to `vz file1 file2`) |
| `-x`, `--x-col` | column | X axis column; supports `col:Label` |
| `-y`, `--y-col` | column[,column...] | Y axis column(s); comma-separated for multi-series; supports `col:Label` |
| `-t`, `--type` | `line`,`bar`,`scatter`,`histogram`,`heatmap` | Override chart type. **diff: silently ignored** |
| `-c`, `--color` | column | Color/group-by column. **diff: ignored with warning** |
| `-W`, `--width` | `u16`; terminal width (min 40; 80 when piped) | Chart width in columns |
| `-H`, `--height` | `u16`; adaptive, capped at 24 | Chart height in rows; shrinks for few categories/rows |
| `-I`, `--info` | flag | Print column metadata (types, stats) without a chart; `-o json` emits info JSON |
| `--no-header` | flag | Treat the first row as data (auto-detected when the first row is all-numeric) |
| `--sort` | `desc`,`asc`,`none` (default `none`) | Sort bar values; warns for chart types that ignore it |
| `-f`, `--format` | `csv`,`tsv`,`json`,`ndjson`,`space` | Force input format (otherwise auto-detected) |
| `-w`, `--where` | filter (repeatable) | Filter rows: `col=value`, `col!=value`, `col>value`, `col>=value`, `col<value`, `col<=value`. **diff: ignored with warning** |
| `--top` | `N ≥ 1` | Keep the top N categories by Y (implies `--sort desc`); conflicts with `--tail` |
| `--tail` | `N ≥ 1` | Keep the bottom N categories (implies `--sort asc`) |
| `--agg` | `sum` (default),`mean`,`count`,`max`,`min` | Bar aggregation. **diff: ignored with warning** |
| `--title` | string | Override the auto-generated title (text/svg/html) |
| `-o`, `--output` | `text` (default),`json`,`table`,`spark`,`svg`,`markdown`,`html` | Output format |
| `--json` | flag | Shorthand for `-o json` |
| `--spark` | flag | Shorthand for `-o spark` |
| `--svg` | flag | Shorthand for `-o svg` |
| `--markdown` | flag | Shorthand for `-o markdown` |
| `--html` | flag | Shorthand for `-o html` |
| `--sample` | `N ≥ 1` | Load at most N rows using systematic sampling; applies whenever `N` is less than the row count. **diff: silently ignored** |
| `-Y`, `--all-y` | flag | Overlay all quantitative columns as multi-series. **diff: silently ignored** |
| `--labels` | flag | Show value + percentage labels on bar bars. **diff: silently ignored** |
| `--watch` | flag | Watch the input file and re-render on change (not valid with stdin) |
| `--theme` | `dark` (default),`light`,`high-contrast` | Color theme (also honored by explore/present) |
| `--bins` | `1`–`10000`, default `10` | Histogram bin count. **diff: silently ignored** |
| `--glob` | pattern | Directory mode: filter files (`*`/`?` only) |
| `-R`, `--recurse` | flag | Directory mode: scan subdirectories (excludes hidden directories) |
| `--catalog` | flag | Directory mode: list per-file columns, row counts, and format; errors on a file |
| `--no-limit` | flag | Directory mode: disable the automatic 1,000,000-row sample |
| `-h`, `--help` | flag | Print help |
| `-V`, `--version` | flag | Print version |

`--bins` (1–10000) and `--top`/`--tail` (≥ 1) are range-validated before any mode runs — single-file, watch,
directory, and diff — even when the chart type ignores the flag. `--sample` must be ≥ 1; that check happens
later, only on paths that reach the render pipeline (single-file and directory), and is skipped in diff.

### Subcommands

| Command | Description |
|---------|-------------|
| `vz explore FILE [FILE2]` | Interactive TUI (2 files ⇒ diff-explore; a directory ⇒ combined, non-recursive). Accepts `-w/--where`. |
| `vz present FILE` | Markdown slide presentation with embedded charts. |
| `vz completions SHELL` | Print a shell completion script to stdout. |

Supported completion shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.

## Directory Mode

Pass a directory path to combine its data files:

```bash
vz logs/                       # combine compatible files
vz data/ --recurse             # include subdirectories
vz data/ --glob "sales_*.csv"  # filter files
vz data/ --catalog             # list schemas instead of rendering
vz data/ -c _source            # group by originating file
vz data/ --no-limit            # disable the 1M-row auto-sample
```

Each file is matched against the first file's schema by header names, compared case-insensitively and
whitespace-trimmed; files whose columns are reordered are reordered to match rather than rejected. The first
file must load successfully; later files that fail to load, have zero data rows, or have a mismatched schema
are skipped with a warning.

Every row gets two appended columns: `_source` (the file stem, or the path relative to the root without
extension when `--recurse` is used) and `_file_date` (a date extracted from the filename, or empty). Filename dates are recognized as `YYYY-MM-DD`, `YYYY_MM_DD`, or `YYYYMMDD` with years
1900–2099.

Above 1,000,000 combined rows, vz systematically samples down to that limit; `--no-limit` disables this. A
separate warning is emitted above 100,000 rows only when auto-sampling did not already fire. Progress is
reported to stderr as `info: N files, M rows`, with an `(K skipped)` suffix when files were skipped.

## Diff Mode

Trigger diff mode with two positional files or one positional plus `--diff FILE2`:

```bash
vz before.csv after.csv
vz before.csv --diff after.csv
vz q1.csv q2.csv --sort desc          # largest increases first
vz q1.csv q2.csv --sort desc --top 5  # top 5 increases
vz q1.csv q2.csv -o spark
vz q1.csv q2.csv -o json
```

Both files must share a schema: the same column count, and every "before" column name must appear in "after"
(case-insensitive, trimmed). X is taken from `-x`, else the first categorical/temporal column, else the first
header. Y is taken from `-y`, else the first quantitative column other than X.

- **Categorical X** → bar chart of per-category change, annotated `label ▲ +20%` / `▼ -10%` / `─ 0%`.
- **Temporal X** → two-series line overlay (before gray, after cyan).

For change markers, `▲ new` / `▼ new` is shown only when the before value is effectively zero and the after
value is non-zero; JSON exposes such rows as `pct_change: null` with no `new` text. When both are zero, the
marker is `─ 0%` and JSON reports `pct_change: 0`.

Diff sorting uses the **signed delta**: `desc` puts the largest increase first, not the largest absolute
change.

`--where`, `--agg`, and `--color` have no effect and warn. `-t`, `--labels`, `--sample`, `--all-y`, and
`--bins` are silently ignored in diff; other flags not in the diff parameter set (`-I`/`--info`, `--watch`,
and the directory-only `--glob`, `-R`, `--catalog`, `--no-limit`) are likewise ignored. Diff supports `text` (default),
`spark`, `json`, `markdown`, and `html`; `-o table` and `-o svg` are not supported and fall back to the
default text renderer.

## Output Format

The default text output writes a chart to stdout and supporting text to stderr:

1. A summary line on stderr, for example:
   `Line │ x=date │ y=revenue (100–500) ▁▃▅▇ │ ↑ +50% │ color=city [Tokyo=cyan, Osaka=yellow] │ 6 rows`
2. The chart with title, axis ticks, and legend on stdout.
3. 0–3 `💡` plain-language takeaways on stderr (growth, leader, clusters); silent when fewer than 2 points.

Summary components include the chart type and axes, the Y range with an inline sparkline plus trend
(`↑ +N%` / `↓ -N%` / `→ stable` within ±5%), non-sum aggregation labels such as `y=mean(revenue)`, the color
legend, the row count with skips (`6 rows (2 skipped)`), and an unused-columns hint.

ANSI color is emitted only when stdout is a TTY. `NO_COLOR` (any non-empty value) disables color;
`FORCE_COLOR` forces it. This applies to both chart output and stderr summary/insights.

Sparkline output is one line per series:

```
revenue  ▁▂▃▅▇  (100–500) ↑ +400%
```

- **JSON** (`-o json` / `--json`) — top-level `version`, `file`, `rows`, `columns[]`
  (`name,type,nulls,stats`), optional `recommendation` (`chart_type,x,y,color`), `data[]` (first 100 rows,
  numeric-parsed, non-finite → `null`), and `truncated`. Chart output adds `chart_data`, `query`
  (`chart_type,x,y,extra_y,color,agg,sort,limit,bins,filters,sample`), and `insights[]`. With `--info -o json`,
  only the info object is emitted.
- **Table** (`-o table`) — bars render as two aggregated columns; other charts emit all columns, capped at 100 rows.
- **Markdown** (`-o markdown` / `--markdown`) — GFM table with the same bar/non-bar split and 100-row cap.
- **Spark** (`-o spark` / `--spark`) — one sparkline per series/group; bars show range only; non-finite values
  are skipped with an `(N skipped)` suffix.
- **SVG** (`-o svg` / `--svg`) — a text-grid SVG plus invisible `circle.vz-point` data marks carrying
  `data-label`/`data-value`/`data-series`.
- **HTML** (`-o html` / `--html`) — self-contained HTML wrapping the SVG with JavaScript hover tooltips.

## Chart Selection Rules

When both axes are given, the chart is chosen from the resolved type pair:

| X type | Y type | Chart |
|--------|--------|-------|
| Temporal | Quantitative | Line |
| Categorical | Quantitative | Bar |
| Quantitative | Quantitative | Scatter |
| Categorical | Categorical | Heatmap |
| Quantitative | Temporal | Line (axes normalized so `x` is temporal) |
| Quantitative | Categorical | Bar (axes normalized so `x` is categorical) |
| Any other resolved pair | — | Bar fallback + warning |

This table is the normative user-facing mapping; the rationale behind it lives in
[docs/DESIGN.md](docs/DESIGN.md), and agent-oriented recipes in [skills/vz/SKILL.md](skills/vz/SKILL.md).

Reversed pairs are normalized: `-x revenue -y date` renders `x=date` as a Line, and `-x revenue -y city`
renders `x=city` as a Bar. The Bar fallback and its `warning: no chart rule for ...` message apply only to an
explicitly resolved axis pair with no dedicated rule (for example a Nominal column, or Temporal × Temporal).
They do **not** fire for automatic selection.

When only one axis is hinted:

- **Only `-y`**: first temporal column ⇒ Line; else first categorical ⇒ Bar; else first quantitative other
  than Y ⇒ Scatter; when Y is the only column, it maps by type: a quantitative or nominal Y ⇒ Histogram, a
  categorical Y ⇒ Heatmap with `x=y=Y`, and a temporal Y ⇒ Bar with the fallback warning.
- **Only `-x`**: first quantitative column other than X ⇒ its pair chart; else a categorical X ⇒ Bar of row
  counts; else a quantitative X ⇒ Histogram; otherwise an error.

With no hints, auto-selection prefers: Temporal × Quantitative ⇒ Line (color = first categorical),
Categorical × Quantitative ⇒ Bar (color none), ≥ 2 Quantitative ⇒ Scatter (color = first categorical), a
single Quantitative ⇒ Histogram, ≥ 2 Categorical ⇒ Heatmap, otherwise an error listing the detected columns.
Nominal columns do not auto-produce a Bar.

Bar charts ignore `-c` for data grouping (grouped bars are unsupported): bars stay aggregated over all rows,
and the color column appears only in the summary legend. An explicit `-c` warns; an auto-detected color
column reaches the legend without a warning.

## Explore Mode Keybindings

`vz explore FILE [FILE2]` requires an interactive terminal.

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

Diff-explore (two files) supports only sort, table scrolling, the table toggle, and yank; axis, color, and
aggregation keys report "N/A in diff mode".

## Present Mode

`vz present FILE` renders a Markdown file as terminal slides and requires a TTY. Slides are separated by
`---`; `# ` sets the slide title, and headings, lists, blockquotes, GFM tables, and fenced code blocks are
rendered. A fenced `chart` block embeds a chart:

````markdown
```chart
source: sales.csv
x: month
y: revenue
type: line
```
````

| Parameter | Description |
|-----------|-------------|
| `source` | Data file path (required) |
| `x` | X axis column |
| `y` | Y axis column |
| `color` | Color/group-by column |
| `title` | Chart title |
| `type` | `line`, `bar`, `scatter`, `histogram`, `heatmap` |
| `where` | Filter rows (repeatable): `where: revenue>1000` |
| `sort` | Bar sorting: `desc` or `asc` |
| `agg` | `sum`, `mean`, `count`, `max`, `min` |
| `top` | Show only the top N categories |
| `bins` | Histogram bin count |
| `height` | Chart height in rows |
| `diff` | "After" file for a diff chart; `source` becomes "before" |

Unknown keys are ignored. Invalid values for `type`/`sort`/`agg`/`top`/`bins`/`height` warn and fall back to
auto behavior. Chart `source` (and `diff`) paths resolve relative to the Markdown file first, then the
current working directory.

Navigation: `→` / `l` / `Space` next, `←` / `h` / `Backspace` previous, `Enter` next (or jump when digits are
pending), `g` / `G` / `Home` / `End` first/last, digits + `Enter` jump to a 1-based slide, `q` / `Esc` quit.

## Supported Input Formats

- **CSV** — comma-separated.
- **TSV** — tab-separated; detected by `.tsv`/`.tab` extension or content.
- **JSON** — array of objects; detected by `.json` extension or a `[` prefix.
- **NDJSON** — newline-delimited JSON; detected by `.ndjson`/`.jsonl` extension or a `{` prefix.
- **Fixed-width** — space-aligned text (for example `kubectl`, `df`, `ps`); auto-detected from content or
  forced with `-f space`.
- **Stdin** — `-`, with optional `-f` to force the format.

A UTF-8 BOM at the start of the input is stripped. If the first row is entirely numeric it is treated as
data, and synthetic headers `col1…colN` are generated; `--no-header` forces headerless parsing.

## Resources

- [CONTRIBUTING.md](CONTRIBUTING.md) — development setup, tests, benchmarks, and PR process.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — module structure and data flow.
- [docs/DESIGN.md](docs/DESIGN.md) — design rationale, philosophy, and scope.
- [docs/GOTCHAS.md](docs/GOTCHAS.md) — surprising behaviors and known bugs.
- [docs/RUNBOOK.md](docs/RUNBOOK.md) — release process and recovery.

## License

MIT
