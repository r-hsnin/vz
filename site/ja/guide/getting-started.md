# はじめに

`vz` は、表をターミナルのチャートに変える CLI BI ツールです。CSV、TSV、JSON、
NDJSON ファイルを渡すと、各列の型を推定し、可視化を選び、一度だけ描画して終了
します。設定も対話セッションも不要です。

## インストール

Git リポジトリからインストールします。

```bash
cargo install --git https://github.com/r-hsnin/vz
```

ローカルのクローンからインストールする場合:

```bash
cargo install --path .
```

Rust 1.88 以降が必要です。

## 最初のチャート

```bash
# 自動可視化: 軸とチャート種別を推定
vz sales.csv

# 軸を指定
vz sales.csv -x month -y revenue

# チャート種別を指定
vz sales.csv -x city -y revenue -t bar
```

チャートは stdout に、要約行・インサイト・警告は stderr に出力されます。この
分離により、要約を失わずにチャートだけをパイプやリダイレクトへ渡せます。描画の
代わりにスキーマを確認するには `vz sales.csv --info` を実行します。JSON 形式に
ついては[出力モード](./output-modes.md)を参照してください。

## 標準入力からの読み込み

ファイル名に `-` を指定します。判定が曖昧な場合は `-f` で形式を強制できます。
たとえば 1 列だけの TSV をパイプする場合:

```bash
cat data.csv | vz -
kubectl top pods | vz - -f space
printf '1,2\n3,4\n' | vz -
```

ファイル引数なしで stdin が TTY の場合、`vz` はファイルかパイプ入力を求める
エラーを出します。ヘルプは表示しません。パイプされた stdin はそのまま読み込ま
れます。

## 入力形式

- **CSV** — カンマ区切り。
- **TSV** — タブ区切り。`.tsv`/`.tab` 拡張子または内容から判定します。
- **JSON** — オブジェクトの配列。
- **NDJSON** — 改行区切り JSON。拡張子または `{` の先頭文字から判定します。
- **固定幅・スペース整列** — `kubectl`、`df`、`ps` などの出力。
- **標準入力** — `-`。任意で `-f` を併用します。

先頭の UTF-8 BOM は除去されます。先頭行がすべて数値の場合はヘッダーなし
データとして扱われ、`col1`、`col2`、... という合成ヘッダーが付きます。
`--no-header` で明示的に指定することもできます。

## モード一覧

| 実行方法 | 内容 |
|---|---|
| `vz FILE` | ワンショットのチャート描画（既定） |
| `vz FILE1 FILE2` / `vz FILE --diff FILE2` | [差分モード](./diff-mode.md) |
| `vz DIRECTORY` | スキーマが一致するファイルを結合 |
| `vz explore FILE [FILE2]` | 対話 TUI（TTY が必要） |
| `vz present FILE` | チャートを埋め込んだ Markdown スライド（TTY が必要） |
| `vz FILE --watch` | ファイル変更時に再描画（stdin では使用不可） |
| `vz completions SHELL` | 補完スクリプトを出力。[シェル補完](./shell-completions.md)を参照 |

## CLI オプション

`FILE` は 0〜2 個のパスを取ります。2 つのパスは差分モード、ディレクトリは
ディレクトリモード、`-` は標準入力を意味します。**diff: ignored** と付いた
フラグは diff モードでは受け付けられますが効果はありません。

