# vz — Architecture

Structural facts: crate layout, module map by plane, visibility and dependency
rules, data flow, the canonical assembler boundary, and change impact.

This document owns **where code lives and what depends on what**. It does not
own design rationale ([DESIGN.md](DESIGN.md)), user-facing CLI/usage
([README.md](../README.md)), dev commands/tests ([CONTRIBUTING.md](../CONTRIBUTING.md)),
release steps ([RUNBOOK.md](RUNBOOK.md)), or behavioural traps
([GOTCHAS.md](GOTCHAS.md)). When this document and the code disagree, `src/` wins.

## Crate Layout

`vz` is currently a **single crate** with a library root (`src/lib.rs`) and a thin
binary root (`src/main.rs`); the same package builds the `vz` executable. The
library surface is deliberately narrow and its doc-comment calls it "the future
`vz-core` body".

| Kind | Items |
|---|---|
| `pub` modules | `chart`, `cli`, `filter`, `infer`, `loader`, `theme`, `util` |
| `pub` re-exports | `vz::run`, `vz::apply_output_shorthands` (from `app`), `vz::infer_from_data` (from `pipeline`) |
| `pub(crate)` modules | `diagnostics`, `diff`, `directory`, `explore`, `info`, `insights`, `oneshot`, `output`, `pipeline`, `present`, `render`, `sparkline`, `watch` |
| private modules | `mod app`, `#[cfg(test)] mod test_helpers` |

- `cli/` is an **app-plane module that is `pub`** because the binary entry and library API both need `Cli` and its value enums; it is the only app-plane module exposed publicly.
- Public items are reachable through their modules (e.g. `vz::cli::Cli`, `vz::chart::Query`, `vz::loader::load_data`), not re-exported at the root.
- There is no `#[doc(hidden)]` compatibility layer; breaking changes are allowed, so no compat shims are maintained.

### Target split (not yet implemented)

The intended end state is **two crates, never more**: a `vz-core` library and a
`vz` binary depending on it. Today this is a target only.

| Plane material | Target crate |
|---|---|
| `loader/`, `infer/`, `filter.rs`, `util.rs`, `sparkline.rs`, `theme.rs`, pure `insights`/`info` | `vz-core` |
| `chart/` (selector + recommend + **canonical data_builder**), `render/` (ratatui stays internal), `output/` (value/String returns), `diff/compute` + `diff/schema`, `present/parser` | `vz-core` |
| `cli/`, `app.rs`, `pipeline.rs`, `watch.rs`, `oneshot/`, `directory/`, `explore/`, `present/` rest, `diff/render/` | `vz` (bin) |

At split time `release-manifest.txt` must change from `src/` to `crates/*/src`
(a publication-scope change; see AGENTS/`RUNBOOK.md`).

## Module Map by Plane

Test siblings (`tests.rs`, `*_tests.rs`) are omitted; test placement follows
[CONTRIBUTING.md](../CONTRIBUTING.md).

### Data plane (`pub` / `pub(crate)`)

Pure processing: no `Cli`, no stdout/stderr, no TUI loop.

| File | Role |
|---|---|
| `src/util.rs`, `theme.rs` | Shared numeric parser/trend/min-max/path helpers; `Theme` (dark/light/high-contrast), series colors |
| `src/loader/mod.rs` | Unified loader: format detect + CSV/TSV/JSON/NDJSON dispatch, sampling, BOM/escape handling |
| `src/loader/space/{mod,detect,parse}.rs` | Fixed-width parser: façade, `looks_like_space_format`/`detect_columns`, `load_space` |
| `src/infer/{mod,types,detector}.rs` | `infer_schema`; `DataType`/`ColumnMeta`/`Schema`; per-value detection + column vote |
| `src/filter.rs` | `--where` predicate parse/apply, `FilterOutcome` |
| `src/chart/{mod,selector}.rs` | Chart façade re-exports; type-pair → `ChartType`, auto-select, fallback warning; home of `AggFunction`/`SortOrder` |
| `src/chart/recommend.rs` | `Query`/`Warnings`/`YOptions`; recommendation assembly, effective agg |
| `src/chart/data_builder.rs` | **Canonical assembler**: aggregation, series, histograms, heatmaps, diff bars/lines, sort/truncate |
| `src/sparkline.rs` | 8-level block sparkline (`pub(crate)`) |

