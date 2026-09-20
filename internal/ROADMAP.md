# 内部ロードマップ・負債・決定ステータス

公開ドキュメントから意図的に外した内部メタ（ロードマップ、決定の進行状況、残負債、
凍結メモ）の受け皿。公開面の境界規則は [PUBLICATION.md](PUBLICATION.md)、所有権表は
`AGENTS.md`、リリース手順は [RUNBOOK.md](RUNBOOK.md) が持つ。

情報を消すのではなく、利用者向けの設計意図と混ざらないよう内部へ移している。公開doc側の
設計判断（why・却下代替・意図的乖離）と対で読む。

## リポジトリ分割（L3）

現状は単一 crate。到達点は **2 crate、N には分割しない**: `vz-core`（library）+ `vz`（binary）。

| Plane material | Target crate |
|---|---|
| `loader/`, `infer/`, `filter.rs`, `util.rs`, `sparkline.rs`, `theme.rs`, pure `insights`/`info` | `vz-core` |
| `chart/` (selector + recommend + **canonical data_builder**), `render/` (ratatui stays internal), `output/` (value/String returns), `diff/compute` + `diff/schema`, `present/parser` | `vz-core` |
| `cli/`, `app.rs`, `pipeline.rs`, `watch.rs`, `oneshot/`, `directory/`, `explore/`, `present/` rest, `diff/render/` | `vz` (bin) |

実施時は `release-manifest.txt` を `src/` から `crates/*/src` へ変更する必要がある
（公開範囲の変更として承認を要する。手順は [PUBLICATION.md](PUBLICATION.md) §8）。

## 決定ステータス（公開 DESIGN から移設）

公開 [docs/DESIGN.md](../docs/DESIGN.md) の Decision Records は rationale を保持し、
進行状況ラベルはここに集約する。

| 決定 | ステータス |
|---|---|
| D1 単一crate → 2 crate | Intent（現状は単一 crate） |
| D2 `Cli` を app plane 下へ漏らさない | Done |
| D3 Ratatui を唯一の描画エンジンに | Active |
| D4 `anyhow` 継続、型付きエラーは分割時 | Deferred to L3 |
| D5 `helpers/` 解体（再作成禁止） | Done |
| D6 外部データエンジン不使用・in-memory | Active |
| D7 狭い公開 API・破壊的変更許容 | Active |
| D8 `--bins` を `MAX_BINS` で境界 | Done |

## 残負債（公開 ARCHITECTURE から移設）

- **`helpers/` を再作成しない。** 解体済み。関数は `cli/resolve.rs` /
  `chart/recommend.rs` / `filter.rs` に所属する。
- **分割まで `thiserror` を入れない。** 全体 `anyhow`、型付きエラーは `vz-core`/`vz`
  分割時に導入する。
- **未使用 `schema` パラメータ。** `output/table.rs` / `output/markdown.rs` は
  `schema: &Schema` を `let _ = schema;` で捨てている（`chart_json.rs` は使用）。該当
  exporter を触る時に除去する。公開 ARCHITECTURE の `Cli` boundary 表からも参照が消えて
  いるのはこのため。
- **`file:line` 参照は脆い。** コード移動で drift するため、モジュール名/関数名を優先する。

## 凍結メモ

凍結の効力・解除条件は `AGENTS.md`「リリース凍結中」が単一の真実。このファイルには写さない。
