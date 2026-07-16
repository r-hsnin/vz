# 出力モード

用途に合わせて出力形式を切り替えられる。

## チャート（デフォルト）

```bash
vz data.csv
```

カラーのターミナルチャートとサマリーラインを表示する。サマリーには以下が含まれる。

- チャート種別・軸名
- 値のレンジとスパークライン
- トレンドの方向（↑ +80% のように）
- マルチシリーズの凡例

## テーブル

```bash
vz data.csv -o table
```

数値を確認したいときに使う。棒グラフの場合は集計後の値、それ以外は元の X/Y データが表示される。

## スパークライン

```bash
vz data.csv --spark
# → ▂▅▃▁█▇
```

1 行の Unicode スパークライン。シェルスクリプトやプロンプトに埋め込める。

```bash
echo "売上: $(vz sales.csv --spark)"
# → 売上: ▂▅▃▁█▇
```

## Info

```bash
vz data.csv --info
```

チャートは描かず、列ごとの型・欠損数・統計量と、推奨チャート設定を表示する。

## JSON

```bash
vz data.csv -o json
```

メタデータを JSON で出力する。`--info` と組み合わせるとメタデータのみ、なしだとチャートデータ + メタデータ。

```bash
# メタデータのみ
vz data.csv --info -o json

# チャートデータ含む
vz data.csv -o json
```

## SVG

チャートを SVG 画像としてエクスポートする。ターミナル出力と同じモノスペーステキスト形式。

```bash
# SVG に保存
vz data.csv --svg > chart.svg

# サイズ指定
vz data.csv -W 100 -H 30 --svg > wide-chart.svg

# 白背景（ドキュメント・Wiki 向け）
vz data.csv --svg --theme light > chart-light.svg
```

`--theme` に連動し、dark は暗い背景、light は白背景の SVG になる。

## HTML

自己完結型の HTML ページとしてエクスポートする。ブラウザで開けばインタラクティブなチャートが表示される。外部 CDN に依存しないので完全にオフラインで動作する。

```bash
# インタラクティブ HTML に保存
vz data.csv --html > chart.html

# ライトテーマ（白背景）
vz data.csv --html --theme light > chart.html

# タイトルとサイズ指定
vz data.csv --html --title "Q2 売上" -W 100 -H 30 > report.html
```

HTML ファイルの内容:
- SVG チャートをインラインで埋め込み（外部リンクなし）
- レスポンシブレイアウトとテーマ背景色のインライン CSS
- データポイントとバーのホバーツールチップ用インライン JavaScript
- 外部スクリプト・スタイルシートなし — 単一ファイルでオフライン動作

## Markdown

集計結果を GitHub Flavored Markdown テーブルで出力する。README や Issue に埋め込むのに便利。

```bash
vz sales.csv -x city -y revenue -t bar --markdown
```

出力例:
```markdown
| city | revenue |
|---|---|
| Tokyo | 4200 |
| Osaka | 3300 |
| Nagoya | 800 |
```

## 3 つのモード

### ワンショット（デフォルト）

チャートを描いて即終了。パイプラインの途中や、ちょっと確認したいときに。

### Explore モード

```bash
vz explore data.csv
```

データを対話的に探索する TUI。

| キー | 操作 |
|------|------|
| `h` / `l` | X 軸を変更 |
| `j` / `k` | Y 軸を変更（テーブル時はスクロール）|
| `d` / `Tab` | チャート ↔ テーブル切替 |
| `1`〜`4` | チャート種別を固定 |
| `0` | 自動に戻す |
| `q` | 終了 |

### Present モード

```bash
vz present slides.md
```

Markdown にチャートを埋め込み、ターミナル上でスライド発表する。

````markdown
# 売上レポート

```chart
source: sales.csv
x: month
y: revenue
type: line
title: 月次売上推移
```

---

# まとめ
- 売上は前年比 80% 増
- 東京が全都市をリード
````

操作: `←` / `→` でページ送り、`g` / `G` で先頭/末尾、`q` で終了。

## ウォッチモード

ファイル変更を監視し、自動的に再描画する。データ探索や ETL 開発時に便利。

```bash
vz data.csv --watch
```

- 200ms のデバウンス（高速保存でもちらつかない）
- エディタのアトミック保存に対応
- `Ctrl+C` で停止

## テーマ

ターミナル背景に合わせてカラーパレットを切り替える。

```bash
# ダークターミナル（デフォルト）
vz data.csv --theme dark

# ライト / 白背景ターミナル
vz data.csv --theme light

# 最大コントラスト（色覚多様性対応）
vz data.csv --theme high-contrast
```

テーマはすべてのチャート、サマリーライン、SVG エクスポートの背景色に反映される。
