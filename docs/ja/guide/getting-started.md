# はじめに

## インストール

::: code-group

```bash [ソースから（推奨）]
cargo install --git https://github.com/r-hsnin/vz
```

```bash [ローカルビルド]
git clone https://github.com/r-hsnin/vz
cd vz && cargo install --path .
```

:::

Rust 1.70 以上が必要。

## 最初の一歩

```bash
vz data.csv
```

vz が行うこと:

1. **フォーマット検出** — CSV / TSV / JSON / NDJSON を拡張子と中身から判定
2. **型推論** — 各列が日付・数値・カテゴリのどれかを分析
3. **チャート選択** — 型の組み合わせから最適なチャートを決定
4. **描画** — カラーチャートをターミナルに出力

## 軸やチャートを指定する

```bash
# X と Y を明示
vz sales.csv -x month -y revenue

# チャート種別を上書き
vz sales.csv -x city -y revenue -t bar

# 複数の Y 軸を重ねる
vz sales.csv -y revenue,profit

# カテゴリで色分け（マルチシリーズ）
vz sales.csv -c city
```

## 出力形式の切り替え

```bash
# チャート（デフォルト）
vz data.csv

# テーブル形式
vz data.csv -o table

# 1行スパークライン
vz data.csv --spark

# 列の型や統計を表示
vz data.csv --info
```

## 対話モード

```bash
# Explore — TUI で軸やチャートを切り替えながら探索
vz explore data.csv

# Present — Markdown にチャートを埋め込んでスライド発表
vz present slides.md
```

## フィルタリング

```bash
vz sales.csv --where "city=Tokyo"
vz sales.csv --where "revenue>1500"
```

## シェル補完

```bash
# Bash
vz completions bash >> ~/.bashrc

# Zsh
vz completions zsh >> ~/.zshrc

# Fish
vz completions fish > ~/.config/fish/completions/vz.fish
```
