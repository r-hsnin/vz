# AGENTS.md

AI エージェントおよびメンテナー向けの内部運用ガイド。公開リポジトリには含まれない。

## リポジトリ

- `dev` (`r-hsnin/vz-dev`, private) — 日常開発。ローカル `main` は `dev/main` を追跡する
- `origin` (`r-hsnin/vz`, public) — リリース公開専用

push 先は `dev`。`origin/main` への直接 push は pre-push hook が阻止する。

## リリース凍結中

当面 `origin` へのリリースは行わない。`./scripts/release.sh` は実行しない。日常作業は `dev` への push のみで進める。

解除後の手順は `RUNBOOK.md` を参照。公開範囲は `release-manifest.txt` が単一の真実で、記載の無いファイルは非公開が既定。manifest の変更は公開範囲の変更として扱い、無断で変更しない。

## 検証（commit 前に通す）

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings    # 警告ゼロ
cargo test
```

## コミット

- 形式: `<type>: <日本語の説明>`。type は feat/fix/refactor/docs/test/chore/perf/ci
- 変更ファイルを明示して stage する
- Cargo.toml の version はリリース時のみ変更する

## ガードレール

- `origin` への push はしない（hook が阻止する）
- `--no-verify` で hook をバイパスしない（緊急時のみ）
- 新規依存は追加前に確認を取る（バイナリサイズに影響）

## Hook インストール

```bash
git config core.hooksPath scripts/hooks
```

## ドキュメント

各文書を内容の単一の真実とし、他文書へは参照を置く。重複を見つけたら所有者へ寄せる。

| 文書 | 役割（単一の真実） | 読む・更新する条件 |
|---|---|---|
| `README.md` | 利用者向けの機能・CLI・チャート選択ルール | CLI や挙動を変えた時 |
| `DESIGN.md` | 設計: モジュール構造・データフロー・設計判断 | 構造を変えた時 |
| `CONTRIBUTING.md` | 開発手順: setup・コマンド・テスト・ベンチ・PR | 開発参加・テスト追加時 |
| `RUNBOOK.md` | リリース手順とトラブルシュート | リリース・障害対応時 |
| `skill/SKILL.md` | エージェントから vz を使う手順 | スキルの提供内容を変えた時 |
