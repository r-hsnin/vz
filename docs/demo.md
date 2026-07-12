# Demo

Real output from `vz` with the included sample data files.

## Line Chart — Multi-Series Time Series

```bash
$ vz sales.csv
```

Automatically detects temporal X, quantitative Y, and categorical color column:

```
Line │ x=date │ y=revenue (800–2.0k) ▂▅▃▁█▇ │ ↑ +80% │ color=city │ 6 rows
┌revenue vs date───────────────────────────────────────────────────────────────┐
│2.0k     │revenue                                             ⡠⠔⠁     ┌──────┐│
│         │                                                 ⢀⠔⠊        │Tokyo ││
│         │                                               ⡠⠒⠁          │Osaka⣀││
│         │                                            ⢀⠔⠉    ⢀⣀⣀⣀⠤⠤⠤⠤⠒│Nagoya││
│         │                                     ⣀⣀⣀⣀⠤⠤⠤⠔⠒⠒⠒⠉⠉⠉⠁        └──────┘│
│         │                       ⣀⣀⣀⡠⠤⠤⠤⠒⠒⠒⠊⠉⠉⠉  ⣀⠔⠁                          │
│         │             ⠠⠤⠔⠒⠒⠒⠉⠉⠉⠉             ⢀⠤⠊                             │
│1.5k     │                                  ⡠⠔⠁                               │
│1.0k     │⠒⠊⠉⠉                                                                │
│         │                                        •                           │
│500      │                                                                date│
│         └────────────────────────────────────────────────────────────────────│
│2024-01-01            2024-02-01 2024-03-01 2024-04-01 2024-05-01   2024-06-01│
└──────────────────────────────────────────────────────────────────────────────┘
```

## Bar Chart — Categorical Aggregation

```bash
$ vz sales.csv -x city -y revenue -t bar
```

Aggregates values by category with colored bars:

```
Bar │ x=city │ y=revenue (800–4.2k) │ 6 rows
     revenue by city───────────────────────────────────────────────┐
4.0k│████████████████████                                          │
    │████████████████████                                          │
    │████████████████████ ▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅▅                    │
3.0k│████████████████████ ████████████████████                      │
2.0k│████████████████████ ████████████████████                      │
1.0k│████████████████████ ████████████████████ ████████████████████  │
   0│████████4.2k████████ ████████3.3k████████ ████████800█████████  │
            Tokyo                Osaka                Nagoya         │
     ──────────────────────────────────────────────────────────────┘
```

## Sparkline — Pipeline-Friendly

```bash
$ vz sales.csv --spark
▂▅▃▁█▇

$ vz stock.csv --spark
▁▂▂▃▄▇▇█

$ echo "Revenue trend: $(vz sales.csv --spark)"
Revenue trend: ▂▅▃▁█▇
```

## Column Info

```bash
$ vz sales.csv --info
```

```
File: sales.csv
Rows: 6
Columns: 4

Name                 Type             Nulls  Stats
----------------------------------------------------------------------
date                 Temporal             0  2024-01-01..2024-06-01
city                 Categorical          0  3 unique
revenue              Quantitative         0  Min=800  Max=2000  Mean=1383.33
profit               Quantitative         0  Min=150  Max=500  Mean=313.33

Recommendation: Line (x=date, y=revenue, color=city)
```

## Table Output

```bash
$ vz sales.csv -o table
date        revenue
----------  -------
2024-01-01     1000
2024-02-01     1500
2024-03-01     1200
2024-04-01      800
2024-05-01     2000
2024-06-01     1800
```

## Filtering

```bash
$ vz sales.csv --where "city=Tokyo" -o table
date        revenue
----------  -------
2024-01-01     1000
2024-03-01     1200
2024-05-01     2000

$ vz sales.csv --where "revenue>1500" -o table
date        revenue
----------  -------
2024-05-01     2000
2024-06-01     1800
```

## JSON Input

```bash
$ echo '[{"name":"Alice","score":92},{"name":"Bob","score":85},{"name":"Carol","score":78}]' | vz - -f json
Bar │ x=name │ y=score (78–92) │ 3 rows
```

## Present Mode

Terminal slide presentations with embedded live charts.

```bash
$ vz present slides.md
```

See [Output Modes → Present Mode](/guide/output-modes#present-mode) for the full chart block syntax.

## SVG Export

```bash
$ vz sales.csv -x city -y revenue -t bar --svg > chart.svg
```

Generates a monospace-text SVG image matching the terminal rendering. Supports `--theme light` for white-background documents.

## Markdown Output

```bash
$ vz sales.csv -x city -y revenue -t bar --markdown
```

```markdown
| city | revenue |
|---|---|
| Tokyo | 4200 |
| Osaka | 3300 |
| Nagoya | 800 |
```

Useful for embedding aggregated results in README files and GitHub issues.

## Labels & Percentages

```bash
$ vz sales.csv -x city -y revenue -t bar --labels
```

Shows value and percentage on each bar: `████ 4.2k (51%)`. Great for presentations and quick data sharing.

## Aggregation Functions

```bash
# Default: sum
$ vz data.csv -x product -y revenue --agg sum

# Average per category
$ vz data.csv -x product -y revenue --agg mean

# Count rows per category
$ vz data.csv -x region -y id --agg count
```

Summary line reflects the function: `y=mean(revenue)` instead of `y=revenue`.
