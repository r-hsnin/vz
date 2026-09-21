# Getting Started

`vz` is a CLI BI tool that turns a table into a terminal chart. Point it at a CSV,
TSV, JSON, or NDJSON file and it infers each column's type, picks a
visualization, renders it once, and exits. No configuration and no interactive
session are required.

## Install

Install from the Git repository:

```bash
cargo install --git https://github.com/r-hsnin/vz
```

Or from a local clone:

```bash
cargo install --path .
```

Rust 1.88 or later is required.

## Your first chart

```bash
# Auto-visualize: infer axes and chart type
vz sales.csv

# Choose axes
vz sales.csv -x month -y revenue

# Override the chart type
vz sales.csv -x city -y revenue -t bar
```

The chart is written to stdout, while the summary line, insights, and warnings
go to stderr. That split lets you pipe or redirect the chart without losing the
supporting text. To inspect the schema instead of rendering, run
`vz sales.csv --info`; see [Output Modes](./output-modes.md) for the JSON form.

## Reading from stdin

Pass `-` as the file name. Use `-f` to force the format when detection is
ambiguous, for example a one-column piped TSV:

```bash
cat data.csv | vz -
kubectl top pods | vz - -f space
printf '1,2\n3,4\n' | vz -
```

With no file argument and a TTY stdin, `vz` errors and asks for a file or piped
data; it does not print help. Piped stdin is read instead.

## Input formats

- **CSV** — comma-separated.
- **TSV** — tab-separated; detected by `.tsv`/`.tab` extension or content.
- **JSON** — array of objects.
- **NDJSON** — newline-delimited JSON; detected by extension or a `{` prefix.
- **Fixed-width / space-aligned** — for example `kubectl`, `df`, or `ps` output.
- **Stdin** — `-`, optionally with `-f`.

A leading UTF-8 BOM is stripped. If the first row is entirely numeric it is
treated as headerless data and gets synthetic headers `col1`, `col2`, ...;
`--no-header` forces this behavior.

## Modes at a glance

| Invocation | What it does |
|---|---|
| `vz FILE` | One-shot chart (default) |
| `vz FILE1 FILE2` / `vz FILE --diff FILE2` | [Diff mode](./diff-mode.md) |
| `vz DIRECTORY` | Combine files with matching schemas |
| `vz explore FILE [FILE2]` | Interactive TUI (requires a TTY) |
| `vz present FILE` | Markdown slides with embedded charts (requires a TTY) |
| `vz FILE --watch` | Re-render when the file changes (not valid with stdin) |
| `vz completions SHELL` | Print a completion script; see [Shell Completions](./shell-completions.md) |

## CLI options

`FILE` takes zero to two paths. Two paths trigger diff mode; a directory
triggers directory mode; `-` reads stdin. Flags marked **diff: ignored** are
accepted but have no effect in diff mode.

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
ignores the flag.

## Next steps

- [Chart Types](./chart-types.md) — how the chart is selected and shaped.
- [Output Modes](./output-modes.md) — text, JSON, table, sparkline, SVG, HTML.
- [Diff Mode](./diff-mode.md) — compare two files.
- [Shell Completions](./shell-completions.md) — completion for bash, zsh, fish, elvish, and PowerShell.
