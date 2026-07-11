---
layout: home

hero:
  name: vz
  text: ターミナルデータ可視化
  tagline: "ゼロコンフィグ。スマートなチャート選択。即座に出力。vz data.csv だけ。"
  image:
    src: /demo-placeholder.svg
    alt: vz ターミナル出力 — マルチシリーズ折れ線グラフ
  actions:
    - theme: brand
      text: はじめる →
      link: /ja/guide/getting-started
    - theme: alt
      text: GitHub で見る
      link: https://github.com/r-hsnin/vz

features:
  - icon: 🧠
    title: スマート自動検出
    details: カラム型（時系列・数値・カテゴリ）を推論し、最適なチャートを自動選択。フラグ不要。
  - icon: ⚡
    title: ゼロコンフィグ
    details: CSV, TSV, JSON, NDJSON に対応。拡張子やコンテンツから自動的にフォーマットを検出。
  - icon: 🎨
    title: マルチシリーズ & カラー
    details: カテゴリ別に自動グループ化＋凡例表示。-c フラグで色分け、カンマ区切りで複数Y軸。
  - icon: 🔍
    title: インタラクティブ TUI
    details: Explore モード — vim風ナビゲーションで軸切替、チャート種別変更、テーブル表示。
  - icon: 🎬
    title: スライドプレゼン
    details: Present モード — Markdown内にライブチャートを埋め込んだターミナルスライド。
  - icon: 📈
    title: リッチサマリー
    details: スパークライン、トレンド(↑ +80%)、レンジ、凡例、追加カラム提案を1行で表示。
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

## デモ

コマンド一つで:

```bash
$ vz sales.csv
```

出力:

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

## チャート選択ルール

データ型に基づいて自動的にチャートを選択:

| X カラム | Y カラム | チャート | 例 |
|----------|----------|----------|------|
| 時系列 | 数値 | 📈 折れ線 | `date × revenue` |
| カテゴリ | 数値 | 📊 棒 | `city × sales` |
| 数値 | 数値 | 🔵 散布図 | `height × weight` |
| — (単一) | 数値 | 📶 ヒストグラム | `exam scores` |
| カテゴリ | カテゴリ | 🟦 ヒートマップ | `dept × level` |

## クイックスタート

::: code-group

```bash [インストール]
cargo install --git https://github.com/r-hsnin/vz
```

```bash [基本操作]
# 自動可視化
vz data.csv

# 軸指定 + チャート種別
vz sales.csv -x month -y revenue -t bar

# マルチシリーズ
vz sales.csv -y revenue -c city

# パイプライン
cat data.json | vz --spark
```

```bash [対話モード]
# インタラクティブ TUI
vz explore data.csv

# プレゼンテーション
vz present slides.md
```

:::
