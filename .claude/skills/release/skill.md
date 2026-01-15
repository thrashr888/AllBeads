---
name: release
description: Release a new version of AllBeads. Use when the user wants to ship, release, publish, or tag a new version. Handles version bumping, quality checks, tagging, pushing, and homebrew tap updates.
allowed-tools: Read, Edit, Bash, Grep, Glob, WebFetch
---

# AllBeads Release Skill

Automate the AllBeads release process including quality checks, version bumping, tagging, and homebrew tap updates.

## Pre-Release Checklist

Before releasing, verify:

1. **Quality gates pass** (ALL THREE are mandatory - never skip tests!):
   ```bash
   cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
   ```

2. **All blockers closed**: Check the release epic in beads
   ```bash
   bd show <epic-id>
   ```

3. **Changes committed**: `git status` shows clean working tree

## Release Process

### Step 1: Determine Version

Ask the user what version to release if not specified. Check current version:
```bash
grep '^version' Cargo.toml
```

Version format: `MAJOR.MINOR.PATCH` (e.g., 0.7.0)

### Step 2: Update Version

Edit `Cargo.toml` to update the version:
```toml
version = "X.Y.Z"
```

### Step 3: Commit Version Bump

```bash
git add Cargo.toml Cargo.lock
git commit -m "Bump version to X.Y.Z"
```

### Step 4: Create Annotated Tag

Create a tag with release notes summarizing the changes:

```bash
git tag -a vX.Y.Z -m "AllBeads X.Y.Z Release

## Highlights
- Key feature 1
- Key feature 2

## Changes
- Change 1
- Change 2

## Bug Fixes
- Fix 1
- Fix 2"
```

### Step 5: Push to GitHub

```bash
git push && git push --tags
```

### Step 6: Monitor Release Build

Watch the GitHub Actions release workflow:
```bash
gh run list --limit 3
gh run watch <run-id>
```

Wait for the Release workflow to complete successfully.

### Step 6b: Update Release Title and Notes

The GitHub Actions workflow creates a release but may not populate the title/body from the tag. Update it:

**Title format**: `vX.Y.Z - Feature 1, Feature 2, Feature 3`

Examples:
- `v0.5.0 - Aiki Integration, Governance, TUI Contexts`
- `v0.6.0 - Context Creation, Claude Plugin Support`

```bash
gh release edit vX.Y.Z --title "vX.Y.Z - Feature 1, Feature 2" --notes "$(cat <<'EOF'
## Highlights
- Key feature 1
- Key feature 2

## Changes
- Change 1
- Change 2

## Bug Fixes
- Fix 1
- Fix 2
EOF
)"
```

### Step 7: Close Release Epic

```bash
bd close <epic-id> --reason="Released vX.Y.Z"
bd sync
```

### Step 8: Update Homebrew Tap

After the release build completes, update the homebrew tap at `../homebrew-allbeads`:

1. Get SHA256 hash for the macOS ARM binary:
   ```bash
   curl -sL https://github.com/thrashr888/AllBeads/releases/download/vX.Y.Z/allbeads-macos-aarch64 | shasum -a 256
   ```

2. Update `../homebrew-allbeads/Formula/allbeads.rb`:
   - Update `version "X.Y.Z"`
   - Update SHA256 hash for aarch64
   - Update download URL with new version

3. Commit and push the homebrew tap:
   ```bash
   cd ../homebrew-allbeads
   git add Formula/allbeads.rb
   git commit -m "Update allbeads to vX.Y.Z"
   git push
   ```

## Post-Release

- Verify installation works: `brew upgrade allbeads` or `brew install thrashr888/allbeads/allbeads`
- Announce release if needed
- Start next development cycle

## Troubleshooting

### Workflow job stuck or cancelled
GitHub Actions runners (especially macOS) can be unreliable. If a job is stuck in "queued" or immediately cancelled:

1. Cancel the stuck workflow: `gh run cancel <run-id>`
2. Delete and recreate the tag to trigger a fresh run:
   ```bash
   git push origin :refs/tags/vX.Y.Z  # Delete remote tag
   git tag -d vX.Y.Z                   # Delete local tag
   git tag -a vX.Y.Z -m "Release"      # Recreate tag
   git push origin vX.Y.Z              # Push new tag
   ```

### Platform support
- **Supported**: Linux x86_64/aarch64, macOS ARM (aarch64), Windows x86_64
- **Note**: macOS Intel (x86_64) runners can be unreliable

### Test workflow changes safely
Before modifying `.github/workflows/release.yml` for a real release, test changes on a separate branch or with a test tag (e.g., `v0.0.0-test`).

## Rollback

If something goes wrong:

```bash
# Delete local tag
git tag -d vX.Y.Z

# Delete remote tag
git push origin :refs/tags/vX.Y.Z

# Revert commits if needed
git revert HEAD
```
