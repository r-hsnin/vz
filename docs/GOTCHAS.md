# Gotchas — vz

Non-obvious user-visible behavior, footguns, and known limitations. Full flag
reference lives in [README.md](../README.md); design rationale in
[DESIGN.md](DESIGN.md).

## Input & parsing

- **No file argument does *not* show help.** With a TTY stdin `vz` errors
  `No input file specified. Usage: vz <file> or pipe data to stdin`; with piped
  stdin it reads stdin instead. Pass a file, `-` for stdin, or pipe data.
- **Piped stdin has three quiet fixes.** Literal `\n`/`\t` escapes are expanded
  when the content has ≤1 real newline and ≥1 literal `\n` (shells that do not
  expand `echo`); a leading UTF-8 BOM is stripped; and a first row whose cells
  are *all* numeric is treated as headerless data, yielding `col1…colN`.
  `printf '1,2\n3,4\n' | vz -` infers `col1`,`col2`.
- **TSV needs extension or tab prevalence.** Format is auto-detected, but a
  piped one-column TSV is CSV. Force it: `vz - -f tsv`.

## Data & inference

- **Inference samples 100 evenly spaced rows, not the first 100.** The sample
  spans head→tail, so reordering the file does not flip the inferred type
  (`infer_from_data`). A 100-dates-then-garbage file infers the same backwards.
- **Type vote ignores nulls and non-finite, needs a floor(80%) majority (min 1).**
  ≤20 unique values ⇒ Categorical, more ⇒ Nominal; `NaN`/`inf` never vote
  Quantitative.
- **Numeric parsing is liberal and shared everywhere.** `1,000`, `$100`,
  `USD 100`, `45%`→`0.45`, `10k`, `10GiB`(binary) vs `10GB`(decimal), `(42)`→-42
  all parse as numbers in inference, aggregation, filters, diff, spark, JSON,
  and stats. `45%`/`$100` columns infer Quantitative, not Text.
- **`NaN`/`inf` are skipped downstream.** They infer as Nominal and are dropped
  from every chart/aggregate; JSON `data[]` shows `null`, and JSON chart series
  omit those points.
- **Column names are case-sensitive.** Unknown `-x`/`-y`/`-c` names error with a
  `Did you mean '...'?` hint; the 2nd+ `-y` columns are validated too,
  `-y revenue,revnue` fails instead of silently rendering one series.

## Filters

- **Values starting with `>`,`<`,`=`,`!` are rejected.** `-w "revenue>>100"` or
  `-w "city=!Tokyo"` fails loudly instead of matching nothing.
- **Empty values are legal; multiple `--where` are ANDed.** `-w "city="` matches
  empty cells, `-w "note=a=b"` keeps the embedded `=`, and
  `-w city=Tokyo -w revenue>800` requires both.
- **Equality is numeric-aware.** `-w "revenue=2000"` matches `$2,000`/`2k`, and
  `-w "rate=0.45"` matches `45%`; text cells keep exact string semantics.

## Charts & axes

- **Bar aggregates by default (sum).** Use `--agg mean` for averages. Count is
  auto-applied when `-t bar` has no explicit Y and Y is non-quantitative, or
  when a lone categorical `-x city` has zero quantitative columns
  (`x=city`, `y=count(city)`).
- **Reversed `-x`/`-y` pairs normalize to canonical orientation.** `-x revenue
  -y date` renders `x=date` (Line) and `-x revenue -y city` renders `x=city`
  (Bar), instead of a literal-order chart with everything skipped.
- **`-c/--color` never splits bar data.** Grouped/stacked bars do not exist;
  the column only reaches the summary legend and heights stay aggregated over
  all rows. An explicit `-c` warns; an auto-detected color does not.
- **Rule-less bar fallbacks warn.** Nominal pairs or Temporal×Temporal emit
  `warning: no chart rule for ... falling back to bar` (oneshot/present; the
  explore TUI stays silent).

## Modes & validation

- **`--bins`/`--top`/`--tail` are range-checked in every mode path.**
  `--bins` must be `1`–`10000` and `--top`/`--tail` `≥1`, validated before any
  mode branches (single-file, watch, directory, diff) even when the chart type
  ignores the flag. `--sample 0` is checked only in `render_data`, so diff
  ignores it.
- **`--watch` rejects stdin and missing files.** It watches the *parent*
  directory (non-recursively), ANSI-clears the screen before each pass, and
  keeps looping after a re-render error.
- **Explore and Present require an interactive terminal.** They error in pipes
  and CI; explore additionally treats the `VZ_TEST_HEADLESS` env var as a no-op
  test seam.

## Diff mode

- **`--sort` is signed-Δ, not absolute.** `--sort desc` puts the largest
  *increase* first; diff-explore's interactive sort is the exception (it uses `|Δ|`).
- **New categories show `▲ new`/`▼ new`, not a percentage.** That marker is
  used when before≈0 and after≠0; both-zero shows `─ 0%`, and JSON exposes
  `pct_change: null` for the new case (no marker in JSON).
- **Diff ignores many flags.** `--where`/`--agg`/`--color` warn and are ignored;
  `-t`/`--labels`/`--sample`/`--all-y`/`--bins` are silently ignored (absent
  from `DiffParams` or unused). `-o table` and `-o svg` fall through to the
  default text renderer — diff has no table/svg format.

## Directory mode

- **Column order follows the first file, not the incoming file.** Files whose
  headers match case-insensitively (trimmed) are reordered to the first file's
  order and merged; only different column sets are skipped.
- **`_file_date` recognizes three patterns only.** `YYYY-MM-DD`, `YYYY_MM_DD`,
  and `YYYYMMDD` with years 1900–2099; anything else yields an empty string.

## Present mode

- **Chart source paths resolve against the markdown file first, then CWD.**
  A `source: data.csv` in `slides/demo.md` loads `slides/data.csv` if it exists,
  else `./data.csv`.
- **Bad chart-block values warn and fall back.** Unknown `type`/`sort`/`agg` and
  invalid `top`/`height` fall back to auto-infer; `bins` outside `1–10000` warns
  and is ignored. Unknown keys are ignored silently.

## Output & formatting

- **Piped stdout forces width 80, ignoring `COLUMNS`.** The chart width is 80
  whenever stdout is not a TTY; when stderr is piped the summary line gets 120
  columns instead of the terminal width.
- **Color obeys `NO_COLOR`/`FORCE_COLOR` then TTY detection.** Any non-empty
  `NO_COLOR` disables ANSI, any non-empty `FORCE_COLOR` enables it for both the
  stdout chart and the stderr summary.
- **`-H/--height` 24 is the cap and fallback, not a universal default.** Bar and
  heatmap shrink to `unique×4+2` (clamped 10–24) for ≤5 categories, line/scatter
  to `rows×3+6` (clamped 12–24) for ≤6 rows.
- **Insights can silently produce nothing.** They are computed on every path but
  emit zero lines when there are <2 points, no parseable values, or unknown
  columns — absence is not an error.

## Known bug

- **SVG/HTML tooltip labels can be wrong for overlaid series.** Marks are placed
  correctly, but labels come from the shared `x_labels` by per-series index, so a
  series missing an X value can show its neighbor's label.