### Render plane (`pub(crate)`)

Sole owner of terminal cell geometry (ratatui).

| File | Role |
|---|---|
| `src/render/mod.rs` | `ChartData` enum + dispatch, axes, `MAX_BINS`, `compute_bins`, `SERIES_COLORS`, number formatting, `render_chart_data` |
| `src/render/{line,bar,scatter,histogram,heatmap}.rs` | ratatui widgets |
| `src/render/nice_numbers.rs` | `nice_scale` tick math |

### Output plane (`pub(crate)`)

Headless producers of machine/human formats plus thin print wrappers.

| File | Role |
|---|---|
| `src/output/mod.rs`, `stats_text.rs` | Info JSON types/builders, `compute_column_stats`; human stat strings for `--info` |
| `src/output/chart_json.rs` | `-o json` chart data + `query` + `insights` |
| `src/output/{table,markdown}.rs` | Table/markdown exporters, shared `TableParams`, row limits |
| `src/output/spark.rs` | Sparkline exporter (`SparkParams`) |
| `src/output/svg.rs` | ratatui `Buffer` → SVG + real data marks; cell constants |
| `src/output/html.rs` | SVG wrapped in self-contained HTML with tooltips |

### App plane (`pub(crate)` / private)

Owns `Cli`, stdin/stdout/stderr, TUI loops, and mode dispatch.

| File | Role |
|---|---|
| `src/main.rs` | Thin entry: parse → shorthands → `run` |
| `src/app.rs` | Mode dispatch, owns `Cli`, shorthand normalization, render-limit validation |
| `src/pipeline.rs` | Shared post-load pipeline: filter → sample → validate → infer → recommend → output |
| `src/cli/mod.rs` | `Cli` (clap `Parser`), `Command` subcommands |
| `src/cli/args.rs` | `PipelineParams`/`DirectoryParams`/`DiffParams`, `Cli::to_*`, column-spec parsers |
| `src/cli/{types,resolve}.rs` | Clap `ValueEnum` spellings (`*Arg`, `OutputFormat`, `ThemeArg`); `resolve_*`, `format_override` |
| `src/oneshot/mod.rs` | One-shot render orchestration, `RenderOptions`, warnings, sizing |
| `src/oneshot/builders.rs` | Build `ChartData` from a recommendation; extra-Y wiring |
| `src/oneshot/{summary,ansi}.rs` | Summary line/legend/hint; Buffer → ANSI with `NO_COLOR`/`FORCE_COLOR` |
| `src/insights.rs`, `info.rs`, `diagnostics.rs` | Plain-language takeaways + diff insights; `--info` text/JSON; error hints/column suggestions |
| `src/watch.rs` | `--watch` notify loop |
| `src/diff/{mod,compute,schema}.rs` | Diff types + `run_diff`/`run_diff_from_cli`; `compute_diff`/`compute_diff_temporal`; schema validation + X/Y resolution |
| `src/diff/render/mod.rs` + `{bar,line,spark,json,markdown,html}.rs` | Diff output variants |
| `src/directory/mod.rs` | Directory orchestration, row limits/warnings |
| `src/directory/{scanner,combiner,catalog,date_extract}.rs` | Discovery/glob/recursion; schema match + concat + `_source`/`_file_date`; schema catalog; filename dates |
| `src/explore/mod.rs`, `{app,state,render,diff,diff_render}.rs` | Explore TUI entry/state/keybindings/draw + diff variant |
| `src/present/mod.rs`, `{parser,render,chart_loader}.rs` | Present TUI entry, markdown parser, slide draw, chart-block → `ChartData` |

