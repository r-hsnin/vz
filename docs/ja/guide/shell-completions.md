# シェル補完

vz はお使いのシェル用のタブ補完スクリプトを生成できます。設定すると、`Tab` キーでサブコマンド、フラグ、オプションが自動補完されます。

## 補完スクリプトの生成

```bash
vz completions <SHELL>
```

対応シェル: `bash`, `zsh`, `fish`, `elvish`, `powershell`

## シェル別セットアップ

### Bash

`~/.bashrc` に追加:

```bash
eval "$(vz completions bash)"
```

シェル起動を高速化するにはファイルに保存:

```bash
vz completions bash > ~/.local/share/bash-completion/completions/vz
```

### Zsh

`~/.zshrc` に追加:

```zsh
eval "$(vz completions zsh)"
```

または補完ディレクトリに保存:

```zsh
vz completions zsh > "${fpath[1]}/_vz"
```

::: tip
補完ディレクトリが存在しない場合は作成して `fpath` に追加:
```zsh
mkdir -p ~/.zfunc
echo 'fpath=(~/.zfunc $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc
vz completions zsh > ~/.zfunc/_vz
```
:::

### Fish

```fish
vz completions fish > ~/.config/fish/completions/vz.fish
```

Fish は補完ファイルを自動で検出します。追加設定は不要です。

### Elvish

```elvish
vz completions elvish > ~/.config/elvish/lib/vz.elv
```

`rc.elv` に追加:

```elvish
use vz
```

### PowerShell

PowerShell プロファイル (`$PROFILE`) に追加:

```powershell
vz completions powershell | Out-String | Invoke-Expression
```

ファイルに保存してドットソースする方法:

```powershell
vz completions powershell > "$HOME\.config\vz-completions.ps1"
# $PROFILE に追加:
. "$HOME\.config\vz-completions.ps1"
```

## 補完される内容

セットアップ後、以下がタブ補完されます:

- サブコマンド (`explore`, `present`, `completions`)
- すべてのフラグとオプション (`--sort`, `--output`, `--type` など)
- フラグの値 (`--output` → `text`, `json`, `svg`, `html`, ...)
- 位置引数のファイルパス

## 補完の更新

補完スクリプトは現在のバイナリから生成されます。vz をアップグレードした後は、新しいフラグやサブコマンドを反映するためにスクリプトを再生成してください:

```bash
# Bash の例
vz completions bash > ~/.local/share/bash-completion/completions/vz
```
