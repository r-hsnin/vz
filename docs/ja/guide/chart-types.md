# チャート種別

列の型の組み合わせで、描画するチャートが決まる。

## 自動選択ルール

| X 列 | Y 列 | チャート | 典型的なデータ |
|------|------|----------|---------------|
| 日付 | 数値 | 📈 折れ線 | 月別売上、株価推移 |
| カテゴリ | 数値 | 📊 棒 | 都市別比較、部門集計 |
| 数値 | 数値 | 🔵 散布図 | 身長と体重、価格と面積 |
| — | 数値のみ | 📶 ヒストグラム | 試験スコアの分布 |
| カテゴリ | カテゴリ | 🟦 ヒートマップ | 部門×職級の人数 |

## 折れ線グラフ

X が日付、Y が数値のとき自動で選ばれる。

```bash
vz stock.csv
# → date × price の折れ線
```

- Braille 文字で高解像度に描画
- `-c` で系列ごとに色分け＋凡例
- サマリーにトレンド表示（↑ +12% など）

## 棒グラフ

X がカテゴリ、Y が数値のとき。

```bash
vz sales.csv -x city -y revenue -t bar
```

- 同じカテゴリの値は自動で合算（デフォルト: sum）
- `--sort desc` で降順ソート
- バーの中に集計値を表示

## 散布図

X と Y の両方が数値のとき。

```bash
vz body_measurements.csv
# → height_cm × weight_kg
```

## ヒストグラム

数値列が1つだけの場合。

```bash
vz exam_scores.csv
# → score の度数分布
```

## ヒートマップ

カテゴリ同士の掛け合わせ。カウントで濃淡を表現。

```bash
vz departments.csv -x department -y level
```

## 手動指定

`-t` で自動選択を上書きできる。

```bash
vz data.csv -t line
vz data.csv -t bar
vz data.csv -t scatter
vz data.csv -t histogram
vz data.csv -t heatmap
```
