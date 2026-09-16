#!/usr/bin/env bash
# lefthook pre-push — origin への直接 push を阻止
# release ブランチ (release/*) と tag のみ許可、それ以外は拒否
#
# 呼び出し: lefthook.yml の pre-push（use_stdin: true）
# $1: remote 名 / stdin: push 対象の refs

remote="$1"

# origin 以外は素通り
if [[ "$remote" != "origin" ]]; then
  exit 0
fi

# push されるブランチを確認
# shellcheck disable=SC2034
while read -r local_ref local_oid remote_ref remote_oid; do
  # tag push は常に許可
  if [[ "$remote_ref" == refs/tags/* ]]; then
    continue
  fi

  branch="${remote_ref#refs/heads/}"

  # release/* ブランチは許可（スクリプト経由）
  if [[ "$branch" == release/* ]]; then
    continue
  fi

  # それ以外は拒否
  echo ""
  echo "⛔ BLOCKED: Direct push to origin/$branch is not allowed."
  echo ""
  echo "   origin (public) へは scripts/release.sh 経由で PR を作成してください。"
  echo ""
  echo "   Usage: ./scripts/release.sh <version-tag> [--dry-run]"
  echo ""
  echo "   バイパス (非推奨): git push --no-verify"
  echo ""
  exit 1
done

exit 0