## Plane Dependency Rules

Intended direction (enforced by review today; compiler-enforced only after the
L3 split):

```
app plane ──uses──▶ render / output planes ──uses──▶ data plane
```

The graph is **not strictly acyclic**. The sanctioned data → render edge is
`render`'s `ChartData` value types (`Axis`, `BarChartData`, `ChartConfig`,
`HistogramData`, `Series`, `HeatmapData`): they are a shared vocabulary, and the
canonical assembler (`chart/data_builder.rs`, data plane) builds them.

Known debt (cross-plane edges violating the intended order): `chart/recommend.rs`,
`chart/data_builder.rs`, and `chart/selector.rs` import `cli` / `diagnostics`
(app plane) for `ChartTypeArg`, column-spec parsers, and suggestions
(data → app); `output/{spark,table,markdown}.rs` and `output/svg.rs` import
`oneshot` for `resolve_chart_type`, `RenderOptions`, `terminal_width`,
`DEFAULT_HEIGHT`, and `build_chart_data_for_svg` (output → app).

Forbidden: `render/` geometry being re-derived elsewhere (only `output/svg.rs`
mirrors it, through the shared cell-geometry contract); the `Cli` type leaking
below the app plane, and app-plane concerns (stdout/stderr, TUI loops) appearing
below it.

### The `Cli` boundary

`Cli` must not leak below the app plane. Sanctioned touchpoints are exactly:

1. `src/cli/` (definition and `Cli::to_*` conversions),
2. `src/app.rs` (dispatch),
3. the `*_from_cli` adapters: `pipeline::render_data_from_cli`,
   `diff::run_diff_from_cli`, `directory::run_directory_from_cli`.

Data/output code instead takes plain, `Cli`-free structs:

| Struct | Defined in | Feeds |
|---|---|---|
| `PipelineParams` | `cli/args.rs` | `pipeline::render_data` |
| `Query` | `chart/recommend.rs` | `build_recommendation`, `effective_agg` |
| `DirectoryParams` | `cli/args.rs` | `directory::run_directory` / `run_catalog` |
| `DiffParams` | `cli/args.rs` | `diff::run_diff` |
| `TableParams` | `output/table.rs` | `output::table` + `output::markdown` |
| `ChartJsonParams` | `output/chart_json.rs` | `-o json` exporter |
| `SparkParams` | `output/spark.rs` | `-o spark` exporter |
| `FilterOutcome` | `filter.rs` | output of `filter::apply_filters` (filtered data + notice, printed at the edge) |

Conversion happens once, in the app plane (`Cli::to_query`,
`to_pipeline_params`, `to_directory_params`, `to_diff_params`). `RenderOptions`
has production `from_params` (takes `PipelineParams`) plus a `#[cfg(test)]`-only
`from_cli` test seam.

**Residual:** `output/table.rs` and `output/markdown.rs` still accept an unused
`schema: &Schema` parameter (discarded with `let _ = schema;`). This is minor
debt, not the documented pattern (`chart_json.rs` uses its parameter). Remove it
when touching those exporters.

## Data Flow

### One-shot canonical path

```
main.rs (clap parse) → app::apply_output_shorthands → app::run / dispatch → app::run_oneshot
    validate_render_limits
    [two files] → diff::run_diff_from_cli → diff::run_diff
    cli::resolve_input_file
    [--watch]   → watch::run_watch
    app::render_once
        [directory] → directory::run_directory_from_cli → directory::run_directory
        loader::load_data_full
        pipeline::render_data
            filter::apply_filters · loader::apply_sampling [--sample]
            validate_loaded_data / validate_color_column
            pipeline::infer_from_data
            chart::recommend::build_recommendation / effective_agg
            expand_all_y [--all-y]
            dispatch_output
                → chart::data_builder::* (canonical assembler)
                → render::render_chart_data (text / svg / html)
                → output::{table,markdown,spark,chart_json}
```

