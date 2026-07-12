#!/usr/bin/env bash
# vz Demo Cheat Sheet — Copy-paste individual commands
# All commands assume you're in the vz project root.
# Replace 'vz' with 'cargo run --quiet --' if not installed.

# ─── AUTO-DETECT (zero config) ─────────────────────────────────────────────
vz demo/saas_revenue.csv              # → Line chart (temporal detected)
vz demo/languages.csv                 # → Bar chart (categorical detected)
vz demo/response_times.csv            # → Histogram (single quantitative)
vz demo/team_skills.csv               # → Heatmap (categorical × categorical)
vz demo/cities.csv                    # → Scatter (quantitative × quantitative)

# ─── LINE CHART (temporal × quantitative) ──────────────────────────────────
vz demo/saas_revenue.csv -x month -y mrr -c plan           # Multi-series by plan
vz demo/company_growth.csv -y revenue,cost,profit           # Multi-Y overlay

# ─── BAR CHART (categorical × quantitative) ────────────────────────────────
vz demo/languages.csv -x language -y developers --sort desc # Sorted descending
vz demo/languages.csv -x language -y satisfaction --top 5   # Top 5 only
vz demo/languages.csv -x language -y avg_salary --tail 3    # Bottom 3 only

# ─── SCATTER PLOT (quantitative × quantitative) ────────────────────────────
vz demo/cities.csv -x avg_rent -y median_income             # Rent vs income
vz demo/benchmarks.json -x latency_us -y rps -c language    # Grouped by language

# ─── DATA FILTERING (--where) ──────────────────────────────────────────────
vz demo/sales_data.csv -t bar --where "product=Widget A"                     # Exact match
vz demo/sales_data.csv --where "region=Asia" --where "revenue>80000" -c product  # Combined

# ─── INPUT FORMATS ─────────────────────────────────────────────────────────
vz demo/api_latency.tsv -x endpoint -y p95_ms --sort desc   # TSV auto-detected
vz demo/benchmarks.json -x framework -y rps --top 5         # JSON array
cat demo/saas_revenue.csv | vz                               # Stdin pipe (no '-')

# ─── METADATA ──────────────────────────────────────────────────────────────
vz demo/saas_revenue.csv --info                              # Column types & stats
vz demo/cities.csv --info                                    # Quick data profiling

# ─── INTERACTIVE MODES ─────────────────────────────────────────────────────
vz explore demo/saas_revenue.csv      # TUI: hjkl to change axes, 1-4 chart type
vz present demo/showcase.md           # Slides: ←/→ navigate, q quit
