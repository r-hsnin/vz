# Shell Completions

`vz completions SHELL` prints a completion script to stdout. Supported shells
are `bash`, `zsh`, `fish`, `elvish`, and `powershell`.

```bash
vz completions bash
vz completions zsh
vz completions fish
vz completions elvish
vz completions powershell
```

The generated script completes the flags, chart types, formats, themes, and
subcommands of your installed version. Keep it in sync by regenerating it after
upgrading `vz`.

## Bash

For the current session only:

```bash
source <(vz completions bash)
```

To install it persistently, write it to a file that your bash-completion setup
sources, for example:

```bash
vz completions bash > ~/.local/share/bash-completion/completions/vz
```

Start a new shell and type `vz --<Tab>` to verify.

## Zsh

Write the script into a directory on `$fpath` as `_vz`, then restart the shell:

```bash
vz completions zsh > "${fpath[1]}/_vz"
```

If that directory is not writable, choose one of your own and add it to
`fpath` before `compinit` runs in `~/.zshrc`.

## Fish

Fish loads completion files from `~/.config/fish/completions/` automatically:

```bash
vz completions fish > ~/.config/fish/completions/vz.fish
```

## Elvish

Append the script to your Elvish configuration:

```bash
vz completions elvish >> ~/.config/elvish/rc.elv
```

## PowerShell

Load the completion script into the current session:

```powershell
vz completions powershell | Out-String | Invoke-Expression
```

To persist it, append the same output to your profile:

```powershell
vz completions powershell | Out-String | Add-Content $PROFILE
```

## Verifying

After installation, type the command name and a partial flag, then press the
completion key (`Tab` in most shells). For example:

```text
vz --theme <Tab>
```

should offer `dark`, `light`, and `high-contrast`.

## Next steps

- [Getting Started](./getting-started.md) — install and first steps.
- [Chart Types](./chart-types.md) — flags you can complete.
- [Output Modes](./output-modes.md) — output flags such as `-o` and `--json`.