Canonical API names: `pipeline::{infer_from_data, render_data}`,
`cli::{to_query, to_pipeline_params, to_directory_params, to_diff_params}`,
`chart::select_chart`, `chart::recommend::{build_recommendation, effective_agg}`,
`chart::data_builder::{aggregate_bar, build_chart_config, build_histogram,
build_heatmap_data, build_diff_line_config, build_diff_bar_data,
histogram_column, sort_bar_data, truncate_bar_data}`, `render::render_chart_data`.

### Modes

| Mode | Entry | CLI adapter | Params |
|---|---|---|---|
| oneshot (default) | `pipeline::render_data` | `pipeline::render_data_from_cli` | `PipelineParams` |
| directory | `directory::run_directory` | `directory::run_directory_from_cli` | `DirectoryParams` |
| catalog | `directory::run_catalog` | (called from the directory path) | `DirectoryParams` |
| diff | `diff::run_diff` | `diff::run_diff_from_cli` | `DiffParams` |
| explore | `explore::run_explore` / `run_explore_diff` | subcommand dispatch in `app.rs` | canonical calls directly |
| present | `present::run_present` | subcommand dispatch in `app.rs` | `present/chart_loader` |
| info | (flag, not a mode) inside `pipeline::render_data` | same as oneshot | `PipelineParams` |

- **oneshot**: only mode through `pipeline::render_data` (all output formats); `--info` short-circuits before recommendation.
- **directory**: scans/merges same-schema files (`_source`/`_file_date`), then delegates to the shared render path.
- **diff**: validates two schemas, resolves X/Y, branches to temporal line / categorical bar; ignores several one-shot flags.
- **explore / present**: call the canonical assembler directly (present via `present/chart_loader`), bypassing oneshot-only concerns.
- **info**: a flag, not a mode; prints metadata inside `pipeline::render_data`.

## Canonical Assembler Contract

`chart/data_builder.rs` is the **single canonical assembler**. All modes obtain
their `ChartData` from its functions; mode adapters may only sort, truncate, fit
labels, apply a theme, or wire interactive/slide state — never re-derive
aggregation or axis spans.

| Function | Responsibility |
|---|---|
| `aggregate_bar` | Categorical X + Y aggregation (grouped or single series) |
| `build_chart_config` | Line/scatter config from resolved axes + rows |
| `build_histogram` | Histogram bins via `compute_bins` |
| `histogram_column` | Bin-column choice (probe first rows; shared by every histogram consumer) |
| `build_heatmap_data` | Categorical × categorical cell grid |
| `build_diff_bar_data` | Categorical diff bars (after values, signed-Δ sort/limit, marker labels) |
| `build_diff_line_config` | Temporal diff two-series line config |
| `sort_bar_data` / `truncate_bar_data` | Post-aggregation bar sort / top-tail limit |
| `append_series_refit_y` | Extra-Y overlay span refit reusing the base config's rows and X mapping |

Adapters: `oneshot::RenderOptions::from_params` + `oneshot/builders.rs`;
`directory::run_directory`; `diff::run_diff`; `explore/app.rs` (direct calls);
`present/chart_loader::load_chart_data`.

### Intentional residual divergences

Deliberate and documented (rationale in [DESIGN.md](DESIGN.md)); not bugs to fix
opportunistically:

1. **Diff-explore sort** orders interactive diff bars by `|Δ|`, while oneshot,
   present, and HTML use signed Δ (non-diff explore uses the canonical signed
   `data_builder::sort_bar_data`).
2. **Group ordering in JSON/spark** uses `BTreeMap` (alphabetical group order),
   whereas the canonical `ChartData` preserves first-appearance order.
3. **JSON line/scatter series** keep raw X strings, while the canonical renderer
   maps X to numeric indices; the JSON representation is string-based by
   contract.

## Change Impact Map