| フラグ | 値 / 既定値 | 説明 |
|------|-----------------|-------------|
| `FILE` | 0〜2 パス | 入力ファイル。`-` は stdin。2 ファイルで diff、ディレクトリでディレクトリモード |
| `--diff` | `FILE2` | diff モードの 2 つ目のファイル（`vz file1 file2` の代替） |
| `-x`, `--x-col` | 列 | X 軸の列。`col:Label` に対応 |
| `-y`, `--y-col` | 列[,列...] | Y 軸の列。カンマ区切りで複数系列。`col:Label` に対応 |
| `-t`, `--type` | `line`,`bar`,`scatter`,`histogram`,`heatmap` | チャート種別の上書き。**diff: 無視** |
| `-c`, `--color` | 列 | 色・グループ化の列。**diff: 警告付きで無視** |
| `-W`, `--width` | `u16`。ターミナル幅（最小 40、パイプ時は 80） | チャートの幅（列数） |
| `-H`, `--height` | `u16`。適応的、上限 24 | チャートの高さ（行数）。カテゴリや行が少ないと縮小 |
| `-I`, `--info` | フラグ | チャートを描かずに列メタデータ（型・統計）を表示。`-o json` で info JSON |
| `--no-header` | フラグ | 先頭行をデータとして扱う（先頭行が全数値なら自動判定） |
| `--sort` | `desc`,`asc`,`none`（既定 `none`） | bar の値の並び替え。無視するチャート種別では警告 |
| `-f`, `--format` | `csv`,`tsv`,`json`,`ndjson`,`space` | 入力形式の強制（既定は自動判定） |
| `-w`, `--where` | フィルタ（繰り返し可） | 行の絞り込み: `col=value`, `col!=value`, `col>value`, `col>=value`, `col<value`, `col<=value`。**diff: 警告付きで無視** |
| `--top` | `N ≥ 1` | Y の上位 N カテゴリを表示（`--sort desc` を含意）。`--tail` と排他 |
| `--tail` | `N ≥ 1` | Y の下位 N カテゴリを表示（`--sort asc` を含意） |
| `--agg` | `sum`（既定）,`mean`,`count`,`max`,`min` | bar の集計方法。**diff: 警告付きで無視** |
| `--title` | 文字列 | 自動生成タイトルの上書き（text/svg/html） |
| `-o`, `--output` | `text`（既定）,`json`,`table`,`spark`,`svg`,`markdown`,`html` | 出力形式 |
| `--json` | フラグ | `-o json` の省略形 |
| `--spark` | フラグ | `-o spark` の省略形 |
| `--svg` | フラグ | `-o svg` の省略形 |
| `--markdown` | フラグ | `-o markdown` の省略形 |
| `--html` | フラグ | `-o html` の省略形 |
| `--sample` | `N ≥ 1` | 系統的サンプリングで最大 N 行を読み込む。行数が N 未満の場合は適用。**diff: 無視** |
| `-Y`, `--all-y` | フラグ | すべての量的列を複数系列として重ね描き。**diff: 無視** |
| `--labels` | フラグ | bar の棒に値と割合のラベルを表示。**diff: 無視** |
| `--watch` | フラグ | 入力ファイルを監視し、変更時に再描画（stdin では使用不可） |
| `--theme` | `dark`（既定）,`light`,`high-contrast` | カラーテーマ（explore/present も対応） |
| `--bins` | `1`〜`10000`、既定 `10` | ヒストグラムのビン数。**diff: 無視** |
| `--glob` | パターン | ディレクトリモード: ファイルの絞り込み（`*`/`?` のみ） |
| `-R`, `--recurse` | フラグ | ディレクトリモード: サブディレクトリを走査（隠しディレクトリを除く） |
| `--catalog` | フラグ | ディレクトリモード: ファイルごとの列・行数・形式を一覧（読み込み失敗時はエラー） |
| `--no-limit` | フラグ | ディレクトリモード: 100 万行の自動サンプルを無効化 |
| `-h`, `--help` | フラグ | ヘルプを表示 |
| `-V`, `--version` | フラグ | バージョンを表示 |

`--bins`（1〜10000）と `--top`/`--tail`（≥ 1）は、チャート種別がそのフラグを
無視する場合でも、単一ファイル・watch・ディレクトリ・diff のすべてのモードで
事前に範囲検査されます。

## 次のステップ

- [チャート種別](./chart-types.md) — チャートの選択と調整。
- [出力モード](./output-modes.md) — text、JSON、table、スパークライン、SVG、HTML。
- [差分モード](./diff-mode.md) — 2 つのファイルの比較。
- [シェル補完](./shell-completions.md) — bash、zsh、fish、elvish、PowerShell の補完。
