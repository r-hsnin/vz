# AGENTS.md

AI エージェントおよびメンテナー向けの内部運用ガイド。公開リポジトリには含まれない。

## リポジトリ

- `dev` (`r-hsnin/vz-dev`, private) — 日常開発。ローカル `main` は `dev/main` を追跡する
- `origin` (`r-hsnin/vz`, public) — リリース公開専用

push 先は `dev`。`origin/main` への直接 push は pre-push hook が阻止する。

## リリース凍結中

当面 `origin` へのリリースは行わない。`./scripts/release.sh` は実行しない。日常作業は `dev` への push のみで進める。

解除後の手順は `docs/RUNBOOK.md` を参照。公開範囲は `release-manifest.txt` が単一の真実で、記載の無いファイルは非公開が既定。manifest の変更は公開範囲の変更として扱い、無断で変更しない。

## 作業フロー

- 作業前に `docs/GOTCHAS.md` を読む。既知の罠を踏まない
- TDD で進める。unit テスト（モジュール内 `#[cfg(test)]` / `tests.rs`）と integration テスト（`tests/`）の両方を追加・更新する
- 変更には必ずドキュメントを連動させる（下の文書表の「読む・更新する条件」に従う）
- 失敗・躓いた点は `docs/GOTCHAS.md` に記す。ただし再現し得るものに限り、単発で今後確実に再現しないものは書かない

## 検証（commit 前に通す）

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings    # 警告ゼロ
cargo test
```

## コミット

- 論理単位ごとにコミットしながら作業を進める（複数の関心事を1コミットに混ぜない）
- 形式: `<type>: <日本語の説明>`。type は feat/fix/refactor/docs/test/chore/perf/ci
- 変更ファイルを明示して stage する
- Cargo.toml の version はリリース時のみ変更する
- push はユーザーの承認後にのみ行う

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
| `docs/ARCHITECTURE.md` | 構造: モジュール・データフロー・変更影響 | 構造を変えた時 |
| `docs/DESIGN.md` | 設計意図: 理念・推論ルール・スコープ・設計判断 | 設計判断を変えた時 |
| `docs/GOTCHAS.md` | 非自明な挙動と既知の不具合 | 新たな罠を発見・解消した時 |
| `docs/RUNBOOK.md` | リリース手順と復旧 | リリース・障害対応時 |
| `CONTRIBUTING.md` | 開発手順: setup・コマンド・テスト・ベンチ・PR | 開発参加・テスト追加時 |
| `skills/vz/SKILL.md` | エージェントから vz を使う手順 | スキルの提供内容を変えた時 |
