# Runbook — vz

Operational procedures for developing, releasing, and troubleshooting vz.

## Build & Release

### Local install

```bash
cargo install --path .
```

This installs the `vz` binary to `~/.cargo/bin/`.

### Release build

```bash
cargo build --release
# Binary at: target/release/vz
```

### Version bump

1. Update `version` in `Cargo.toml`
2. Commit: `git commit -am "chore: bump version to X.Y.Z"`
3. Tag: `git tag vX.Y.Z`
4. Push: `git push && git push --tags`

### Publish to crates.io (future)

```bash
cargo publish --dry-run   # Verify
cargo publish             # Publish
```

## Quality Checks

Run before every commit:

```bash
cargo fmt                                    # Format
cargo clippy --all-targets -- -D warnings    # Lint (zero warnings)
cargo test                                   # All tests pass
```

### Expected test results

- ~244 unit tests (in-source `#[cfg(test)]` modules)
- ~49 integration tests (`tests/integration_test.rs`)
- ~4 snapshot tests (`tests/snapshot_test.rs`)
- Total runtime: < 2 seconds

## Troubleshooting

### Build failures

| Symptom | Cause | Fix |
|---------|-------|-----|
| `error[E0658]: let chains` | Rust version too old | Update: `rustup update` (requires 1.70+) |
| `crossterm` compile error | Missing system deps | Linux: ensure `libxcb` or similar available |
| `ratatui` version mismatch | Lockfile stale | `cargo update` |

### Runtime issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| `No input file specified` | Missing positional arg | Provide file or pipe: `vz data.csv` or `cat f.csv \| vz -` |
| `Could not determine chart type` | Invalid column hint | Check `-x`/`-y` column names match CSV headers |
| `Failed to read file` | File not found or permission | Verify path and permissions |
| Chart renders garbled | Terminal doesn't support Unicode | Try a terminal with Braille/Unicode support (iTerm2, kitty, WezTerm) |
| Explore/Present panics | No TTY available | These modes require an interactive terminal; use one-shot mode in CI/pipes |
| Bar chart shows no labels | Terminal too narrow | Widen terminal to ≥ 40 columns |

### TSV not detected

If a TSV file isn't auto-detected:
- Ensure the file extension is `.tsv` or `.tab`, **or**
- Ensure the header line contains more tab characters than commas, **or**
- Use the `--format tsv` (or `-f tsv`) flag to force TSV parsing

### Present mode chart not loading

Chart source paths resolve relative to the Markdown file's directory. If charts don't render:
1. Ensure the `source:` path in the chart block is relative to where the `.md` file lives
2. As a fallback, the tool also tries the current working directory

## Performance Notes

- vz processes data in-memory; files up to ~1GB are fine
- Type inference samples the first 100 rows
- No streaming mode yet; the entire file is loaded before rendering

## Dependencies

| Dependency | Version | Purpose |
|-----------|---------|---------|
| clap | 4 | CLI argument parsing |
| ratatui | 0.30 | Terminal UI rendering |
| crossterm | 0.28 | Terminal manipulation |
| csv | 1 | CSV/TSV parsing |
| serde_json | 1 | JSON/NDJSON parsing |
| chrono | 0.4 | Date parsing (type inference) |
| regex | 1 | Pattern matching (type inference) |
| anyhow | 1 | Error handling |
