# vz — Architecture

Structural facts: component overview, module layout, data flow, change impact.
Design intent and rationale live in [DESIGN.md](DESIGN.md).

## Component Overview

```
┌─────────────────────────────────────────────────────┐
│                    CLI (clap 4)                      │
│   vz <file>  |  vz explore <file>  |  vz present   │
└────────────────────────┬────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────┐
│               Data Loader (loader/)                  │
│   CSV / TSV / JSON / NDJSON  (format auto-detect)   │
└────────────────────────┬────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────┐
│            Type Inference Engine (infer/)            │
│   temporal / quantitative / categorical / nominal    │
└────────────────────────┬────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────┐
│         Chart Selection (chart/selector.rs)          │
│   Types → Best chart (line/bar/scatter/histogram)   │
│         Data Builder (chart/data_builder.rs)         │
│   Schema + Rows → Renderable chart data structures  │
└────────────────────────┬────────────────────────────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  One-shot    │ │   Explore    │ │   Present    │
│  (stdout)    │ │   (TUI)      │ │  (Slides)    │
│  oneshot/    │ │  explore/    │ │  present/    │
└──────────────┘ └──────────────┘ └──────────────┘
```

## Module Structure

```
src/
├── main.rs                 — thin binary entry (parse → apply_output_shorthands → run)
├── app.rs                  — binary dispatch (owns Cli end-to-end; private to crate)
├── lib.rs                  — narrow surface: data-plane modules + run/Cli/infer_from_data
├── pipeline.rs             — render pipeline: infer → select → build → render → output (crate-private)
├── cli/                    — clap definitions + Cli-derived resolutions (mod.rs, args.rs, types.rs, resolve.rs)
├── loader/                 — CSV/TSV/JSON/NDJSON/space unified loader, format auto-detect
├── filter.rs               — --where predicate engine + apply_filters
├── infer/                  — type inference (types.rs: Schema/ColumnMeta, detector.rs)
├── chart/                  — selector.rs (types → chart + AggFunction/SortOrder home), data_builder.rs (rows → chart data), recommend.rs (CLI hints → recommendation)
├── render/                 — ratatui widgets: line, bar, scatter, histogram, heatmap, nice_numbers (crate-private)
├── oneshot/                — stdout rendering: builders, summary, ansi (+ RenderOptions::from_cli adapter; crate-private)
├── insights.rs             — plain-language takeaways (pure logic; oneshot stderr + JSON `insights` + diff; crate-private)
├── info.rs                 — --info column metadata (crate-private)
├── output/                 — machine-readable exporters: chart_json, markdown, spark, stats_text, svg, html, table (crate-private)
├── diff/                   — two-file comparison: schema, compute, render/{bar,line,spark,json,markdown,html} (crate-private)
├── directory/              — directory mode: scanner, combiner, catalog, date_extract (crate-private)
├── explore/                — interactive TUI: app, state, render, diff, diff_render (crate-private)
├── present/                — slides: parser, render, chart_loader (crate-private)
├── watch.rs                — --watch auto-redraw (crate-private)
├── theme.rs                — color themes (dark/light/high-contrast)
├── sparkline.rs            — shared sparkline generation (crate-private)
├── util.rs                 — shared numeric utilities
└── diagnostics.rs          — error hints & file suggestions (crate-private)
```

Public surface (`lib.rs`): `chart`, `cli`, `filter`, `infer`, `loader`,
`theme`, `util`, `run`, `apply_output_shorthands`, `infer_from_data`
(benches use the last). Everything else is `pub(crate)` or private —
no `#[doc(hidden)]` compat layer (postcompat rebuild, no downstream).

Unit tests live beside their module (`tests.rs` / `*_tests.rs`); end-to-end tests in `tests/`.

## Planes and Dependency Rules

