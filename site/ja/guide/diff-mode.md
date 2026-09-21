# 差分モード

差分モードは、同じスキーマを持つ 2 つのファイルを比較し、変化を強調します。
2 つの位置引数、または 1 つの位置引数と `--diff` で起動します。

```bash
vz before.csv after.csv
vz before.csv --diff after.csv
vz q1.csv q2.csv --sort desc          # 増加の大きい順
vz q1.csv q2.csv --sort desc --top 5  # 増加の上位 5 件
vz q1.csv q2.csv -o spark
vz q1.csv q2.csv -o json
```

## スキーマの要件

両方のファイルが同じスキーマを共有している必要があります。

- 列数が同じであること。
- 「前」のすべての列名が「後」に存在すること（大文字小文字を区別せず、前後の
  空白は無視）。

X は `-x` から、なければ最初のカテゴリ列または時系列列から、それもなければ最初
のヘッダーから取られます。Y は `-y` から、なければ X 以外の最初の量的列から取
られます。

## カテゴリの X: 変化の bar

カテゴリの X はカテゴリごとの変化を bar チャートにし、`label ▲ +20%` /
`▼ -10%` / `─ 0%` と注釈します。

- `▲ new` / `▼ new` は「前」が実質 0 で「後」が非 0 のときだけ表示されます。
- 両方とも 0 の場合は `─ 0%` になります。
- JSON では新規カテゴリは `pct_change: null`（`new` というテキストはなし）、両
  方 0 の行は `pct_change: 0` になります。

## 時系列の X: 折れ線の重ね描き

時系列の X は 2 系列の折れ線を重ねます。「前」はグレー、「後」はシアンです。

## 並び替え

差分の並び替えは**符号付きの差分**を使います。

- `--sort desc` は絶対値ではなく増加の大きい順に並べます。
- `--sort asc` は減少の大きい順に並べます。
- `--sort none` は入力順を保ちます。

diff-explore の対話的な並び替えだけは例外で `|Δ|` を使います。

## 差分モードでのフラグ

`--where`、`--agg`、`--color` は効果がなく、警告します。`-t`、`--labels`、
`--sample`、`--all-y`、`--bins` は黙って無視されます。diff のパラメータ集合に
含まれないフラグ（`-I`/`--info`、`--watch`、およびディレクトリ専用の
`--glob`、`-R`、`--catalog`、`--no-limit`）も同様に無視されます。`--sample 0`
は差分モードでは検証されません。

## 出力形式

差分は `text`（既定）、`spark`、`json`、`markdown`、`html` に対応します。
`-o table` と `-o svg` は未対応で、既定の text レンダラーにフォールバックします。

```bash
vz before.csv after.csv --markdown
vz before.csv after.csv -o json | jq '.data[] | {name, pct_change}'
```

## 対話的に差分を見る

`vz explore FILE1 FILE2` は diff-explore を開きます。並び替え、テーブルのスク
ロール、テーブル切替、ヤンクのみに対応し、軸・色・集計のキーは
「N/A in diff mode」と表示します。共通のキーバインドは
[チャート種別](./chart-types.md)を参照してください。

## 次のステップ

- [チャート種別](./chart-types.md) — カテゴリと軸の解決方法。
- [出力モード](./output-modes.md) — JSON のフィールドとその他の形式。
- [はじめに](./getting-started.md) — インストールと基本的な使い方。
