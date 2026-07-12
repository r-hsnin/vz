---
layout: home

hero:
  name: vz
  text: ターミナルでデータを見る
  tagline: "データを入れるだけ。型推論で、最適な形に可視化する。"
  image:
    src: /demo-placeholder.svg
    alt: vz の出力例 — 売上データの折れ線グラフ
  actions:
    - theme: brand
      text: 使ってみる →
      link: /ja/guide/getting-started
    - theme: alt
      text: GitHub
      link: https://github.com/r-hsnin/vz

features:
  - icon: 🧠
    title: データ型を自動認識
    details: 日付・数値・カテゴリを自動判定し、最適なチャートで描画する。
  - icon: ⚡
    title: フォーマットの指定不要
    details: CSV, TSV, JSON, NDJSON — 拡張子や中身から自動で判別する。
  - icon: 🎨
    title: マルチシリーズ対応
    details: カテゴリで自動グループ化し、色分けと凡例を表示する。
  - icon: 🔍
    title: インタラクティブ TUI
    details: vim キーバインドで軸やチャートをリアルタイムに切り替える。
  - icon: 🎬
    title: ターミナルでプレゼン
    details: Markdown にチャートを埋め込み、ターミナル上でスライド発表する。
  - icon: 📈
    title: サマリーを1行で表示
    details: スパークライン・トレンド・レンジ・凡例を1行に凝縮する。
---

<style>
:root {
  --vp-home-hero-name-color: transparent;
  --vp-home-hero-name-background: -webkit-linear-gradient(120deg, #58a6ff 30%, #7ee787);
  --vp-home-hero-image-background-image: linear-gradient(-45deg, #58a6ff22 50%, #7ee78722 50%);
  --vp-home-hero-image-filter: blur(44px);
}

.dark {
  --vp-home-hero-image-background-image: linear-gradient(-45deg, #58a6ff33 50%, #7ee78733 50%);
}
</style>

## 動作イメージ

```bash
$ vz sales.csv
```

```
Line │ x=date │ y=revenue (800–2.0k) ▂▅▃▁█▇ │ ↑ +80% │ color=city │ 6 rows
┌revenue vs date───────────────────────────────────────────────────────────────┐
│2.0k     │revenue                                             ⡠⠔⠁     ┌──────┐│
│         │                                                 ⢀⠔⠊        │Tokyo ││
│         │                                               ⡠⠒⠁          │Osaka⣀││
│         │                                            ⢀⠔⠉    ⢀⣀⣀⣀⠤⠤⠤⠤⠒│Nagoya││
│         │                                     ⣀⣀⣀⣀⠤⠤⠤⠔⠒⠒⠒⠉⠉⠉⠁        └──────┘│
│         │                       ⣀⣀⣀⡠⠤⠤⠤⠒⠒⠒⠊⠉⠉⠉  ⣀⠔⠁                          │
│1.5k     │                                  ⡠⠔⠁                               │
│1.0k     │⠒⠊⠉⠉                                                                │
│         │                                        •                           │
│500      │                                                                date│
│         └────────────────────────────────────────────────────────────────────│
│2024-01-01            2024-02-01 2024-03-01 2024-04-01 2024-05-01   2024-06-01│
└──────────────────────────────────────────────────────────────────────────────┘
```

## チャートの自動選択

列の型の組み合わせで、描画するチャートが決まる。

| X 列 | Y 列 | チャート | 典型的なデータ |
|------|------|----------|---------------|
| 日付 | 数値 | 📈 折れ線 | 売上推移、株価 |
| カテゴリ | 数値 | 📊 棒 | 都市別売上、部門比較 |
| 数値 | 数値 | 🔵 散布図 | 身長×体重、価格×面積 |
| — | 数値 | 📶 ヒストグラム | 試験点数の分布 |
| カテゴリ | カテゴリ | 🟦 ヒートマップ | 部門×スキルレベル |

`-t bar` のように手動指定もできる。

## すぐに始める

::: code-group

```bash [インストール]
cargo install --git https://github.com/r-hsnin/vz
```

```bash [使い方]
# 自動可視化
vz data.csv

# 軸を指定して棒グラフ
vz sales.csv -x city -y revenue -t bar

# 都市ごとに色分け
vz sales.csv -y revenue -c city

# パイプで受け取って1行スパークライン
cat data.json | vz --spark
```

```bash [対話モード]
# TUI で探索（vim キーバインド）
vz explore data.csv

# Markdown スライドでプレゼン
vz present slides.md
```

:::
