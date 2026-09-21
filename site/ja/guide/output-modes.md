# 出力モード

`vz` はチャートと機械可読な形式を stdout に書き、要約・インサイト・警告は
stderr に残します。例外は JSON で、エラーも stdout に出力されるため、常に終了
コードを確認してください（成功 `0`、失敗 `1`）。

| 形式 | フラグ | 備考 |
|--------|------|-------|
| text | 既定 | チャートは stdout、要約・インサイトは stderr |
| json | `-o json` / `--json` | 構造化オブジェクト全体。下記参照 |
| table | `-o table` | bar は集計済み 2 列、その他のチャートは全列。100 行上限 |
| markdown | `-o markdown` / `--markdown` | GFM テーブル。bar/非 bar の扱いと 100 行上限は同じ |
| spark | `-o spark` / `--spark` | 系列・グループごとに 1 行のスパークライン |
| svg | `-o svg` / `--svg` | テキストグリッドの SVG と不可視のデータマーク |
| html | `-o html` / `--html` | ホバーツールチップ付きの単体 HTML |

## text（既定）

既定の出力は 3 つを書き分けます。

1. stderr の要約行。例:

   `Line │ x=date │ y=revenue (100–500) ▁▃▅▇ │ ↑ +50% │ color=city [Tokyo=cyan, Osaka=yellow] │ 6 rows`

2. stdout のチャート（タイトル、軸目盛り、凡例）。
3. stderr の 0〜3 行の平易な気づき（成長、最大、クラスタ）。データ点が 2 未満
   のときは何も出力しません。

要約にはチャート種別と軸、Y の範囲とインラインのスパークラインおよび傾向
（`↑ +N%` / `↓ -N%` / ±5% 以内は `→ stable`）、`y=mean(revenue)` のような非
sum 集計の表示、色の凡例、スキップ数付きの行数（`6 rows (2 skipped)`）、未使用
列のヒントが含まれます。

## table と markdown

`-o table` と `-o markdown` はチャートの代わりにデータを表として出力します。
bar チャートは集計済みの 2 列になり、その他のチャート種別はすべての列を出しま
す。どちらも 100 行で打ち切られます。

```bash
vz sales.csv -o table
vz sales.csv --markdown
```

## spark

スパークライン出力は系列ごとに 1 行で、範囲と傾向を添えます。

```
revenue  ▁▂▃▅▇  (100–500) ↑ +400%
```

```bash
vz sales.csv -y latency --spark
```

bar は範囲のみを表示します。非有限値はスキップされ、`(N skipped)` が付きます。

## SVG と HTML

`-o svg` はテキストグリッドの SVG と、`data-label`、`data-value`、
`data-series` を持つ不可視の `circle.vz-point` マークを出力します。`-o html` は
その SVG を JavaScript のホバーツールチップ付きの単体 HTML で包みます。

```bash
vz sales.csv --svg > chart.svg
vz sales.csv --html > chart.html
```

## JSON

`-o json`（または `--json`）は構造化オブジェクトを stdout に出力します。

- `version` — スキーマバージョン（現在は `1`）。
- `file` — 入力パス。
- `rows` — 行数。
- `columns[]` — `{ name, type, nulls, stats }`。
- `recommendation` — 省略可能な `{ chart_type, x, y, color }`。
- `data[]` — 先頭 100 行のオブジェクト。
- `truncated` — `data[]` が打ち切られたか。
- `chart_data` — チャート種別ごとの集計（チャートのみ）。
- `query` — `{ chart_type, x, y, extra_y, color, agg, sort, limit, bins, filters, sample }`。
- `insights[]` — 平易な気づき。

値の規則:

- `data[]` は 100 行上限です。全体の集計には `data[]` ではなく
  `chart_data`/`query` を使ってください。
- 値は数値として解釈されます。`$100` は `100`、`45%` は `0.45` になります。
- 非有限値（`NaN`/`inf`）は `data[]` では `null` になり、チャート系列からは除かれます。
- diff の行は `pct_change` を使います。新規カテゴリでは `null`、両側とも 0 の場合は `0` です。

`--info -o json` では info オブジェクトのみが出力されます: `version`、`file`、
`rows`、`columns[]`、推薦できる場合の `recommendation`、`data[]`（先頭 100 行）、
`truncated`。`chart_data` などのチャート用フィールドは含まれません:

```bash
vz sales.csv --info -o json | jq '.columns[] | {name, type, stats}'
```

解決された形式が JSON の場合、エラーは整形された JSON オブジェクト
（`{ "version": 1, "error": "..." }`）として stdout に出力されます。stdout を
信用する前に終了コードを確認してください。

## 色と幅

- ANSI の色は stdout が TTY のときだけ出力されます。空でない `NO_COLOR` は無効化し、空でない `FORCE_COLOR` は強制します。stdout のチャートと stderr の要約の両方に適用されます。
- stdout がパイプの場合、`COLUMNS` を無視してチャート幅は 80 になります。stderr がパイプの場合、要約行はターミナル幅ではなく 120 列になります。
- `--theme dark|light|high-contrast` は配色を選びます。explore と present でも有効です。

## present モード

`vz present FILE` は Markdown ファイルをターミナルのスライドとして描画します
（TTY が必要です）。スライドは `---` で区切られ、`# ` がスライドタイトルを設定します。見出し、リスト、引用、GFM テーブル、フェンス付きコードブロックが描画されます。フェンス付きの `chart` ブロックはチャートを埋め込みます。

````markdown
```chart
source: sales.csv
x: month
y: revenue
type: line
```
````

| パラメータ | 説明 |
|-----------|-------------|
| `source` | データファイルのパス（必須） |
| `x` | X 軸の列 |
| `y` | Y 軸の列 |
| `color` | 色・グループ化の列 |
| `title` | チャートのタイトル |
| `type` | `line`、`bar`、`scatter`、`histogram`、`heatmap` |
| `where` | 行の絞り込み（繰り返し可）: `where: revenue>1000` |
| `sort` | bar の並び替え: `desc` または `asc` |
| `agg` | `sum`、`mean`、`count`、`max`、`min` |
| `top` | 上位 N カテゴリのみ表示 |
| `bins` | ヒストグラムのビン数 |
| `height` | チャートの高さ（行数） |
| `diff` | diff チャートの「後」ファイル。`source` が「前」になる |

未知のキーは無視されます。`type`/`sort`/`agg`/`top`/`bins`/`height` の不正な値は警告し、自動の挙動へフォールバックします。チャートの `source`（および `diff`）パスは、まず Markdown ファイルからの相対、次にカレントディレクトリからの相対で解決されます。

操作: `→` / `l` / `Space` で次、`←` / `h` / `Backspace` で前、`Enter` で次（数字入力中はそのスライドへジャンプ）、`g` / `G` / `Home` / `End` で最初/最後、数字 + `Enter` で 1 始まりのスライドへジャンプ、`q` / `Esc` で終了します。

## 次のステップ

- [チャート種別](./chart-types.md) — 選択ルールと bar のオプション。
- [差分モード](./diff-mode.md) — 差分の出力形式と注釈。
- [シェル補完](./shell-completions.md) — シェルでフラグを補完。
