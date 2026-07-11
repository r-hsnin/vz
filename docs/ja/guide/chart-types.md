# チャート種別

vz はデータのカラム型に基づいて、最適なチャートを自動選択します。

## 選択ルール

| X 型 | Y 型 | チャート | 用途 |
|------|------|----------|------|
| 時系列 | 数値 | 📈 折れ線 | 時系列データ |
| カテゴリ | 数値 | 📊 棒 | カテゴリ比較 |
| 数値 | 数値 | 🔵 散布図 | 2数値カラムの相関 |
| — | 数値 | 📶 ヒストグラム | 単一数値の分布 |
| カテゴリ | カテゴリ | 🟦 ヒートマップ | 2カテゴリの密度 |

## 折れ線グラフ

**時系列データ**に最適。X が日付、Y が数値のとき自動選択。

```bash
vz stock.csv
# 自動的に折れ線: date × price
```

特徴:
- Braille文字による高解像度描画
- `-c` フラグでマルチシリーズ＋凡例
- トレンド表示 (↑ +80% / ↓ -20%)

## 棒グラフ

**カテゴリ比較**に最適。X がカテゴリ、Y が数値のとき自動選択。

```bash
vz sales.csv -x city -y revenue -t bar
```

特徴:
- 自動集計 (デフォルト: sum、`--agg` で変更可)
- ソート: `--sort desc` / `--sort asc`
- 上位/下位制限: `--top 10` / `--tail 5`
- バー内に値ラベル表示

## 散布図

**相関分析**に最適。X と Y の両方が数値のとき自動選択。

```bash
vz body_measurements.csv
# 散布図: height × weight
```

## ヒストグラム

**分布分析**に最適。数値カラムが1つだけのとき選択。

```bash
vz exam_scores.csv
# スコアの分布を表示
```

## ヒートマップ

**2つのカテゴリカラム**の組み合わせ。カウントベースの密度表示。

```bash
vz departments.csv -x department -y level
```

## チャート種別の手動指定

`-t` で任意のチャートを強制:

```bash
vz data.csv -t line
vz data.csv -t bar
vz data.csv -t scatter
vz data.csv -t histogram
vz data.csv -t heatmap
```
