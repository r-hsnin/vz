# Output Modes

vz supports multiple output formats to fit different workflows.

## Chart (Default)

Renders a colored terminal chart with summary line.

```bash
vz data.csv
```

The summary includes:
- Chart type and axes
- Value range and sparkline
- Trend direction (↑/↓ with percentage)
- Color legend for multi-series
- Suggestions for additional columns

## Table

Structured text table output. Useful for quick data inspection.

```bash
vz data.csv -o table
```

For bar charts, shows aggregated values. For other types, shows raw X/Y data.

## Sparkline

Single-line Unicode sparkline. Perfect for embedding in scripts, prompts, or dashboards.

```bash
vz data.csv --spark
# Output: ▂▅▃▁█▇
```

Combine with other tools:
```bash
echo "Revenue: $(vz sales.csv --spark)"
# Revenue: ▂▅▃▁█▇
```

## Info

Column metadata without rendering a chart. Shows types, stats, and chart recommendation.

```bash
vz data.csv --info
```

Output includes:
- File name, row count, column count
- Per-column: name, inferred type, null count, statistics
- Recommended chart type and axis mapping

## JSON

Machine-readable metadata output. Includes column stats, chart recommendation, and data sample.

```bash
vz data.csv -o json
```

Use with `--info` for metadata only, or without for chart data + metadata:
```bash
# Metadata only
vz data.csv --info -o json

# Full: metadata + chart_data (aggregated values, series)
vz data.csv -o json
```

## SVG

Export charts as SVG images. The output is a monospace-text SVG that matches the terminal rendering.

```bash
# Save chart as SVG
vz data.csv --svg > chart.svg

# With specific dimensions
vz data.csv -W 100 -H 30 --svg > wide-chart.svg

# Light theme for white backgrounds (documents/wikis)
vz data.csv --svg --theme light > chart-light.svg
```

SVG output respects the `--theme` flag — dark produces a dark background, light produces white.

## Markdown

Export aggregated data as GitHub Flavored Markdown (GFM) tables. Perfect for README files, issues, and documentation.

```bash
# Markdown table of bar chart aggregation
vz sales.csv -x city -y revenue -t bar --markdown

# With sorting
vz data.csv -x category -y value --sort desc --markdown
```

Output example:
```markdown
| city | revenue |
|---|---|
| Tokyo | 4200 |
| Osaka | 3300 |
| Nagoya | 800 |
```

## Three Interactive Modes

### One-shot (default)
Renders chart and exits. Ideal for pipelines and quick inspection.

### Explore Mode
Interactive TUI with vim-style navigation.

```bash
vz explore data.csv
```

| Key | Action |
|-----|--------|
| `h`/`l` | Change X axis |
| `j`/`k` | Change Y axis / scroll table |
| `d` / `Tab` | Toggle chart ↔ table |
| `1`-`4` | Force chart type |
| `0` | Auto (reset) |
| `q` | Quit |

### Present Mode
Terminal slide presentation with embedded live charts.

```bash
vz present slides.md
```

Write slides in Markdown with chart blocks:

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
````

Navigate: `←`/`→` or `h`/`l`, jump: `g`/`G`, quit: `q`

## Watch Mode

Auto-redraw when the input file changes. Useful during data exploration or ETL development.

```bash
vz data.csv --watch
```

- Debounced at 200ms (no flicker on rapid saves)
- Works with atomic writes (editor save)
- Press `Ctrl+C` to stop

## Themes

Control color palette for different terminal backgrounds.

```bash
# Dark terminal (default)
vz data.csv --theme dark

# Light/white terminal background
vz data.csv --theme light

# Maximum contrast, colorblind-friendly
vz data.csv --theme high-contrast
```

Themes affect all charts, summary lines, and SVG export background color.
