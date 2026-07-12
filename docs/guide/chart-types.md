# Chart Types

vz automatically selects the best chart type based on your data's column types.

## Selection Rules

| X Type | Y Type | Chart | When |
|--------|--------|-------|------|
| Temporal | Quantitative | 📈 Line | Time series data |
| Categorical | Quantitative | 📊 Bar | Categories with values |
| Quantitative | Quantitative | 🔵 Scatter | Two numeric columns |
| — | Quantitative | 📶 Histogram | Single numeric distribution |
| Categorical | Categorical | 🟦 Heatmap | Two categorical columns |

## Line Chart

Best for **time series** data. Detected when X is temporal (dates) and Y is numeric.

```bash
vz stock.csv
# Automatically renders a line chart: date × price
```

Features:
- Braille-character rendering for high resolution
- Multi-series with color legend (via `-c` flag)
- Trend indicator (↑ +80% or ↓ -20%)

## Bar Chart

Best for **categorical comparisons**. Detected when X is categorical and Y is numeric.

```bash
vz sales.csv -x city -y revenue -t bar
```

Features:
- Auto-aggregation (sum by default, configurable with `--agg`)
- Sorting: `--sort desc` or `--sort asc`
- Top/tail limiting: `--top 10` or `--tail 5`
- Value labels embedded in bars

## Scatter Plot

Best for **correlation analysis**. Detected when both X and Y are numeric.

```bash
vz body_measurements.csv
# Renders: height × weight scatter plot
```

Features:
- Braille dots for precise positioning
- Multi-series color support

## Histogram

Best for **distribution analysis**. Selected when only one numeric column is available.

```bash
vz exam_scores.csv
# Shows distribution of scores across bins
```

Features:
- Automatic bin calculation
- Frequency labels

## Heatmap

For **two categorical** columns. Shows count-based density.

```bash
vz departments.csv -x department -y level
```

## Override Chart Type

Force any chart type with `-t`:

```bash
vz data.csv -t line
vz data.csv -t bar
vz data.csv -t scatter
vz data.csv -t histogram
vz data.csv -t heatmap
```
