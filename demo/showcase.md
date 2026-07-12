# SaaS Business Review — 2024

Annual metrics dashboard generated with `vz`

---

# MRR Growth by Plan

```chart
source: saas_revenue.csv
x: month
y: mrr
color: plan
type: line
title: Monthly Recurring Revenue
```

---

# Top Languages by Developer Count

```chart
source: languages.csv
x: language
y: developers
type: bar
title: Developer Survey 2024
```

---

# Cost of Living vs Income

```chart
source: cities.csv
x: avg_rent
y: median_income
type: scatter
title: Rent vs Income by City
```

---

# API Response Time Distribution

```chart
source: response_times.csv
type: histogram
title: Response Latency (ms)
```

---

# Team Skills Matrix

```chart
source: team_skills.csv
x: team
y: skill
type: heatmap
title: Skill Distribution
```

---

# Revenue vs Cost Trend

```chart
source: company_growth.csv
x: quarter
y: revenue,cost,profit
type: line
title: Financial Overview
```

---

# Framework Benchmark — Top 5

```chart
source: benchmarks.json
x: framework
y: rps
color: language
type: bar
title: Requests per Second
```

---

# Key Takeaways

- MRR grew 3.5× across all plans in 2024
- Enterprise tier shows strongest revenue growth
- Churn rate dropped from 2.1% to under 1%
- Engineering team leads in skill diversity

---

# Next Steps

- Expand to Parquet/SQLite data sources
- Streaming data support for dashboards
- Additional aggregation functions (median, percentile)
