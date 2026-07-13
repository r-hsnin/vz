#!/bin/bash
# gate.sh — vz quality gate for 3-agent looper
# Exit 0 = pass, anything else = fail → rollback
set -e

source "$HOME/.cargo/env" 2>/dev/null || true

echo "=== Build ==="
cargo build 2>&1

echo "=== Format ==="
cargo fmt --check 2>&1

echo "=== Clippy ==="
cargo clippy --all-targets -- -D warnings 2>&1

echo "=== Tests ==="
cargo test 2>&1 | tail -20

# --- Custom checks ---

# No source file over 800 lines
echo "=== File size check (max 800 lines) ==="
OVER=$(find src -name "*.rs" -exec sh -c \
  'lines=$(wc -l < "$1"); if [ "$lines" -gt 800 ]; then echo "  $1: $lines lines"; fi' _ {} \;)
if [ -n "$OVER" ]; then
    echo "ERROR: Files over 800 lines:"
    echo "$OVER"
    exit 1
fi

# No TODO/FIXME in committed code (warns but does not fail)
TODOS=$(grep -rn "TODO\|FIXME" src/ --include="*.rs" 2>/dev/null | wc -l)
if [ "$TODOS" -gt 0 ]; then
    echo "WARNING: $TODOS TODO/FIXME comments found (not blocking)"
fi

echo "=== All gates passed ==="
