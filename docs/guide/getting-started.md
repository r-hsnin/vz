# Getting Started

## Installation

::: code-group

```bash [From Source]
cargo install --git https://github.com/r-hsnin/vz
```

```bash [Clone & Build]
git clone https://github.com/r-hsnin/vz
cd vz && cargo install --path .
```

:::

Requires Rust 1.70+.

## Your First Chart

```bash
vz data.csv
```

That's it. vz will:

1. **Detect the format** (CSV, TSV, JSON, NDJSON)
2. **Infer column types** (temporal, quantitative, categorical)
3. **Select the best chart** based on data types
4. **Render** a colored chart to your terminal

## Specifying Axes

```bash
# Explicit X and Y
vz sales.csv -x month -y revenue

# Override chart type
vz sales.csv -x city -y revenue -t bar

# Multi-Y overlay
vz sales.csv -y revenue,profit

# Color grouping (multi-series)
vz sales.csv -c city
```

## Output Modes

```bash
# Default: chart to stdout
vz data.csv

# Table output
vz data.csv -o table

# Sparkline (pipeline-friendly)
vz data.csv --spark

# Column metadata
vz data.csv --info
```

## Interactive Modes

```bash
# Explore: interactive TUI
vz explore data.csv

# Present: terminal slides
vz present slides.md
```

## Filtering

```bash
vz sales.csv --where "city=Tokyo"
vz sales.csv --where "revenue>1500"
```

## Shell Completions

```bash
# Bash
vz completions bash >> ~/.bashrc

# Zsh
vz completions zsh >> ~/.zshrc

# Fish
vz completions fish > ~/.config/fish/completions/vz.fish
```