| If you change… | Affected |
|---|---|
| `cli/` (`Cli`, flags, `to_*`) | Every mode; flag semantics, README's CLI table, this module map. Run flags + modes tests. |
| `pipeline::render_data` / `PipelineParams` | Single-file oneshot, directory, `--info`; not diff. |
| `Query` (`chart/recommend.rs`) | X/Y resolution + effective agg in single-file oneshot, directory, **and** diff (`DiffParams.query` read by `diff/schema.rs`). |
| `chart/data_builder.rs` (canonical) | oneshot, explore, present, categorical + temporal diff, every Bar-consuming exporter (`table`, `markdown`, `json`, `spark`). Highest blast radius. |
| `chart/selector.rs` type table | Chart type in every mode; fallback warnings; bar adjustments. |
| `chart/recommend.rs` | Effective agg, auto color, Y options; modes building recommendations. |
| a single `render/*.rs` widget | Only that chart type's terminal output (and SVG/HTML mirroring it). |
| an output exporter (`output/*`) | Only that `-o` format for the modes using it; diff has separate renderers for some formats. |
| `loader/` or `infer/` | All modes: loading, format detection, inference, downstream selection. Run full integration tests. |
| `filter.rs` | Single-file oneshot, single-file explore, directory, present `where:` blocks; **not** diff (ignores `--where`). |

`pipeline` never depends on `diff`/`directory`/`present`/`explore`; those modes
call into `pipeline` and shared helpers, not the reverse. This local layering
does not make the whole graph acyclic (see Plane Dependency Rules).

## Constants and Limits

Structural limits that shape behaviour:

| Constant | Value | Module |
|---|---|---|
| `SAMPLE_SIZE` / `CATEGORICAL_THRESHOLD` / type-vote threshold | 100 / 20 / ≥80% | `infer/detector.rs` |
| `MAX_BINS` / `DEFAULT_BINS` / `HISTOGRAM_PROBE_ROWS` | 10,000 / 10 / 5 | `render/mod.rs`, `chart/data_builder.rs` |
| `MAX_CHART_POINTS` / `SPARK_MAX_POINTS` | 5,000 / 200 | `chart/data_builder.rs`, `output/spark.rs` |
| `DATA_SAMPLE_LIMIT` / `TABLE_ROW_LIMIT` / `MARKDOWN_ROW_LIMIT` | 100 / 100 / 100 | `output/` |
| `DEFAULT_HEIGHT` / `MIN_WIDTH` / `DEFAULT_TERMINAL_WIDTH` | 24 / 40 / 80 | `oneshot/mod.rs` |
| `LARGE_DATASET_THRESHOLD` / `MAX_COMBINED_ROWS` | 100,000 / 1,000,000 | `directory/mod.rs` |
| `DEFAULT_DIFF_BAR_WIDTH` / `SERIES_COLORS` / sparkline levels | 60 / 6 / 8 | `diff/render/bar.rs`, `render/mod.rs`, `sparkline.rs` |
| watch poll / debounce | 500 ms / 200 ms | `watch.rs` |

Data is processed fully in memory; there is no streaming mode. Type inference
samples up to 100 **evenly spaced** rows (first and last kept). The claim that
"files up to ~1 GB are fine" is **UNVERIFIED**: the code enforces row/point/bin
limits only, with no byte-size guard.

## Known Debt and Structural Constraints

- **`helpers/` must not be recreated.** Dissolved; its functions live in `cli/resolve.rs`, `chart/recommend.rs`, and `filter.rs`.
- **No `thiserror` until the L3 split.** `anyhow` throughout; typed errors are deferred to the `vz-core`/`vz` separation.
- **No external data engine.** No Polars/DuckDB; in-memory only. `notify` exists solely for `--watch`.
- **Release-freeze policy is out of scope** (AGENTS/`RUNBOOK.md`); only its structural consequence — the L3 split would change `release-manifest.txt` — belongs here.
- **Unused `schema` parameter** in `output/table.rs` / `output/markdown.rs` is residual debt (see the `Cli` boundary section).
- **Line-number references are fragile.** Prefer module/function names over `file:line`, which drift as code moves.
