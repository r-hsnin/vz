# vz

CLI BI tool with smart visualization and terminal presentation.

**Convention over Configuration** — data types determine the visualization automatically.

## Features

- **Auto-inference** — Detects temporal, quantitative, categorical columns from data
- **Smart chart selection** — Picks the best chart type based on column types
- **One-shot output** — Renders chart to stdout and exits (no TUI needed)
- **Multi-series** — Auto-groups data by color column with legend
- **Explore mode** — Interactive TUI with vim-style navigation
- **Present mode** — Terminal slides with embedded charts from Markdown
- **Zero-config** — Just `vz data.csv` and you're done

## Install

```bash
cargo install --path .
```

Requires Rust 1.70+.

## Usage

<!-- AUTO-GENERATED: CLI reference from src/cli/mod.rs -->

```bash
# Auto-visualize (infers chart type, renders to stdout)
vz sales.csv

# Specify axes
vz sales.csv -x month -y revenue

# Override chart type
vz sales.csv -x city -y revenue -t bar

# Label override
vz sales.csv -y revenue:"Revenue (USD)"

# Multi-series Y (comma-separated)
vz sales.csv -y revenue,profit

# Group by color column (multi-series)
vz sales.csv -c city

# Read from stdin
cat data.csv | vz -

# Force input format for stdin pipes
kubectl top pods | vz - -f tsv

# TSV files (auto-detected by extension or content)
vz data.tsv

# Show column metadata instead of chart
vz sales.csv --info

# Export chart as SVG image
vz sales.csv --svg > chart.svg

# Custom chart dimensions
vz sales.csv -W 80 -H 20

# Sort bar chart
vz sales.csv -x city -y revenue -t bar --sort desc

# Headerless data
vz raw_numbers.csv --no-header

# Interactive explore mode
vz explore sales.csv

# Presentation mode
vz present slides.md
```

### CLI Options

| Flag | Long | Description |
|------|------|-------------|
| `FILE` | — | Input file (CSV/TSV/JSON/NDJSON). Use `-` for stdin |
| `-x` | `--x-col` | Column for X axis |
| `-y` | `--y-col` | Column(s) for Y axis. Comma-separated, supports `col:Label` override |
| `-t` | `--type` | Override chart type: `line`, `bar`, `scatter`, `histogram`, `heatmap` |
| `-c` | `--color` | Color/group-by column for multi-series |
| `-f` | `--format` | Force input format: `csv`, `tsv`, `json`, `ndjson` |
| `-W` | `--width` | Chart width in columns (default: terminal width) |
| `-H` | `--height` | Chart height in rows (default: 24) |
| `-I` | `--info` | Show column metadata without rendering a chart |
| `-w` | `--where` | Filter rows: `col=value`, `col>value`, `col<value` (repeatable) |
| `-o` | `--output` | Output format: `text`, `json`, `table`, `spark`, `svg` |
| `-Y` | `--all-y` | Plot all quantitative columns as multi-series overlay |
| | `--no-header` | Treat first row as data (auto-detected if all-numeric) |
| | `--sort` | Sort bar chart values: `desc`, `asc`, `none` |
| | `--top` | Show only the top N categories (implies `--sort desc`) |
| | `--tail` | Show only the bottom N categories (implies `--sort asc`) |
| | `--agg` | Aggregation: `sum` (default), `mean`, `count`, `max`, `min` |
| | `--title` | Custom chart title |
| | `--labels` | Show value + percentage labels on bar chart bars |
| | `--sample` | Sample at most N rows (systematic sampling) |
| | `--watch` | Watch file for changes and auto-redraw |
| | `--theme` | Color theme: `dark` (default), `light`, `high-contrast` |
| | `--json` | Shorthand for `--output json` |
| | `--spark` | Shorthand for `--output spark` |
| | `--svg` | Shorthand for `--output svg` |
| `-h` | `--help` | Print help |
| `-V` | `--version` | Print version |

### Subcommands

| Command | Description |
|---------|-------------|
| `vz explore <FILE>` | Interactive TUI exploration mode |
| `vz present <FILE>` | Slide presentation with embedded charts |
| `vz completions <SHELL>` | Generate shell completion scripts |

<!-- /AUTO-GENERATED -->

## Output Format

The default mode renders a chart to stdout with:
1. A summary line: `Line │ x=date │ y=revenue │ color=city │ 6 rows`
2. A Braille/Unicode chart with title, axis tick labels, and legend

Bar charts automatically aggregate (sum) values by category.

## Chart Selection Rules

| X type | Y type | Chart |
|--------|--------|-------|
| Temporal | Quantitative | Line |
| Categorical | Quantitative | Bar |
| Quantitative | Quantitative | Scatter |
| Single Quantitative | — | Histogram |
| Categorical | Categorical | Heatmap |

## Explore Mode Keybindings

| Key | Action |
|-----|--------|
| `h`/`l` (←/→) | Change X axis column |
| `j`/`k` (↑/↓) | Change Y axis column (chart) / Scroll rows (table) |
| `c` | Cycle color/group-by column |
| `y` | Show equivalent oneshot command |
| `d` / `Tab` | Toggle between chart and data table view |
| `1`-`4` | Force chart type (Line/Bar/Scatter/Histogram) |
| `0` | Auto chart type (reset) |
| `?` | Show/hide help overlay |
| `q` / `Esc` | Quit |

## Present Mode

Write Markdown with embedded chart blocks:

````markdown
# Revenue Report

```chart
source: sales.csv
x: month
y: revenue
type: line
title: Monthly Revenue
```

---

# Takeaways

- Revenue grew 80%
- Tokyo leads
````

Chart source paths resolve relative to the Markdown file's directory.

Navigate: `←`/`→` or `h`/`l`, jump: `g`/`G`, quit: `q`

## Supported Input Formats

- CSV (comma-separated)
- TSV (tab-separated, auto-detected by `.tsv`/`.tab` extension or content)
- JSON (array of objects, auto-detected by `.json` extension or `[` prefix)
- NDJSON (newline-delimited JSON, auto-detected by `.ndjson`/`.jsonl` extension or `{` prefix)
- Stdin pipe (`-`), with optional `-f` to force format

## License

MIT
