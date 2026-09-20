# Gotchas — vz

Non-obvious behaviors and known failure modes, for users and operators.
Release procedures live in [RUNBOOK.md](RUNBOOK.md).

## Non-obvious Behaviors

- **No file argument shows help.** Always pass a file or `-` for stdin.
- **Bar chart aggregates by default (sum).** Use `--agg mean` if you want averages. A lone categorical `-x city` (no numeric columns) counts rows per category (`y=count(city)`).
- **Reversed `-x`/`-y` pairs are normalized to the canonical orientation.** `-x revenue -y date` draws `x=date` (Line); `-x revenue -y city` draws `x=city` (Bar) — the literal order no longer renders an empty chart.
- **Type inference samples 100 evenly spaced rows (head→tail), not just the first 100.** Row order no longer flips the inferred type.
- **`-c/--color` has no effect on bar chart data.** Grouped/stacked bars are not implemented: bars stay aggregated over all rows and an explicit `-c` only reaches the summary legend (oneshot warns; explore reports it in the status line when cycling color on a Bar). An auto-detected color column (any unused categorical column) also reaches the legend, but no longer warns; pass `-x`/`-y` explicitly to keep the legend from being inferred.
- **Filter values starting with `>`, `<`, `=`, `!` are rejected.** A doubled operator like `-w "revenue>>100"` fails loudly instead of silently matching nothing. Empty values (`-w "city="`) still match empty cells.
- **Column names are case-sensitive.** Check with `vz data.csv --info`. Typo'd names get a `Did you mean '...' ?` hint. Unknown `x`/`y`/`color` names are an error everywhere, including present chart blocks (previously silently charted the first columns). The 2nd+ `-y` columns are validated too (`-y revenue,revnue` fails instead of rendering a single series silently). Present chart blocks warn on unknown `type`/`sort`/`agg` and invalid or out-of-range `top`/`bins`/`height`, then fall back to auto-infer.
- **Out-of-range `--bins`/`--top`/`--tail` fail in every mode.** `--bins` is validated as `1`–`10000` and `--top`/`--tail` as `≥ 1` before any mode runs (single-file, watch, directory, and diff), even when the chart type ignores the flag.
- **TSV detection relies on extension or tab prevalence.** When piping, use `-f tsv` explicitly.
- **Large datasets (>100k rows):** Use `--sample N` to keep rendering fast.
- **JSON output includes only the first 100 rows in `data[]`.** The `chart_data` field contains the full aggregated result. JSON sets `"truncated": true` when capped.
- **Table/Markdown output is capped at 100 rows.** Non-bar tables show the first 100 rows with an `info: showing 100/N rows` notice on stderr (`--top`/`--sort` only apply to bar aggregations; other chart types warn instead of silently ignoring). Bar tables show aggregated values with `mean(...)`/`count(...)` headers for non-sum aggs, and numbers use the same `4.2k`/`1.5M` format as charts. Markdown cells escape `|` and newlines. Spark bar output shows range only (no trend arrow — category order is not time). Line/scatter/spark trend arrows use the absolute start value as denominator: `-100 → -50` reports `↑ +50%` (improvement), not `↓`.
- **JSON output records the resolved query.** In addition to `recommendation` (auto-inferred), `-o json` includes a `query` object with the actually-rendered `chart_type` (`-t` applied), `agg`, `sort`, `limit`, `extra_y`, `filters`, `sample`, and `bins`, so agents can reproduce the numbers in `chart_data`. Single-file and diff JSON both carry an `insights` array with the same plain-language takeaways printed as `💡` on stderr.
- **Every chart prints plain-language takeaways.** 0–3 `💡` lines on stderr after the summary: overall move (same ±5% stable band as the trend arrow), extremes + biggest move (line/scatter), leader + share and runner-up gap (bar, same `agg` as the chart), densest bin + range (histogram), hottest cell (heatmap). With `-c`, line charts report the fastest-moving group instead of a cross-group jump. Silent when there is nothing to say (<2 points, no parseable values). Breaking change: stderr gains up to 3 lines; stdout charts are unchanged.
- **Any Bar fallback without a chart rule warns.** Beyond `Nominal` pairs, uncovered pairs (e.g. Temporal × Temporal) also emit `warning: no chart rule for ... falling back to bar` in oneshot/present; the explorer stays silent.
- **Numbers parse liberally everywhere (`util::parse_number`).** `1,000`, `$100`, `45%` (= 0.45), `10k`, `10GiB`, `(42)` (= -42) are Numeric on all paths — inference, bar aggregation, line/scatter series, histogram bins, `--where` comparisons, diff, sparkline, summary ranges, and JSON `data[]`/stats. Breaking change: `$100`/`45%` columns that used to infer as Text now infer as Numeric (Bar instead of Heatmap, fraction values for percents).
- **Filter equality is numeric-aware.** `-w "revenue=2000"` matches `$2,000`/`2k`/quoted cells, and `-w "rate=0.45"` matches `45%`. Breaking change: formatted cells that used to require exact raw-string equality now compare by parsed value. Text cells keep exact string semantics.
- **SVG data marks carry every series.** Line/scatter overlays emit one `<circle class="vz-point" data-series="…">` per vertex of every series (not just the first), positioned on the union axis span; HTML tooltips prefix the series name (`series — label: value`). Bar marks use the full min→max span so negative values sit below positives. Breaking change: snapshots with `-c` overlays now show more marks at shifted coordinates. Known approximation: tooltip labels come from the shared `x_labels` by per-series index, so overlaid series with missing x values can show the wrong label while the dot position stays correct.
- **NaN/inf are skipped in aggregations, diffs, and every chart path** (line/scatter/histogram/spark/JSON series), and ignored in the type-inference vote like nulls. `--agg max/min` never emits `±inf`; text-only diffs report no entries instead of `0→0`. JSON `data[]` renders them as `null`, while `chart_data` series omit those points.
- **Diff mode ignores `--where`/`--agg`/`--color`.** A `no effect in diff mode` warning is printed; filter before comparing instead.
- **New diff entries show `▲ new` / `▼ new`, not a percentage.** When a category or point has no value on the before side, the percentage delta is undefined, so charts and tables use the `new` marker instead of an absolute delta like `▲ +800`.
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
- **MSRV check:** use the explicit toolchain — `cargo +1.88.0 check --locked` (what CI's msrv job runs). Local commands may resolve a newer default channel, so a bare `cargo check` does not validate MSRV.

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
