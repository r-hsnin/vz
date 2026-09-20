# vz — Design Document

Rationale, intent, and the decisions behind vz. Structural facts (module map, data flow,
change impact) live in [ARCHITECTURE.md](ARCHITECTURE.md). CLI reference is in
[README.md](../README.md), development commands in [CONTRIBUTING.md](../CONTRIBUTING.md),
known pitfalls in [GOTCHAS.md](GOTCHAS.md), and release/publication procedure in
[RUNBOOK.md](RUNBOOK.md). Where prose and code disagree, code wins.

## What This Document Owns

- **DESIGN (this file) — why.** Product intent, scope, inference and chart-selection
  rationale, the canonical-assembler contract, and architecture-level decisions with
  rejected alternatives.
- **ARCHITECTURE — what/how.** Modules, planes, dependency rules, data flow, and the
  change-impact map.

A statement about code layout belongs in ARCHITECTURE. A statement about why a behavior
exists belongs here.

## Philosophy

1. **Convention over configuration.** The inferred column types decide the chart. The
   common case (`vz data.csv`) needs no flags; every override (axes, type, aggregation,
   filtering, theme) is opt-in, and a file path alone is a complete command.
2. **Plain language is a first-class output.** Each chart may carry 0–3 takeaway
   sentences computed from the same parsed numbers as the chart, so a reader gets the
   point without reading axes. They are silent when there is nothing to say.
3. **Terminal-first, no external engine.** Charts render in the terminal; SVG/HTML/JSON
   exist so agents and reports can consume the same chart data headlessly. There is no
   database or query engine behind vz.
4. **Determinism over cleverness.** The same input yields the same type, chart, and
   numbers on every path. Shared parsers and a single chart-data assembler enforce it.

## Scope

### In scope

- File and stdin batch visualization: CSV, TSV, JSON array, NDJSON, fixed-width.
- Automatic column-type inference and chart selection.
- Output modes: one-shot stdout, interactive explore (and diff-explore), markdown
  present, two-file diff, directory combine + schema catalog.
- Exports: text, JSON, table, markdown, sparkline, SVG, HTML.
- Filtering (`--where`), aggregation, sampling, sorting, themes.
- `--info` schema inspection, `--watch` auto-redraw, and shell completions.

### Non-goals

- Database connections and analytical query engines (Parquet, SQLite, PostgreSQL,
  Polars, DuckDB).
- Raster/PNG export.
- Streaming or real-time data beyond `--watch`.
- Data transformation/ETL, joins, or reshaping.
- Custom color palettes beyond the built-in themes.
- Release and publication mechanics (see [RUNBOOK.md](RUNBOOK.md)).

## Type System and Inference Rationale

Every column is one of four types, defined in `infer/types.rs`:

| Type | Meaning | Role in chart selection |
|---|---|---|
| `Temporal` | Date/time values | Ordered X axis (trends) |
| `Quantitative` | Numeric magnitude | Measured Y axis, histograms |
| `Categorical` | Small set of labels | Grouping column or discrete X axis |
| `Nominal` | High-cardinality or non-numeric identity | Not charted as a category by default |

Inference has three stages, implemented in `infer/detector.rs` and driven by
`pipeline::infer_from_data`:

1. **Per-value classification.** Empty values are nulls. Temporal patterns are tested
   before numeric ones. A value that the shared numeric parser
   (`util::parse_number`) accepts is `Quantitative`; this one parser is reused by
   aggregation, filters, diff, sparklines, and JSON samples, so a number means the same
   thing on every path. Everything else is `Nominal`. Non-finite values (`NaN`, `inf`)
   are `Nominal` and are excluded from the vote because downstream paths skip them.
2. **Column vote.** Over the sampled non-empty, finite values, a column is `Temporal`
   if at least 80% vote temporal, then `Quantitative` if at least 80% vote numeric
   (a minimum of one vote). Empty and non-finite values abstain.
3. **Cardinality fallback.** Otherwise, at most 20 distinct values ⇒ `Categorical`;
   more ⇒ `Nominal`.

Design rationale:

