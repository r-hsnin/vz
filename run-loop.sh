#!/bin/bash
# run-loop.sh — External failsafe loop for looper agent
# Repeatedly invokes the agent until PROGRESS.md shows "Status: Complete"
set -uo pipefail

MAX=${1:-20}  # Default: max 20 iterations. Override: ./run-loop.sh 50
LOG_DIR="./run-logs"
mkdir -p "$LOG_DIR"

echo "=== Looper: max $MAX iterations ==="
echo "Started: $(date)"
echo ""

for i in $(seq 1 $MAX); do
    echo ""
    echo "========== Iteration $i / $MAX =========="
    START=$(date +%s)

    kiro-cli chat --no-interactive --agent looper-3agent --trust-all-tools "ループ継続" \
        > "$LOG_DIR/run-$i.log" 2>&1

    END=$(date +%s)
    ELAPSED=$((END - START))
    echo ""
    echo "  Duration: ${ELAPSED}s"

    # Check completion
    if [ -f PROGRESS.md ] && grep -qi "Status:.*Complete" PROGRESS.md; then
        echo ""
        echo "=== COMPLETED at iteration $i ==="
        break
    fi

    sleep 1
done

echo ""
echo "=== Summary ==="
echo "Ended: $(date)"
[ -f PROGRESS.md ] && head -10 PROGRESS.md
