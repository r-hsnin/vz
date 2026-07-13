#!/usr/bin/env bash
set -euo pipefail

# release.sh — manifest ベースの公開リリーススクリプト
# 使い方: ./scripts/release.sh v0.2.0 [--dry-run]
#
# 動作:
#   1. 事前チェック (tag, manifest, remote, gh auth, branch既存)
#   2. release-manifest.txt で公開対象をフィルタ
#   3. origin に release/vX.Y.Z ブランチを push
#   4. gh pr create で PR を作成

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MANIFEST="$REPO_ROOT/release-manifest.txt"
ORIGIN_REMOTE="origin"
DEV_REMOTE="dev"

# --- 引数パース ---
VERSION=""
DRY_RUN=false

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=true ;;
    v*) VERSION="$arg" ;;
    *) echo "ERROR: Unknown argument: $arg"; exit 1 ;;
  esac
done

if [[ -z "$VERSION" ]]; then
  echo "Usage: $0 <version-tag> [--dry-run]"
  echo "Example: $0 v0.2.0 --dry-run"
  exit 1
fi

RELEASE_BRANCH="release/$VERSION"

# --- 事前チェック ---
echo "=== Release: $VERSION ==="
echo ""

# tag 存在確認
if ! git rev-parse "$VERSION" >/dev/null 2>&1; then
  echo "ERROR: Tag '$VERSION' not found."
  echo "  Run: git tag $VERSION"
  exit 1
fi

# manifest 存在確認
if [[ ! -f "$MANIFEST" ]]; then
  echo "ERROR: $MANIFEST not found."
  exit 1
fi

# origin リモート確認
if ! git remote get-url "$ORIGIN_REMOTE" >/dev/null 2>&1; then
  echo "ERROR: Remote '$ORIGIN_REMOTE' not found."
  exit 1
fi

# gh CLI 認証確認
if ! gh auth status >/dev/null 2>&1; then
  echo "ERROR: gh CLI is not authenticated."
  echo "  Run: gh auth login"
  exit 1
fi

# リモートブランチ既存チェック (べき等性)
if git ls-remote --heads "$ORIGIN_REMOTE" "$RELEASE_BRANCH" 2>/dev/null | grep -q .; then
  echo "ERROR: $RELEASE_BRANCH already exists on $ORIGIN_REMOTE."
  echo "  Delete it first: git push $ORIGIN_REMOTE --delete $RELEASE_BRANCH"
  echo "  Or close the existing PR and try again."
  exit 1
fi

# tag ancestry 検証 (dev/main に含まれるか)
if git rev-parse "$DEV_REMOTE/main" >/dev/null 2>&1; then
  if ! git merge-base --is-ancestor "$VERSION" "$DEV_REMOTE/main"; then
    echo "WARNING: Tag $VERSION is NOT on $DEV_REMOTE/main."
    echo "  This may release code from an unreviewed branch."
    if ! $DRY_RUN; then
      read -rp "  Continue anyway? [y/N] " confirm_ancestry
      if [[ "$confirm_ancestry" != "y" && "$confirm_ancestry" != "Y" ]]; then
        echo "Aborted."
        exit 0
      fi
    fi
  fi
fi

# Cargo.toml version と tag の整合チェック
CARGO_VERSION=$(git show "$VERSION:Cargo.toml" 2>/dev/null | grep '^version' | head -1 | sed 's/.*"\(.*\)"/\1/')
TAG_VERSION="${VERSION#v}"
if [[ -n "$CARGO_VERSION" && "$CARGO_VERSION" != "$TAG_VERSION" ]]; then
  echo "WARNING: Cargo.toml version ($CARGO_VERSION) does not match tag ($TAG_VERSION)."
  echo "  Consider updating Cargo.toml before tagging."
  if ! $DRY_RUN; then
    read -rp "  Continue anyway? [y/N] " confirm_version
    if [[ "$confirm_version" != "y" && "$confirm_version" != "Y" ]]; then
      echo "Aborted."
      exit 0
    fi
  fi
fi

