#!/usr/bin/env bash
# vz Demo Script — Showcases all features and chart types
# Usage: ./demo/run_demo.sh
# Each section can be run independently by copy-pasting commands.

set -euo pipefail
cd "$(dirname "$0")/.."

# Use installed vz or fall back to cargo run
if command -v vz &>/dev/null; then
    VZ="vz"
else
    VZ="cargo run --quiet --"
fi

# Colors for section headers
BOLD='\033[1m'
DIM='\033[2m'
RESET='\033[0m'

section() {
    echo ""
    echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo -e "${BOLD}  $1${RESET}"
    echo -e "${DIM}  $2${RESET}"
    echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
}

pause() {
    echo ""
    echo -e "${DIM}  [Press Enter to continue]${RESET}"
    read -r
}

# ═══════════════════════════════════════════════════════════════════════════════
# 1. AUTO-DETECT: Zero configuration
# ═══════════════════════════════════════════════════════════════════════════════

section "1. Auto-Detect" "vz automatically infers types and picks the best chart"

echo '$ vz demo/saas_revenue.csv'
$VZ demo/saas_revenue.csv
pause

echo '$ vz demo/languages.csv'
$VZ demo/languages.csv
pause

echo '$ vz demo/response_times.csv'
$VZ demo/response_times.csv
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 2. LINE CHART: Multi-series with color grouping
# ═══════════════════════════════════════════════════════════════════════════════

section "2. Line Chart" "Temporal × Quantitative → Line, with multi-series"

echo '$ vz demo/saas_revenue.csv -x month -y mrr -c plan'
$VZ demo/saas_revenue.csv -x month -y mrr -c plan
pause

echo '$ vz demo/company_growth.csv -y revenue,cost,profit'
$VZ demo/company_growth.csv -y revenue,cost,profit
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 3. BAR CHART: Categories, sorting, top/tail
# ═══════════════════════════════════════════════════════════════════════════════

section "3. Bar Chart" "Categorical × Quantitative → Bar, with --sort and --top"

echo '$ vz demo/languages.csv -x language -y developers --sort desc'
$VZ demo/languages.csv -x language -y developers --sort desc
pause

echo '$ vz demo/languages.csv -x language -y satisfaction --top 5'
$VZ demo/languages.csv -x language -y satisfaction --top 5
pause

echo '$ vz demo/languages.csv -x language -y avg_salary --tail 3'
$VZ demo/languages.csv -x language -y avg_salary --tail 3
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 4. SCATTER PLOT: Two quantitative columns
# ═══════════════════════════════════════════════════════════════════════════════

section "4. Scatter Plot" "Quantitative × Quantitative → Scatter"

echo '$ vz demo/cities.csv -x avg_rent -y median_income'
$VZ demo/cities.csv -x avg_rent -y median_income
pause

echo '$ vz demo/benchmarks.json -x latency_us -y rps -c language'
$VZ demo/benchmarks.json -x latency_us -y rps -c language
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 5. HISTOGRAM: Distribution of a single variable
# ═══════════════════════════════════════════════════════════════════════════════

section "5. Histogram" "Single Quantitative → Histogram"

echo '$ vz demo/response_times.csv'
$VZ demo/response_times.csv
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 6. HEATMAP: Categorical × Categorical
# ═══════════════════════════════════════════════════════════════════════════════

section "6. Heatmap" "Categorical × Categorical → Heatmap (count)"

echo '$ vz demo/team_skills.csv'
$VZ demo/team_skills.csv
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 7. FILTERING: --where flag
# ═══════════════════════════════════════════════════════════════════════════════

section "7. Data Filtering" "--where for subsetting data"

echo '$ vz demo/sales_data.csv -x region -y revenue -t bar --where "product=Widget A"'
$VZ demo/sales_data.csv -x region -y revenue -t bar --where "product=Widget A"
pause

echo '$ vz demo/sales_data.csv -x quarter -y revenue --where "region=Asia" --where "revenue>80000" -c product'
$VZ demo/sales_data.csv -x quarter -y revenue --where "region=Asia" --where "revenue>80000" -c product
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 8. INPUT FORMATS: CSV, TSV, JSON, stdin
# ═══════════════════════════════════════════════════════════════════════════════

section "8. Input Formats" "CSV, TSV, JSON auto-detected. Stdin pipe support."

echo '$ vz demo/api_latency.tsv -x endpoint -y p95_ms --sort desc'
$VZ demo/api_latency.tsv -x endpoint -y p95_ms --sort desc
pause

echo '$ vz demo/benchmarks.json -x framework -y rps --top 5'
$VZ demo/benchmarks.json -x framework -y rps --top 5
pause

echo '$ seq 1 20 | awk ... | vz   (stdin pipe, no file argument)'
seq 1 20 | awk '{print $1","($1*$1)}' | (echo "x,y"; cat) | $VZ
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 9. METADATA: --info flag
# ═══════════════════════════════════════════════════════════════════════════════

section "9. Column Info" "--info shows metadata without rendering"

