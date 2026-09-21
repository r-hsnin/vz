# Output Modes

`vz` writes charts and machine-readable formats to stdout, and keeps the
summary, insights, and warnings on stderr. JSON is the one place where errors
also land on stdout, so always check the exit code (success `0`, failure `1`).

| Format | Flag | Notes |
|--------|------|-------|
| text | default | Chart on stdout, summary/insights on stderr |
| json | `-o json` / `--json` | Full structured object; see below |
| table | `-o table` | Bars as two aggregated columns; other charts emit all columns, capped at 100 rows |
| markdown | `-o markdown` / `--markdown` | GFM table, same bar/non-bar split and 100-row cap |
| spark | `-o spark` / `--spark` | One sparkline per series/group |
| svg | `-o svg` / `--svg` | Text-grid SVG with invisible data marks |
| html | `-o html` / `--html` | Self-contained HTML with hover tooltips |

## Text (default)

The default output writes three things:

1. A summary line on stderr, for example:

   `Line │ x=date │ y=revenue (100–500) ▁▃▅▇ │ ↑ +50% │ color=city [Tokyo=cyan, Osaka=yellow] │ 6 rows`

2. The chart (title, axis ticks, legend) on stdout.
3. 0–3 plain-language takeaways on stderr (growth, leader, clusters); they are
   silent when there are fewer than 2 points.

Summary components include the chart type and axes, the Y range with an inline
sparkline plus trend (`↑ +N%` / `↓ -N%` / `→ stable` within ±5%), non-sum
aggregation labels such as `y=mean(revenue)`, the color legend, the row count
with skips (`6 rows (2 skipped)`), and an unused-columns hint.

## Table and Markdown

`-o table` and `-o markdown` export the data as a table instead of a chart. Bar
charts render as two aggregated columns; other chart types emit all columns.
Both formats cap the output at 100 rows.

```bash
vz sales.csv -o table
vz sales.csv --markdown
```

## Spark

Sparkline output is one line per series, with the range and trend:

```
revenue  ▁▂▃▅▇  (100–500) ↑ +400%
```

```bash
vz sales.csv -y latency --spark
```

Bars show only the range; non-finite values are skipped with an `(N skipped)` suffix.

## SVG and HTML

`-o svg` emits a text-grid SVG plus invisible `circle.vz-point` marks carrying
`data-label`, `data-value`, and `data-series`. `-o html` wraps the SVG in a
self-contained HTML page with JavaScript hover tooltips.

```bash
vz sales.csv --svg > chart.svg
vz sales.csv --html > chart.html
```

## JSON

`-o json` (or `--json`) emits a structured object on stdout:

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

Value rules:

- `data[]` is capped at 100 rows; use `chart_data`/`query` for the full aggregate.
- Values are numeric-parsed: `$100` becomes `100`, `45%` becomes `0.45`.
- Non-finite values (`NaN`/`inf`) become `null` in `data[]` and are omitted from chart series.
- Diff rows use `pct_change`: `null` for new categories, `0` when both sides are zero.

With `--info -o json`, only the info object is emitted: `version`, `file`,
`rows`, `columns[]`, `recommendation` (when one can be produced), `data[]`
(first 100 rows), and `truncated`; chart fields such as `chart_data` are absent:

```bash
vz sales.csv --info -o json | jq '.columns[] | {name, type, stats}'
```

When the resolved format is JSON, errors are printed as a pretty JSON object
(`{ "version": 1, "error": "..." }`) on stdout. Check the exit code before
trusting stdout.

## Color and width

- ANSI color is emitted only when stdout is a TTY. Any non-empty `NO_COLOR` disables it; any non-empty `FORCE_COLOR` forces it, for both the stdout chart and the stderr summary.
- Piped stdout forces a chart width of 80, ignoring `COLUMNS`. When stderr is piped, the summary line uses 120 columns instead of the terminal width.
- `--theme dark|light|high-contrast` selects the palette and is also honored by explore and present.

## Present mode

`vz present FILE` renders a Markdown file as terminal slides (requires a TTY).
Slides are separated by `---`; `# ` sets the slide title, and headings, lists, blockquotes, GFM tables, and fenced code blocks are rendered. A fenced `chart` block embeds a chart:

````markdown
```chart
source: sales.csv
x: month
y: revenue
type: line
```
````

| Parameter | Description |
|-----------|-------------|
| `source` | Data file path (required) |
| `x` | X axis column |
| `y` | Y axis column |
| `color` | Color/group-by column |
| `title` | Chart title |
| `type` | `line`, `bar`, `scatter`, `histogram`, `heatmap` |
| `where` | Filter rows (repeatable): `where: revenue>1000` |
| `sort` | Bar sorting: `desc` or `asc` |
| `agg` | `sum`, `mean`, `count`, `max`, `min` |
| `top` | Show only the top N categories |
| `bins` | Histogram bin count |
| `height` | Chart height in rows |
| `diff` | "After" file for a diff chart; `source` becomes "before" |

Unknown keys are ignored. Invalid values for `type`/`sort`/`agg`/`top`/`bins`/`height` warn and fall back to auto behavior. Chart `source` (and `diff`) paths resolve relative to the Markdown file first, then the current working directory.

Navigation: `→` / `l` / `Space` next, `←` / `h` / `Backspace` previous, `Enter` next (or jump when digits are pending), `g` / `G` / `Home` / `End` first/last, digits + `Enter` jump to a 1-based slide, `q` / `Esc` quit.

## Next steps

- [Chart Types](./chart-types.md) — selection rules and bar options.
- [Diff Mode](./diff-mode.md) — the diff output formats and annotations.
- [Shell Completions](./shell-completions.md) — complete flags in your shell.
