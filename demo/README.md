# vz Demo

High-quality datasets and scripts for demonstrating all vz features.

## Quick Start

```bash
# Run the full interactive demo (10 sections)
./demo/run_demo.sh

# Or run individual commands:
vz demo/saas_revenue.csv                           # Auto-detect → Line
vz demo/languages.csv -x language -y developers    # Bar chart
vz demo/cities.csv -x avg_rent -y median_income    # Scatter plot
vz demo/response_times.csv                         # Histogram
vz demo/team_skills.csv                            # Heatmap
vz demo/company_growth.csv -y revenue,cost,profit  # Multi-Y series

# Present mode (slides)
vz present demo/showcase.md
```

## Datasets

| File | Chart Type | Rows | Description |
|------|-----------|------|-------------|
| `saas_revenue.csv` | Line (multi-series) | 36 | Monthly recurring revenue by plan tier |
| `languages.csv` | Bar | 15 | Developer survey — popularity, salary, satisfaction |
| `cities.csv` | Scatter | 20 | Global cities — rent vs median income |
| `response_times.csv` | Histogram | 100 | API response time distribution (ms) |
| `team_skills.csv` | Heatmap | 60 | Team × skill matrix (count) |
| `company_growth.csv` | Line (multi-Y) | 12 | Quarterly revenue, cost, profit |
| `sales_data.csv` | Bar (filtered) | 36 | Multi-product sales for `--where` demos |
| `api_latency.tsv` | Bar (TSV) | 10 | Endpoint latency percentiles |
| `benchmarks.json` | Scatter/Bar (JSON) | 12 | Web framework benchmarks |

## Feature Coverage

| Feature | Demo Command |
|---------|-------------|
| Auto-detect | `vz demo/saas_revenue.csv` |
| `-x` / `-y` axis | `vz demo/cities.csv -x avg_rent -y median_income` |
| `-c` color grouping | `vz demo/saas_revenue.csv -c plan` |
| `--sort` | `vz demo/languages.csv -x language -y developers --sort desc` |
| `--top N` | `vz demo/languages.csv -x language -y satisfaction --top 5` |
| `--tail N` | `vz demo/languages.csv -x language -y avg_salary --tail 3` |
| `--where` filter | `vz demo/sales_data.csv --where "product=Widget A" -t bar` |
| Multi-Y | `vz demo/company_growth.csv -y revenue,cost,profit` |
| TSV input | `vz demo/api_latency.tsv` |
| JSON input | `vz demo/benchmarks.json` |
| Stdin pipe | `cat demo/saas_revenue.csv \| vz` |
| `--info` | `vz demo/cities.csv --info` |
| Present mode | `vz present demo/showcase.md` |
| Explore mode | `vz explore demo/saas_revenue.csv` |
