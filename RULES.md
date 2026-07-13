# Rules

## Gate
bash gate.sh

## Scope

vz (CLI BI tool, Rust) の継続的品質改善。
13K LOC / 42ファイル / 580テスト / 3モード (oneshot, explore, present)。

改善対象:
- コード品質: 大きいファイル分割、unwrap削減、重複排除
- UX改善: エラーメッセージ、出力の見た目、ヘルプ文
- テスト: エッジケース追加、カバレッジ向上
- 設計: DESIGN.md との整合、モジュール責務の明確化

各サイクル (Phase 3 通過後) に `git add` + `git commit` する。
コミットメッセージ: `<type>: <日本語の説明>` (feat/fix/refactor/test/chore)
`git add -A` 禁止。変更ファイルを明示的に指定する。

やらないこと:
- 新機能追加 (既存機能の改善のみ)
- 依存ライブラリの追加
- ratatui のメジャーバージョンアップ
- パフォーマンス最適化 (ボトルネックが証明されない限り)

## Perspectives

Phase 1 の3並列サブエージェントは以下の観点で分析する:

### Agent A: UX / Product Value
- ユーザー視点での使いにくさ、分かりにくさ
- エラーメッセージの品質
- 出力フォーマットの見やすさ
- ドキュメントとの一貫性

### Agent B: Code Quality
- 50行超の関数
- unwrap() (ユーザー入力に起因するもの)
- 重複コードパターン
- テストカバレッジの穴

### Agent C: Design / Extensibility
- DESIGN.md と実装の乖離
- モジュール境界の適切さ
- 型の設計、エラー型の一貫性
- 将来の拡張を阻害するハードコード

## Stop

以下のいずれかで停止:
- 3サイクル連続で、3エージェントすべてが「重大な改善点なし」と判定
- テスト数 600 以上に到達
- PROGRESS.md の Failures が 5件を超えた（行き詰まりと判断）
