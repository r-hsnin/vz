# シェル補完

`vz completions SHELL` は補完スクリプトを stdout に出力します。対応するシェルは
`bash`、`zsh`、`fish`、`elvish`、`powershell` です。

```bash
vz completions bash
vz completions zsh
vz completions fish
vz completions elvish
vz completions powershell
```

生成されるスクリプトは、インストールされているバージョンのフラグ、チャート種別、
入出力形式、テーマ、サブコマンドを補完します。`vz` を更新したら再生成して同期し
てください。

## bash

現在のセッションだけ有効にする場合:

```bash
source <(vz completions bash)
```

永続的にインストールするには、bash-completion が読み込むファイルへ書き出します。
パスは環境によって異なります。例:

```bash
vz completions bash > ~/.local/share/bash-completion/completions/vz
```

新しいシェルを起動し、`vz --<Tab>` と入力して確認します。

## zsh

`$fpath` 上のディレクトリに `_vz` として書き出し、シェルを再起動します。

```bash
vz completions zsh > "${fpath[1]}/_vz"
```

そのディレクトリに書き込めない場合は、任意のディレクトリを選び、`~/.zshrc` で
`compinit` より前に `fpath` へ追加してください。

## fish

fish は `~/.config/fish/completions/` の補完ファイルを自動で読み込みます。

```bash
vz completions fish > ~/.config/fish/completions/vz.fish
```

## elvish

Elvish の設定にスクリプトを追記します。

```bash
vz completions elvish >> ~/.config/elvish/rc.elv
```

## PowerShell

現在のセッションへ補完スクリプトを読み込みます。

```powershell
vz completions powershell | Out-String | Invoke-Expression
```

永続化するには、同じ出力をプロファイルへ追記します。

```powershell
vz completions powershell | Out-String | Add-Content $PROFILE
```

## 確認

インストール後、コマンド名とフラグの一部を入力して補完キー（多くのシェルでは
`Tab`）を押します。たとえば:

```text
vz --theme <Tab>
```

`dark`、`light`、`high-contrast` が候補に出れば成功です。

## 次のステップ

- [はじめに](./getting-started.md) — インストールと最初の一歩。
- [チャート種別](./chart-types.md) — 補完されるフラグの説明。
- [出力モード](./output-modes.md) — `-o` や `--json` などの出力フラグ。
