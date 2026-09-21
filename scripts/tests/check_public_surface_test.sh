#!/usr/bin/env bash
# check-public-surface.sh の自己テスト。実 repo は汚さず一時 git repo で検証する。
set -uo pipefail

TESTS_DIR="$(cd "$(dirname "$0")" && pwd)"
CHECK="$TESTS_DIR/../check-public-surface.sh"

pass=0
fail=0
LAST_OUT=""

ok() { pass=$((pass + 1)); printf 'ok   - %s\n' "$1"; }
ng() {
  fail=$((fail + 1))
  printf 'FAIL - %s\n' "$1"
  printf '%s\n' "$LAST_OUT" | sed 's/^/       /'
}

new_repo() {
  local d
  d="$(mktemp -d)"
  git -C "$d" init -q
  git -C "$d" config user.email test@example.com
  git -C "$d" config user.name test
  printf '%s\n' "$d"
}

seed() {
  local d="$1"
  mkdir -p "$d/pub" "$d/priv"
  cat >"$d/release-manifest.txt" <<'EOF'
# test manifest
pub/
README.md
EOF
  printf '# Readme\n' >"$d/README.md"
  printf '# Pub\n\n[readme](../README.md)\n' >"$d/pub/a.md"
  printf 'private notes\n' >"$d/priv/b.md"
  git -C "$d" add -A
}

run_check() {
  local d="$1" want="$2" name="$3"
  LAST_OUT="$("$CHECK" --root "$d" 2>&1)"
  local code=$?
  if [[ "$code" -eq "$want" ]]; then
    ok "$name"
  else
    ng "$name (exit $code, want $want)"
  fi
}

expect_contains() {
  local needle="$1" name="$2"
  if printf '%s\n' "$LAST_OUT" | grep -qF -- "$needle"; then
    ok "$name"
  else
    ng "$name (missing: $needle)"
  fi
}

# --- T1: クリーンな公開集合は成功 ---
d="$(new_repo)"
seed "$d"
run_check "$d" 0 "T1 clean surface passes"

# --- T2: 公開ファイルの禁止トークンを検知 ---
d="$(new_repo)"
seed "$d"
printf '\nsee release-manifest.txt\n' >>"$d/pub/a.md"
git -C "$d" add -A
run_check "$d" 1 "T2 forbidden token fails"
expect_contains "pub/a.md" "T2 reports the offending file"

# --- T3: 公開docの相対リンクが公開集合外を指すと失敗 ---
d="$(new_repo)"
seed "$d"
printf 'secret\n' >"$d/secret.md"
printf '# Pub\n\n[s](../secret.md)\n' >"$d/pub/a.md"
git -C "$d" add -A
run_check "$d" 1 "T3 link outside public set fails"
expect_contains "pub/a.md" "T3 reports the offending link"

# --- T4: 実ファイルに一致しない manifest パターンを drift として検知 ---
d="$(new_repo)"
seed "$d"
printf 'nope/\n' >>"$d/release-manifest.txt"
git -C "$d" add -A
run_check "$d" 1 "T4 dead manifest pattern fails"
expect_contains "nope/" "T4 reports the dead pattern"

# --- T5: 非公開ファイルの内容は検査対象外 ---
d="$(new_repo)"
seed "$d"
printf 'lefthook guard-origin\n' >>"$d/priv/b.md"
git -C "$d" add -A
run_check "$d" 0 "T5 non-public files are not scanned"

# --- T6: --tag は作業ツリーではなく tag の tree を検査する ---
d="$(new_repo)"
seed "$d"
git -C "$d" commit -qm init
git -C "$d" tag v0
printf '\nsee release-manifest.txt\n' >>"$d/pub/a.md"
git -C "$d" add -A
LAST_OUT="$("$CHECK" --root "$d" --tag v0 2>&1)"
code=$?
if [[ "$code" -eq 0 ]]; then
  ok "T6 clean tag passes despite dirty worktree"
else
  ng "T6 clean tag passes despite dirty worktree (exit $code, want 0)"
fi
run_check "$d" 1 "T6 dirty worktree fails without --tag"

# --- T7: 明示列挙した .github ファイルのみ公開 / site/ の相対リンクは公開集合内で解決 ---
d="$(new_repo)"
seed "$d"
mkdir -p "$d/.github/workflows" "$d/site/guide"
cat >>"$d/release-manifest.txt" <<'EOF'
site/
.github/workflows/site.yml
EOF
printf 'name: site\n' >"$d/.github/workflows/site.yml"
printf 'name: internal\n# lefthook guard-origin\n' >"$d/.github/workflows/internal.yml"
printf '# Site\n\n[guide](./guide/a.md)\n' >"$d/site/index.md"
printf '# A\n\n[home](../index.md)\n' >"$d/site/guide/a.md"
git -C "$d" add -A
run_check "$d" 0 "T7 site links resolve and unlisted .github files stay private"

printf '\n%d passed, %d failed\n' "$pass" "$fail"
[[ "$fail" -eq 0 ]]
