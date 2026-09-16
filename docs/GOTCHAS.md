# Gotchas — vz

Non-obvious behaviors and known failure modes, for users and operators.
Release procedures live in [RUNBOOK.md](RUNBOOK.md).

## Non-obvious Behaviors

- **No file argument shows help.** Always pass a file or `-` for stdin.
- **Bar chart aggregates by default (sum).** Use `--agg mean` if you want averages.
- **Column names are case-sensitive.** Check with `vz data.csv --info`.
- **TSV detection relies on extension or tab prevalence.** When piping, use `-f tsv` explicitly.
- **Large datasets (>100k rows):** Use `--sample N` to keep rendering fast.
- **JSON output includes only the first 100 rows in `data[]`.** The `chart_data` field contains the full aggregated result.
- **Explore/Present require an interactive terminal.** In CI or pipes, use one-shot mode.

## Build Failures

| Symptom | Cause | Fix |
|---------|-------|-----|
| `error[E0658]: let chains` | Rust version too old | Update: `rustup update` (requires 1.87+) |
| `crossterm` compile error | Missing system deps | Linux: ensure `libxcb` or similar available |
| `ratatui` version mismatch | Lockfile stale | `cargo update` |

## Runtime Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| `No input file specified` | Missing positional arg | Provide file or pipe: `vz data.csv` or `cat f.csv \| vz -` |
| `Could not determine chart type` | Invalid column hint | Check `-x`/`-y` column names match CSV headers |
| `Failed to read file` | File not found or permission | Verify path and permissions |
| Chart renders garbled | Terminal doesn't support Unicode | Try a terminal with Braille/Unicode support (iTerm2, kitty, WezTerm) |
| Explore/Present panics | No TTY available | These modes require an interactive terminal; use one-shot mode in CI/pipes |
| Bar chart shows no labels | Terminal too narrow | Widen terminal to ≥ 40 columns |

## TSV Not Detected

If a TSV file isn't auto-detected:
- Ensure the file extension is `.tsv` or `.tab`, **or**
- Ensure the header line contains more tab characters than commas, **or**
- Use the `--format tsv` (or `-f tsv`) flag to force TSV parsing

## Present Mode Chart Not Loading

Chart source paths resolve relative to the Markdown file's directory. If charts don't render:
1. Ensure the `source:` path in the chart block is relative to where the `.md` file lives
2. As a fallback, the tool also tries the current working directory
