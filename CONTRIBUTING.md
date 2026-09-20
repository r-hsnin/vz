# Contributing to vz

Everything needed to build, test, lint, bench, and submit changes. For module structure see
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md); for design rationale see
[docs/DESIGN.md](docs/DESIGN.md). User-facing
CLI behavior is documented in [README.md](README.md); do not duplicate it here.

## Prerequisites

- [rustup](https://rustup.rs/) with the pinned toolchain from `rust-toolchain.toml`
  (channel `1.97.0`, components `rustfmt`, `clippy`). Running `cargo` inside the repo selects it
  automatically.
- Minimum supported Rust version (MSRV): **1.88**. This is **only** verified in CI by
  `cargo +1.88.0 check --locked` (the `msrv` job). A bare `cargo check` uses the pinned 1.97.0
  toolchain and does not validate MSRV — install 1.88.0 via rustup if you want to check locally.

## Setup, build, run

```bash
git clone <repo-url> && cd vz   # fork on GitHub, or clone your fork

cargo build                     # debug build
cargo build --release           # optimized binary at target/release/vz

cargo run -- fixtures/sales.csv
cargo run -- fixtures/sales.csv -o json
cargo run -- fixtures/diff/sales_before.csv fixtures/diff/sales_after.csv
```

Example datasets live in `fixtures/`. `demo/` contains larger showcase assets.

## Pre-commit verification

Run these before every commit. `cargo fmt --check`, `cargo clippy`, and `cargo test` are the
repo's verification commands and run in the local hooks and CI; `git diff --check` is an
additional local whitespace sanity check (not run by hooks or CI):

```bash
cargo fmt                                              # or: cargo fmt --check
cargo clippy --all-targets -- -D warnings              # zero warnings required
cargo test
git diff --check                                       # whitespace sanity check, local only
```

Never bypass a failing hook with `--no-verify` (emergency only).

## Test layout

`vz` uses unit tests, integration tests, and `insta` snapshots.

### Unit tests

Unit tests live beside the code they exercise, pulled in by one of:

```rust
#[cfg(test)] mod tests;                       // e.g. src/render/tests.rs
#[cfg(test)] #[path = "data_builder_tests.rs"] mod tests;   // sibling *_tests.rs
```

The aggregate `tests.rs` form exists for legacy modules only; new modules must use per-module
sibling files. Shared test fixtures `make_schema` / `make_recommendation` live in
`src/test_helpers.rs` (test builds only). Use `pretty_assertions` for readable diffs.

### Integration tests

Integration tests live in `tests/`, one target per feature area:

| Target | Area |
|---|---|
| `tests/oneshot.rs` | Default one-shot rendering |
| `tests/flags.rs` | Flag parsing and interactions |
| `tests/inputs.rs` | Loaders and input formats |
| `tests/output.rs` | Output formats (JSON/table/markdown/spark/svg/html) |
| `tests/directory.rs` | Directory combine and catalog modes |
| `tests/diff.rs` | Two-file diff mode |
| `tests/modes.rs` | Watch, explore, present, completions |
| `tests/snapshot_test.rs` | `insta` snapshot tests |

Shared helpers live **only** in `tests/common/mod.rs`. Reuse them; do not copy helpers into a
target. Available helpers:

| Helper | Purpose |
|---|---|
| `vz_binary()` | Command with `TERM=dumb` for stable widths |
| `vz_command()` | Command with no environment overrides |
| `vz_no_color()` | Command with `NO_COLOR=1`, `FORCE_COLOR` removed |
| `run_vz_stdout(args)` | Deterministic run (`NO_COLOR=1`, `COLUMNS=80`), returns stdout |
| `temp_csv(rows)` | Temp file with the given rows |
| `temp_csv_with_suffix(suffix, rows)` | Same, with an extension for format detection |
| `MIN_CHART_LINES` | Minimum chart height for smoke assertions |

Use `tempfile` for temporary files.

### Snapshots

Rendered-output snapshots use [`insta`](https://insta.rs/), stored under `tests/snapshots/`. A
changed rendering writes a pending `*.snap.new` and fails the test (expected Red):

1. Run `cargo test --test snapshot_test`.
2. Review each `tests/snapshots/*.snap.new` and confirm the rendering is correct, not merely
   different.
3. Accept with `cargo insta accept` (if `cargo-insta` is installed) or by renaming `.snap.new`
   to `.snap`.
4. Re-run to confirm green.

Keep snapshots deterministic: helpers must pin `NO_COLOR=1` and `COLUMNS=80` (as `run_vz_stdout`
does). Verify a new snapshot is byte-identical across two runs before accepting.

### Fixtures

| Path | Contents |
|---|---|
| `fixtures/` | `sales`, `departments`, `stock`, `temperature`, `exam_scores`, `body_measurements`, `access_log`, `bom_sales`, `messy_data`, `mixed_values` (CSV); `scores.json`; `demo.md`, `code_demo.md` |
| `fixtures/diff/` | before/after pairs: `sales_before`/`sales_after`, `timeseries_before`/`timeseries_after`, `ts_daily_before`/`ts_daily_after`, plus `identical`, `schema_mismatch` |
| `fixtures/dir_test/` | 13 directory-mode cases: `case_insensitive`, `dated`, `empty`, `header_only`, `mixed_extensions`, `mixed_format`, `mixed_schema`, `nested` (includes `empty_sub` and `.hidden_dir`), `ragged`, `reordered`, `same_schema`, `single_file`, `with_hidden` |
| `fixtures/fixed_width/` | Space-aligned samples: `kubectl_get_pods`, `kubectl_top_pods`, `df_h`, `ps_aux`, `lsblk`, `separator_lines`, `empty_values`, `single_row` |

## Running specific tests

```bash
cargo test                              # all unit + integration + snapshot tests
cargo test --test diff                  # one integration target (area)
cargo test --test directory             # another target
cargo test snapshot_test                # snapshot target
cargo test oneshot::tests               # all tests in a unit module
cargo test test_basic_csv               # a single test by name
cargo test -- --nocapture               # show stdout/stderr from tests
```

## Benchmarking

Benchmarks use [Criterion](https://github.com/bheisler/criterion.rs), declared as
`[[bench]] name = "loading"` with `harness = false` in `Cargo.toml`, source in
`benches/loading.rs`:

```bash
cargo bench                             # all benchmarks
cargo bench -- csv_parse                # filter by name
cargo bench -- --save-baseline before   # save a baseline
cargo bench -- --baseline before        # compare after changes
```

| Benchmark | What it measures |
|---|---|
| `csv_parse_1000` | CSV parsing of 1000 rows |
| `json_parse_1000` | JSON array parsing of 1000 rows |
| `space_parse_1000` | Fixed-width/space parsing of 1000 rows |
| `infer_1000` | `infer_from_data` on 1000 rows |
| `infer_10000_rows` | `infer_from_data` scaling (sampled) |
| `pipeline_csv_1000` | CSV load **plus** `infer_from_data` only — **no chart selection** |
| `full_pipeline/csv_load_infer_1000` | End-to-end CSV load + infer (group) |
| `full_pipeline/json_load_infer_1000` | End-to-end JSON load + infer (group) |

Results and HTML reports are written under `target/criterion/`; open
`target/criterion/report/index.html` for comparisons.

## Dependency hygiene

The dependency list lives in `Cargo.toml` (`[dependencies]` / `[dev-dependencies]`). Check for
unused dependencies with:

```bash
cargo machete    # manual; requires: cargo install cargo-machete
```

Ask a maintainer before adding a new dependency — binary size is a first-class concern, and the
core stays deliberately dependency-light (no external data engine).

## Local hooks

Maintainers wire the verification commands into local Git hooks. Hooks only run when installed in a
clone, so do not assume they are active — run the commands in
[Pre-commit verification](#pre-commit-verification) manually if unsure.

## Continuous integration

`.github/workflows/ci.yml` runs on pushes and pull requests targeting `main` (ignoring `**/*.md`)
and on manual dispatch:

| Job | Steps |
|---|---|
| `test` (ubuntu, toolchain 1.97.0) | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` |
| `msrv` (ubuntu, toolchain 1.88.0) | `cargo +1.88.0 check --locked` |

There is no release job; releases are handled manually by the maintainers.

## Commit and pull request conventions

- Commit format: `<type>: <description>`. Allowed types: `feat`, `fix`, `refactor`, `docs`,
  `test`, `chore`, `perf`, `ci`.
- Keep commits logical and focused — do not mix unrelated concerns.
- Stage files explicitly (`git add <paths>`); do not blanket-add.
- Do **not** change the `version` in `Cargo.toml` except as part of a release.
- External contributors: fork the repository and open a pull request against `main`.
- Documentation is part of the change: update `README.md` for CLI changes,
  [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for structure, [docs/DESIGN.md](docs/DESIGN.md) for
  design decisions, and [docs/GOTCHAS.md](docs/GOTCHAS.md) for non-obvious pitfalls.
- New functionality requires tests: add or update both unit and integration tests.

Before opening a PR:

- [ ] `cargo fmt` applied
- [ ] `cargo clippy --all-targets -- -D warnings` is clean
- [ ] `cargo test` passes
- [ ] Tests added or updated for the change
- [ ] Relevant docs updated
