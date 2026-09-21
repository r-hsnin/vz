# vz CLI reference

Complete flag and recipe reference for the `vz` command. Load this when you need
a flag that is not in the quick start, or a worked example for a specific mode.

## Contents

- Recipes: charts, series, filters, outputs, stdin, directory, diff, interactive
- CLI options
- Subcommands
- Mode notes (directory, diff)

## Recipes

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

`FILE` takes zero to two paths. Two paths trigger diff mode; a directory
triggers directory mode; `-` reads stdin. Flags marked **diff: ignored** are
accepted but have no effect in diff mode.

## CLI options

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

`--bins` (1–10000) and `--top`/`--tail` (≥ 1) are range-validated before any
mode runs — single-file, watch, directory, and diff — even when the chart type
ignores the flag. `--sample` must be ≥ 1; that check happens later, only on
paths that reach the render pipeline (single-file and directory), and is
skipped in diff.

## Subcommands

| Command | Description |
|---------|-------------|
| `vz explore FILE [FILE2]` | Interactive TUI (2 files ⇒ diff-explore; a directory ⇒ combined, non-recursive). Accepts `-w/--where`. |
| `vz present FILE` | Markdown slide presentation with embedded charts. |
| `vz completions SHELL` | Print a shell completion script to stdout. |

Supported completion shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.

## Mode notes

**Directory mode** combines a directory's data files, matching each to the first
file's schema by header name (case-insensitive, trimmed); reordered columns are
reordered to match rather than rejected. Every row gets `_source` (file stem, or
path relative to root without extension when `--recurse`) and `_file_date` (a
date from the filename, or empty). Above 1,000,000 combined rows vz samples down
to that limit unless `--no-limit`. Files that fail to load, have zero data rows,
or mismatch the schema are skipped with a warning.

**Diff mode** requires both files to share a schema (same column count; every
"before" column appears in "after"). Categorical X ⇒ per-category change bars
(`▲ +20%` / `▼ -10%` / `─ 0%`); Temporal X ⇒ a two-series line overlay. New
categories render `▲ new`/`▼ new` (JSON `pct_change: null`); both-zero renders
`─ 0%` (JSON `pct_change: 0`). See [troubleshooting.md](troubleshooting.md) for
the flags diff ignores.
