# vz output and JSON contract

How `vz` splits stdout/stderr, what each output format emits, and the shape of
`-o json`. Load this when scripting against `vz` or parsing its output.

## Contents

- Streams and exit codes
- Error shape
- Output formats
- JSON schema
- Color and width environment

## Streams and exit codes

- Charts and machine formats (json, table, markdown, spark, svg, html, info) go
  to **stdout**.
- The summary line, `💡` insights, and all warnings go to **stderr**. Do not
  parse stderr as data.
- Success exits `0`; failure exits `1` with no chart. Always check the exit
  code — stdout parsing alone is not a success test.

The default text output writes three things:

1. A summary line on stderr, for example:
   `Line │ x=date │ y=revenue (100–500) ▁▃▅▇ │ ↑ +50% │ color=city [Tokyo=cyan, Osaka=yellow] │ 6 rows`
2. The chart (title, axis ticks, legend) on stdout.
3. 0–3 `💡` plain-language takeaways on stderr; silent when fewer than 2 points.

Summary components include the chart type and axes, the Y range with an inline
sparkline plus trend (`↑ +N%` / `↓ -N%` / `→ stable` within ±5%), non-sum
aggregation labels such as `y=mean(revenue)`, the color legend, the row count
with skips (`6 rows (2 skipped)`), and an unused-columns hint.

## Error shape

Normally errors go to stderr. The exception: when the resolved format is JSON
(`-o json`, `--json`, or `--info -o json`), the error is a pretty JSON object on
**stdout**:

```json
{ "version": 1, "error": "..." }
```

Always check the exit code, because a JSON consumer may see a parseable object
on stdout even when the command failed.

## Output formats

| Format | Flag | Notes |
|--------|------|-------|
| text | default | Chart on stdout, summary/insights on stderr |
| json | `-o json` / `--json` | Full structured object; see below |
| table | `-o table` | Bars as two aggregated columns; other charts emit all columns, capped at 100 rows |
| markdown | `-o markdown` / `--markdown` | GFM table, same bar/non-bar split and 100-row cap |
| spark | `-o spark` / `--spark` | One sparkline per series/group; bars show range only; non-finite values skipped with `(N skipped)` |
| svg | `-o svg` / `--svg` | Text-grid SVG plus invisible `circle.vz-point` marks with `data-label`/`data-value`/`data-series` |
| html | `-o html` / `--html` | Self-contained HTML wrapping the SVG with JavaScript hover tooltips |

Spark output is one line per series:

```
revenue  ▁▂▃▅▇  (100–500) ↑ +400%
```

## JSON schema

Top-level fields:

- `version` — schema version (currently `1`).
- `file` — input path.
- `rows` — row count.
- `columns[]` — `{ name, type, nulls, stats }`.
- `recommendation` — optional `{ chart_type, x, y, color }`.
- `data[]` — first 100 rows as objects.
- `truncated` — whether `data[]` was cut.
- `chart_data` — type-specific aggregate (charts only).
- `query` — `{ chart_type, x, y, extra_y, color, agg, sort, limit, bins, filters, sample }`.
- `insights[]` — plain-language takeaways.

With `--info -o json` only the info object (`version`, `file`, `rows`,
`columns[]`) is emitted, with no `data[]`/chart fields.

Value rules:

- `data[]` is capped at **100 rows**; use `chart_data`/`query` for the full
  aggregate instead of relying on `data[]`.
- Values are numeric-parsed (`$100` → `100`, `45%` → `0.45`).
- Non-finite values (`NaN`/`inf`) become `null` in `data[]` and are omitted from
  chart series.
- Diff rows use `pct_change`, which is `null` for new categories and `0` when
  both sides are zero.

## Color and width environment

- ANSI color is emitted only when stdout is a TTY. `NO_COLOR` (any non-empty
  value) disables it; `FORCE_COLOR` (any non-empty value) forces it. This
  applies to both the stdout chart and the stderr summary/insights.
- Piped stdout forces chart width to **80**, ignoring `COLUMNS`. When stderr is
  piped the summary line gets 120 columns instead of the terminal width.
