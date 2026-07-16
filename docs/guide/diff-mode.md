# Diff Mode

Compare two data files to visualize changes between them.

## Quick Start

```bash
# Positional syntax: two files
vz before.csv after.csv

# Flag syntax: --diff
vz file1.csv --diff file2.csv
```

Both files must have matching schemas — same column names (case-insensitive). vz automatically detects whether the X column is categorical or temporal and chooses the appropriate visualization.

## Categorical Diff (Bar Chart)

When the X column is categorical (e.g., city, product name), vz renders a bar chart with ▲/▼ direction markers and percentage changes.

```bash
vz sales_before.csv sales_after.csv
```

Output:
```
Diff │ x=city │ y=revenue │ sales_before vs sales_after │ Δ net +5% │ 4 entries
  Tokyo     ████████████  1,000 → 1,200  ▲ +20%
  Osaka     ██████████    1,500 → 1,350  ▼ -10%
  Nagoya    ████████      800 → 950      ▲ +19%
  Fukuoka   █████         600 → 600      ─ 0%
```

Each entry shows:
- Category label
- Scaled bar (proportional to "after" value)
- Before → After values
- Direction marker: `▲` (increase), `▼` (decrease), `─` (unchanged)
- Percentage change

When a category exists only in "after" (new entry), the absolute value is shown instead of percentage.

## Temporal Diff (Line Chart Overlay)

When the X column is temporal (dates, timestamps), vz renders a 2-series line chart overlay:

```bash
vz timeseries_before.csv timeseries_after.csv
```

Output:
```
Line │ x=date │ timeseries_before vs timeseries_after │ Δ +25% │ 6 rows
```

The chart displays:
- **Before** series in gray (DarkGray)
- **After** series in cyan
- Legend showing file names
- Shared X axis with the union of all dates

## Output Formats

### Sparkline

```bash
vz before.csv after.csv --spark
```

Categorical:
```
Δ revenue  ▅▁▃▁  (+5%)
```

Temporal:
```
timeseries_before  ▂▃▅▆▇█
timeseries_after   ▃▄▆▇██  (+25%)
```

### JSON

```bash
vz before.csv after.csv --json
```

Categorical output:
```json
{
  "version": 1,
  "mode": "diff",
  "before": { "file": "before.csv", "rows": 4 },
  "after": { "file": "after.csv", "rows": 4 },
  "x_column": "city",
  "y_column": "revenue",
  "categories": [
    { "label": "Tokyo", "before": 1000, "after": 1200, "delta": 200, "pct_change": 20.0 }
  ],
  "overall_delta_pct": 5.1
}
```

Temporal output:
```json
{
  "version": 1,
  "mode": "diff",
  "chart_type": "line",
  "before": { "file": "before.csv", "rows": 6, "series": [...] },
  "after": { "file": "after.csv", "rows": 6, "series": [...] },
  "x_column": "date",
  "y_column": "revenue",
  "dates": ["2024-01", "2024-02", "2024-03"],
  "overall_delta_pct": 25.0
}
```

## Options

| Flag | Description |
|------|-------------|
| `--sort desc` | Sort categories by largest increase first |
| `--sort asc` | Sort categories by largest decrease first |
| `--top N` | Show only top N categories (implies `--sort desc`) |
| `--tail N` | Show only bottom N categories (implies `--sort asc`) |
| `-x` | Override X column |
| `-y` | Override Y column |

```bash
# Top 3 biggest changes
vz q1.csv q2.csv --top 3

# Sort by largest decrease
vz q1.csv q2.csv --sort asc
```

::: tip
`--sort`, `--top`, and `--tail` apply to categorical diffs only. Temporal diffs always show all dates in order.
:::

## Schema Requirements

Both files must have matching column names (case-insensitive). If schemas don't match, vz shows an error:

```
Error: Schema mismatch: column 'revenue' in 'before.csv' not found in 'after.csv'.
Before columns: [city, revenue]
After columns: [city, sales]
```

Column order doesn't matter — only names must match.

## Tips

- Duplicate categories are aggregated by sum before comparison
- Use `-x` and `-y` to override auto-detected axes
- Diff mode currently supports text, spark, and JSON output formats
