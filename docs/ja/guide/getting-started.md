# はじめに

## インストール

::: code-group

```bash [ソースから]
cargo install --git https://github.com/r-hsnin/vz
```

```bash [クローン & ビルド]
git clone https://github.com/r-hsnin/vz
cd vz && cargo install --path .
```

:::

Rust 1.70+ が必要です。

## 最初のチャート

```bash
vz data.csv
```

これだけで vz は:

1. **フォーマットを検出** (CSV, TSV, JSON, NDJSON)
2. **カラム型を推論** (時系列、数値、カテゴリ)
3. **最適なチャートを選択**
4. **カラーチャートをターミナルに描画**

## 軸の指定

```bash
# X軸とY軸を明示
vz sales.csv -x month -y revenue

# チャート種別を指定
vz sales.csv -x city -y revenue -t bar

# 複数Y軸
vz sales.csv -y revenue,profit

# カラーグループ（マルチシリーズ）
vz sales.csv -c city
```

## 出力モード

```bash
# デフォルト: チャートを stdout に出力
vz data.csv

# テーブル出力
vz data.csv -o table

# スパークライン（パイプライン向け）
vz data.csv --spark

# カラムメタデータ
vz data.csv --info
```

## 対話モード

```bash
# Explore: インタラクティブ TUI
vz explore data.csv

# Present: ターミナルスライド
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