Structural grouping; the reasons behind it live in
[DESIGN.md](DESIGN.md#module-boundaries-intent-layout-lives-in-architecturemd).

- **Data plane** (must be `Cli`-free, no stdout/stderr, no TUI loop):
  `loader/`, `infer/`, `filter.rs`, `chart/`, `util.rs`, `sparkline.rs`,
  pure parts of `insights.rs` / `info.rs` / `diagnostics.rs`.
- **Render plane** (sole owner of cell geometry): `render/`
  (`Buffer`/`Rect` layout; `SERIES_COLORS`, `ChartData`, `render_chart_data`).
  `output/svg.rs` mirrors this geometry for the text-grid layer.
- **Output plane** (headless producers + thin print wrappers): `output/`.
- **App plane** (owns `Cli`, stdout/stderr, TUI loops, mode dispatch):
  `app.rs`, `pipeline.rs`, `cli/`, `oneshot/`, `diff/`,
  `directory/`, `explore/`, `present/`, `watch.rs`, `main.rs` (thin entry).
  (`helpers/` was dissolved in Phase 1: `resolve_*`→`cli/resolve.rs`,
  `build_*/parse_*`→`chart/recommend.rs`, `apply_filters`→`filter.rs`,
  `build_render_options`→`oneshot::RenderOptions::from_cli`.)
  `AggFunction`/`SortOrder` live in `chart::selector` (CLI spellings are
  `AggFunctionArg`/`SortOrderArg` + `to_*` converters); all other modules
  are `pub(crate)` or private.

Allowed direction (enforced at L3 by crate split; today by review):

```
app plane ──uses──▶ render/output planes ──uses──▶ data plane
app.rs → pipeline/cli → {oneshot, diff, directory, explore, present, watch}
        → chart/infer/loader/filter → util
```

Forbidden (compiler-unchecked today — do not add new instances):

- `&Cli` parameters outside `cli/` + binary adapters. Known instances:
  (none — Phase 2 complete).
  (Done: `diagnostics::error_hint` takes `Option<&Path>` since Phase 2-1;
  `output/table.rs` + `output/markdown.rs` take `TableParams` since Phase 2-2;
  `chart/recommend.rs` takes `Query` + returns `Warnings` since Phase 2-3,
  with the sole `Cli → Query` conversion at `Cli::to_query`;
  `pipeline::render_data` + `dispatch_output`/`print_spark`/`print_chart_json`
  take `PipelineParams` since Phase 2-4 — the only `&Cli` touchpoint left is
  the `render_data_from_cli` adapter — and `filter::apply_filters` returns
  `FilterOutcome` with the `info:` notice printed by callers;
  `diff::run_diff` + `diff/schema.rs` resolution + `diff/render/` take
  `DiffParams` since Phase 2-5 — the only `&Cli` touchpoint left is the
  `run_diff_from_cli` adapter, which keeps the `--where`/`--agg`/`--color`
  no-effect warnings in the bin;
  `directory::run_directory` + `directory::run_catalog` take
  `DirectoryParams` since Phase 2-6 — the only `&Cli` touchpoint left is
  the `run_directory_from_cli` adapter.)
- `println!/eprintln!` in data/render planes. Known instances:
  `output/markdown.rs` + `output/table.rs` warnings,
  `chart/data_builder.rs` (`maybe_sample` sampling notice).
  Precedent to copy:
  `output/chart_json.rs` (`ChartJsonParams`), `output/spark.rs`
  (`SparkParams`), `output/table.rs` (`TableParams`, shared with
  `output/markdown.rs`), `chart/recommend.rs` (`Query` in / `Warnings`
  out), `pipeline.rs` (`PipelineParams` in), `diff/` (`DiffParams` in;
  `--where`/`--agg`/`--color` warnings stay in the `run_diff_from_cli`
  adapter) and `filter.rs`
  (`FilterOutcome` out: filtered data + `info:` notice, printed at the edge),
  `directory::run_directory`/`run_catalog` (`DirectoryParams` in; sole
  `&Cli` touchpoint is the `run_directory_from_cli` adapter).
- `render/` geometry invented anywhere else; `ratatui` types in `output/`
  public signatures (only `output/svg.rs` touches `Buffer`, via the shared
  cell-geometry contract).

### L3 Crate Mapping (target, not yet implemented)

| Current `src/` path | Target crate | Notes |
|---|---|---|
| `loader/`, `infer/`, `filter.rs`, `util.rs`, `sparkline.rs` | `vz-core` | Move as-is; drop `Cli` uses on the way |
| `chart/` (selector + data_builder + recommend) | `vz-core` | Canonical `ChartData` assembler lives here; `Query`/`Warnings` seam done at Phase 2-3, `PipelineParams` seam at Phase 2-4, `DiffParams` seam at Phase 2-5, `DirectoryParams` seam at Phase 2-6 |
| `render/` | `vz-core` | Keep ratatui inside; hide from public signatures |
| `output/` | `vz-core` | Convert to `String`/value returns; print wrappers stay in bin |
| `diff/compute.rs`, `diff/schema.rs` (+ pure types) | `vz-core` | `run_diff_from_cli` CLI behavior stays in bin |
| `diff/render/` | `vz` (bin) | TUI/CLI-coupled rendering |
| `present/parser.rs` | `vz-core` | Markdown → `Presentation` only |
| `present/` rest, `explore/`, `oneshot/`, `directory/` | `vz` (bin) | Mode dispatch + loops |
| `pipeline.rs`, `cli/`, `watch.rs` | `vz` (bin) | `Cli → Query` conversion in one adapter |
| `theme.rs`, `insights.rs`, pure `info.rs`/`diagnostics.rs` | `vz-core` | `error_hint` CLI part stays in bin |
| `tests/`, `tests/common`, `fixtures/`, `benches/` | workspace root | Shared; never copy per crate |

At split time `release-manifest.txt` must change from `src/` to
`crates/*/src` + `crates/*/Cargo.toml` (publication-scope change).

## Data Flow & Dependencies

```
app.rs ─── cli/        (parse args)
   │
   ├──────── loader/    (file → LoadedData{headers, rows})
   │
   ├──────── filter/    (--where predicates → row subset)
   │
   ├──────── infer/     (LoadedData → Schema{columns: Vec<ColumnMeta>})
   │
   ├──────── chart/
   │         ├── selector   (Schema → ChartType)
   │         └── data_builder (Schema + rows → chart-specific data structs)
   │
   └──────── render/    (data structs → ratatui Buffer)
             │
             ├── oneshot/  (builders.rs → ChartData → render_chart_data → Buffer → ANSI)
             ├── explore/  (canonical assembler calls → ChartData → ChartWidget → TUI)
             └── present/  (chart_loader.rs → ChartData → render_chart_data → slide)
```

Each mode has a **mode-specific builder layer** that adapts the shared `ChartData`
structures before passing them to `render_chart_data()`:
- `chart/data_builder.rs` — canonical assemblers (`build_chart_config`,
  `aggregate_bar`, `build_histogram`, `build_heatmap_data`,
  `histogram_column` (bin-column choice shared by every histogram consumer),
  `build_diff_line_config` since Phase 3-1, categorical diff annotation
  (`diff_direction_marker`/`format_diff_change`/`build_diff_bar_data`)
  since Phase 3-2, post-aggregation Bar adapters
  (`sort_bar_data`/`truncate_bar_data`) and extra-Y overlay span refit
  (`append_series_refit_y`) since Phase 3-3)
- `oneshot/builders.rs` — axis resolution from the recommendation, title
  derivation, extra-Y wiring (series via `build_multi_y_series`, span refit
  via `append_series_refit_y`), label fitting, theme application
- `explore/` — interactive column selection → canonical assembler calls;
  deliberately not routed through `oneshot/builders.rs` (oneshot-only
  concerns such as extra-Y/fitting must not leak into other modes)
- `present/chart_loader.rs` — Markdown chart block → canonical assembler
  calls (block title/bins/top/sort come from the block at the edge)

Unification direction (see DESIGN.md decision 1): `chart/data_builder.rs` is
the canonical assembler; mode adapters may only sort, truncate, fit labels,
apply theme, or wire slide/interactive state — never re-derive aggregation
or axis spans. Temporal diff Line assembly is unified (Phase 3-1);
categorical diff Bar annotation is unified since Phase 3-2 (values = after,
labels = `label ▲ +20%`, signed-Δ sort/limit via `build_diff_bar_data`;
color-by-direction stays at the edge — html green/red/gray, explore table
Dir column). Bar post-aggregation and extra-Y span refit are unified since
Phase 3-3 (extra-Y series reuse the base config's sampled rows *and* its X
mapping — the row index when ungrouped, the unique-category index when a
color column groups the base — so overlay coordinates stay aligned with the
series they annotate); Line/Scatter/Histogram/Heatmap adapters only resolve
their input plane and derive titles on top of canonical calls. The histogram
bin-column choice (`histogram_column`) and the summary/spark/insights
description of the binned column are shared by every histogram consumer
(oneshot text/JSON/spark, insights, explore, present).

Known residual divergences (documented, intentional until decided otherwise):
- explore interactive sort uses |Δ| (`sorted_entries`) while
  oneshot/present/html use signed-Δ.
- Color-group ordering: canonical `ChartData` series follow first-appearance
  order, while JSON (`build_grouped_series_json`) and spark color output use
  BTreeMap (alphabetical). JSON line/scatter series also keep raw X strings
  instead of the canonical numeric index mapping, because the JSON
  representation is string-based by contract.

Layering note: modes call into `pipeline::render_data` / `pipeline::infer_from_data`
and `diff` column resolution (`diff::auto_x_column` et al.). `pipeline` itself
never depends on `diff` / `directory` / `present` / `explore`, so the module
graph stays acyclic. `pipeline` is the app-plane orchestrator (the only `&Cli`
touchpoint is the `render_data_from_cli` adapter; everything downstream takes
`PipelineParams`); `diff::run_diff` is the diff-plane orchestrator (the only
`&Cli` touchpoint is the `run_diff_from_cli` adapter; everything downstream
takes `DiffParams`); `directory::run_directory` is the directory-plane
orchestrator (the only `&Cli` touchpoint is the `run_directory_from_cli`
adapter; everything downstream takes `DirectoryParams`);
the `Cli`-free functions it calls (`infer_from_data`, `diff::schema`
resolution) are the shared services that will move to `vz-core` at L3.

### Change Impact Map

- `loader/` change → affects all modes. Run full integration tests.
- `filter.rs` change → affects single-file oneshot (via `pipeline::render_data`),
  single-file explore, directory mode, and present chart blocks (`where:` field).
  Does **not** affect diff mode: neither oneshot diff (`diff::run_diff`) nor
  explore diff applies `--where`.
- `infer/` change → affects chart selection + all modes.
- `chart/selector.rs` change → affects all modes.
- `chart/data_builder.rs` change → affects oneshot, explore, present, diff
  rendering, and every Bar-consuming output (`table`/`markdown`/`json`/`spark`).
- `render/` change → affects only the corresponding chart type.
- `oneshot/`, `explore/`, `present/` → affects only that mode.

## Performance Characteristics

- Data is processed in-memory; files up to ~1GB are fine
- Type inference samples 100 evenly spaced rows (head→tail); see DESIGN.md
- No streaming mode; the entire file is loaded before rendering
