---
layout: home

hero:
  name: vz
  text: Terminal Data Visualization
  tagline: "Zero config. Smart charts. Instant output. Just vz data.csv."
  image:
    src: /demo-placeholder.svg
    alt: vz terminal output showing a multi-series line chart
  actions:
    - theme: brand
      text: Get Started →
      link: /guide/getting-started
    - theme: alt
      text: View on GitHub
      link: https://github.com/r-hsnin/vz

features:
  - icon: 🧠
    title: Smart Auto-Detection
    details: Infers column types and picks the best chart type. No flags needed — convention over configuration.
  - icon: ⚡
    title: Zero Config
    details: Supports CSV, TSV, JSON, NDJSON. Format auto-detected from extension or content sniffing.
  - icon: 🎨
    title: Multi-Series & Color
    details: Auto-groups data by category with color legends. Multi-Y overlay and -c flag for grouping.
  - icon: 🔍
    title: Interactive TUI
    details: Explore mode with vim-style navigation. Switch axes, chart types, toggle chart ↔ table.
  - icon: 🎬
    title: Slide Presentations
    details: Present mode renders Markdown with embedded live charts. Terminal-native data talks.
  - icon: 📈
    title: Rich Summary
    details: Sparkline, trend (↑ +80%), range, color legend, and column suggestions — all in one line.
---

<style>
:root {
  --vp-home-hero-name-color: transparent;
  --vp-home-hero-name-background: -webkit-linear-gradient(120deg, #58a6ff 30%, #7ee787);
  --vp-home-hero-image-background-image: linear-gradient(-45deg, #58a6ff22 50%, #7ee78722 50%);
  --vp-home-hero-image-filter: blur(44px);
}

.dark {
  --vp-home-hero-image-background-image: linear-gradient(-45deg, #58a6ff33 50%, #7ee78733 50%);
}
</style>

## Demo

Just one command:

```bash
$ vz sales.csv
```

Output:

```
Line │ x=date │ y=revenue (800–2.0k) ▂▅▃▁█▇ │ ↑ +80% │ color=city │ 6 rows
┌revenue vs date───────────────────────────────────────────────────────────────┐
│2.0k     │revenue                                             ⡠⠔⠁     ┌──────┐│
│         │                                                 ⢀⠔⠊        │Tokyo ││
│         │                                               ⡠⠒⠁          │Osaka⣀││
│         │                                            ⢀⠔⠉    ⢀⣀⣀⣀⠤⠤⠤⠤⠒│Nagoya││
│         │                                     ⣀⣀⣀⣀⠤⠤⠤⠔⠒⠒⠒⠉⠉⠉⠁        └──────┘│
│         │                       ⣀⣀⣀⡠⠤⠤⠤⠒⠒⠒⠊⠉⠉⠉  ⣀⠔⠁                          │
│1.5k     │                                  ⡠⠔⠁                               │
│1.0k     │⠒⠊⠉⠉                                                                │
│         │                                        •                           │
│500      │                                                                date│
│         └────────────────────────────────────────────────────────────────────│
│2024-01-01            2024-02-01 2024-03-01 2024-04-01 2024-05-01   2024-06-01│
└──────────────────────────────────────────────────────────────────────────────┘
```

## Chart Selection

vz picks the right chart based on your data types:

| X Column | Y Column | Chart | Example |
|----------|----------|-------|---------|
| Temporal | Quantitative | 📈 Line | `date × revenue` |
| Categorical | Quantitative | 📊 Bar | `city × sales` |
| Quantitative | Quantitative | 🔵 Scatter | `height × weight` |
| — (single) | Quantitative | 📶 Histogram | `exam scores` |
| Categorical | Categorical | 🟦 Heatmap | `dept × level` |

## Quick Start

::: code-group

```bash [Install]
cargo install --git https://github.com/r-hsnin/vz
```

```bash [Usage]
# Auto-visualize
vz data.csv

# Axes + chart type
vz sales.csv -x month -y revenue -t bar

# Multi-series
vz sales.csv -y revenue -c city

# Pipeline
cat data.json | vz --spark
```

```bash [Explore]
# Interactive TUI
vz explore data.csv

# Presentations
vz present slides.md
```

:::