# --- manifest 読み込み ---
allowed_patterns=()
while IFS= read -r line; do
  # コメントと空行をスキップ
  [[ "$line" =~ ^#.*$ ]] && continue
  [[ -z "$line" ]] && continue
  allowed_patterns+=("$line")
done < "$MANIFEST"

# 空 manifest チェック
if [[ ${#allowed_patterns[@]} -eq 0 ]]; then
  echo "ERROR: No allowed patterns in $MANIFEST."
  echo "  The manifest must contain at least one path entry."
  exit 1
fi

echo "Manifest entries: ${#allowed_patterns[@]}"

# --- tag のファイル一覧を取得 ---
all_files_list=()
while IFS= read -r f; do
  all_files_list+=("$f")
done < <(git ls-tree -r --name-only "$VERSION")

# manifest パターンの有効性チェック (0 ファイルにマッチするパターンを警告)
for pattern in "${allowed_patterns[@]}"; do
  match_count=0
  for f in "${all_files_list[@]}"; do
    if [[ "$pattern" == */ ]]; then
      [[ "$f" == "${pattern}"* ]] && ((match_count++)) && break
    else
      [[ "$f" == "$pattern" ]] && ((match_count++)) && break
    fi
  done
  if [[ $match_count -eq 0 ]]; then
    echo "WARNING: Manifest pattern '$pattern' matches no files in $VERSION"
  fi
done

# --- manifest でフィルタ ---
allowed_files=()
excluded_files=()

for file in "${all_files_list[@]}"; do
  matched=false
  for pattern in "${allowed_patterns[@]}"; do
    if [[ "$pattern" == */ ]]; then
      if [[ "$file" == "${pattern}"* ]]; then
        matched=true
        break
      fi
    else
      if [[ "$file" == "$pattern" ]]; then
        matched=true
        break
      fi
    fi
  done

  if $matched; then
    allowed_files+=("$file")
  else
    excluded_files+=("$file")
  fi
done

echo "Allowed files: ${#allowed_files[@]}"
echo "Excluded files: ${#excluded_files[@]}"
echo ""

if [[ ${#excluded_files[@]} -gt 0 ]]; then
  echo "--- Excluded (will NOT be published) ---"
  for f in "${excluded_files[@]}"; do
    echo "  ✗ $f"
  done
  echo ""
fi

echo "--- Allowed (will be published) ---"
for f in "${allowed_files[@]}"; do
  echo "  ✓ $f"
done
echo ""

# --- dry-run ならここで終了 ---
if $DRY_RUN; then
  echo "[DRY-RUN] No changes made. Remove --dry-run to execute."
  exit 0
fi

# --- 確認プロンプト ---
read -rp "Proceed with release $VERSION? [y/N] " confirm
if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
  echo "Aborted."
  exit 0
fi

# --- release ブランチ作成 (manifest ファイルのみ) ---
echo ""
echo "=== Creating release branch: $RELEASE_BRANCH ==="

# 一時ワークツリーで作業
WORK_DIR=$(mktemp -d)

cleanup() {
  cd "$REPO_ROOT" 2>/dev/null || true
  git worktree remove --force "$WORK_DIR" 2>/dev/null || true
  rm -rf "$WORK_DIR" 2>/dev/null || true
}
trap cleanup EXIT

# origin/main をベースに release ブランチを作成
git fetch "$ORIGIN_REMOTE" main
git worktree add "$WORK_DIR" "$ORIGIN_REMOTE/main" --detach 2>/dev/null

cd "$WORK_DIR"
git checkout -b "$RELEASE_BRANCH"

# tag から許可ファイルを上書き
for file in "${allowed_files[@]}"; do
  mkdir -p "$(dirname "$file")"
  git -C "$REPO_ROOT" show "$VERSION:$file" > "$file"
done

# 不要ファイル削除（origin/main にあるが manifest に無いもの）
while IFS= read -r file; do
  found=false
  for af in "${allowed_files[@]}"; do
    if [[ "$file" == "$af" ]]; then
      found=true
      break
    fi
  done
  if ! $found; then
    git rm -f "$file" >/dev/null 2>&1 || true
  fi
done < <(git ls-files)

# コミット
git add -- "${allowed_files[@]}"
if git diff --cached --quiet; then
  echo "No changes to release."
  cd "$REPO_ROOT"
  exit 0
fi

# コミットログ生成 (前回タグからの dev コミット)
PREV_TAG=$(git -C "$REPO_ROOT" describe --tags --abbrev=0 "$VERSION^" 2>/dev/null || echo "")
if [[ -n "$PREV_TAG" ]]; then
  CHANGELOG=$(git -C "$REPO_ROOT" log --oneline "$PREV_TAG..$VERSION" --no-merges)
else
  CHANGELOG=$(git -C "$REPO_ROOT" log --oneline "$VERSION" --no-merges -20)
fi

git commit -m "release: $VERSION" >/dev/null

# --- origin に push ---
echo "=== Pushing $RELEASE_BRANCH to $ORIGIN_REMOTE ==="
git push "$ORIGIN_REMOTE" "$RELEASE_BRANCH" -u

# --- PR 作成 ---
echo ""
echo "=== Creating PR ==="

PR_TITLE="release: $VERSION"
PR_BODY="## Release $VERSION

### Changes
$CHANGELOG"

cd "$REPO_ROOT"

gh pr create \
  --repo "$(git remote get-url "$ORIGIN_REMOTE" | sed 's/.*github.com[:/]//' | sed 's/\.git$//')" \
  --base main \
  --head "$RELEASE_BRANCH" \
  --title "$PR_TITLE" \
  --body "$PR_BODY"

echo ""
echo "=== Done! ==="
echo "Next steps:"
echo "  1. Review the PR on GitHub"
echo "  2. Merge the PR"
echo "  3. Push the tag to origin: git push $ORIGIN_REMOTE $VERSION"
