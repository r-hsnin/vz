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

Machine-readable metadata output.

```bash
vz data.csv --info -o json
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
