# TODO

## 機能追加

- `--output svg` — チャートを SVG としてエクスポート
- `--watch` — ファイル変更を監視して自動再描画
- `vz diff a.csv b.csv` — 2ファイルの差分を可視化
- Explore 内フィルタ — TUI 上で `--where` 相当の絞り込み
- カスタムカラーテーマ — ターミナル背景に応じた配色切替
- Parquet 入力 — arrow/parquet crate で対応
- scatter ドット密度 — 大量データのオーバーラップ表現

## コード品質

- `oneshot/mod.rs` (36KB) — チャート描画ロジックを分離
- `present/mod.rs` (33KB) — レンダリング部分を切り出し
- `explore/mod.rs` (25KB) — イベントハンドリングと描画を分離
- stdin が空のときのエラーメッセージ改善

## 足回り

- CI に `cargo test` + clippy を追加
- `cargo-dist` または `release-please` でバイナリ配布
- 大規模データ（100万行）のベンチマーク
