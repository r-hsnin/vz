# Runbook — Releasing vz

Release procedure, publication scope, and failure recovery. Single source of truth for the
release process. Dev setup and test commands are owned by [CONTRIBUTING.md](../CONTRIBUTING.md);
structure and design rationale by [ARCHITECTURE.md](ARCHITECTURE.md) and [DESIGN.md](DESIGN.md).

> **Freeze status:** this runbook describes the procedure assuming releases are enabled. Whether
> releases to `origin` are currently allowed is defined in maintainer-internal operational notes;
> do not read this document's existence as evidence that releases are active.

## 1. Repository model

| Remote | Repository | Visibility | Role |
|---|---|---|---|
| `dev` | `r-hsnin/vz-dev` | Private | Daily development; all commits land here |
| `origin` | `r-hsnin/vz` | Public | Release-only; receives tags and `release/*` branches |

Local `main` tracks `dev/main`. `origin` never receives normal development branches; code reaches
it only as a `release/<tag>` branch (a PR to `main`) plus the release tag. Push to `origin` is
guarded by a pre-push hook (§5).

## 2. Publication scope (`release-manifest.txt`)

`release-manifest.txt` is the single source of truth for what is published. **Default is private**:
any path not covered by the manifest is excluded.

- Directory entries end with `/` and allow every descendant.
- File entries are exact matches.
- Blank lines and lines starting with `#` are ignored.

**Changing the manifest is a publication-scope change; it must be explicit and approved, never
expanded silently.** Current allowlist:

```
.github/  .gitignore  benches/  CONTRIBUTING.md  Cargo.lock  Cargo.toml
LICENSE   README.md   demo/     docs/            fixtures/   rust-toolchain.toml
skills/   src/        tests/
```

Notably **not** published: `AGENTS.md`, `lefthook.yml`, `internal/`, `release-manifest.txt`,
`scripts/`, and `target/`. The public/private boundary and its enforcement are owned by
[PUBLICATION.md](PUBLICATION.md).

## 3. Preconditions

1. The version tag exists locally (`git rev-parse <tag>` succeeds).
2. `release-manifest.txt` exists (the script aborts otherwise).
3. `gh auth status` succeeds (the script aborts otherwise).
4. The `origin` remote is configured and reachable.
5. No `release/<tag>` branch already exists on `origin` (the script aborts if it does).
6. The tag should be an ancestor of `dev/main`, and `Cargo.toml` version at the tag should equal
   the tag without `v`. A mismatch warns and asks for interactive confirmation; `--dry-run` only
   warns.

## 4. Release procedure

Always run `--dry-run` first and review the printed allowed/excluded lists before writing anything
to `origin`.

1. On `dev`, update `version` in `Cargo.toml`, commit, tag, and push:

   ```bash
   git tag vX.Y.Z
   git push dev main --tags
   ```

2. Dry-run to inspect exactly what would be published:

   ```bash
   ./scripts/release.sh vX.Y.Z --dry-run
   ```

   The script filters the tag's file list (`git ls-tree -r --name-only <tag>`) through the manifest,
   warns about patterns matching no files, prints `Allowed`/`Excluded` lists, then exits without
   changes.

3. Execute the release:

   ```bash
   ./scripts/release.sh vX.Y.Z
   ```

   End-to-end, the script: re-runs the preconditions (§3) and the filter/print step; prompts
   `Proceed with release <tag>? [y/N]`; creates a temporary detached worktree at `origin/main`;
   checks out `release/<tag>` there; copies each allowed file from the tag, overwriting the
    `origin/main` version; removes tracked files not in the manifest; commits with the subject-only
    message `release: <tag>`; pushes `release/<tag>` to `origin`; and opens a PR to `main` via
    `gh pr create`, whose body lists the commits since the previous tag (or the last 20 when none
    exists).

4. **No-op path:** if staging produces no diff, the script prints `No changes to release.` and
   exits successfully without committing, pushing, or opening a PR. Expected when `origin/main`
   already matches the tag within manifest scope.

5. The script prints the remaining manual steps: review the PR, merge it, then push the tag with
   `git push origin vX.Y.Z`.

## 5. Origin guard

`lefthook.yml` runs `bash scripts/hooks/guard-origin-push.sh {1}` on pre-push with the remote name
and ref list on stdin. The guard passes through non-`origin` remotes, always allows tags
(`refs/tags/*`) and `release/*` branches, and blocks every other branch push to `origin` with a
message pointing at `scripts/release.sh`.

Do not bypass with `--no-verify` except in a genuine emergency. If hooks are not installed in a
clone the guard does not run locally, but the policy still applies: use the script.

## 6. Recovery

**Orphan `release/<tag>` branch** (push succeeded, then the script failed or the PR closed):

```bash
git push origin --delete release/vX.Y.Z
./scripts/release.sh vX.Y.Z
```

The pre-check aborts when the branch exists, so delete it first. For a failed or partial PR, close
the PR, delete the remote branch as above, then re-run; re-running recreates the branch from
`origin/main` and re-filters the tag.

**Detached worktree:** created under a temp directory and removed on exit via a trap. If the
process is killed hard, inspect and clean up a stale entry:

```bash
git worktree list
git worktree remove --force <path>
```

## 7. Post-release verification

1. CI (`.github/workflows/ci.yml`) is green on the PR: fmt check, clippy with `-D warnings`, tests,
   and the MSRV (`1.88.0`) job.
2. Version/tag consistency: merged `origin/main` `Cargo.toml` version equals the tag without `v`.
3. Manifest scope review: the merged file set matches the allowlist and nothing outside leaked.

## 8. Local install and candidate verification

Build, install, and quality-gate commands (fmt, clippy, tests, snapshots, benchmarks) are owned by
[CONTRIBUTING.md](../CONTRIBUTING.md); use them there. For a release candidate, additionally
exercise the installed binary against the tag's fixtures before accepting the PR — this is a
release-specific smoke check, not a substitute for the CONTRIBUTING build and test commands.
