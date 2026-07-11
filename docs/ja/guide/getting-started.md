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

Rust 1.70 以上が必要です。

## 最初の一歩

```bash
vz data.csv
```

これだけで vz が裏側で以下を行います:

1. **フォーマット検出** — CSV か TSV か JSON か、拡張子と中身から判定
2. **型推論** — 各列が日付なのか数値なのかカテゴリなのかを分析
3. **チャート選択** — 型の組み合わせから最適なチャートを決定
4. **描画** — カラーチャートをターミナルにレンダリング

## 軸やチャートを指定する

自動選択で十分なことがほとんどですが、明示的に制御したい場合:

```bash
# X と Y を明示
vz sales.csv -x month -y revenue

# チャート種別を上書き
vz sales.csv -x city -y revenue -t bar

# 複数の Y 軸を重ねて表示
vz sales.csv -y revenue,profit

# カテゴリで色分け（マルチシリーズ）
vz sales.csv -c city
```

## 出力の使い分け

用途に合わせて出力形式を選べます:

```bash
# チャート（デフォルト）
vz data.csv

# テーブル形式で数値を確認
vz data.csv -o table

# 1行スパークライン — シェルスクリプトに埋め込みやすい
vz data.csv --spark

# 列の型や統計を確認
vz data.csv --info
```

## 対話的に探索する

```bash
# Explore モード — TUI でリアルタイムに軸やチャートを切り替え
vz explore data.csv

# Present モード — Markdown にチャートを埋め込んでスライド発表
vz present slides.md
```

## データを絞り込む

```bash
# 特定の値でフィルタ
vz sales.csv --where "city=Tokyo"

# 条件式でフィルタ
vz sales.csv --where "revenue>1500"
```

## シェル補完を設定する

```bash
# Bash
vz completions bash >> ~/.bashrc

# Zsh
vz completions zsh >> ~/.zshrc

# Fish
vz completions fish > ~/.config/fish/completions/vz.fish
```

設定後、`vz` のあとに Tab を押すとフラグやオプションが補完されます。