- **The ≥80% vote is robust, not strict.** A mostly-numeric column with a few `N/A` or
  typo rows still infers as numeric; a mostly-text column is not flipped by a handful of
  numeric cells. Requiring unanimity would let one bad cell change the chart.
- **The 20-unique boundary separates labels from identifiers.** A category axis or
  legend is only readable for a small number of values. A high-cardinality column is
  an identifier (UUID, ID), and auto-charting it as a category would produce an
  unreadable axis, so it becomes `Nominal` instead.
- **Sampling is 100 evenly spaced rows covering head to tail**, not the first 100. This
  makes inference **order-independent**: a file with 100 dates followed by garbage
  infers the same as the reversed file, and a reordered export does not change the
  chart. There is no full-scan fallback; the fixed, bounded sample keeps inference cost
  constant as files grow.
- **The same parsed numbers drive every path.** Because inference, aggregation,
  filtering, and statistics share `parse_number`, a value classified `Quantitative`
  also aggregates and compares as the same number.

## Chart Selection Rationale

The type pair determines the chart (`chart/selector.rs`). The **normative** mapping
lives in [README.md#chart-selection-rules](../README.md#chart-selection-rules); this
section explains the rationale, not a second spec:

| X type | Y type | Chart | Why |
|---|---|---|---|
| `Temporal` | `Quantitative` | Line | Ordered X expresses a trend over time |
| `Categorical` | `Quantitative` | Bar | Compare magnitudes across discrete groups |
| `Quantitative` | `Quantitative` | Scatter | Relationship between two measures |
| `Categorical` | `Categorical` | Heatmap | Count matrix over two dimensions |
| `Quantitative` | `Temporal` | Line (normalized) | Same pair, reversed input |
| `Quantitative` | `Categorical` | Bar (normalized) | Same pair, reversed input |
| anything else | — | Bar (fallback, warns) | No dedicated rule |

Selection order, and why each step precedes the next:

1. **Both axes given.** Validate both, pick the chart from the pair, and normalize
   reversed pairs so `Quantitative × Temporal` and `Quantitative × Categorical` become
   the canonical `x = temporal`/`x = categorical`. What matters is the *semantics* of
   the pair, not which flag the user happened to type on the left; without
   normalization `-x revenue -y date` would plot dates on Y and render empty. The color
   column is the first categorical not already used as X or Y.
2. **Only Y given.** Prefer the first temporal column as X, then the first
   categorical, then another quantitative; the chart type always follows
   `chart_type_for_pair(x, y)` rather than being implied by the preference order. If Y
   is the only column, a lone temporal Y is caught by the temporal branch and yields
   `chart_type_for_pair(Temporal, Temporal)` → Bar with a fallback warning; a
   quantitative or nominal Y renders a Histogram, while a lone categorical Y becomes a
   Heatmap with `x = y = Y`.
3. **Only X given.** If another quantitative column exists, use the pair rule. With no
   quantitative Y, a categorical X renders a **Bar of row counts** (the automatic
   application of Count over `x = y`), and a quantitative X renders a Histogram.
4. **No hints.** Apply the table in priority order: temporal × quantitative → Line
   (color = first categorical); categorical × quantitative → Bar (no auto color, so
   bars stay a single aggregate); two or more quantitative → Scatter; exactly one
   quantitative → Histogram; two or more categorical → Heatmap. If nothing matches, the
   error lists the detected columns and suggests `-x`/`-y`.

**Fallback semantics are deliberately narrow.** The "no chart rule" warning is emitted
only when the recommended chart is Bar *and* the resolved axis pair has no dedicated
rule: any pair involving `Nominal`, plus the exhaustive non-Nominal set
{`Temporal × Temporal`, `Temporal × Categorical`, `Categorical × Temporal`}. It is not emitted on the no-hint
auto path, because auto-selection never lands on an arbitrary Bar; it either matches a
rule or errors — though the only-Y and only-X hint paths can trigger it. The message
names both columns and their types and suggests `-t`. One-shot and present print it to
stderr; explore stays silent to keep the TUI clean.

**Bar aggregation.** Sum is the default. Count is applied automatically in exactly two
situations: when `-t bar` is forced with no explicit Y and the resolved Y is not
quantitative, and when no Y is given for a categorical X in a dataset with no
quantitative columns. Both cases describe "count rows per category", which is what a
bar of a lone categorical column should mean.

**Grouped bars are unsupported.** The canonical bar data model holds one aggregate per
category, and the terminal bar widget has no grouped-series geometry. A `-c` column on
a Bar therefore feeds only the summary legend. When the user passes `-c` explicitly, vz
warns that the data is aggregated over all rows rather than split; silently dropping it
would read as if the chart were grouped when it is not.

## Canonical Assembler and Mode Adapters

`chart/data_builder.rs` is the **single canonical source** of chart-ready data:
aggregation, series construction, histograms, heatmaps, diff bars/lines, the histogram
bin-column choice, and the shared post-aggregation sort/truncate helpers. The selector
and recommender decide *what* to chart; the assembler builds it.

Mode adapters (one-shot builders, directory, diff, explore, present) obey the
canonical-assembler contract owned by
[ARCHITECTURE.md](ARCHITECTURE.md#canonical-assembler-contract) — sort/truncate/fit/
theme/wire only, never re-deriving aggregation or axis spans. Explore and present call
the canonical assembler directly rather than routing through the one-shot builder, so
one-shot-only concerns (extra-Y wiring, terminal-width fitting) do not leak into other
modes.

Rejected alternatives:

- **Per-mode aggregation.** Each mode computing its own totals drifts over time until
  two views of the same data disagree. It also multiplies the test surface by the
  number of modes.
- **A trait-based renderer per mode.** An abstraction over "what a mode is" hides the
  shared data contract and adds indirection without a performance or clarity gain.
- **Routing every mode through the one-shot builder.** This drags one-shot-specific
  fitting and extra-Y logic into explore and present, coupling modes that should only
  share the data layer.

Accepted residual divergences (deliberate tradeoffs, not oversights):

1. **Explore sorts interactive bars by `|Δ|`, while one-shot/present/HTML sort by signed
   Δ.** In an interactive session "largest change" naturally means magnitude; batch
   output should preserve sign so direction is visible. Unifying would make one of the
   two uses worse.
2. **JSON grouped series and sparkline color groups use `BTreeMap` (alphabetical), while
   canonical `ChartData` uses first-appearance order.** Machine-readable output favors a
   stable, sortable ordering; the canonical order is the domain order. Unifying would
   change the machine format without a consumer need.
3. **JSON line/scatter series keep raw X strings, while the renderer maps X to numeric
   indices.** JSON is string-based by contract; the index mapping is a rendering
   concern and stays at the edge.

## Decision Records

### D1. Single crate now, two crates later, never N crates

- **Context.** The API is still churning, but benchmarks and future reuse need a stable
  library surface.
- **Decision.** Ship one crate today. The target is a two-crate workspace: `vz-core`
  (library) plus `vz` (binary). The narrow public surface in `src/lib.rs` is the future
  `vz-core` body. Never split into N crates.
- **Rationale.** One crate keeps velocity and build/binary size while types move. Two
  crates give exactly one public surface to stabilize; the benchmark already consumes
  the library entry points.
- **Alternatives rejected.** A many-crate split (data/chart/render/output) would make
  one fix bump three or four crates, turn cross-crate renames into breaking changes, and
  raise onboarding cost for a single-product tool.
- **Status.** Intent. Today the code is a single crate. At split time the publication
  manifest must change from `src/` to `crates/*/src`; publication procedure is owned by
  [RUNBOOK.md](RUNBOOK.md).

### D2. `Cli` must not leak below the app plane

- **Context.** `&Cli` parameters in data and output code force every test through
  `Cli::try_parse_from`, block reuse from other products, and block the L3 split.
- **Decision.** `Cli` lives only in the app plane and `cli/`. Everything downstream
  takes plain structures — `PipelineParams`, `DirectoryParams`, `DiffParams`,
  `TableParams`, `ChartJsonParams`, `SparkParams`, `Query`, and `FilterOutcome`. Each
  mode has exactly one `&Cli → Params` conversion in its `*_from_cli` adapter.
- **Rationale.** Core logic is testable without parsing arguments, and the conversion
  point is the seam the crate split will cut along.
- **Alternatives rejected.** Passing `&Cli` into core; a global configuration
  singleton.
- **Status.** Done. New code must not add a `&Cli` parameter below the app plane.

### D3. Ratatui is the render engine; SVG/HTML mirror its cell geometry

- **Context.** Terminal charts need a real layout engine, and non-terminal consumers
  need a faithful rendering of the same chart.
- **Decision.** Ratatui is the sole render engine, and `render/` is the sole owner of
  cell geometry. `output/svg.rs` consumes the same `Buffer` and mirrors that geometry;
  HTML wraps that SVG.
- **Rationale.** A mature, Rust-native engine avoids reinventing layout, and one
  geometry source prevents terminal and SVG layouts from drifting apart.
- **Alternatives rejected.** A bespoke cell renderer; a headless geometry layer with a
  separate terminal renderer (two layout engines to keep in sync).
- **Status.** Active. Ratatui types must not appear in output public signatures.

### D4. `anyhow` now, typed errors only at the L3 split

- **Context.** A single binary needs no stable error API.
- **Decision.** Use `anyhow::Result` throughout today. When core splits, core gains a
  typed `thiserror` enum (`Io`/`Parse`/`Schema`/`Empty`) and `anyhow` retreats to the
  binary.
- **Rationale.** A type taxonomy invented before its consumers exist is churn; core
  gets a real error contract exactly when it becomes a library.
- **Alternatives rejected.** Introducing `thiserror` now; leaking `anyhow` into core's
  public surface after the split.
- **Status.** Deferred to L3.

### D5. `helpers/` is dissolved and must not be recreated

- **Context.** `helpers/` accumulated cross-plane couplings as a migration station.
- **Decision.** The module is gone: `resolve_*` belongs to `cli`, `build_*`/`parse_*`
  to `chart`, `apply_filters` to `filter`, and render options to `oneshot`.
- **Rationale.** Functions live with their owner, and the coupling L3 must delete is
  not re-grown.
- **Alternatives rejected.** Keeping `helpers/` as a facade; adding a new catch-all
  `utils` module.
- **Status.** Done.

### D6. No external data engine; in-memory; no streaming

- **Context.** The product charts small tabular files, and binary size is a stated
  constraint.
- **Decision.** Do not depend on Polars, DuckDB, or Arrow. Process data in memory. No
  streaming mode beyond `--watch`, and `notify` is used only to power `--watch`.
- **Rationale.** The value is visualization, not query execution; a heavy engine would
  slow builds and bloat the binary for capabilities the product does not offer.
- **Alternatives rejected.** An embedded query engine or columnar store for scale.
- **Status.** Active. Revisit only if real datasets outgrow memory; row/point limits
  exist, but no byte-size guard does.

### D7. Narrow public API; breaking changes allowed

- **Context.** Pre-1.0 with no downstream consumers.
- **Decision.** `lib.rs` exports only the public modules — data plane plus `cli` — and
  the three entry points;
  everything else is `pub(crate)` or private. Do not add a `#[doc(hidden)]` compatibility
  layer. Breaking changes are allowed when they improve the design.
- **Rationale.** A small surface is the only thing that can be stabilized at the split;
  hidden compatibility items silently become contracts.
- **Alternatives rejected.** `#[doc(hidden)]` deprecation shims kept for compatibility.
- **Status.** Active.

### D8. `--bins` is bounded by `MAX_BINS` and defensively clamped

- **Context.** A user-supplied bin count can be enormous (or zero), and the assembler is
  callable without the CLI.
- **Decision.** Validate `--bins` to `1..=10000` on input, and additionally clamp the
  computed bin count to `MAX_BINS` inside the render layer.
- **Rationale.** The bound protects memory and time, and the clamp covers library
  callers that bypass CLI validation.
- **Alternatives rejected.** Relying on CLI validation alone.
- **Status.** Done.
