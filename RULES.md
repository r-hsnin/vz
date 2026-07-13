# Rules

## Gate
bash gate.sh

## Scope

vz (CLI BI tool, Rust) の機能強化。
既存機能の深化・表現力の向上に集中する。

改善対象:
- explore モード: 操作性向上、情報密度の改善、新しいインタラクション
- present モード: Markdown 表現力、スライド機能の充実
- oneshot モード: 出力の表現力向上、新しいビジュアル要素
- chart: scatter plot の実装完成（現在 stub）、チャート表現力の向上

各サイクル (Phase 3 通過後) に `git add` + `git commit` する。
コミットメッセージ: `<type>: <日本語の説明>` (feat/fix/refactor/test/chore)
`git add -A` 禁止。変更ファイルを明示的に指定する。

やらないこと:
- 依存ライブラリの追加
- ratatui のメジャーバージョンアップ
- データベース接続、ストリーミング等の Non-goals (DESIGN.md 参照)
- 大規模なアーキテクチャ変更

## Perspectives

Phase 1 の3並列サブエージェントは以下の観点で分析する:

### Agent A: Explore Mode Enhancement
- vim-style ナビゲーションの未実装バインド（ソート切替、フィルタ入力など）
- データテーブルビューの改善（列幅、スクロール、ハイライト）
- チャートとテーブルの連動（選択行のハイライトなど）
- ステータスバー情報の充実

### Agent B: Chart & Rendering Power
- scatter.rs が stub 状態 → 実装完成（Braille ドットプロット）
- 軸ラベルのフォーマット改善（大きい数値の略記、日付フォーマット）
- チャートタイトル・凡例の表現力
- Heatmap のカラーグラデーション改善

### Agent C: Present Mode & Output
- present モードのスライド要素（画像参照、テーブル、コードブロック対応）
- chart ブロックの新パラメータ（title, color, sort 等）
- SVG 出力の品質向上
- spark 出力の表現力（トレンド矢印、min/max 表示等）

## Stop

以下のいずれかで停止:
- 3サイクル連続で、3エージェントすべてが「実装すべき改善点なし」と判定
- PROGRESS.md の Failures が 5件を超えた（行き詰まりと判断）