echo '$ vz demo/saas_revenue.csv --info'
$VZ demo/saas_revenue.csv --info
pause

echo '$ vz demo/cities.csv --info'
$VZ demo/cities.csv --info
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 10. AGGREGATION: --agg mean/count/max/min
# ═══════════════════════════════════════════════════════════════════════════════

section "10. Aggregation" "--agg changes how bar charts aggregate values"

echo '$ vz demo/sales_data.csv -x product -y revenue --agg sum'
$VZ demo/sales_data.csv -x product -y revenue --agg sum
pause

echo '$ vz demo/sales_data.csv -x product -y revenue --agg mean'
$VZ demo/sales_data.csv -x product -y revenue --agg mean
pause

echo '$ vz demo/sales_data.csv -x region -y units_sold --agg count'
$VZ demo/sales_data.csv -x region -y units_sold --agg count
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 11. LABELS: Value + percentage on bars
# ═══════════════════════════════════════════════════════════════════════════════

section "11. Labels & Percentage" "--labels shows values and percentages on bars"

echo '$ vz demo/languages.csv -x language -y developers --top 5 --sort desc --labels'
$VZ demo/languages.csv -x language -y developers --top 5 --sort desc --labels
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 12. THEMES: dark, light, high-contrast
# ═══════════════════════════════════════════════════════════════════════════════

section "12. Themes" "--theme changes color palette for different environments"

echo '$ vz demo/saas_revenue.csv --theme dark   (default)'
$VZ demo/saas_revenue.csv --theme dark
pause

echo '$ vz demo/saas_revenue.csv --theme light  (for white backgrounds)'
$VZ demo/saas_revenue.csv --theme light
pause

echo '$ vz demo/saas_revenue.csv --theme high-contrast  (accessibility)'
$VZ demo/saas_revenue.csv --theme high-contrast
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 13. OUTPUT FORMATS: svg, markdown, spark
# ═══════════════════════════════════════════════════════════════════════════════

section "13. Output Formats" "--svg, --markdown, --spark for export and pipelines"

echo '$ vz demo/languages.csv -x language -y developers --top 5 --svg | head -5'
$VZ demo/languages.csv -x language -y developers --top 5 --svg | head -5
echo '  ...(SVG output continues)'
pause

echo '$ vz demo/languages.csv -x language -y developers --top 5 --markdown'
$VZ demo/languages.csv -x language -y developers --top 5 --markdown
pause

echo '$ vz demo/saas_revenue.csv --spark'
$VZ demo/saas_revenue.csv --spark
pause

echo '$ vz demo/saas_revenue.csv --spark -c plan'
$VZ demo/saas_revenue.csv --spark -c plan
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 14. HISTOGRAM BINS: --bins N
# ═══════════════════════════════════════════════════════════════════════════════

section "14. Histogram Bins" "--bins controls the number of histogram bins"

echo '$ vz demo/response_times.csv --bins 5'
$VZ demo/response_times.csv --bins 5
pause

echo '$ vz demo/response_times.csv --bins 20'
$VZ demo/response_times.csv --bins 20
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 15. ALL-Y OVERLAY: -Y plots all numeric columns
# ═══════════════════════════════════════════════════════════════════════════════

section "15. All-Y Overlay" "-Y / --all-y overlays all numeric columns"

echo '$ vz demo/company_growth.csv -Y'
$VZ demo/company_growth.csv -Y
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 16. WATCH MODE & COMPLETIONS
# ═══════════════════════════════════════════════════════════════════════════════

section "16. Watch & Completions" "--watch for live updates, completions for shells"

echo '$ vz demo/saas_revenue.csv --watch'
echo -e "${DIM}  (Watches file and re-renders on change. Press Ctrl+C to stop.)${RESET}"
echo -e "${DIM}  Skipping in automated demo. Try it manually!${RESET}"
pause

echo '$ vz completions bash | head -5'
$VZ completions bash | head -5
echo '  ...(completions continue for all flags)'
pause

# ═══════════════════════════════════════════════════════════════════════════════
# 17. PRESENT MODE: Markdown slides with embedded charts
# ═══════════════════════════════════════════════════════════════════════════════

section "17. Present Mode" "vz present <file.md> — terminal slide deck"

echo '$ vz present demo/showcase.md'
echo -e "${DIM}  (Interactive mode — use ←/→ to navigate, q to quit)${RESET}"
echo -e "${DIM}  Skipping in automated demo. Run manually:${RESET}"
echo -e "${DIM}    vz present demo/showcase.md${RESET}"
pause

# ═══════════════════════════════════════════════════════════════════════════════

section "Demo Complete!" "Try 'vz explore demo/saas_revenue.csv' for interactive mode"
echo -e "${DIM}  All demos use data from the demo/ directory.${RESET}"
echo -e "${DIM}  Run 'vz <file>' on any CSV/TSV/JSON for instant visualization.${RESET}"
echo -e "${DIM}  Full CLI reference: vz --help${RESET}"
echo ""
