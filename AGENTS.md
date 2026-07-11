# AGENTS.md — vz エージェント向けガイド

## プロダクトビジョン

「ターミナルで `vz data.csv` と打てば、即座に意味のあるチャートが出る」
— ゼロコンフィグで最大の価値を提供する CLI BI ツール。

ユーザーは: データアナリスト、バックエンドエンジニア、CLI愛好家。
競合優位は: 速さ、設定不要、ターミナル完結。

## 環境

```bash
source "$HOME/.cargo/env"
cd /home/user/vz
```

## 品質ゲート（全変更に適用）

```bash
cargo fmt --check                            # フォーマット
cargo clippy --all-targets -- -D warnings    # 警告ゼロ
cargo test                                   # 全テスト pass (244 unit + 49 integration + 4 snapshot)
```

## ファイルマップ

| パス | 役割 | 変更時の影響範囲 |
|------|------|------------------|
| `src/main.rs` | エントリ、CLI dispatch | 全モード |
| `src/cli/mod.rs` | clap4 引数定義 | CLI UX |
| `src/loader/mod.rs` | CSV/TSV/JSON/NDJSON ローダー | **全モード** |
| `src/infer/` | 型推論エンジン | chart selection + 全モード |
| `src/chart/selector.rs` | 型→チャート種別マッピング | 全モード |
| `src/chart/data_builder.rs` | Schema+rows → 描画用データ構造 | **全モード**（共有層） |
| `src/render/` | ratatui ウィジェット群 | 対応チャート種別のみ |
| `src/render/nice_numbers.rs` | 軸ティック計算 | 軸表示のあるチャート |
| `src/oneshot/mod.rs` | ワンショット描画（Buffer→ANSI） | oneshot のみ |
| `src/explore/mod.rs` | インタラクティブTUI | explore のみ |
| `src/present/mod.rs` | スライドプレゼン | present のみ |
| `tests/integration_test.rs` | E2E テスト | — |
| `fixtures/sales.csv` | テスト用データ | — |
| `fixtures/demo.md` | テスト用スライド | — |

## コーディング規約

- `cargo fmt` デフォルト設定
- `cargo clippy` 警告ゼロ
- 関数50行以内、ファイル800行以内
- ミューテーション最小化（新しい値を作る）
- エラーを黙って握りつぶさない
- パブリックAPIには doc comment 必須

## テスト方針

- ユニットテスト: 各ソースファイル内 `#[cfg(test)]` モジュール
- 統合テスト: `tests/integration_test.rs`（バイナリの E2E テスト）
- スナップショットテスト: `tests/snapshots/`
- 新機能にはテスト必須（TDD: RED → GREEN → REFACTOR）
- `pretty_assertions` でdiff表示、`tempfile` で一時ファイル

## 改善ループ

**詳細な手順は IMPROVE_LOOP.md を参照。**

要約: 評価（3並列サブエージェント）→ 選定（RICEスコア）→ TDD実装 → 検証（サブエージェント）→ 記録 → 繰り返し

品質改善だけでなく、プロダクト価値を高める機能追加も対象。

## 次に実装すべき機能候補（優先度順の目安）

| 機能 | 理由 | 難易度 |
|------|------|--------|
| Explore にデータテーブルビュー | TUIの実用性を大きく向上 | 中 |
| `--output svg` エクスポート | 共有・ドキュメント埋め込みの価値 | 高 |
| カスタムカラーテーマ | 視認性・アクセシビリティ | 中 |
| stdin パイプの改善 | パイプラインとの統合価値 | 低 |
| 大規模データのサンプリング | 1GB超対応 | 中 |
| Parquet 入力 | データエンジニア向け価値 | 高 |

これらはあくまで候補。選定は IMPROVE_LOOP.md の RICE スコアで行う。
