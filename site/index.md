---
layout: home

hero:
  name: vz
  text: Zero-config terminal data visualization
  tagline: Point vz at a table. It infers column types, picks a chart, and renders it in your terminal.
  actions:
    - theme: brand
      text: Getting Started
      link: /guide/getting-started
    - theme: alt
      text: GitHub
      link: https://github.com/r-hsnin/vz

features:
  - title: Auto-inference
    details: Detects temporal, quantitative, categorical, and nominal columns.
  - title: Smart chart selection
    details: Picks line, bar, scatter, histogram, or heatmap from the resolved axis types.
  - title: Many outputs
    details: Text, JSON, table, Markdown, sparkline, SVG, and self-contained HTML.
  - title: Flexible inputs
    details: CSV, TSV, JSON, NDJSON, fixed-width text, directories, and stdin.
  - title: Diff mode
    details: Compares two files with per-category change annotations or a temporal line overlay.
  - title: Terminal native
    details: One-shot rendering to stdout, plus interactive explore and present modes.
---

## Install

```bash
cargo install --git https://github.com/r-hsnin/vz
```

Rust 1.88 or later is required.

## Quick start

```bash
# Auto-visualize: infer axes and chart type
vz sales.csv

# Choose axes and chart type
vz sales.csv -x month -y revenue
vz sales.csv -x city -y revenue -t bar

# Read from stdin
cat data.csv | vz -
```

Read the [Getting Started](./guide/getting-started.md) guide for installation
details and a tour of the CLI, or jump to [Chart Types](./guide/chart-types.md),
[Output Modes](./guide/output-modes.md), [Diff Mode](./guide/diff-mode.md), and
[Shell Completions](./guide/shell-completions.md).
