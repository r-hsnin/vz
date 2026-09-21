# チャート種別

`vz` は各列を時系列・量的・カテゴリ・名義のいずれかとして推定し、解決された軸の
型からチャートを選びます。`-t line|bar|scatter|histogram|heatmap` で上書きでき
ます。

## 軸を明示した場合

X と Y の両方を指定すると、解決された型の組み合わせからチャートが決まります。

| X の型 | Y の型 | チャート |
|--------|--------|-------|
| 時系列 | 量的 | Line |
| カテゴリ | 量的 | Bar |
| 量的 | 量的 | Scatter |
| カテゴリ | カテゴリ | Heatmap |
| 量的 | 時系列 | Line（`x` が時系列になるよう軸を正規化） |
| 量的 | カテゴリ | Bar（`x` がカテゴリになるよう軸を正規化） |
| その他の解決済みの組 | — | Bar へのフォールバック + 警告 |

逆順の指定は正規化されます。`-x revenue -y date` は `x=date` の Line として、
`-x revenue -y city` は `x=city` の Bar として描画されます。フォールバックと
`warning: no chart rule for ...` は、専用ルールのない明示的な軸の組（名義列や
時系列 × 時系列など）にのみ適用されます。自動選択では発生しません。

## 片方の軸だけ指定した場合

**`-y` のみ:**

- 最初の時系列列 ⇒ Line。なければ最初のカテゴリ列 ⇒ Bar。なければ Y 以外の最初
  の量的列 ⇒ Scatter。
- Y だけが列の場合、型に応じてマッピングされます。量的または名義の Y ⇒
  Histogram、カテゴリの Y ⇒ `x=y=Y` の Heatmap、時系列の Y ⇒ フォールバック警
  告付きの Bar。

**`-x` のみ:**

- X 以外の最初の量的列 ⇒ 対応する組のチャート。
- それ以外でカテゴリの X ⇒ 行数の Bar。
- 量的の X ⇒ Histogram。
- それ以外はエラー。

## 軸を指定しない場合

自動選択の優先順位は次のとおりです。

| データの形 | チャート |
|---|---|
| 時系列 × 量的 | Line（color = 最初のカテゴリ列） |
| カテゴリ × 量的 | Bar（color なし） |
| 量的が 2 列以上 | Scatter（color = 最初のカテゴリ列） |
| 量的が 1 列 | Histogram |
| カテゴリが 2 列以上 | Heatmap |
| それ以外 | 検出した列を列挙するエラー |

名義列が自動で Bar になることはありません。

## bar チャート

bar チャートは既定で `sum` 集計します。数値の Y を指定しない `-x city` はカテゴ
リごとの行数を数えます（`y=count(city)`）。`-t bar` で Y が非量的の場合にも
`count` が自動適用されます。

```bash
vz sales.csv -x city -y revenue -t bar --sort desc
vz sales.csv -x city -y revenue -t bar --top 5
vz sales.csv -x city -y revenue -t bar --tail 5
vz sales.csv -x city -y revenue -t bar --agg mean
vz sales.csv -x city -y revenue -t bar --labels
```

- `--sort desc|asc|none` は値の順に並べます。無視するチャート種別では警告します。
- `--top N` は Y の上位 N カテゴリを残し `--sort desc` を含意します。`--tail N`
  は下位 N を残し `--sort asc` を含意します。
- `--agg` は `sum`（既定）、`mean`、`count`、`max`、`min` を受け付けます。
- `--labels` は棒に値と割合のラベルを表示します。

## ヒストグラム

ヒストグラムは 1 つの量的列の分布を表示します。ビン数は `--bins`（1〜10000、
既定 10）で指定します。

```bash
vz data.csv -y age -t histogram --bins 20
vz data.csv -y age -t histogram
```

## 複数系列

- Y 列はカンマ区切りで指定します: `-y revenue,profit`。
- `col:Label` で系列名を変更できます: `-y revenue:"Revenue (USD)"`。
- `-Y`/`--all-y` ですべての量的列を重ね描きします。
- `-c city` は line と scatter を系列ごとに分割します。

列名は大文字小文字を区別します。存在しない `-x`、`-y`、`-c` の名前は
`Did you mean '...'?` のヒント付きでエラーになります。2 つ目以降の `-y` 列も検証
されるため、`-y revenue,revnue` は 1 系列だけを黙って描画せず失敗します。

## bar での色

bar チャートが色でデータを分割することはありません。グループ化された棒や積み上
げ棒は存在せず、高さは全行で集計されたまま、色の列は要約の凡例にのみ現れます。
明示的な `-c` は警告し、自動検出された色は警告しません。

## 対話的に試す

`vz explore FILE` は同じ選択肢をその場で切り替えられる TUI を開きます（対話的な
ターミナルが必要です）。

| キー | 動作 |
|-----|--------|
| `h` / `l`（←/→） | X 軸の列を変更 |
| `j` / `k`（↑/↓） | Y 軸の列を変更（チャート）/ 行をスクロール（テーブル） |
| `g` / `G`（`Home`/`End`） | 先頭/末尾の行へ移動（テーブル） |
| `PgUp` / `PgDn` | テーブルをページ送り |
| `c` | 色・グループ化の列を巡回（bar では「legend only」と表示） |
| `s` | 並び順を巡回（desc/asc/none） |
| `a` | 集計方法を巡回（sum/mean/count/max/min） |
| `y` | 同等のワンショットコマンドをヤンク |
| `d` / `Tab` | チャート ↔ テーブル表示を切替 |
| `1`〜`5` | チャート種別を強制: Line/Bar/Scatter/Histogram/Heatmap |
| `0` | 自動チャート種別に戻す |
| `?` | ヘルプの表示/非表示 |
| `q` / `Esc` | 終了 |

diff-explore（2 ファイル）は並び替え、テーブルのスクロール、テーブル切替、ヤン
クのみに対応します。軸・色・集計のキーは「N/A in diff mode」と表示します。

## 次のステップ

- [はじめに](./getting-started.md) — インストールと最初の一歩。
- [出力モード](./output-modes.md) — 同じチャートを JSON、SVG、HTML で出力。
- [差分モード](./diff-mode.md) — 2 つのファイル間で値を比較。
