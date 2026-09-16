# Runbook — vz

Operational procedures for releasing vz.
Known failure modes live in [GOTCHAS.md](GOTCHAS.md).

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

### Release workflow (dual-repo)

This project uses a dual-repo model:
- **dev** (`r-hsnin/vz-dev`): private, daily development
- **origin** (`r-hsnin/vz`): public, release-only

Releases go through `scripts/release.sh` which filters files via `release-manifest.txt` and creates a PR on the public repo. Direct push to origin/main is blocked.

```bash
# 1. Version bump & commit (on dev)
# Update `version` in Cargo.toml
git commit -am "chore: bump version to X.Y.Z"

# 2. Tag
git tag vX.Y.Z

# 3. Push tag to dev
git push dev main --tags

# 4. Dry-run to verify what gets published
./scripts/release.sh vX.Y.Z --dry-run

# 5. Execute (creates PR on public repo)
./scripts/release.sh vX.Y.Z

# 6. Review & merge the PR on GitHub
# 7. After merge, tag the public repo:
#    git push origin vX.Y.Z
```

#### Key files

| File | Purpose |
|------|---------|
| `release-manifest.txt` | Allowlist of files published to origin (default=private) |
| `scripts/release.sh` | Release script (tag → filter → PR) |
| lefthook pre-push | Blocks direct push to origin (install with `lefthook install`) |

#### Recovery

If the script fails mid-way (e.g., push succeeded but PR creation failed):
```bash
# Delete the orphan remote branch
git push origin --delete release/vX.Y.Z
# Fix the issue (e.g. gh auth login), then re-run
./scripts/release.sh vX.Y.Z
```

### Publish to crates.io (future)

```bash
cargo publish --dry-run   # Verify
cargo publish             # Publish
```

## Quality Checks

Run the commands in [CONTRIBUTING.md](../CONTRIBUTING.md#available-commands) before every commit.
