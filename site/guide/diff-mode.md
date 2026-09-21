# Diff Mode

Diff mode compares two files with the same schema and highlights what changed.
Trigger it with two positional paths or one positional plus `--diff`:

```bash
vz before.csv after.csv
vz before.csv --diff after.csv
vz q1.csv q2.csv --sort desc          # largest increases first
vz q1.csv q2.csv --sort desc --top 5  # top 5 increases
vz q1.csv q2.csv -o spark
vz q1.csv q2.csv -o json
```

## Schema requirements

Both files must share a schema:

- The same column count.
- Every "before" column name must appear in "after" (case-insensitive,
  trimmed).

X is taken from `-x`, otherwise the first categorical or temporal column,
otherwise the first header. Y is taken from `-y`, otherwise the first
quantitative column other than X.

## Categorical X: change bars

A categorical X produces a bar chart of per-category change, annotated
`label ▲ +20%` / `▼ -10%` / `─ 0%`.

- `▲ new` / `▼ new` is shown only when the before value is effectively zero and
  the after value is non-zero.
- When both values are zero, the marker is `─ 0%`.
- JSON exposes new categories as `pct_change: null` with no `new` text, and
  both-zero rows as `pct_change: 0`.

## Temporal X: line overlay

A temporal X produces a two-series line overlay: before in gray, after in cyan.

## Sorting

Diff sorting uses the **signed delta**:

- `--sort desc` puts the largest increase first, not the largest absolute
  change.
- `--sort asc` puts the largest decrease first.
- `--sort none` keeps the input order.

Diff-explore's interactive sort is the exception: it uses `|Δ|`.

## Flags in diff mode

`--where`, `--agg`, and `--color` have no effect and warn. `-t`, `--labels`,
`--sample`, `--all-y`, and `--bins` are silently ignored. Other flags not in
the diff parameter set (`-I`/`--info`, `--watch`, and the directory-only
`--glob`, `-R`, `--catalog`, `--no-limit`) are likewise ignored. `--sample 0`
is not validated in diff mode.

## Output formats

Diff supports `text` (default), `spark`, `json`, `markdown`, and `html`.
`-o table` and `-o svg` are not supported and fall back to the default text
renderer.

```bash
vz before.csv after.csv --markdown
vz before.csv after.csv -o json | jq '.data[] | {name, pct_change}'
```

## Explore diff interactively

`vz explore FILE1 FILE2` opens diff-explore. It supports only sort, table
scrolling, the table toggle, and yank; axis, color, and aggregation keys report
"N/A in diff mode". See [Chart Types](./chart-types.md) for the shared
keybindings.

## Next steps

- [Chart Types](./chart-types.md) — how categories and axes are resolved.
- [Output Modes](./output-modes.md) — the JSON fields and other formats.
- [Getting Started](./getting-started.md) — install and basic usage.
