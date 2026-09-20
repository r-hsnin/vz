#!/usr/bin/env bash
# check-public-surface.sh — 公開予定ファイルだけを検査する。
#
# release-manifest.txt と git tree から公開集合を算出し、その集合に属するファイルに
# 対してのみ次を検査する:
#   1. 禁止トークン（内部専用パス・リポジトリ名・運用語）が無いこと
#   2. Markdown の相対リンクが公開集合内で解決すること
#   3. manifest の全パターンが実ファイルに 1 件以上一致すること（drift 検知）
#
# 非公開ファイルが内部物を参照するのは正常であり、対象外とする。
#
# 使い方: ./scripts/check-public-surface.sh [--root <dir>] [--manifest <path>] [--tag <ref>]
#   既定は作業ツリー（git ls-files）。--tag 指定時はその tag の tree を検査する。
#   違反があれば違反内容を stderr に列挙し非ゼロ終了する。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MANIFEST=""
TAG=""

usage() {
  echo "Usage: $0 [--root <dir>] [--manifest <path>] [--tag <ref>]"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
  --root)
    ROOT="$2"
    shift 2
    ;;
  --manifest)
    MANIFEST="$2"
    shift 2
    ;;
  --tag)
    TAG="$2"
    shift 2
    ;;
  -h | --help)
    usage
    exit 0
    ;;
  *)
    echo "ERROR: unknown argument: $1" >&2
    usage >&2
    exit 2
    ;;
  esac
done

[[ -n "$MANIFEST" ]] || MANIFEST="$ROOT/release-manifest.txt"

if [[ ! -f "$MANIFEST" ]]; then
  echo "ERROR: manifest not found: $MANIFEST" >&2
  exit 2
fi

# 公開ファイルの内容を読む（tag 指定時はその tree から）
if [[ -n "$TAG" ]]; then
  read_content() { git -C "$ROOT" show "$TAG:$1"; }
  list_files() { git -C "$ROOT" ls-tree -r --name-only "$TAG"; }
else
  read_content() { cat "$ROOT/$1" 2>/dev/null || true; }
  list_files() { git -C "$ROOT" ls-files; }
fi

# --- manifest 読み込み ---
PATTERNS=()
while IFS= read -r line; do
  [[ "$line" =~ ^#.*$ ]] && continue
  [[ -z "$line" ]] && continue
  PATTERNS+=("$line")
done <"$MANIFEST"

if [[ ${#PATTERNS[@]} -eq 0 ]]; then
  echo "ERROR: no manifest patterns in $MANIFEST" >&2
  exit 2
fi

in_allowed() {
  local file="$1" p
  for p in "${PATTERNS[@]}"; do
    if [[ "$p" == */ ]]; then
      [[ "$file" == "$p"* ]] && return 0
    else
      [[ "$file" == "$p" ]] && return 0
    fi
  done
  return 1
}

# --- ファイル一覧と公開集合 ---
FILES=()
while IFS= read -r line; do
  [[ -n "$line" ]] && FILES+=("$line")
done < <(list_files)

ALLOWED=()
for f in "${FILES[@]}"; do
  in_allowed "$f" && ALLOWED+=("$f")
done

violations=0
report() { echo "$1" >&2; }

# --- 検査1: 禁止トークン ---
FORBIDDEN=(
  'AGENTS'
  'release-manifest'
  'vz-dev'
  'dev/main'
  'guard-origin'
  'lefthook'
  'scripts/hooks'
  'scripts/release\.sh'
  'RUNBOOK'
  'PLAN\.md'
  'internal/'
  'origin/main'
  'origin remote'
)
TOKEN_RE="$(IFS='|'; echo "${FORBIDDEN[*]}")"

for f in "${ALLOWED[@]}"; do
  while IFS= read -r hit; do
    [[ -z "$hit" ]] && continue
    report "LEAK $f:${hit%%:*}: forbidden token"
    violations=$((violations + 1))
  done < <(read_content "$f" | grep -nI -E "$TOKEN_RE" || true)
done

# --- 検査2: Markdown 相対リンクの公開集合内解決 ---
for f in "${ALLOWED[@]}"; do
  [[ "$f" == *.md ]] || continue
  dir="$(dirname "$f")"
  while IFS= read -r raw; do
    [[ -z "$raw" ]] && continue
    target="${raw#](}"
    target="${target%)}"
    target="${target%% *}"
    target="${target#<}"
    target="${target%>}"
    [[ -z "$target" ]] && continue
    case "$target" in
    http://* | https://* | mailto:* | tel:* | '#') continue ;;
    esac
    case "$target" in
    *'#'*) target="${target%%#*}" ;;
    esac
    [[ -z "$target" ]] && continue

    dir_link=false
    [[ "$target" == */ ]] && dir_link=true

    case "$target" in
    /*) candidate="$ROOT$target" ;;
    *) candidate="$ROOT/$dir/$target" ;;
    esac
    abs="$(realpath -m "$candidate")"

    if [[ "$abs" != "$ROOT" && "$abs" != "$ROOT"/* ]]; then
      report "LINK $f: '$target' resolves outside the public surface"
      violations=$((violations + 1))
      continue
    fi
    rel="${abs#"$ROOT"/}"
    [[ "$abs" == "$ROOT" ]] && rel="."

    if $dir_link; then
      found=false
      for a in "${ALLOWED[@]}"; do
        [[ "$a" == "$rel"/* ]] && {
          found=true
          break
        }
      done
      if ! $found; then
        report "LINK $f: '$target' is not in the public surface"
        violations=$((violations + 1))
      fi
    elif ! in_allowed "$rel"; then
      report "LINK $f: '$target' is not in the public surface"
      violations=$((violations + 1))
    fi
  done < <(read_content "$f" | grep -oI -E '\]\([^)]*\)' || true)
done

# --- 検査3: manifest パターンの drift ---
for p in "${PATTERNS[@]}"; do
  matched=false
  for f in "${FILES[@]}"; do
    if [[ "$p" == */ ]]; then
      [[ "$f" == "$p"* ]] && {
        matched=true
        break
      }
    else
      [[ "$f" == "$p" ]] && {
        matched=true
        break
      }
    fi
  done
  if ! $matched; then
    report "DRIFT manifest pattern '$p' matches no files"
    violations=$((violations + 1))
  fi
done

if [[ "$violations" -gt 0 ]]; then
  echo "Public surface check FAILED: $violations violation(s)." >&2
  exit 1
fi

echo "Public surface check OK (${#ALLOWED[@]} files inspected)."
