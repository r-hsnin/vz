# 差分モード

2 つのデータファイルを比較し、変化を可視化する。

## クイックスタート

```bash
# 位置引数: 2 ファイルを並べる
vz before.csv after.csv

# フラグ構文: --diff
vz file1.csv --diff file2.csv
```

両ファイルのスキーマ（列名）が一致している必要がある（大文字小文字は区別しない）。X 列がカテゴリカルか時系列かを自動判定し、適切な可視化を選択する。

## カテゴリカル差分（バーチャート）

X 列がカテゴリカル（都市名、商品名など）の場合、▲/▼ マーカー付きのバーチャートを表示する。

```bash
vz sales_before.csv sales_after.csv
```

出力:
```
Diff │ x=city │ y=revenue │ sales_before vs sales_after │ Δ net +5% │ 4 entries
  Tokyo     ████████████  1,000 → 1,200  ▲ +20%
  Osaka     ██████████    1,500 → 1,350  ▼ -10%
  Nagoya    ████████      800 → 950      ▲ +19%
  Fukuoka   █████         600 → 600      ─ 0%
```

各エントリの構成:
- カテゴリラベル
- スケーリングされたバー（after 値に比例）
- Before → After の値
- 方向マーカー: `▲`（増加）、`▼`（減少）、`─`（変化なし）
- 変化率

「after」にのみ存在する新規カテゴリは、パーセンテージではなく絶対値を表示する。

## 時系列差分（ラインチャート オーバーレイ）

X 列が時系列（日付、タイムスタンプ）の場合、2 系列のラインチャートを重ねて表示する:

```bash
vz timeseries_before.csv timeseries_after.csv
```

出力:
```
Line │ x=date │ timeseries_before vs timeseries_after │ Δ +25% │ 6 rows
```

チャートの内容:
- **Before** 系列: グレー（DarkGray）
- **After** 系列: シアン
- 凡例にファイル名を表示
- X 軸は両ファイルの全日付の和集合

## 出力形式

### スパークライン

```bash
vz before.csv after.csv --spark
```

カテゴリカル:
```
Δ revenue  ▅▁▃▁  (+5%)
```

時系列:
```
timeseries_before  ▂▃▅▆▇█
timeseries_after   ▃▄▆▇██  (+25%)
```

### JSON

```bash
vz before.csv after.csv --json
```

カテゴリカル出力:
```json
{
  "version": 1,
  "mode": "diff",
  "before": { "file": "before.csv", "rows": 4 },
  "after": { "file": "after.csv", "rows": 4 },
  "x_column": "city",
  "y_column": "revenue",
  "categories": [
    { "label": "Tokyo", "before": 1000, "after": 1200, "delta": 200, "pct_change": 20.0 }
  ],
  "overall_delta_pct": 5.1
}
```

時系列出力:
```json
{
  "version": 1,
  "mode": "diff",
  "chart_type": "line",
  "before": { "file": "before.csv", "rows": 6, "series": [...] },
  "after": { "file": "after.csv", "rows": 6, "series": [...] },
  "x_column": "date",
  "y_column": "revenue",
  "dates": ["2024-01", "2024-02", "2024-03"],
  "overall_delta_pct": 25.0
}
```

## オプション

| フラグ | 説明 |
|--------|------|
| `--sort desc` | 増加量が大きい順にソート |
| `--sort asc` | 減少量が大きい順にソート |
| `--top N` | 上位 N カテゴリのみ表示（`--sort desc` を暗黙適用）|
| `--tail N` | 下位 N カテゴリのみ表示（`--sort asc` を暗黙適用）|
| `-x` | X 列を手動指定 |
| `-y` | Y 列を手動指定 |

```bash
# 変化量トップ 3
vz q1.csv q2.csv --top 3

# 減少量が大きい順
vz q1.csv q2.csv --sort asc
```

::: tip
`--sort`、`--top`、`--tail` はカテゴリカル差分にのみ適用される。時系列差分は常に日付順で全データを表示する。
:::

## スキーマ要件

両ファイルの列名が一致している必要がある（大文字小文字は区別しない）。不一致の場合はエラーを表示する:

```
Error: Schema mismatch: column 'revenue' in 'before.csv' not found in 'after.csv'.
Before columns: [city, revenue]
After columns: [city, sales]
```

列の順番は問わない — 名前のみ一致すればよい。

## Tips

- 重複カテゴリは比較前に sum で集約される
- `-x`、`-y` で自動検出の軸を上書きできる
- 差分モードは現在 text、spark、JSON 出力に対応している
