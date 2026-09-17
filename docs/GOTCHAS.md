# Gotchas — vz

Non-obvious behaviors and known failure modes, for users and operators.
Release procedures live in [RUNBOOK.md](RUNBOOK.md).

## Non-obvious Behaviors

- **No file argument shows help.** Always pass a file or `-` for stdin.
- **Bar chart aggregates by default (sum).** Use `--agg mean` if you want averages.
- **Filter values starting with `>`, `<`, `=`, `!` are rejected.** A doubled operator like `-w "revenue>>100"` fails loudly instead of silently matching nothing. Empty values (`-w "city="`) still match empty cells.
- **Column names are case-sensitive.** Check with `vz data.csv --info`. Typo'd names get a `Did you mean '...' ?` hint. Unknown `x`/`y`/`color` names are an error everywhere, including present chart blocks (previously silently charted the first columns).
- **TSV detection relies on extension or tab prevalence.** When piping, use `-f tsv` explicitly.
- **Large datasets (>100k rows):** Use `--sample N` to keep rendering fast.
- **JSON output includes only the first 100 rows in `data[]`.** The `chart_data` field contains the full aggregated result. JSON sets `"truncated": true` when capped.
- **NaN/inf are skipped in aggregations, diffs, and every chart path** (line/scatter/histogram/spark/JSON series), and ignored in the type-inference vote like nulls. `--agg max/min` never emits `±inf`; text-only diffs report no entries instead of `0→0`. JSON `data[]` renders them as `null`, while `chart_data` series omit those points.
- **Diff mode ignores `--where`/`--agg`/`--color`.** A `no effect in diff mode` warning is printed; filter before comparing instead.
- **Explore/Present require an interactive terminal.** In CI or pipes, use one-shot mode.

## Build Failures

| Symptom | Cause | Fix |
|---------|-------|-----|
| `error[E0658]: let chains` | Rust version too old | Update: `rustup update` (requires 1.88+) |
| `crossterm` compile error | Missing system deps | Linux: ensure `libxcb` or similar available |
| `ratatui` version mismatch | Lockfile stale | `cargo update` |

## Development Setup

- **Git hooks not running:** if `core.hooksPath` points to a custom path (legacy `scripts/hooks` setup), Git ignores lefthook's hooks. Run `lefthook install --reset-hooks-path` once.
- **pre-push jobs don't receive git args automatically:** pass them explicitly via `{1}` in `run:` (see `lefthook.yml`). Hooks reading the ref list from stdin also need `use_stdin: true`; without it lefthook can hang.
- **Pinned toolchain:** `rust-toolchain.toml` forces 1.97.0 for all cargo/rustc commands in this repo. Check MSRV with `cargo +1.88.0 check --locked`; `rustup update` does not move the pinned channel.

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
