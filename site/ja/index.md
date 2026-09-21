---
layout: home

hero:
  name: vz
  text: ゼロコンフィグのターミナルデータ可視化
  tagline: 表形式ファイルを vz に渡すだけ。列の型を推定し、チャートを選び、ターミナルに描画します。
  actions:
    - theme: brand
      text: はじめる
      link: /ja/guide/getting-started
    - theme: alt
      text: GitHub
      link: https://github.com/r-hsnin/vz

features:
  - title: 型の自動推定
    details: 時系列・量的・カテゴリ・名義の各列を自動で判定します。
  - title: チャートの自動選択
    details: 解決された軸の型から line / bar / scatter / histogram / heatmap を選びます。
  - title: 多彩な出力
    details: text、JSON、table、Markdown、スパークライン、SVG、単体 HTML に対応します。
  - title: 柔軟な入力
    details: CSV、TSV、JSON、NDJSON、固定幅テキスト、ディレクトリ、標準入力に対応します。
  - title: 差分モード
    details: 2 つのファイルを比較し、カテゴリごとの変化や時系列の重ね描きを表示します。
  - title: ターミナルネイティブ
    details: ワンショットで stdout に出力。対話的な explore / present モードも備えます。
---

## インストール

```bash
cargo install --git https://github.com/r-hsnin/vz
```

Rust 1.88 以降が必要です。

## クイックスタート

```bash
# 自動可視化: 軸とチャート種別を推定
vz sales.csv

# 軸とチャート種別を指定
vz sales.csv -x month -y revenue
vz sales.csv -x city -y revenue -t bar

# 標準入力から読み込む
cat data.csv | vz -
```

インストール手順と CLI の概要は[はじめに](./guide/getting-started.md)を参照してください。
[チャート種別](./guide/chart-types.md)、[出力モード](./guide/output-modes.md)、
[差分モード](./guide/diff-mode.md)、[シェル補完](./guide/shell-completions.md)へも直接進めます。
