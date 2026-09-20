# vz — Design Document

Design intent: vision, philosophy, behavioral rules, and key decisions.
Structural facts (modules, data flow, change impact) live in [ARCHITECTURE.md](ARCHITECTURE.md).

## Vision

CLI BI tool that auto-visualizes data in the terminal with zero configuration.
Three output modes: **One-shot** (default stdout), **Explore** (interactive TUI), **Present** (slide-based).

## Core Philosophy

- **Convention over Configuration** — Data types determine visualization
- **Zero-config by default** — Override only when needed
- **Terminal-native** — No browser, no GUI, just your terminal
- **Instant value** — `vz data.csv` produces a meaningful chart immediately

## Type Inference Rules

Implemented in `infer/detector.rs`. Each value is classified first, then the
column type is decided by majority vote:

| Value pattern | Detected as | Notes |
|---------------|-------------|-------|
| `YYYY-MM-DD` (optional time), `YYYY/MM/DD`, `MM/DD/YYYY`, `DD-Mon-YYYY`, `DD.MM.YYYY`, month names (`Jan 15 2024`, `Jan 15, 2024`, `15 Jan 2024`), `YYYY-MM` | `Temporal` | Checked before numeric |
| Display-formatted numbers via `util::parse_number` | `Quantitative` | `1,000`, `$100`, `€50`, `45%` (= 0.45), `10k`, `10GiB`, `(42)` (= -42), `USD 100` all parse → Quantitative. One parser shared by inference, aggregation, series, filters, diff, sparkline, and JSON samples, so a value means the same number on every path. `%` is a fraction (`50%` = 0.5); storage suffixes are decimal except `KiB/MiB/GiB/TiB` (binary). `--where` equality is numeric too (`revenue=2000` matches `$2,000`) |
| Trend annotation via `util::trend_label`/`trend_from_slice` | `→ stable` band ±5% | Single implementation shared by oneshot summary lines and spark suffixes; denominator is `first.abs()` (`-100 → -50` = `↑ +50%`), near-zero start yields no trend |
| `NaN`, `inf`, `-inf`, `Infinity` | `Nominal` (excluded from column vote) | `parse_number` returns `None` for non-finite → treated like nulls: skipped in inference, aggregation, and all chart paths |
| Empty string | `Nominal` (ignored in column vote) | Nulls don't vote |
| Anything else | `Nominal` | e.g. UUIDs, free text |

Column decision (100 evenly spaced rows head→tail, empty values excluded):

1. ≥ 80% of sampled values Temporal → `Temporal`
2. ≥ 80% Quantitative → `Quantitative`
3. Otherwise by cardinality: ≤ 20 unique values → `Categorical`, else `Nominal`

Only 100 evenly spaced rows (covering head to tail) are sampled; there is no full-scan fallback.
The same even sampling is applied in `pipeline::infer_from_data`.

## Chart Selection Design

