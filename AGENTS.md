# AGENTS.md

AI エージェントおよびメンテナー向けの内部運用ガイド。公開リポジトリには含まれない。

## リポジトリ

- `dev` (`r-hsnin/vz-dev`, private) — 日常開発。ローカル `main` は `dev/main` を追跡する
- `origin` (`r-hsnin/vz`, public) — リリース公開専用

push 先は `dev`。`origin/main` への直接 push は pre-push hook が阻止する。

## リリース凍結中

当面 `origin` へのリリースは行わない。`./scripts/release.sh` は実行しない。日常作業は `dev` への push のみで進める。

解除後の手順は `internal/RUNBOOK.md` を参照。公開範囲は `release-manifest.txt` が単一の真実で、記載の無いファイルは非公開が既定。境界の設計と検査は `internal/PUBLICATION.md`。manifest の変更は公開範囲の変更として扱い、無断で変更しない。

## 作業フロー

- 作業前に `docs/GOTCHAS.md` を読む。既知の罠を踏まない
- TDD で進める。unit テスト（モジュール内 `#[cfg(test)]` / `tests.rs`）と integration テスト（`tests/`）の両方を追加・更新する
- テスト置き場: unit は対象モジュール直下（大規模は同名 `*_tests.rs` に分離、`tests.rs` 集約は新規不可）。integration は機能別ターゲット（`tests/{oneshot,flags,inputs,output,directory,diff,modes}.rs`）＋共有は `tests/common`・`src/test_helpers` のみ。新規ヘルパーの複写は不可
- 変更には必ずドキュメントを連動させる（下の文書表の「読む・更新する条件」に従う）
- 失敗・躓いた点は `docs/GOTCHAS.md` に記す。ただし再現し得るものに限り、単発で今後確実に再現しないものは書かない

## 検証（commit 前に通す）

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings    # 警告ゼロ
cargo test
```

lefthook 導入済みなら pre-commit（fmt --check / clippy）と pre-push（origin ガード / test）が自動で走る。手動実行は `lefthook run pre-commit`。

依存を変更したら `cargo machete`（要 `cargo install cargo-machete`）で未使用依存を確認する。

ツールチェーンは `rust-toolchain.toml`（1.97.0）で固定。MSRV 1.88 は CI の msrv ジョブで検証する。

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
bun add -g lefthook    # または npm i -g lefthook
lefthook install       # 既存 clone に core.hooksPath が残っていれば --reset-hooks-path を付ける
```

## ドキュメント

各文書を内容の単一の真実とし、他文書へは参照を置く。重複を見つけたら所有者へ寄せる。

`docs/` と `skills/` は公開専用。リリース手順・公開範囲・ロードマップ等の内部文書は `internal/` に置く（境界規則は `internal/PUBLICATION.md`）。

| 文書 | 役割（単一の真実） | 読む・更新する条件 |
|---|---|---|
| `README.md` | 利用者向けの機能・CLI・チャート選択ルール | CLI や挙動を変えた時 |
| `docs/ARCHITECTURE.md` | 構造: モジュール・データフロー・変更影響 | 構造を変えた時 |
| `docs/DESIGN.md` | 設計意図: 理念・推論ルール・スコープ・設計判断 | 設計判断を変えた時 |
| `docs/GOTCHAS.md` | 非自明な挙動と既知の不具合 | 新たな罠を発見・解消した時 |
| `CONTRIBUTING.md` | 開発手順: setup・コマンド・テスト・ベンチ・PR | 開発参加・テスト追加時 |
| `skills/vz/SKILL.md` | エージェントから vz を使う手順 | スキルの提供内容を変えた時 |
| `internal/RUNBOOK.md` | リリース手順と復旧 | リリース・障害対応時 |
| `internal/PUBLICATION.md` | 公開/非公開の境界と検査 | 公開範囲を変えた時 |
| `internal/ROADMAP.md` | 内部ロードマップ・負債・決定ステータス | 計画や負債を更新した時 |
