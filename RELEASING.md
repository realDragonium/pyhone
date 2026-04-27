# Releasing

Releases are built and published by `.github/workflows/release.yml`, which is
triggered **only by pushing a tag matching `v*`**. Bumping the version in the
manifest files alone does nothing — the tag push is what fires the workflow.

## Checklist

Replace `X.Y.Z` with the new version (no `v` prefix in the manifests; `v` prefix
on the git tag).

1. **Bump the version in three places — all must match.**
   - `Cargo.toml` → `[package].version`
   - `pyproject.toml` → `[project].version`
   - `Cargo.lock` → the `pyhone` package entry (run `cargo build` and it will
     update on its own; don't hand-edit)

2. **Sanity check locally.**
   ```bash
   cargo build --release
   cargo test
   ```

3. **Commit the bump.**
   ```bash
   git add Cargo.toml pyproject.toml Cargo.lock
   git commit -m "Bump version to X.Y.Z"
   git push origin main
   ```

4. **Tag and push the tag.** This is the step that triggers the release.
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. **Verify the workflow.**
   - Watch `Release` run at https://github.com/realDragonium/pyhone/actions
   - Confirm wheels appear on the GitHub Release page
   - Confirm the new version is live on PyPI: https://pypi.org/project/pyhone/

## What the workflow produces

- **Linux** wheels for `x86_64` and `aarch64` (manylinux)
- **Windows** wheel for `x64`
- **macOS** wheels for `x86_64` and `aarch64`
- **sdist** (source distribution)
- **GitHub Release** with auto-generated notes and all wheels attached
- **PyPI publish** via trusted publishing (no API token in repo)

## If something goes wrong

- **Tag pushed but workflow didn't run** → check the tag matches `v*` exactly
  (lowercase `v`, then the version).
- **Workflow ran but PyPI step failed** → it uses `--skip-existing`, so re-running
  the workflow is safe. Trusted publishing must be configured on PyPI for the
  `realDragonium/pyhone` repo + `Publish to PyPI` job.
- **Tag points to the wrong commit** → delete locally and on origin, retag,
  push again. Be aware this is only safe if the previous tag did not produce
  a published release yet.
  ```bash
  git tag -d vX.Y.Z
  git push origin :refs/tags/vX.Y.Z
  git tag vX.Y.Z <correct-sha>
  git push origin vX.Y.Z
  ```
- **Forgot to tag after bumping** (the 0.1.5 case) → just tag the existing bump
  commit and push the tag. No need to re-bump.

## Common mistake

Bumping the version, committing, pushing — and then forgetting to tag. The
manifests will be on the new version but no release is built. The fix is just
to create and push the tag against the existing bump commit.