The user-facing selection table and behavior live in [README.md](../README.md#chart-selection-rules).

Design intent: the selector maps inferred column types to a chart type, normalizes
reversed user axes to the canonical orientation (Quantitative × Temporal →
x=temporal Line; Quantitative × Categorical → x=categorical Bar), renders a
count-Bar for a lone categorical `-x`, and falls
back to Bar for unmatched type pairs.

## CLI Design

Flag definitions and examples live in [README.md](../README.md#usage); `vz --help` is authoritative at runtime.

Design guideline: the common case needs no flags (`vz data.csv`), and every override
(axes, type, aggregation, filtering) is opt-in.

## Scope

### In Scope (v0.2)
- File-based batch visualization (CSV/TSV/JSON/NDJSON)
- Auto-inference of column types and chart selection
- Three output modes: oneshot (stdout), explore (TUI), present (slides)
- Machine-readable exports: JSON, SVG, HTML, Markdown, sparkline, table
- Row filtering, aggregation, sampling
- Color themes (dark, light, high-contrast)
- File watch mode for iterative exploration
- Diff mode (two-file comparison)
- Directory mode (multi-file combine)
- Shell completions

### Non-goals (for now)
- Database connections (Parquet, SQLite, PostgreSQL)
- PNG raster export
- Streaming / real-time data beyond `--watch`
- Data transformation / ETL operations
- Custom color palettes (beyond the 3 built-in themes)

## Key Design Decisions

1. **Plain words before chart literacy** — Every chart ships 0–3 `💡` takeaway sentences (movement/extremes/leader/clusters) computed from the same parsed numbers as the chart, so a non-engineer gets the point without reading axes. Same ±5% stable band and `parse_number` as every other path; silent when there is nothing to say.
2. **Ratatui for rendering** — Mature, active, Rust-native
3. **No external data engine** — Keep binary small, no Polars/DuckDB dep for v1
4. **In-memory processing** — v1 targets files that fit in memory (< 1GB)
5. **Convention-first CLI** — Minimal flags needed for 80% of use cases
6. **Shared data_builder** — All 3 modes build on the same `ChartData`
   structures from `chart/data_builder.rs` to avoid divergence; each mode keeps
   only a thin adaptation layer (sorting, truncation, slide wiring, see
   ARCHITECTURE.md) on top
 7. **Format auto-detection** — Extension first, then content heuristics (tabs vs commas, JSON detection)

## Module Boundaries (intent; layout lives in ARCHITECTURE.md)

Intent is a one-way dependency: **data plane → render plane → app plane
never reverses**. Concretely:

- **Data plane** (`loader/`, `infer/`, `filter.rs`, `chart/`, `util.rs`,
  `sparkline.rs`, pure parts of `insights.rs`/`info.rs`/`diagnostics.rs`) is
  pure: no `Cli`, no stdout/stderr, no TUI event loop. It is the future
  `vz-core` body, so every new function here must stay callable without
  `Cli::try_parse_from`.
- **Render plane** (`render/`) owns ratatui `Buffer`/`Rect` geometry. It is
  the single owner of cell layout; `output/svg.rs` mirrors that geometry for
  the text-grid layer (see its layout-contract comment). Nothing else may
  invent coordinates.
- **Output plane** (`output/`) must be headless `String`/value producers with
  thin `print_*` wrappers. `chart_json`/`spark` already take `*Params`
  structs instead of `&Cli` — that is the pattern to copy. `markdown`/`table`
  still taking `&Cli` and `svg` rendering via `Buffer` are known debt, not
  precedent (see below).
- **App plane** (`main.rs`, `pipeline.rs`, `oneshot/`, `diff/`, `directory/`,
  `explore/`, `present/`, `watch.rs`, `helpers/`, `cli/`) owns `Cli`,
  stdout/stderr, and TUI loops. Only this plane converts `Cli` into plain
  parameter structs.

## Decision Records (why the boundaries exist)

1. **Shared `ChartData` with thin mode adapters** (extends decision 6 above).
   `oneshot/builders.rs` is the canonical assembler
   (`ResolvedAxes` → `data_builder` → `ChartData`); `present/chart_loader.rs`
   and `explore/diff_render.rs` re-assemble the same chain inline. That
   duplication is debt: unify toward one assembler so a `data_builder` change
   cannot diverge between modes. A mode adapter may only sort, truncate, fit
   labels, apply theme, or wire slide/interactive state — never re-derive
   aggregation.
2. **`Cli` must not leak below the app plane.** Every `&Cli` parameter in
   data/output code forces tests through `Cli::try_parse_from`, blocks reuse
   from other products, and blocks the L3 crate split. Current violations are
   debt, not examples to follow: `pipeline::render_data(&Cli)` (the worst —
   the whole pipeline hangs off CLI), `diff::run_diff(&Cli)`,
   `directory::run_directory(&Cli)`, `output/markdown.rs` + `output/table.rs`
   taking `&Cli`, `diagnostics::error_hint(_, &Cli)`. New code must take a
   plain `*Params`/`*Options` struct (precedent: `ChartJsonParams`,
   `SparkParams`) or `Option<&Path>` instead.
3. **Ratatui stays inside the render contract.** SVG/HTML/JSON exist so
   agents and reports can consume charts without a terminal; exposing
   `Buffer`/`Rect`/`Color` in their signatures would drag the TUI stack into
   every consumer. `output/svg.rs` using a `Buffer` today is tolerated only
   because of the shared cell-geometry contract — the direction is toward
   `render_*(&ChartData, &Opts) -> String` functions with printing left to
   the binary.
4. **`anyhow` now, typed errors at L3.** A single binary needs no stable error
   API, so `anyhow::Result` everywhere is correct today. When `vz-core`
   splits out, core gains a `thiserror` enum (`Io`/`Parse`/`Schema`/`Empty`)
   and `anyhow` retreats to the binary — do not introduce typed errors before
   the split, and do not leak `anyhow` into core's public surface after it.
5. **Single crate now, two crates next, never N crates.** One crate keeps
   velocity and binary size while the API is still churning; a 2-crate
   workspace (`vz-core` lib + `vz` bin) is the reuse target because it has
   exactly one public surface to stabilize. Finer splits (data/chart/render/
   output/…) were evaluated and rejected: one fix would bump 3–4 crates,
   cross-crate renames become breaking changes, and beginners get lost.
   Revisit only when two independent products pin different core versions.
6. **`Query` (Cli-independent params) is the L3 seam.** All core entry points
   will take one plain `Query` struct; `Cli → Query` conversion lives in
   exactly one place in the binary. This is why params structs already exist
   for JSON/spark output — extend the pattern, never add another `&Cli`.
7. **`helpers/` dissolves, never grows.** `resolve_*` belongs to `cli`,
   `build_*/parse_*` belongs to `chart`, `apply_filters` belongs to `filter`.
   The module exists only as a migration station; adding new helpers there
   re-creates the coupling L3 must delete.

## Reuse Roadmap to L3 (`vz-core` + `vz`)

Target (workspace, `edition 2024` / MSRV 1.88 inherited, single lockfile):

- `crates/vz-core` (lib, publishable): `loader/`, `infer/`, `filter`,
  `util`, `sparkline`, `chart/` (selector + data_builder), `render/`
  (ratatui hidden from public signatures), `output/` as `String`-returning
  functions, `diff/{compute,schema,types}` (pure parts only),
  `present/parser` (markdown → `Presentation` only), `theme`, `insights`,
  pure parts of `info`/`diagnostics`, plus `query.rs` (`Query` +
  `load_infer_select` / `build_chart`).
- `crates/vz` (bin + thin adapters, `publish = false`): `cli/` (clap only),
  `Cli → Query` conversion, `pipeline`, `oneshot/`, `diff` rendering +
  `run_diff`, `directory/`, `explore/`, `present/` (loop + chart_loader +
  render), `watch`, printing wrappers.
- Ambiguous rulings (decided, do not relitigate without new evidence):
  `output/svg,html` → core as `String` functions; `present/parser` → core but
  slide running → bin; `diff` compute/schema → core but `--where`-ignoring
  CLI behavior → bin; `diagnostics::suggest_column` → core but
  `error_hint` → bin with `Option<&Path>`.
- Public-surface rules for core: new public structs get
  `#[non_exhaustive]`; no `ratatui`/`crossterm` types in public signatures
  (convert internally, e.g. own `Color` + `to_ratatui()`); document with
  rustdoc + one example per entry point (`loader+infer+select`,
  `build_chart` embedding, `render_svg` report).
- `release-manifest.txt` must switch from `src/` to `crates/*/src` +
  `crates/*/Cargo.toml` at split time. Manifest edits are publication-scope
  changes: never expand silently (AGENTS.md guardrail).

## Agent Judgment Guide (when unsure, read this)

- **Where does new logic go?** Data parsing/inference/selection/aggregation
  → data plane (no `Cli`). Pixels/cells/geometry → `render/`. New
  machine-readable shape → `output/` as a `String`/value function + thin
  print wrapper. Flags plumbing/validation messages/TUI wiring → app plane.
- **Forbidden without explicit approval:** new `&Cli` parameters outside
  `cli/` + binary adapters; new `println!/eprintln!` in data/render/output
  cores; new `pub mod` in `lib.rs` (default `pub(crate)`); new files under
  `helpers/`; new `tests/` targets or helper copies; new dependencies
  (binary size); `Cargo.toml` version bumps (release-time only).
- **Prefer:** extending a `*Params` struct over adding a flag parameter;
  returning `Vec<Warning>` over printing warnings from core; moving a
  function toward its owning plane over adding a cross-plane `use`.
- **If two placements seem valid,** choose the one that keeps the data plane
  `Cli`-free and the dependency arrow pointing app → data. Duplication
  across modes is never the answer — lift the shared piece into
  `chart/` or `output/`.
