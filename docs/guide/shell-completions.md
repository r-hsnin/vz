# Shell Completions

vz can generate tab-completion scripts for your shell. Once installed, pressing `Tab` will auto-complete subcommands, flags, and options.

## Generating Completions

```bash
vz completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`

## Setup by Shell

### Bash

Add to your `~/.bashrc`:

```bash
eval "$(vz completions bash)"
```

Or generate a file for faster shell startup:

```bash
vz completions bash > ~/.local/share/bash-completion/completions/vz
```

### Zsh

Add to your `~/.zshrc`:

```zsh
eval "$(vz completions zsh)"
```

Or save to your completions directory:

```zsh
vz completions zsh > "${fpath[1]}/_vz"
```

::: tip
If the completions directory doesn't exist yet, create it and add it to `fpath`:
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

Fish picks up completion files automatically — no extra configuration needed.

### Elvish

```elvish
vz completions elvish > ~/.config/elvish/lib/vz.elv
```

Then add to your `rc.elv`:

```elvish
use vz
```

### PowerShell

Add to your PowerShell profile (`$PROFILE`):

```powershell
vz completions powershell | Out-String | Invoke-Expression
```

Or save to a file and dot-source it:

```powershell
vz completions powershell > "$HOME\.config\vz-completions.ps1"
# Add to $PROFILE:
. "$HOME\.config\vz-completions.ps1"
```

## What Gets Completed

Once set up, tab completion covers:

- Subcommands (`explore`, `present`, `completions`)
- All flags and options (`--sort`, `--output`, `--type`, etc.)
- Flag values where applicable (`--output` → `text`, `json`, `svg`, `html`, ...)
- File paths for positional arguments

## Updating Completions

Completions are generated from the current binary. After upgrading vz, regenerate the completion script to pick up new flags and subcommands:

```bash
# Example for bash
vz completions bash > ~/.local/share/bash-completion/completions/vz
```
