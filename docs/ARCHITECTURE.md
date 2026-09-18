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
├── main.rs                 — binary entry, CLI dispatch
├── lib.rs                  — library crate re-exports (for benches/tests)
├── pipeline.rs             — render pipeline: infer → select → build → render → output
├── cli/                    — clap definitions (mod.rs, args.rs, types.rs)
├── helpers/                — CLI arg processing, format detection, data transforms
├── loader/                 — CSV/TSV/JSON/NDJSON/space unified loader, format auto-detect
├── filter.rs               — --where predicate engine
├── infer/                  — type inference (types.rs: Schema/ColumnMeta, detector.rs)
├── chart/                  — selector.rs (types → chart), data_builder.rs (rows → chart data)
├── render/                 — ratatui widgets: line, bar, scatter, histogram, heatmap, nice_numbers
├── oneshot/                — stdout rendering: builders, summary, ansi
├── insights.rs             — plain-language takeaways (pure logic; oneshot stderr + JSON `insights` + diff)
├── info.rs                 — --info column metadata
├── output/                 — machine-readable exporters: chart_json, markdown, spark, stats_text, svg, html, table
├── diff/                   — two-file comparison: schema, compute, render/{bar,line,spark,json,markdown,html}
├── directory/              — directory mode: scanner, combiner, catalog, date_extract
├── explore/                — interactive TUI: app, state, render, diff, diff_render
├── present/                — slides: parser, render, chart_loader
├── watch.rs                — --watch auto-redraw
├── theme.rs                — color themes (dark/light/high-contrast)
├── sparkline.rs            — shared sparkline generation
├── util.rs                 — shared numeric utilities
└── diagnostics.rs          — error hints & file suggestions

Unit tests live beside their module (`tests.rs` / `*_tests.rs`); end-to-end tests in `tests/`.
```

## Data Flow & Dependencies

```
main.rs ─── cli/        (parse args)
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
             ├── explore/  (inline construction → ChartData → ChartWidget → TUI)
             └── present/  (chart_loader.rs → ChartData → render_chart_data → slide)
```

Each mode has a **mode-specific builder layer** that adapts the shared `ChartData`
structures before passing them to `render_chart_data()`:
- `oneshot/builders.rs` — sorting, truncation, label fitting, theme application
- `explore/mod.rs` — interactive column selection → ChartData construction
- `present/chart_loader.rs` — Markdown chart block → ChartData

Layering note: modes call into `pipeline::render_data` / `pipeline::infer_from_data`
and `diff` column resolution (`diff::auto_x_column` et al.). This direction is
intentional — `pipeline` and `diff::schema` are shared services, not layers above
the modes. `pipeline` itself never depends on `diff` / `directory` / `present` /
`explore`, so the module graph stays acyclic.

### Change Impact Map

- `loader/` change → affects all modes. Run full integration tests.
- `filter.rs` change → affects single-file oneshot (via `pipeline::render_data`),
  single-file explore, directory mode, and present chart blocks (`where:` field).
  Does **not** affect diff mode: neither oneshot diff (`diff::run_diff`) nor
  explore diff applies `--where`.
- `infer/` change → affects chart selection + all modes.
- `chart/selector.rs` change → affects all modes.
- `chart/data_builder.rs` change → affects oneshot, explore, present.
- `render/` change → affects only the corresponding chart type.
- `oneshot/`, `explore/`, `present/` → affects only that mode.

## Performance Characteristics

- Data is processed in-memory; files up to ~1GB are fine
- Type inference samples the first 100 rows
- No streaming mode; the entire file is loaded before rendering
