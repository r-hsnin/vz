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
# 10. PRESENT MODE: Markdown slides with embedded charts
# ═══════════════════════════════════════════════════════════════════════════════

section "10. Present Mode" "vz present <file.md> — terminal slide deck"

echo '$ vz present demo/showcase.md'
echo -e "${DIM}  (Interactive mode — use ←/→ to navigate, q to quit)${RESET}"
echo -e "${DIM}  Skipping in automated demo. Run manually:${RESET}"
echo -e "${DIM}    vz present demo/showcase.md${RESET}"
pause

# ═══════════════════════════════════════════════════════════════════════════════

section "Demo Complete!" "Try 'vz explore demo/saas_revenue.csv' for interactive mode"
echo -e "${DIM}  All demos use data from the demo/ directory.${RESET}"
echo -e "${DIM}  Run 'vz <file>' on any CSV/TSV/JSON for instant visualization.${RESET}"
echo ""
