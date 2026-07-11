---
layout: home

hero:
  name: vz
  text: ターミナルで、データを見る
  tagline: "CSV を渡すだけ。型を読み取り、最適なチャートを即座に描画。設定ファイル不要。"
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
    details: 日付・数値・カテゴリを自動判定し、折れ線/棒/散布図/ヒストグラムを選び分ける。指定不要。
  - icon: ⚡
    title: 設定なしで動く
    details: CSV, TSV, JSON, NDJSON — 拡張子や中身を見て判別するので、フォーマット指定すら不要。
  - icon: 🎨
    title: 複数系列を色分け表示
    details: カテゴリ列で自動グループ化。凡例付き。-c で色分け、カンマ区切りで複数指標を重ねて表示。
  - icon: 🔍
    title: 対話的に探索
    details: Explore モードで TUI を起動。vim キーバインドで軸やチャートを即座に切り替え。
  - icon: 🎬
    title: ターミナルでプレゼン
    details: Markdown にチャートを埋め込んでスライド発表。データの話をターミナルだけで完結。
  - icon: 📈
    title: 1行に情報を凝縮
    details: スパークライン・トレンド・レンジ・凡例・「この列も試してみては？」を1行のサマリーに。
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

これだけで、こうなる:

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

列の型の組み合わせで、描画するチャートが決まる:

| X 列 | Y 列 | チャート | よくあるデータ |
|------|------|----------|---------------|
| 日付 | 数値 | 📈 折れ線 | 売上推移、株価 |
| カテゴリ | 数値 | 📊 棒 | 都市別売上、部門比較 |
| 数値 | 数値 | 🔵 散布図 | 身長×体重、価格×面積 |
| — | 数値 | 📶 ヒストグラム | 試験点数の分布 |
| カテゴリ | カテゴリ | 🟦 ヒートマップ | 部門×スキルレベル |

もちろん `-t bar` のように手動指定もできる。

## すぐに始める

::: code-group

```bash [インストール]
cargo install --git https://github.com/r-hsnin/vz
```

```bash [よく使うコマンド]
# とりあえず見る
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
